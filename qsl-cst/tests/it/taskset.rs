// SPDX-License-Identifier: AGPL-3.0-or-later
//! S1 parses the task-set productions of QSpec's shared grammar -- `taskset-decl`,
//! `task`, `signed-quantity` and `schedulability-clause` (QSpec FR-419). The
//! grammar admits an empty task set, a negative time value and a `lo` task
//! with a `wcet_hi`, so the recognizer accepts each and S3 refuses it.
use ix_trace_rs::trace;
use qsl_cst::{Limits, ParsedSource, Production};
use qsl_foundation::{SourceIdentity, Span};

fn source(declarations: &str) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\nprofile Complete = \"quire.value.complete/v1\";\n{declarations}"
    )
}

fn parse(text: &str) -> ParsedSource {
    qsl_cst::parse(
        SourceIdentity {
            authority: "test".into(),
            identity: "test:taskset".into(),
            revision_namespace: "test".into(),
            revision: "1".into(),
        },
        "taskset.native",
        text.as_bytes(),
        Limits::default(),
    )
    .expect("S1 reads the unit")
}

fn texts<'t>(parsed: &ParsedSource, text: &'t str, production: Production) -> Vec<&'t str> {
    parsed
        .cst()
        .nodes()
        .iter()
        .filter(|node| node.production() == production)
        .map(|node| node.span())
        .map(|Span { start, end }| &text[start..end])
        .collect()
}

#[trace("TC-701")]
#[test]
fn task_sets_and_schedulability_claims_parse_to_their_productions() {
    let tasks = [
        "task a periodic 4 ms deadline 4 ms wcet 1 ms priority 1 lo;",
        "task b sporadic 6 ms deadline 6 ms wcet 2 ms priority 2 jitter 1 ms blocking 3 ms hi wcet_hi 3 ms;",
    ];
    let set = format!("taskset Ctl {{ {} {} }}", tasks[0], tasks[1]);
    let schedulable = "schedulable CtlFp on Ctl under fixed_priority;";
    let response = "response R on Ctl.b <= 12 ms under fixed_priority;";
    let text = source(&format!("{set}\n{schedulable}\nschedulable CtlEdf on Ctl under edf;\nschedulable CtlAmc on Ctl under amc;\n{response}"));
    let parsed = parse(&text);
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    assert_eq!(texts(&parsed, &text, Production::TasksetDeclaration), [set]);
    assert_eq!(texts(&parsed, &text, Production::Task), tasks);
    assert_eq!(
        texts(&parsed, &text, Production::SchedulableDeclaration),
        [
            schedulable,
            "schedulable CtlEdf on Ctl under edf;",
            "schedulable CtlAmc on Ctl under amc;"
        ]
    );
    assert_eq!(
        texts(&parsed, &text, Production::ResponseDeclaration),
        [response]
    );
}

#[trace("TC-701")]
#[test]
fn the_grammar_admits_what_s3_refuses() {
    let text = source(
        "taskset Empty { }\n\
         taskset Odd { task t periodic 4 ms deadline 4 ms wcet 1 ms jitter -1 ms lo wcet_hi 2 ms; }",
    );
    let parsed = parse(&text);
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    assert_eq!(
        texts(&parsed, &text, Production::TasksetDeclaration).len(),
        2
    );
    assert_eq!(
        texts(&parsed, &text, Production::SignedQuantity),
        ["4 ms", "4 ms", "1 ms", "-1 ms", "2 ms"]
    );
}

#[trace("TC-701")]
#[test]
fn the_task_set_words_stay_identifiers_elsewhere() {
    let text = source("function f using Complete (taskset: Integer): Integer pure { taskset }");
    let parsed = parse(&text);
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    assert!(texts(&parsed, &text, Production::TasksetDeclaration).is_empty());
}

/// Every word the task-set family adds is contextual: it is an ordinary
/// identifier as a parameter, a function name or the name of a task, and a
/// leading `response` or `schedulable` is a declaration only in leading
/// position.
#[trace("TC-701")]
#[test]
fn every_added_word_is_an_identifier_outside_its_position() {
    for word in [
        "taskset",
        "task",
        "schedulable",
        "response",
        "periodic",
        "sporadic",
        "deadline",
        "wcet",
        "wcet_hi",
        "priority",
        "jitter",
        "blocking",
        "lo",
        "hi",
        "under",
        "fixed_priority",
        "edf",
        "amc",
    ] {
        let text = source(&format!(
            "function {word} using Complete ({word}: Integer): Integer pure {{ {word} }}"
        ));
        let parsed = parse(&text);
        assert!(parsed.is_admissible(), "{word}: {:?}", parsed.diagnostics());
        assert!(texts(&parsed, &text, Production::TasksetDeclaration).is_empty());
        assert!(texts(&parsed, &text, Production::ResponseDeclaration).is_empty());
        assert!(texts(&parsed, &text, Production::SchedulableDeclaration).is_empty());
    }
    // Tasks named by the member words.
    let tasks = [
        "task lo periodic 4 ms deadline 4 ms wcet 1 ms lo;",
        "task hi sporadic 4 ms deadline 4 ms wcet 1 ms hi wcet_hi 2 ms;",
        "task wcet_hi periodic 4 ms deadline 4 ms wcet 1 ms lo;",
        "task periodic periodic 4 ms deadline 4 ms wcet 1 ms lo;",
        "task deadline periodic 4 ms deadline 4 ms wcet 1 ms priority 1 lo;",
    ];
    let text = source(&format!("taskset Ctl {{ {} }}", tasks.join(" ")));
    let parsed = parse(&text);
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    assert_eq!(texts(&parsed, &text, Production::Task), tasks);
}

/// QSpec `release-decl`: `release S.t on M::o;` parses to its production,
/// beside a temporal `release` operator in the same unit. The word is the
/// reserved temporal operator, which is infix, so each keeps its position and
/// neither reads as the other.
#[trace("TC-704", "FR-249-AC-3")]
#[test]
fn a_release_declaration_parses_beside_the_temporal_release_operator() {
    let release = "release Ctl.a on Config::ConfigVersion::sample;";
    let temporal = "temporal W using Complete over (s: Integer) clock \"t\" on origin \
         { holds(s > 0) release holds(s > 1) }";
    let text = source(&format!("{release}\n{temporal}\n{release}"));
    let parsed = parse(&text);
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    assert_eq!(
        texts(&parsed, &text, Production::ReleaseDeclaration),
        [release, release]
    );
    assert_eq!(
        texts(&parsed, &text, Production::TemporalClause).len(),
        1,
        "the operator form is still one temporal clause"
    );
}

/// A declaration head `release` needs its own shape: the temporal operator
/// does not open a declaration, and a malformed release declaration is not
/// admitted.
#[trace("TC-704")]
#[test]
fn a_malformed_release_declaration_is_not_admitted() {
    for declaration in [
        "release Ctl on M::o;",
        "release Ctl.a M::o;",
        "release Ctl.a on M::o",
        "release Ctl.a on ;",
        "release holds(true);",
    ] {
        let text = source(declaration);
        let parsed = parse(&text);
        assert!(!parsed.is_admissible(), "{declaration}");
        assert!(texts(&parsed, &text, Production::ReleaseDeclaration).is_empty());
    }
}
