// SPDX-License-Identifier: AGPL-3.0-or-later
//! `cargo xtask seam-probe` / `cargo xtask string-edge` / `cargo xtask
//! route-lint`: the gates `make ci` runs standalone.
#![forbid(unsafe_code)]

use std::ffi::OsString;
use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;

use xtask::{
    error::{Error, Result},
    route_lint, seam_probe, string_edge,
};

const USAGE: &str =
    "usage: cargo xtask seam-probe\n       cargo xtask string-edge\n       cargo xtask route-lint";

fn run(arguments: &[OsString]) -> Result<String> {
    let Some((command, operands)) = arguments.split_first() else {
        return Err(Error::Usage(USAGE));
    };
    if !operands.is_empty() {
        return Err(Error::Usage(USAGE));
    }
    let command = command.to_str().ok_or(Error::Usage(USAGE))?;
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask is one level under the workspace root")
        .to_path_buf();
    match command {
        "seam-probe" => seam_probe::run(&workspace_root),
        "string-edge" => string_edge::run(&workspace_root),
        "route-lint" => route_lint::run(&workspace_root),
        _ => Err(Error::Usage(USAGE)),
    }
}

fn main() -> ExitCode {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    match run(&arguments) {
        Ok(summary) => match write!(io::stdout().lock(), "{summary}") {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                let _ = writeln!(io::stderr().lock(), "io: cannot write summary: {error}");
                ExitCode::from(2)
            }
        },
        Err(error) => {
            let _ = writeln!(io::stderr().lock(), "{error}");
            ExitCode::from(error.exit_code())
        }
    }
}
