// SPDX-License-Identifier: AGPL-3.0-or-later
//! Emit the `quire.checked-package/v2` bytes of a complete-V1 QSL source: a
//! thin caller of `quire_spec_language::emit_checked_v2::run`. Pass the same
//! fixed, cwd-relative source path every time: the path is embedded in the
//! bytes. Every failure exits 20 (or its catalog category) with a stderr
//! diagnostic.

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
