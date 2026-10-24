// SPDX-License-Identifier: AGPL-3.0-or-later
//! Records Cargo-boundary arguments and target environment, not compilation.
#![forbid(unsafe_code)]

use std::io::Write;

fn main() -> std::io::Result<()> {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let mut log = std::fs::OpenOptions::new()
        .append(true)
        .open(std::env::var_os("QSL_ARGV_LOG").expect("argument receipt path"))?;
    match std::env::var_os("CARGO_TARGET_DIR") {
        None => log.write_all(&[0])?,
        Some(root) => {
            log.write_all(&[1])?;
            write_text(&mut log, &root.into_string().expect("UTF-8 test root"))?;
        }
    }
    log.write_all(
        &u64::try_from(arguments.len())
            .expect("argument count fits u64")
            .to_le_bytes(),
    )?;
    for argument in arguments {
        write_text(&mut log, &argument)?;
    }
    Ok(())
}

fn write_text(log: &mut impl Write, text: &str) -> std::io::Result<()> {
    log.write_all(
        &u64::try_from(text.len())
            .expect("text length fits u64")
            .to_le_bytes(),
    )?;
    log.write_all(text.as_bytes())
}
