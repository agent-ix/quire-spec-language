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
use quire_semantic_value::declaration::TypeEnvironment;

/// Why the backend stopped a run.
pub(crate) enum Halt {
    /// A kernel stop, at the node that made the call.
    Stop(Stop, ExpressionLocation),
    /// A backend invariant broke.
    Fault(&'static str),
}

/// FR-561: the fixed set of kernel entry points generated code may call.
/// This backend's table has one entry, the shim, which dispatches on the
/// operation table by index.
#[derive(Clone, Copy)]
pub(crate) struct KernelTable {
    shim: extern "C" fn(*mut c_void, i64, i64, i64) -> i64,
}

impl KernelTable {
    /// The shim this table lists.
    pub(crate) fn shim(self) -> extern "C" fn(*mut c_void, i64, i64, i64) -> i64 {
        self.shim
    }
}

/// FR-568: the frame generated code runs over. Generated code reads
/// `failed`, `arguments` and `result` at their `repr(C)` offsets (0, 8, 16);
/// every other field is read only by `exec`.
#[repr(C)]
pub(crate) struct EntryFrame<'a> {
    failed: u64,
    arguments: *const i64,
    result: i64,
    kernel: KernelTable,
    meter: &'a mut Meter,
    ops: &'a [Op],
    types: &'a TypeEnvironment,
    halt: Option<Halt>,
}

impl<'a> EntryFrame<'a> {
    /// A frame with nothing failed, metering into `meter`.
    pub(crate) fn new(meter: &'a mut Meter, ops: &'a [Op], types: &'a TypeEnvironment) -> Self {
        Self {
            failed: 0,
            arguments: std::ptr::null(),
            result: 0,
            kernel: KernelTable { shim },
            meter,
            ops,
            types,
            halt: None,
        }
    }

    /// Why the run stopped, if it did.
    pub(crate) fn into_halt(self) -> Option<Halt> {
        self.halt
    }

    /// The value the entry returned.
    pub(crate) fn result(&self) -> i64 {
        self.result
    }
}

/// The kernel shim every generated operation calls: `(frame, op index,
/// left, right) -> value`. On a stop it records the stop and the node's
/// location in the frame, sets `failed`, and returns 0; the generated code
/// then returns without another call.
extern "C" fn shim(frame: *mut c_void, node: i64, a: i64, b: i64) -> i64 {
    // SAFETY: `frame` is the `&mut EntryFrame` that `Entry::call` passed to
    // the generated function, which passes it back unchanged to every shim
    // call; the frame outlives the call and nothing else aliases it while the
    // generated function runs.
    let frame = unsafe { &mut *frame.cast::<EntryFrame<'_>>() };
    let Some(op) = usize::try_from(node)
        .ok()
        .and_then(|index| frame.ops.get(index))
    else {
        frame.halt = Some(Halt::Fault("jit-shim-operation-index"));
        frame.failed = 1;
        return 0;
    };
    match kernel::run(op, a, b, frame.meter, frame.types) {
        Ok(value) => value,
        Err(KernelHalt::Stop(stop)) => {
            frame.halt = Some(Halt::Stop(stop, op.location.clone()));
            frame.failed = 1;
            0
        }
        Err(KernelHalt::Fault(invariant)) => {
            frame.halt = Some(Halt::Fault(invariant));
            frame.failed = 1;
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
        builder.symbol("qsl_jit_shim", KernelTable { shim }.shim() as *const u8);
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
    /// arguments of 64 bits, with the signature the translator recorded for it.
    pub(crate) fn entry(&mut self, id: FuncId, arity: usize) -> Entry {
        let address = self.module().get_finalized_function(id);
        Entry {
            address,
            arity,
            signature: ENTRY_SIGNATURE,
        }
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

/// The signature of an entry as recorded at generation: parameter count and
/// whether the result is a `u32`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Signature {
    parameters: u8,
    returns_u32: bool,
}

/// The one entry signature, `extern "C" fn(*const EntryFrame) -> u32`.
pub(crate) const ENTRY_SIGNATURE: Signature = Signature {
    parameters: 1,
    returns_u32: true,
};

impl Signature {
    /// A signature with `parameters` parameters returning a `u32` or not.
    pub(crate) const fn new(parameters: u8, returns_u32: bool) -> Self {
        Self {
            parameters,
            returns_u32,
        }
    }
}

/// How an entry stopped, in the closed status type.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Status {
    /// The body ran to its result.
    Completed,
    /// A kernel call recorded a stop in the frame.
    Stopped,
}

/// Why `PreparedJit` refused to enter or accept an entry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum EntryRefusal {
    /// The recorded signature is not the entry signature; nothing was called.
    Signature,
    /// The function returned a status outside the closed status type.
    Status,
    /// Fewer arguments than the function reads.
    Arity,
}

/// A finalized generated function `(frame) -> status`.
#[derive(Clone, Copy)]
pub(crate) struct Entry {
    address: *const u8,
    arity: usize,
    signature: Signature,
}

extern "C" fn out_of_range_status(_frame: *const c_void) -> u32 {
    u32::MAX
}

impl Entry {
    /// The same entry with `signature` recorded in place of the real one
    /// (a test hook for FR-568-AC-3).
    pub(crate) fn with_recorded_signature(self, signature: Signature) -> Self {
        Self { signature, ..self }
    }

    /// An entry that returns a status outside the closed status type
    /// (a test hook for FR-568-AC-3).
    pub(crate) fn returning_out_of_range_status(self) -> Self {
        Self {
            address: out_of_range_status as *const u8,
            ..self
        }
    }

    /// Call the function over `frame` with `arguments`, one 64-bit slot per
    /// parameter.
    pub(crate) fn call(
        self,
        frame: &mut EntryFrame<'_>,
        arguments: &[i64],
    ) -> Result<Status, EntryRefusal> {
        if self.signature != ENTRY_SIGNATURE {
            return Err(EntryRefusal::Signature);
        }
        if arguments.len() < self.arity {
            return Err(EntryRefusal::Arity);
        }
        frame.arguments = arguments.as_ptr();
        // SAFETY: `address` is the finalized code of a function the
        // translator built with exactly the entry signature (checked above
        // against the signature recorded at generation): one pointer
        // parameter and a `u32` result. It reads at most `arity` slots of
        // `arguments`, checked above; `frame` is valid and unaliased for the
        // call; the module that owns the code is alive (see `Drop for
        // Machine`).
        let function: extern "C" fn(*const c_void) -> u32 =
            unsafe { std::mem::transmute(self.address) };
        let status = function(std::ptr::from_mut(frame).cast_const().cast());
        match status {
            0 => Ok(Status::Completed),
            1 => Ok(Status::Stopped),
            _ => Err(EntryRefusal::Status),
        }
    }
}
