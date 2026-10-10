// SPDX-License-Identifier: AGPL-3.0-or-later
//! Records Cargo-boundary arguments without pretending to compile anything.
#![forbid(unsafe_code)]

use std::io::Write;

fn main() -> std::io::Result<()> {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let mut log = std::fs::OpenOptions::new()
        .append(true)
        .open(std::env::var_os("QSL_ARGV_LOG").expect("argument receipt path"))?;
    log.write_all(
        &u64::try_from(arguments.len())
            .expect("argument count fits u64")
            .to_le_bytes(),
    )?;
    for argument in arguments {
        log.write_all(
            &u64::try_from(argument.len())
                .expect("argument length fits u64")
                .to_le_bytes(),
        )?;
        log.write_all(argument.as_bytes())?;
    }
    Ok(())
}
