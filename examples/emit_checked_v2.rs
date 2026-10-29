// SPDX-License-Identifier: AGPL-3.0-or-later
//! Emit the `quire.checked-package/v2` bytes of a complete-V1 QSL source: a
//! thin caller of `quire_spec_language::emit_checked_v2::run`.

use quire_spec_language::emit_checked_v2::run;

fn main() -> std::process::ExitCode {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    match run(&arguments, &mut std::io::stdout().lock()) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            std::process::ExitCode::from(error.exit_code())
        }
    }
}
