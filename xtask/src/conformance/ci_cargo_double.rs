// SPDX-License-Identifier: AGPL-3.0-or-later
//! Ordinary Cargo lane process double for the canonical CI environment test.
#![forbid(unsafe_code)]

use std::fs::OpenOptions;
use std::io::Write;

fn main() {
    let log = std::env::var("CI_CARGO_DOUBLE_LOG").expect("log path");
    let checkout = std::env::var("QSPEC_DIR").expect("QSpec path");
    let command = std::env::args().nth(1).expect("Cargo command");
    writeln!(
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(log)
            .unwrap(),
        "{command}\t{checkout}"
    )
    .unwrap();
}
