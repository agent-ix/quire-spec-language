// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-052 canonical formula-wide native temporal evaluation owner boundary.

mod common;
pub mod request;
pub mod result;

pub use common::{
    Error, ErrorCode, EvidenceRef, Limits, Report, Usage, REQUEST_CONTRACT, RESULT_CONTRACT,
};
