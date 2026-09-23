// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-011 §6.1 layer 5, S6a (QSL-183, ADR-011 §7.3 X-8): the evaluator
//! `value::expression` -- `CheckedPackage::call` and `evaluate` through
//! [`value::CheckedPackageEvaluation`], the S6a seam and its
//! `ReferenceEvaluation` hook, the family evaluators, and the layer-5
//! prototype v2 function-package codec -- and the finite exploration engine
//! [`simulation`].
//!
//! This crate depends on layer 4 (`qsl-package`), layer 3 (`qsl-semantics`),
//! F (`qsl-foundation`) and K (`quire-exact`). It never names the root crate
//! or a SEAM module (the old `state` and `temporal` evaluators stay in the
//! root crate until M-6c deletes them): Cargo refuses the first, and the root
//! crate is the only home of the second. Every later layer depends on this
//! crate, never the other way.

pub mod simulation;
pub mod value;
