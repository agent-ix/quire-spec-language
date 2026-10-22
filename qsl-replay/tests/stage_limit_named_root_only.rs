// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1093 (FR-399-AC-1 to FR-399-AC-4): a consumer that names only the
//! `qsl_replay` root sets a named stage limit and compiles a source larger
//! than the default `s1.input_bytes` through `compile_admitted`. This file
//! names no `qsl_foundation`, `qsl_replay::spine` or `qsl_semantics` item, so
//! it stops compiling if the capability needs one.

use ix_trace_rs::trace;
use qsl_replay::{
    compile_admitted, AdmittedCompilation, AdmittedRefusal, AdmittedResource, DependencyInput,
    ReplayLimits, SourceIdentity, StageLimitNameRefusal, StageLimits,
};

const SOURCE: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile v = \"quire.value.complete/v1\";\n\
     enum Color { BLUE, RED }\n";

/// A valid unit padded with line comments to a little over 1 MiB.
fn large_source() -> String {
    let line = format!("// {}\n", "x".repeat(96));
    let mut text = SOURCE.to_owned();
    while text.len() <= (1 << 20) + 4096 {
        text.push_str(&line);
    }
    text
}

fn admit(
    text: &str,
    stages: StageLimits,
    replay: ReplayLimits,
) -> Result<AdmittedCompilation, AdmittedRefusal> {
    compile_admitted(
        SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        text.as_bytes(),
        [],
        &DependencyInput::default(),
        stages,
        replay,
    )
}

#[trace("TC-1093", "FR-399-AC-1")]
#[test]
fn a_source_over_the_default_is_refused_with_the_s1_limit() {
    let text = large_source();
    assert!(text.len() > 1 << 20);
    assert!(matches!(
        admit(&text, StageLimits::default(), ReplayLimits::default()),
        Err(AdmittedRefusal::Resource(AdmittedResource::StageLimit(_)))
    ));
}

#[trace("TC-1093", "FR-399-AC-2")]
#[test]
fn a_named_s1_limit_admits_the_large_source() {
    let text = large_source();
    let bound = 4 << 20;
    let stages = StageLimits::default()
        .with_named("s1.input_bytes", bound)
        .expect("a stage limit");

    // The reader limit still caps the S1 limit (ADR-013 O-26).
    assert!(matches!(
        admit(&text, stages.clone(), ReplayLimits::default()),
        Err(AdmittedRefusal::Resource(AdmittedResource::AboveReader(_)))
    ));

    let admitted = admit(
        &text,
        stages,
        ReplayLimits::default().with_input_bytes(bound),
    )
    .expect("the raised limit admits the source");
    let (_, context) = admitted.into_parts();
    assert_eq!(context.enums().count(), 1);
}

#[trace("TC-1093", "FR-399-AC-3")]
#[test]
fn an_unknown_or_unavailable_name_refuses_without_a_value() {
    for name in [
        "s1.no_such_limit",
        "",
        "replay.input_bytes",
        "format.output_bytes",
    ] {
        assert_eq!(
            StageLimits::default().with_named(name, 1),
            Err(StageLimitNameRefusal {
                name: name.to_owned()
            }),
            "{name}"
        );
    }
}

#[trace("TC-1093", "FR-399-AC-4")]
#[test]
fn the_default_stays_one_mebibyte_and_a_later_entry_replaces() {
    assert_eq!(StageLimits::default().iter().count(), 0);
    let text = large_source();
    let tight = StageLimits::default()
        .with_named("s1.input_bytes", 1 << 30)
        .and_then(|limits| limits.with_named("s1.input_bytes", 4))
        .expect("a stage limit");
    assert!(matches!(
        admit(
            &text,
            tight,
            ReplayLimits::default().with_input_bytes(1 << 30)
        ),
        Err(AdmittedRefusal::Resource(AdmittedResource::StageLimit(_)))
    ));
}
