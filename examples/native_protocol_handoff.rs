// SPDX-License-Identifier: AGPL-3.0-only
//! FR-042/TC-121: real native producer fixture; B's IT-001 remains a separate gate.

#[path = "protocol-handoff/producer.rs"]
mod producer;

fn main() -> Result<(), producer::Error> {
    let mut arguments = std::env::args_os().skip(1);
    let directory = arguments.next().ok_or(producer::Error::Arguments)?;
    if arguments.next().is_some() {
        return Err(producer::Error::Arguments);
    }
    producer::write(std::path::Path::new(&directory))
}
