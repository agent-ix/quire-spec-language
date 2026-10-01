// SPDX-License-Identifier: AGPL-3.0-or-later
//! `quire-semantic-value`: the ADR-011 §6.1 layer SV, a shared `no_std`
//! leaf crate (FB-05) above the `quire-exact` kernel (K < SV < 3).
//!
//! It holds the runtime semantic values QSL's layer 3 and above and a backend
//! share, so the code exists once. It depends on no QSL layer: its
//! dependencies are the `quire-exact` kernel, ADR-013 §2's one RFC 8785
//! encoder (`quire-canonical`, built without `std`), `serde` and
//! `thiserror`. It uses only `core` and `alloc`.
#![no_std]

extern crate alloc;

pub mod quantity;
pub mod semantic_node;
pub mod stop;
pub mod unit;
