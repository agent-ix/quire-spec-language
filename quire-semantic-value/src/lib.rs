// SPDX-License-Identifier: AGPL-3.0-or-later
//! `quire-semantic-value`: the ADR-011 §6.1 layer SV, a shared `no_std`
//! leaf crate (FB-05) above the `quire-exact` kernel (K < SV < 3).
//!
//! It holds the runtime semantic values QSL's layer 3 and above and a backend
//! share, so the code exists once. It depends on `quire-exact` only, and uses
//! only `core` and `alloc`.
#![no_std]

extern crate alloc;

pub mod quantity;
pub mod stop;
pub mod unit;
