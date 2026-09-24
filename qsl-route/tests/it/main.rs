// SPDX-License-Identifier: AGPL-3.0-or-later
//! `qsl-route`'s integration tests: the ones that exercise only ADR-011 §6.1
//! layer R (QSL-184), moved from the root crate's `tests/it/`. One binary,
//! the same shape as the root crate's `it` target.

mod route_registry;
mod routing;
