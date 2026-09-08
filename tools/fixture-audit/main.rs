// SPDX-License-Identifier: AGPL-3.0-only
//! FR-012/NFR-005: Rust-only audit entry points, with explicit claim boundaries.
#![forbid(unsafe_code)]
mod checkpoint;
mod error;
mod input;
mod review;
mod roles;

use error::{Code, Error, Result};
use std::{
    ffi::OsString,
    io::{self, Write},
    path::Path,
    process::ExitCode,
};

fn run(arguments: &[OsString]) -> Result<String> {
    let usage = || {
        Error::new(Code::Usage,"fixture-audit self-test | <review|roles|model-bytes|rule-syntax> <fixture-root> | model-producer")
    };
    let Some(mode) = arguments.first().and_then(|arg| arg.to_str()) else {
        return Err(usage());
    };
    match (mode,&arguments[1..]) {
        ("self-test",[]) => review::self_test(),
        ("model-producer",[]) => Err(Error::new(Code::ProducerUnapproved,"fresh TypeSpec/Node producer qualification requires a separate explicit owner disposition; historical bytes remain available through model-bytes")),
        ("review",[root]) => review::audit(Path::new(root)),
        ("roles",[root]) => roles::audit(Path::new(root)),
        ("model-bytes",[root]) => checkpoint::model(Path::new(root)),
        ("rule-syntax",[root]) => checkpoint::syntax(Path::new(root)),
        _ => Err(usage()),
    }
}

fn main() -> ExitCode {
    // Only a mode plus one root is valid. Bound collection independently of OS limits.
    let arguments: Vec<_> = std::env::args_os().skip(1).take(3).collect();
    match run(&arguments) {
        Ok(summary) => match writeln!(io::stdout().lock(), "{summary}") {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                let _ = writeln!(
                    io::stderr().lock(),
                    "io: cannot write audit summary: {error}"
                );
                ExitCode::from(2)
            }
        },
        Err(error) => {
            let _ = writeln!(io::stderr().lock(), "{error}");
            ExitCode::from(error.exit_code())
        }
    }
}
