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
//! - [`value`]: `Value::Population` has no kernel payload at all (the
//!   `ValueType::Enum` shape, by contrast, carries its variant set inline
//!   per ADR-013 O-14, so it needs no declaration lookup and is not a
//!   capability loss).
//! - [`key`] and [`equality`]: an enum pair keys and compares equal by raw
//!   digest, with no declaration-aware ordering or same-enum check.
//! - [`quantity`]: no cross-unit arithmetic, comparison or equality; only
//!   same-unit operations.
//! - [`equality`]: the top-level text/enum/quantity schedule selection and
//!   the closed equality-conversion table are dropped along with the
//!   declaration registry and unit graph they need.
//!
//! **The ADR-011 §2.3 kernel proof gate does not exist yet.** This crate
//! ships with zero discharged propositions and no claimed-module list.
//! `cargo kani` cannot run against this workspace at all today: Kani 0.67.0's
//! bundled toolchain is `rustc 1.93.0-nightly`, while this workspace declares
//! `rust-version = "1.98"`, and `cargo kani` separately fails on a vendored
//! dependency's fixture `Cargo.toml`. Building the gate is tracked
//! separately (QSL-130) and left to ADR-011 §2.3's own named enforcer, #219.

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
