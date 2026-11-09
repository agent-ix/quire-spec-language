// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-568 (ADR-035): the one module of `qsl-jit` that holds `unsafe` code.
//!
//! Three things need it, and nothing else in the crate does:
//!
//! 1. the `extern "C"` kernel shim, which turns the context pointer generated
//!    code passes into a reference;
//! 2. the cast of a finalized function's address to a callable entry, and the
//!    call through it;
//! 3. releasing the executable memory of a module on drop.
//!
//! The translator ([`crate::translate`]) and every kernel operation
//! ([`crate::kernel`]) are safe code. Generated code reads `Ctx::failed`, the
//! first field, and leaves the function when a shim set it.

#![allow(unsafe_code)]

use std::ffi::c_void;

use cranelift_codegen::ir::{types, AbiParam};
use cranelift_codegen::settings::{self, Configurable};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{FuncId, Linkage, Module};
use quire_exact::Meter;
use quire_semantic_value::location::ExpressionLocation;
use quire_semantic_value::stop::Stop;

use crate::kernel::{self, Halt as KernelHalt};
use crate::translate::Op;

/// Why the backend stopped a run.
pub(crate) enum Halt {
    /// A kernel stop, at the node that made the call.
    Stop(Stop, ExpressionLocation),
    /// A backend invariant broke.
    Fault(&'static str),
}

/// The state generated code and its shim share for one call. `failed` is
/// first and `repr(C)` so generated code reads it at offset 0.
#[repr(C)]
pub(crate) struct Ctx<'a> {
    failed: u64,
    meter: &'a mut Meter,
    ops: &'a [Op],
    halt: Option<Halt>,
}

impl<'a> Ctx<'a> {
    /// A context with nothing failed, metering into `meter`.
    pub(crate) fn new(meter: &'a mut Meter, ops: &'a [Op]) -> Self {
        Self {
            failed: 0,
            meter,
            ops,
            halt: None,
        }
    }

    /// Why the run stopped, if it did.
    pub(crate) fn into_halt(self) -> Option<Halt> {
        self.halt
    }
}

/// The kernel shim every generated operation calls: `(ctx, op index,
/// left, right) -> value`. On a stop it records the stop and the node's
/// location in the context, sets `failed`, and returns 0; the generated code
/// then returns without another call.
extern "C" fn shim(ctx: *mut c_void, node: i64, a: i64, b: i64) -> i64 {
    // SAFETY: `ctx` is the `&mut Ctx` that `Entry::call` passed to the
    // generated function, which passes it back unchanged to every shim call;
    // the context outlives the call and nothing else aliases it while the
    // generated function runs.
    let ctx = unsafe { &mut *ctx.cast::<Ctx<'_>>() };
    let Some(op) = usize::try_from(node)
        .ok()
        .and_then(|index| ctx.ops.get(index))
    else {
        ctx.halt = Some(Halt::Fault("jit-shim-operation-index"));
        ctx.failed = 1;
        return 0;
    };
    match kernel::run(op, a, b, ctx.meter) {
        Ok(value) => value,
        Err(KernelHalt::Stop(stop)) => {
            ctx.halt = Some(Halt::Stop(stop, op.location.clone()));
            ctx.failed = 1;
            0
        }
        Err(KernelHalt::Fault(invariant)) => {
            ctx.halt = Some(Halt::Fault(invariant));
            ctx.failed = 1;
            0
        }
    }
}

/// The declared kernel shim of a module.
pub(crate) struct Shim(FuncId);

impl Shim {
    pub(crate) fn id(&self) -> FuncId {
        self.0
    }
}

/// A module of generated functions and the executable memory behind them.
pub(crate) struct Machine {
    module: Option<JITModule>,
}

/// The host refused to build a code generator (an unsupported architecture or
/// a refused executable mapping).
pub(crate) struct Refused;

impl Machine {
    /// A module with the kernel shim declared, built for the host. The
    /// generated code is position-dependent only through the shim's address,
    /// and Cranelift reads no ambient clock, so equal inputs give equal code.
    pub(crate) fn new() -> Result<(Self, Shim), Refused> {
        let mut flags = settings::builder();
        flags.set("opt_level", "speed").map_err(|_| Refused)?;
        flags.set("is_pic", "false").map_err(|_| Refused)?;
        let isa = cranelift_native::builder()
            .map_err(|_| Refused)?
            .finish(settings::Flags::new(flags))
            .map_err(|_| Refused)?;
        let mut builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());
        builder.symbol("qsl_jit_shim", shim as *const u8);
        let mut module = JITModule::new(builder);
        let pointer = module.target_config().pointer_type();
        let mut signature = module.make_signature();
        for ty in [pointer, types::I64, types::I64, types::I64] {
            signature.params.push(AbiParam::new(ty));
        }
        signature.returns.push(AbiParam::new(types::I64));
        let id = module
            .declare_function("qsl_jit_shim", Linkage::Import, &signature)
            .map_err(|_| Refused)?;
        Ok((
            Self {
                module: Some(module),
            },
            Shim(id),
        ))
    }

    /// The module generated functions are defined in.
    pub(crate) fn module(&mut self) -> &mut JITModule {
        self.module
            .as_mut()
            .unwrap_or_else(|| unreachable!("a module is present until drop"))
    }

    /// Make every defined function callable.
    pub(crate) fn finalize(&mut self) -> Result<(), Refused> {
        self.module().finalize_definitions().map_err(|_| Refused)
    }

    /// The callable entry of the finalized function `id`, taking `arity`
    /// arguments of 64 bits.
    pub(crate) fn entry(&mut self, id: FuncId, arity: usize) -> Entry {
        let address = self.module().get_finalized_function(id);
        Entry { address, arity }
    }
}

impl Drop for Machine {
    fn drop(&mut self) {
        if let Some(module) = self.module.take() {
            // SAFETY: no entry of this module is called after the machine is
            // dropped: `Entry` is stored beside the machine in one prepared
            // value, and the machine is its last field.
            unsafe { module.free_memory() };
        }
    }
}

/// A finalized generated function `(ctx, arguments) -> value`.
#[derive(Clone, Copy)]
pub(crate) struct Entry {
    address: *const u8,
    arity: usize,
}

impl Entry {
    /// Call the function with `arguments`, one 64-bit slot per parameter.
    /// `None` when `arguments` is shorter than the function reads.
    pub(crate) fn call(self, ctx: &mut Ctx<'_>, arguments: &[i64]) -> Option<i64> {
        if arguments.len() < self.arity {
            return None;
        }
        // SAFETY: `address` is the finalized code of a function the
        // translator built with exactly this signature (a context pointer and
        // an argument pointer, returning an `i64`); it reads at most `arity`
        // slots of `arguments`, checked above; `ctx` is valid and unaliased
        // for the call; the module that owns the code is alive (see `Drop for
        // Machine`).
        let function: extern "C" fn(*mut c_void, *const i64) -> i64 =
            unsafe { std::mem::transmute(self.address) };
        Some(function(std::ptr::from_mut(ctx).cast(), arguments.as_ptr()))
    }
}
