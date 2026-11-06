// SPDX-License-Identifier: AGPL-3.0-or-later
//! In-process calls of the library `command` API, shaped as the exit code,
//! stdout and stderr FR-285 and FR-287 assign them.
//!
//! The root package builds no executable (FR-287-AC-1), so these tests call
//! `quire_spec_language::command` directly. The rendering is the driver's
//! contract for a library outcome: a success writes its bytes to stdout, a
//! refusal writes its envelope to stderr, and the exit code is FR-285's.
//! The process boundary (arguments, streams, `/dev/full`) is the driver's
//! and is tested there (FR-287-AC-3 to AC-9, TC-1262).

use quire_spec_language::command::{self, RunLimits};
use std::path::Path;

/// The exit status of one in-process call.
#[derive(Debug)]
pub(crate) struct Status(u8);

impl Status {
    pub(crate) fn code(&self) -> Option<i32> {
        Some(i32::from(self.0))
    }

    pub(crate) fn success(&self) -> bool {
        self.0 == 0
    }
}

/// What the driver would write for one in-process call.
#[derive(Debug)]
pub(crate) struct Output {
    pub(crate) status: Status,
    pub(crate) stdout: Vec<u8>,
    pub(crate) stderr: Vec<u8>,
}

fn refusal(error: &command::RunError, artifact: bool) -> Output {
    let value = if artifact {
        error.artifact_value()
    } else {
        error.value()
    }
    .expect("refusal envelope serializes");
    Output {
        status: Status(error.category().exit_code()),
        stdout: Vec::new(),
        stderr: format!("{value}\n").into_bytes(),
    }
}

/// `command::compile` over `<directory>/compile.json`.
pub(crate) fn compile(directory: &Path) -> Output {
    compile_file(&directory.join("compile.json"))
}

/// `command::compile` over the request file at `path`.
pub(crate) fn compile_file(path: &Path) -> Output {
    match command::compile(path) {
        Ok(bytes) => Output {
            status: Status(0),
            stdout: bytes,
            stderr: Vec::new(),
        },
        Err(error) => refusal(&error, true),
    }
}

/// `command::run` over the request file at `path`.
pub(crate) fn run_file(path: &Path) -> Output {
    run_file_with_limits(path, RunLimits::default())
}

/// `command::run_with_limits` over the request file at `path`.
pub(crate) fn run_file_with_limits(path: &Path, limits: RunLimits) -> Output {
    match command::run_with_limits(path, limits) {
        Ok(result) => Output {
            status: Status(result.exit_code),
            stdout: format!("{}\n", result.value).into_bytes(),
            stderr: Vec::new(),
        },
        Err(error) => refusal(&error, false),
    }
}
