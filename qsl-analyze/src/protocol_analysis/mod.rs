// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-036 PA-1: the protocol analysis engine, layer A, beside `model_check`.
//! This slice (PA-9 to PA-12, PA-20, PA-21, PA-24, PA-25) projects a global
//! protocol onto one role (FR-570). It reads the checked protocol through
//! `qsl_package::protocol_view` alone (PA-3) and builds no transition system
//! of its own (PA-22). Nothing here serves a kind: the provider manifest is a
//! later slice (QSL-1897).

mod profile;
mod project;

pub use profile::unsupported_construct;
pub use project::{project, ProjectionOutcome, ProjectionRefusal, ProjectionRequest, PROFILE};
