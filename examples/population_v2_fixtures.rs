// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-771: thin CLI over `population_fixtures::write`, the `Population<T>[N]` Contract IR v2 packages the admission corpus emits.

use quire_spec_language::population_fixtures;

fn main() -> std::process::ExitCode {
    let mut arguments = std::env::args_os().skip(1);
    let (Some(directory), None) = (arguments.next(), arguments.next()) else {
        eprintln!("usage: population_v2_fixtures <new-output-directory>");
        return std::process::ExitCode::FAILURE;
    };
    match population_fixtures::write(std::path::Path::new(&directory)) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            std::process::ExitCode::FAILURE
        }
    }
}
