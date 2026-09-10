// SPDX-License-Identifier: AGPL-3.0-only
//! FR-032: generate the named native/Markdown ConfigVersion workflow.

#[path = "config-version/fixtures.rs"]
mod fixtures;

fn main() -> std::process::ExitCode {
    let arguments: Vec<_> = std::env::args_os().skip(1).take(2).collect();
    let [directory] = arguments.as_slice() else {
        eprintln!("usage: cargo run --example config_version_fixtures -- <output-directory>");
        return std::process::ExitCode::from(2);
    };
    let directory = std::path::Path::new(directory);
    let model = match fixtures::model() {
        Ok(model) => model,
        Err(error) => {
            eprintln!("cannot construct ConfigVersion model: {error}");
            return std::process::ExitCode::from(2);
        }
    };
    for &case in fixtures::CASES {
        if let Err(error) = fixtures::write(&directory.join(case.id()), &model, case) {
            eprintln!("cannot write {}: {error}", case.id());
            return std::process::ExitCode::from(2);
        }
    }
    std::process::ExitCode::SUCCESS
}
