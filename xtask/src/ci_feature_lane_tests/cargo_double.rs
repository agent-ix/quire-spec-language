// SPDX-License-Identifier: AGPL-3.0-or-later
//! Records actual Cargo argv and target selection at the Make process seam.
#![forbid(unsafe_code)]

use std::io::Write;

fn main() {
    let mut log = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(std::env::var_os("FEATURE_LANE_CALL_LOG").expect("call log"))
        .expect("open call log");
    let target = std::env::var("CARGO_TARGET_DIR").unwrap_or_default();
    let args: Vec<String> = std::env::args().skip(1).collect();
    writeln!(log, "{target}\t{}", args.join("\t")).expect("record invocation");
}
