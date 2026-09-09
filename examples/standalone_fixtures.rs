// SPDX-License-Identifier: AGPL-3.0-only
//! FR-026/029/031/033: selected output directory for native/Markdown workflows.

#[path = "../tests/support/standalone_setup.rs"]
mod setup;

fn main() -> std::process::ExitCode {
    let arguments: Vec<_> = std::env::args_os().skip(1).take(2).collect();
    let [directory] = arguments.as_slice() else {
        eprintln!("usage: cargo run --example standalone_fixtures -- <output-directory>");
        return std::process::ExitCode::from(2);
    };
    let directory = std::path::Path::new(directory);
    for (name, case) in [
        ("healthy", setup::Case::Aggregate(2)),
        ("violating", setup::Case::Aggregate(1)),
        ("operation", setup::Case::Operation(false)),
        ("refused", setup::Case::Operation(true)),
        ("boolean", setup::Case::Boolean(true)),
        ("integer-healthy", setup::Case::Integer(1)),
        ("integer-violating", setup::Case::Integer(10)),
    ] {
        if let Err(error) = setup::write(&directory.join(name), case) {
            eprintln!("cannot write {name}: {error}");
            return std::process::ExitCode::from(2);
        }
    }
    for (name, case) in [
        ("markdown-healthy", setup::Case::Aggregate(2)),
        ("markdown-violating", setup::Case::Aggregate(1)),
        ("markdown-refused", setup::Case::Operation(true)),
    ] {
        if let Err(error) = setup::write_extracted(&directory.join(name), case, true) {
            eprintln!("cannot write {name}: {error}");
            return std::process::ExitCode::from(2);
        }
    }
    std::process::ExitCode::SUCCESS
}
