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

/// A valid unit padded with line comments to at least `len` bytes.
fn padded(len: usize) -> String {
    let line = format!("// {}\n", "x".repeat(96));
    let mut text = SOURCE.to_owned();
    while text.len() < len {
        text.push_str(&line);
    }
    text
}

/// A valid unit of real declarations, one enum per line, over 1 MiB: about
/// 220 thousand tokens and well over 50 thousand syntax nodes, past the
/// default `s1.tokens` and `s1.nodes` (the S1 ceilings real sources reach
/// before `s1.input_bytes`).
fn declarations() -> String {
    let mut text = SOURCE.to_owned();
    let mut index = 0;
    while text.len() <= (1 << 20) + 4096 {
        text.push_str(&format!(
            "enum E{index:06}_{} {{ A, B, C, D, E, F, G, H }}\n",
            "n".repeat(40)
        ));
        index += 1;
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

/// The text of the stage-limit refusal a compile returned.
fn fired(result: Result<AdmittedCompilation, AdmittedRefusal>) -> String {
    match result {
        Err(AdmittedRefusal::Resource(AdmittedResource::StageLimit(refusal))) => {
            refusal.to_string()
        }
        Err(other) => panic!("not a stage limit: {other}"),
        Ok(_) => panic!("the source was admitted"),
    }
}

fn named(limits: &[(&str, u64)]) -> StageLimits {
    limits
        .iter()
        .try_fold(StageLimits::default(), |stages, (name, bound)| {
            stages.with_named(name, *bound)
        })
        .expect("stage limits")
}

const BOUND: u64 = 4 << 20;

#[trace("TC-1093", "FR-399-AC-1")]
#[test]
fn a_source_over_the_default_is_refused_with_the_s1_input_limit() {
    let text = padded((1 << 20) + 4096);
    assert!(text.len() > 1 << 20);
    let result = admit(&text, StageLimits::default(), ReplayLimits::default());
    assert!(fired(result).contains("s1.input_bytes"));
}

#[trace("TC-1093", "FR-399-AC-2")]
#[test]
fn a_raised_s1_input_limit_above_the_reader_limit_refuses_above_reader() {
    let text = padded((1 << 20) + 4096);
    let stages = named(&[("s1.input_bytes", BOUND)]);
    assert!(matches!(
        admit(&text, stages, ReplayLimits::default()),
        Err(AdmittedRefusal::Resource(AdmittedResource::AboveReader(_)))
    ));
}

#[trace("TC-1093", "FR-399-AC-3")]
#[test]
fn a_raised_s1_input_limit_and_reader_limit_admit_a_comment_padded_source() {
    let text = padded((1 << 20) + 4096);
    let admitted = admit(
        &text,
        named(&[("s1.input_bytes", BOUND)]),
        ReplayLimits::default().with_input_bytes(BOUND),
    )
    .expect("the raised limits admit the source");
    assert_eq!(admitted.into_parts().1.enums().count(), 1);
}

#[trace("TC-1093", "FR-399-AC-4")]
#[test]
fn a_source_of_declarations_needs_s1_tokens_and_s1_nodes_raised_as_well() {
    let text = declarations();
    let replay = ReplayLimits::default().with_input_bytes(BOUND);

    let default = admit(&text, StageLimits::default(), ReplayLimits::default());
    assert!(fired(default).contains("s1.input_bytes"));

    let input = named(&[("s1.input_bytes", BOUND)]);
    let tokens = fired(admit(&text, input.clone(), replay));
    assert!(tokens.contains("s1.tokens"), "{tokens}");

    let input_tokens = named(&[("s1.input_bytes", BOUND), ("s1.tokens", BOUND)]);
    let nodes = fired(admit(&text, input_tokens, replay));
    assert!(nodes.contains("s1.nodes"), "{nodes}");

    let all = named(&[
        ("s1.input_bytes", BOUND),
        ("s1.tokens", BOUND),
        ("s1.nodes", BOUND),
    ]);
    let admitted = admit(&text, all, replay).expect("all four limits raised admit the source");
    assert!(admitted.into_parts().1.enums().count() > 10_000);
}

#[trace("TC-1093", "FR-399-AC-5")]
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

#[trace("TC-1093", "FR-399-AC-6")]
#[test]
fn the_default_admits_just_under_one_mebibyte_and_a_later_entry_replaces() {
    assert_eq!(StageLimits::default().iter().count(), 0);
    let under = padded((1 << 20) - 4096);
    assert!(under.len() < 1 << 20);
    admit(&under, StageLimits::default(), ReplayLimits::default())
        .expect("a source under the default admits");

    let tight = named(&[("s1.input_bytes", 1 << 30), ("s1.input_bytes", 4)]);
    let result = admit(
        &under,
        tight,
        ReplayLimits::default().with_input_bytes(1 << 30),
    );
    assert!(fired(result).contains("limit 4 reached"));
}
