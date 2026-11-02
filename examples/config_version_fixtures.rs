// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-108: generate the named ConfigVersion spine corpus, one directory per case.

#[path = "config-version/cases.rs"]
mod cases;
#[path = "config-version/spine.rs"]
mod spine;

fn main() -> std::process::ExitCode {
    let arguments: Vec<_> = std::env::args_os().skip(1).take(2).collect();
    let [directory] = arguments.as_slice() else {
        eprintln!("usage: cargo run --example config_version_fixtures -- <output-directory>");
        return std::process::ExitCode::from(2);
    };
    let directory = std::path::Path::new(directory);
    for &case in cases::CASES {
        if let Err(error) = spine::write(&directory.join(case.id()), case) {
            eprintln!("cannot write {}: {error}", case.id());
            return std::process::ExitCode::from(2);
        }
    }
    std::process::ExitCode::SUCCESS
}
