// SPDX-License-Identifier: AGPL-3.0-only
//! TC-090: author fixed test data from the independent Rust fixture recipe.
//! This maintenance executable never calls the package producer or reader.

#![forbid(unsafe_code)]

#[path = "../tests/support/native_rule_model.rs"]
#[expect(
    dead_code,
    reason = "The author uses from_text; the shared default fixture and parts convenience function are unused in this executable."
)]
mod native_rule_model;
#[path = "../tests/package_construction_cases/vectors.rs"]
mod vectors;

use std::{env, fs, io, path::PathBuf};

const HEADER: &str = include_str!("../tests/fixtures/native-package/header.native");

fn main() -> io::Result<()> {
    let mut arguments = env::args_os().skip(1);
    let output = arguments.next().map(PathBuf::from).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "usage: author_native_package_vectors <new-output-directory>",
        )
    })?;
    if arguments.next().is_some() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "expected one output directory",
        ));
    }
    // A fresh directory prevents regeneration from silently replacing reviewed
    // fixtures. Compare and deliberately promote the candidate files afterwards.
    fs::create_dir(&output)?;
    let model = vectors::model();
    assert_eq!(
        model.roles().objects[0].record,
        native_rule_model::symbol("Node")
    );
    fs::write(
        output.join("model-source.json"),
        model.source().source().text(),
    )?;
    fs::write(output.join("model-artifact.json"), model.artifact_bytes())?;
    for (name, source, expected) in [
        (
            "minimal",
            vectors::source(&model),
            vectors::expected(&model, r#""test:package""#, r#""draft:1""#, 1),
        ),
        (
            "controls",
            vectors::source(&model),
            vectors::expected(
                &model,
                r#""test:\"\\/\u0000\b\f\n\r\té🦀""#,
                r#""draft:é\u0001""#,
                9_007_199_254_740_993,
            ),
        ),
        (
            "multiple",
            vectors::source_multiple(&model),
            vectors::expected_multiple(&model, r#""test:multiple""#, r#""draft:1""#, 1),
        ),
    ] {
        fs::write(output.join(format!("{name}.native")), source)?;
        fs::write(
            output.join(format!("{name}.canonical.json")),
            expected.canonical,
        )?;
        fs::write(
            output.join(format!("{name}.package.json")),
            expected.artifact,
        )?;
        fs::write(
            output.join(format!("{name}.sha256")),
            format!("{}\n", expected.digest),
        )?;
    }
    Ok(())
}
