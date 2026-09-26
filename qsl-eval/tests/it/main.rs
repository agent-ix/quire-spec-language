// SPDX-License-Identifier: AGPL-3.0-or-later
//! `qsl-eval`'s integration tests: the ones that exercise only ADR-011 §6.1
//! layer 5 and the layers below it (QSL-183), moved from the root crate's
//! `tests/it/`. One binary, one link step, the same shape as the root
//! crate's `it` target.

pub(crate) mod support;

mod call_verdicts;
mod checked_package_call;
mod collection_algebra;
mod collection_queries;
mod composite_values;
mod dispatch_calls;
mod equality_matrix;
mod finite_simulation;
mod float_rounding;
mod inherited_attributes;
mod model_reference_queries;
mod source_call;
mod total_functions;
