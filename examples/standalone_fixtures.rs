// SPDX-License-Identifier: AGPL-3.0-only
//! FR-026/029: explicitly selected output directory for the synthetic native workflow.

#[path = "../tests/support/standalone_setup.rs"]
mod setup;

fn main() {
    let arguments: Vec<_> = std::env::args_os().skip(1).take(2).collect();
    let [directory] = arguments.as_slice() else {
        eprintln!("usage: cargo run --example standalone_fixtures -- <output-directory>");
        std::process::exit(2);
    };
    let directory = std::path::Path::new(directory);
    for (name, case) in [
        ("healthy", setup::Case::Aggregate(2)),
        ("violating", setup::Case::Aggregate(1)),
        (
            "operation",
            setup::Case::Operation {
                violate_frame: false,
            },
        ),
        (
            "refused",
            setup::Case::Operation {
                violate_frame: true,
            },
        ),
        ("boolean", setup::Case::Boolean { flag: true }),
    ] {
        setup::write(&directory.join(name), case);
    }
}
