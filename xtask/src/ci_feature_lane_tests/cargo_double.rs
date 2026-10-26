// SPDX-License-Identifier: AGPL-3.0-or-later
//! Records actual Cargo argv and target selection at the Make process seam.
#![forbid(unsafe_code)]

use std::io::Write;

fn main() {
    let incremental_log = std::env::var_os("FEATURE_LANE_INCREMENTAL_LOG")
        .expect("incremental mode log");
    let cargo_settings = [
        "CARGO_INCREMENTAL",
        "CARGO_PROFILE_DEV_DEBUG",
        "CARGO_PROFILE_TEST_DEBUG",
        "CARGO_PROFILE_DEV_DEBUG_ASSERTIONS",
        "CARGO_PROFILE_TEST_DEBUG_ASSERTIONS",
    ]
    .map(|name| {
        let value = std::env::var(name).unwrap_or_else(|_| "<unset>".to_owned());
        format!("{name}={value}")
    })
    .join("\n");
    std::fs::write(incremental_log, format!("{cargo_settings}\n"))
        .expect("record Cargo build settings");

    let mut log = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(std::env::var_os("FEATURE_LANE_CALL_LOG").expect("call log"))
        .expect("open call log");
    let target = std::env::var("CARGO_TARGET_DIR").unwrap_or_default();
    let args: Vec<String> = std::env::args().skip(1).collect();
    writeln!(log, "{target}\t{}", args.join("\t")).expect("record invocation");

    if args.iter().any(|argument| argument == "--list") {
        let test = args
            .iter()
            .position(|argument| argument == "--exact")
            .and_then(|index| args.get(index + 1))
            .expect("test filter after --exact");
        let count = std::env::var("FEATURE_LANE_LIST_COUNT")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(1);
        for _ in 0..count {
            println!("{test}: test");
        }
    }
}
