// SPDX-License-Identifier: AGPL-3.0-or-later
//! `cargo xtask seam-probe` / `cargo xtask string-edge` / `cargo xtask
//! route-lint` / `cargo xtask checked-input`: the gates `make ci` runs
//! standalone. `cargo xtask canonical-types [<workspace>]` runs on demand,
//! over this workspace or the one named.
#![forbid(unsafe_code)]

use std::ffi::OsString;
use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;

use xtask::{
    canonical_types,
    checked_input,
    error::{Error, Result},
    route_lint, seam_probe, string_edge,
};

const USAGE: &str = "usage: cargo xtask seam-probe\n       cargo xtask string-edge\n       \
                     cargo xtask route-lint\n       cargo xtask checked-input\n       \
                     cargo xtask canonical-types [<workspace>]";

#[qsl_attrs::string_edge]
fn run(arguments: &[OsString]) -> Result<String> {
    let Some((command, operands)) = arguments.split_first() else {
        return Err(Error::Usage(USAGE));
    };
    let command = command.to_str().ok_or(Error::Usage(USAGE))?;
    let workspace_operand = match (command, operands) {
        (_, []) => None,
        ("canonical-types", [workspace]) => Some(Path::new(workspace).to_path_buf()),
        _ => return Err(Error::Usage(USAGE)),
    };
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask is one level under the workspace root")
        .to_path_buf();
    match command {
        "seam-probe" => seam_probe::run(&workspace_root),
        "string-edge" => string_edge::run(&workspace_root),
        "route-lint" => route_lint::run(&workspace_root),
        "checked-input" => checked_input::run(&workspace_root),
        "canonical-types" => {
            canonical_types::run(workspace_operand.as_deref().unwrap_or(&workspace_root))
        }
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
