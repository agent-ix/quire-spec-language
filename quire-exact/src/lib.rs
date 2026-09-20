// SPDX-License-Identifier: AGPL-3.0-or-later
//! `quire-exact`: the QSL kernel row (QSL#213 S-1, ADR-011 X-1, ADR-013 §7
//! S-1).
//!
//! This crate is the AD-016/ADR-011 module-DAG leaf layer `K`: checked
//! identity ([`node`], [`identity`]), provenance ([`location`]), kernel
//! outcomes and refusals ([`outcome`]), bounds and accounting
//! ([`accounting`], [`integer`]'s `BoundedInteger`, [`collection`]'s
//! `CardinalityBound`), and the exact semantic value kernel ([`value`] and
//! every value-family module it composes: [`numeric`], [`rational`],
//! [`decimal`], [`text`], [`ieee`], [`division`], [`comparison`],
//! [`equality`], [`key`], [`quantity`], [`reference`]).
//!
//! It depends on nothing else in the `quire-spec-language` workspace (ADR-011
//! §6.1, §7.1: every crate-DAG edge points *into* this crate, never out of
//! it), and on no wire format, hashing or JCS canonicalization crate: every
//! digest identity here ([`node::NodeKey`] and the six [`identity`] types) is
//! minted by wrapping an already-computed digest through its one public
//! `from_digest` constructor (ADR-013 T-6), never by hashing internally.
//!
//! Several real, deliberate capability losses at this kernel boundary are
//! documented where they occur rather than silently absorbed:
//! - [`value`]: no `ValueType` variant admits a `Value::Enum`, and
//!   `Value::Population` has no kernel payload at all.
//! - [`key`] and [`equality`]: an enum pair keys and compares equal by raw
//!   digest, with no declaration-aware ordering or same-enum check.
//! - [`quantity`]: no cross-unit arithmetic, comparison or equality; only
//!   same-unit operations.
//! - [`equality`]: the top-level text/enum/quantity schedule selection and
//!   the closed equality-conversion table are dropped along with the
//!   declaration registry and unit graph they need.
//!
//! Every one of these is a candidate for the ADR-011 §2.3 kernel proof gate's
//! claimed-module list (see `docs/kernel-proof-gate.md`), which currently
//! claims a first slice of modules with no cross-crate dependency risk.

#![forbid(unsafe_code)]

pub mod accounting;
pub mod collection;
pub mod comparison;
pub mod decimal;
pub mod division;
pub mod equality;
pub mod identity;
pub mod ieee;
pub mod integer;
mod key;
pub mod location;
pub mod node;
pub mod numeric;
pub mod outcome;
pub mod quantity;
pub mod rational;
pub mod reference;
pub mod text;
pub mod value;
