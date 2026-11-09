// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-560 to FR-568, ADR-029 EB-1, ADR-035: `qsl-jit`, the Cranelift JIT
//! backend behind the [`qsl_eval::execution::ExecutionBackend`] seam.
//!
//! The backend translates the checked representation of a function
//! (`qsl_semantics::check::CheckedNode`) and nothing else: a [`Jit`] prepares
//! entries of a `CheckedPackage`. An entry with a construct outside
//! [`coverage`] is listed unsupported with FR-294's `Untranslatable` cause
//! and the interpreter runs it; the JIT never runs it and `execute` never
//! falls back (FR-296).
//!
//! Generated code reaches the kernel only through the shims of [`exec`]
//! (FR-561), each of which runs the same `quire-exact` operation the
//! interpreter runs, with the same charges at the same points (FR-562).
//! `unsafe` code is confined to the [`exec`] module (FR-568): the crate
//! denies it everywhere else.

mod coverage;
#[allow(unsafe_code)]
mod exec;
mod kernel;
mod prepared;
mod translate;

pub use coverage::coverage;
pub use prepared::{CallableEntry, Jit, PreparedJit};
