//! QSL-602 step 4a throwaway spike: Cranelift cost/gain for a clause fast path.
//! Clause: Arith = `(v*3+7) mod 11 >= 0 and v+1 > v`, v: Int[0, 1_000_000].
use cranelift_codegen::ir::{types, AbiParam, InstBuilder};
use cranelift_codegen::settings::{self, Configurable};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module};
use quire_exact::*;
use std::time::Instant;

struct Ctx {
    meter: Meter,
    fault: bool,
}
fn limits() -> ScalarLimits {
    ScalarLimits { integer_bits: u64::MAX, decimal_digits: u64::MAX, scale_expansion: u64::MAX,
        text_input_bytes: u64::MAX, text_scalars: u64::MAX, normalized_scalars: u64::MAX,
        unit_edges: u64::MAX, value_occurrences: u64::MAX, work_units: u64::MAX, result_units: u64::MAX }
}
fn iv(lo: i128, hi: i128) -> NativeIntegerInterval { NativeIntegerInterval::new(lo, hi).unwrap() }

fn arith(ctx: &mut Ctx, op: u8, a: i64, b: i64) -> i64 {
    let (l, r) = (iv(0, 4_000_000), iv(0, 4_000_000));
    let operation = match op {
        0 => NativeIntegerOperation::Add { left: l, right: r },
        _ => NativeIntegerOperation::Multiply { left: iv(0, 4_000_000), right: iv(0, 4_000_000) },
    };
    let plan = NativeIntegerPlan::new(operation, None).unwrap();
    let adm = plan.admit(NativeIntegerOperands::Binary { left: NativeInteger::new(a as i128), right: NativeInteger::new(b as i128) }).unwrap();
    match evaluate_native_integer_arithmetic(adm, &mut ctx.meter) {
        Outcome::Completed(v) => v.value() as i64,
        _ => { ctx.fault = true; 0 }
    }
}
extern "C" fn k_add(c: *mut Ctx, a: i64, b: i64) -> i64 { arith(unsafe { &mut *c }, 0, a, b) }
extern "C" fn k_mul(c: *mut Ctx, a: i64, b: i64) -> i64 { arith(unsafe { &mut *c }, 1, a, b) }
extern "C" fn k_mod(c: *mut Ctx, a: i64, b: i64) -> i64 {
    let c = unsafe { &mut *c };
    match modulo(&Integer::from(a), &Integer::from(b), &IntegerDomain::Mathematical, &mut c.meter) {
        Outcome::Completed(v) => v.to_u64().unwrap_or(0) as i64,
        _ => { c.fault = true; 0 }
    }
}
fn ord(c: &mut Ctx, op: OrderingOperator, a: i64, b: i64) -> i64 {
    match evaluate_native_integer_ordering(op, NativeInteger::new(a as i128), NativeInteger::new(b as i128), &mut c.meter) {
        Outcome::Completed(v) => v as i64,
        _ => { c.fault = true; 0 }
    }
}
extern "C" fn k_ge(c: *mut Ctx, a: i64, b: i64) -> i64 { ord(unsafe { &mut *c }, OrderingOperator::GreaterOrEqual, a, b) }
extern "C" fn k_gt(c: *mut Ctx, a: i64, b: i64) -> i64 { ord(unsafe { &mut *c }, OrderingOperator::Greater, a, b) }

/// Same kernel-call sequence the compiled code makes, called from Rust (no JIT).
fn reference(c: &mut Ctx, v: i64) -> i64 {
    let t1 = k_mul(c, v, 3);
    let t2 = k_add(c, t1, 7);
    let t3 = k_mod(c, t2, 11);
    let a = k_ge(c, t3, 0);
    let t4 = k_add(c, v, 1);
    let b = k_gt(c, t4, v);
    a & b
}
/// Upper bound: inline machine arithmetic, no charges (not FR-562 compliant).
fn inline_nocharge(v: i64) -> i64 { (((v * 3 + 7) % 11 >= 0) & (v + 1 > v)) as i64 }

fn main() {
    // --- compile
    let mut flags = settings::builder();
    flags.set("opt_level", "speed").unwrap();
    flags.set("is_pic", "false").unwrap();
    let isa = cranelift_native::builder().unwrap().finish(settings::Flags::new(flags)).unwrap();
    let mut jb = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());
    for (n, f) in [("k_add", k_add as usize), ("k_mul", k_mul as usize), ("k_mod", k_mod as usize), ("k_ge", k_ge as usize), ("k_gt", k_gt as usize)] {
        jb.symbol(n, f as *const u8);
    }
    let t0 = Instant::now();
    let mut m = JITModule::new(jb);
    let ptr = m.target_config().pointer_type();
    let mut ksig = m.make_signature();
    for t in [ptr, types::I64, types::I64] { ksig.params.push(AbiParam::new(t)); }
    ksig.returns.push(AbiParam::new(types::I64));
    let mut ids = std::collections::HashMap::new();
    for n in ["k_add", "k_mul", "k_mod", "k_ge", "k_gt"] {
        ids.insert(n, m.declare_function(n, Linkage::Import, &ksig).unwrap());
    }
    let mut sig = m.make_signature();
    sig.params.push(AbiParam::new(ptr)); sig.params.push(AbiParam::new(types::I64));
    sig.returns.push(AbiParam::new(types::I64));
    let fid = m.declare_function("arith", Linkage::Export, &sig).unwrap();
    let mut ctx = m.make_context();
    ctx.func.signature = sig;
    let mut fb_ctx = FunctionBuilderContext::new();
    {
        let mut b = FunctionBuilder::new(&mut ctx.func, &mut fb_ctx);
        let blk = b.create_block();
        b.append_block_params_for_function_params(blk);
        b.switch_to_block(blk);
        let (c, v) = (b.block_params(blk)[0], b.block_params(blk)[1]);
        let mut call = |b: &mut FunctionBuilder, n: &str, x, y| {
            let fr = m.declare_func_in_func(ids[n], b.func);
            let i = b.ins().call(fr, &[c, x, y]);
            b.inst_results(i)[0]
        };
        let three = b.ins().iconst(types::I64, 3);
        let seven = b.ins().iconst(types::I64, 7);
        let eleven = b.ins().iconst(types::I64, 11);
        let zero = b.ins().iconst(types::I64, 0);
        let one = b.ins().iconst(types::I64, 1);
        let t1 = call(&mut b, "k_mul", v, three);
        let t2 = call(&mut b, "k_add", t1, seven);
        let t3 = call(&mut b, "k_mod", t2, eleven);
        let a = call(&mut b, "k_ge", t3, zero);
        let t4 = call(&mut b, "k_add", v, one);
        let bb = call(&mut b, "k_gt", t4, v);
        let r = b.ins().band(a, bb);
        b.ins().return_(&[r]);
        b.seal_all_blocks();
        b.finalize(m.target_config());
    }
    m.define_function(fid, &mut ctx).unwrap();
    let code_bytes = ctx.compiled_code().map(|c| c.code_buffer().len()).unwrap_or(0);
    let h = ctx.compiled_code().unwrap().code_buffer().iter().fold(0xcbf29ce484222325u64, |a, b| (a ^ *b as u64).wrapping_mul(0x100000001b3));
    println!("code_hash={h:016x}");
    m.clear_context(&mut ctx);
    m.finalize_definitions().unwrap();
    let compile_us = t0.elapsed().as_micros();
    let f: extern "C" fn(*mut Ctx, i64) -> i64 = unsafe { std::mem::transmute(m.get_finalized_function(fid)) };
    println!("compile_us(incl. module+isa setup)={compile_us} code_bytes={code_bytes}");
    // second-compile (steady) timing
    let t = Instant::now();
    for _ in 0..50 {
        let mut m2 = {
            let mut fl = settings::builder(); fl.set("opt_level","speed").unwrap(); fl.set("is_pic","false").unwrap();
            let isa = cranelift_native::builder().unwrap().finish(settings::Flags::new(fl)).unwrap();
            JITModule::new(JITBuilder::with_isa(isa, cranelift_module::default_libcall_names()))
        };
        let _ = &mut m2;
    }
    println!("module+isa setup_us={}", t.elapsed().as_micros() / 50);

    // --- run
    let n: i64 = 2_000_000;
    let mut c = Ctx { meter: Meter::new(limits()), fault: false };
    let mut chk = 0;
    let t = Instant::now();
    for v in 0..n { chk += reference(&mut c, v % 1_000_000); }
    let ref_ns = t.elapsed().as_nanos() as f64 / n as f64;
    let ref_charges = c.meter.admission_count() / n as u64;
    let mut c2 = Ctx { meter: Meter::new(limits()), fault: false };
    let mut chk2 = 0;
    let t = Instant::now();
    for v in 0..n { chk2 += f(&mut c2, v % 1_000_000); }
    let jit_ns = t.elapsed().as_nanos() as f64 / n as f64;
    let mut chk3 = 0;
    let t = Instant::now();
    for v in 0..n { chk3 += std::hint::black_box(inline_nocharge(std::hint::black_box(v % 1_000_000))); }
    let inl_ns = t.elapsed().as_nanos() as f64 / n as f64;
    per_op();
    println!("kernel-sequence-from-Rust ns/eval={ref_ns:.1} jit(kernel shims) ns/eval={jit_ns:.1} inline-nocharge ns/eval={inl_ns:.2} charges/eval={ref_charges} fault={} {} chk={chk}/{chk2}/{chk3}", c.fault, c2.fault);
}

#[allow(dead_code)]
pub fn per_op() {
    let mut c = Ctx { meter: Meter::new(limits()), fault: false };
    let n = 1_000_000i64;
    for (name, f) in [("mul", k_mul as extern "C" fn(*mut Ctx, i64, i64) -> i64), ("add", k_add), ("mod", k_mod), ("ge", k_ge)] {
        let t = Instant::now();
        let mut s = 0;
        for v in 0..n { s += f(&mut c, v % 1000 + 1, 11); }
        println!("op {name}: {:.1} ns (s={s})", t.elapsed().as_nanos() as f64 / n as f64);
    }
}
