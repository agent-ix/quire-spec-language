// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-172, FR-173, FR-174 and FR-180 (S3): the `TemporalTrace` family's check
//! of a `hyper` clause over `behaviours` and of a `relation` over model
//! executions (TC-597, TC-598, TC-599, TC-605). The vault is the
//! `ConfigVersion` fixture package: `versionNumber` stands for the vault's
//! low field and `step(i)` and `mix()` for its operations; each device model is
//! the same shape under its own package identity, with `tick(load)`.

use ix_trace_rs::trace;
use qsl_foundation::diagnostic::{Code, StageFailure};
use qsl_foundation::Span;
use qsl_semantics::check::{
    check_hyper_clauses, Capability, CheckCause, CheckRefusal, CheckedHyper, CheckedHyperClause,
    CheckedModelRelation, HyperCause, HyperForm, HyperTraceFamily, HyperUnsupported,
    PackageDeclarations, StepRead, TemporalCause, TemporalProfile,
};
use qsl_semantics::family::{ClaimExtent, FamilyContract};
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::intake::{admit_unit, package_input};
use quire_checked::node::NodeKind;
use quire_exact::{NodeKey, PathSegment};
use quire_semantic_value::checking::CheckingLimits;
use serde_json::json;

use crate::model_operations::{
    config_version_document_with_operations, empty_frame, operation, operation_parameter,
    parse_and_build, version_number_identity,
};

const INFINITE: &str = "quire.temporal.infinite-trace/v1";
const EVENT: &str = "quire.temporal.event-position.false-extension/v1";
const VAULT: &str = "example/config-version";
const SAVING: &str = "example/device-saving";
const NORMAL: &str = "example/device-normal";

/// The vault: `step(i)` and `mix()` on `ConfigVersion`.
fn vault() -> Vec<u8> {
    let step = operation(
        "step",
        json!([operation_parameter("step", "i", &version_number_identity())]),
        None,
        empty_frame(),
    );
    let mix = operation("mix", json!([]), None, empty_frame());
    config_version_document_with_operations(vec![step, mix])
}

/// A device model under the package identity `identity`: `tick(load)`.
fn device(identity: &str) -> Vec<u8> {
    let tick = operation(
        "tick",
        json!([operation_parameter(
            "tick",
            "load",
            &version_number_identity()
        )]),
        None,
        empty_frame(),
    );
    let document = config_version_document_with_operations(vec![tick]);
    String::from_utf8(document)
        .expect("the document is UTF-8")
        .replace(VAULT, identity)
        .into_bytes()
}

/// The model declarations of the vault (`V`), the saving device (`S`) and the
/// normal device (`N`), by alias.
fn model(alias: &str) -> (String, Vec<u8>) {
    let (identity, document) = match alias {
        "V" => (VAULT, vault()),
        "S" => (SAVING, device(SAVING)),
        "N" => (NORMAL, device(NORMAL)),
        other => panic!("no model {other}"),
    };
    (identity.to_owned(), document)
}

/// The unit text selecting `aliases`' models under the temporal profile
/// `profile`, and the declarations of `declarations`, with its package.
fn assemble(aliases: &[&str], profile: &str, declarations: &str) -> (String, PackageDeclarations) {
    assemble_under("quire.value.complete/v1", aliases, profile, declarations)
}

/// [`assemble`], with the unit's `v` header profile `layer`.
fn assemble_under(
    layer: &str,
    aliases: &[&str],
    profile: &str,
    declarations: &str,
) -> (String, PackageDeclarations) {
    let mut packages = std::collections::BTreeMap::new();
    let mut selections = String::new();
    for alias in aliases {
        let (identity, document) = model(alias);
        let input = package_input([document.as_slice()]).unwrap();
        assert_eq!(input.len(), 1, "one document");
        selections.push_str(&format!("model {alias} = {identity:?};\n"));
        packages.extend(input);
    }
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"{layer}\";\n\
         profile inf = \"{profile}\";\n\
         {selections}{declarations}\n"
    );
    let built = parse_and_build(&unit);
    let models = admit_unit(
        &built.selections().models,
        &packages,
        ModelNormalizationLimits::default(),
    )
    .unwrap_or_else(|refusal| panic!("I1 refused: {refusal:?}"));
    let raw = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("test", "tc-597", "fixture", "fixture:1"),
        "unit.native",
        unit.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 admits the unit")
    .source()
    .reference()
    .clone();
    let package = PackageDeclarations::assemble(
        raw,
        crate::TEST_PACKAGE_NAME.to_owned(),
        built,
        models,
        Vec::new(),
    )
    .unwrap_or_else(|refusal| panic!("{declarations}: the unit assembles: {refusal:?}"));
    (unit, package)
}

type Outcome = Result<CheckedHyper, StageFailure<CheckRefusal>>;

/// The one clause of `declaration` over `aliases`, checked under `profile`,
/// with the unit text its spans index.
fn check_under(aliases: &[&str], profile: &str, declaration: &str) -> (String, Outcome) {
    let (text, package) = assemble(aliases, profile, declaration);
    let mut checked = check_hyper_clauses(&package);
    let Some(outcome) = checked.pop() else {
        panic!("{declaration}: one clause");
    };
    assert!(checked.is_empty(), "{declaration}: one clause");
    (text, outcome)
}

fn clause_under(aliases: &[&str], profile: &str, declaration: &str) -> CheckedHyperClause {
    match check_under(aliases, profile, declaration) {
        (_, Ok(CheckedHyper::Behaviours(clause))) => *clause,
        (_, other) => panic!("{declaration}: a checked hyper clause, not {other:?}"),
    }
}

fn clause(aliases: &[&str], declaration: &str) -> CheckedHyperClause {
    clause_under(aliases, INFINITE, declaration)
}

fn relation(declaration: &str) -> CheckedModelRelation {
    match check_under(&["V"], INFINITE, declaration) {
        (_, Ok(CheckedHyper::Relation(relation))) => *relation,
        (_, other) => panic!("{declaration}: a checked relation, not {other:?}"),
    }
}

/// The refusal `declaration` gets and the unit text its span indexes.
fn refusal(aliases: &[&str], declaration: &str) -> (String, CheckCause) {
    match check_under(aliases, INFINITE, declaration) {
        (text, Err(StageFailure::Refused(refusal))) => (text, refusal.cause),
        (_, other) => panic!("{declaration}: a refusal, not {other:?}"),
    }
}

fn hyper_cause(aliases: &[&str], declaration: &str) -> (String, HyperCause) {
    match refusal(aliases, declaration) {
        (text, CheckCause::Hyper(cause)) => (text, *cause),
        (_, other) => panic!("{declaration}: a hyper refusal, not {other:?}"),
    }
}

/// The span of the first `needle` in `text`.
fn first(text: &str, needle: &str) -> Span {
    let start = text.find(needle).expect("the needle is in the unit");
    Span {
        start,
        end: start + needle.len(),
    }
}

/// The span of `needle` where it follows `context`.
fn after(text: &str, context: &str, needle: &str) -> Span {
    let start = text.find(context).expect("the context is in the unit") + context.len();
    assert!(text[start..].starts_with(needle), "{context}{needle}");
    Span {
        start,
        end: start + needle.len(),
    }
}

const READ_A_B: &str = "always holds(v.versionNumber @ a = v.versionNumber @ b)";
const STEP_INPUT_MATCH: &str = "match { a.step.op = b.step.op and \
     a.step.ConfigVersion::step.i = b.step.ConfigVersion::step.i }";

/// ADR-023 section 1's noninterference.
fn non_interference() -> String {
    format!(
        "hyper NonInterference using inf over behaviours (v: V::ConfigVersion) \
         forall trace a of V forall trace b of V {STEP_INPUT_MATCH} {{ {READ_A_B} }}"
    )
}

/// ADR-023 section 1's power comparison.
const SAVES_POWER: &str = "hyper SavesPower using inf over behaviours \
     (d: S::ConfigVersion, e: N::ConfigVersion) forall trace s of S forall trace n of N \
     match { s.step.ConfigVersion::tick.load = n.step.ConfigVersion::tick.load } \
     { always holds(d.versionNumber @ s <= e.versionNumber @ n) }";

/// ADR-023 section 8.2's opacity: the secret is not revealed.
const OPAQUE: &str = "hyper Opaque using inf over behaviours (v: V::ConfigVersion) \
     forall trace a of V exists trace b of V \
     match { a.step.ConfigVersion::step.i = b.step.ConfigVersion::step.i } \
     { holds(v.versionNumber @ a != v.versionNumber @ b) and \
     always holds(v.versionNumber @ a = v.versionNumber @ b) }";

const DET: &str =
    "relation Det using v over (x: V::ConfigVersion::step, y: V::ConfigVersion::step) \
     { x.i = y.i and pre(x.self.versionNumber) = pre(y.self.versionNumber) \
     implies x.self.versionNumber = y.self.versionNumber }";

/// A structural fixture node key named `name`.
fn fixture_node(name: &str) -> NodeKey {
    NodeKey::new("fixture", vec![PathSegment::Text(name.to_owned())], 0)
        .expect("a fixture node key is well formed")
}

fn unbounded(clause: &CheckedHyper) -> bool {
    let [claim] = HyperTraceFamily::requirements(clause)
        .try_into()
        .expect("one claim");
    let params: Vec<NodeKey> = (0..4_u8)
        .map(|index| fixture_node(&format!("parameter{index}")))
        .collect();
    let record = claim
        .requirements(fixture_node("clause"), &params, &[], &[])
        .expect("the record is keyed");
    assert_eq!(record.kind(), Capability::TemporalSatisfaction);
    matches!(record.extent(), ClaimExtent::Unbounded(_))
}

/// FR-172-AC-1: `NonInterference` checks with one alias, two universal
/// quantifiers over it, one object parameter and both `match` conjuncts in
/// `mu_universal`; `SavesPower` with the aliases `S` and `N`; `Opaque` with
/// its one conjunct in `mu_existential`.
#[trace("TC-597")]
#[test]
fn the_adr_clauses_bind_their_aliases_parameters_and_match_conjuncts() {
    let noninterference = clause(&["V"], &non_interference());
    let aliases: Vec<_> = noninterference
        .aliases()
        .iter()
        .map(|a| a.as_str())
        .collect();
    assert_eq!(aliases, ["V"]);
    let prefix: Vec<_> = noninterference
        .quantifiers()
        .iter()
        .map(|quantifier| (quantifier.kind, quantifier.variable.as_str()))
        .collect();
    assert_eq!(
        prefix,
        [
            (qsl_forms::QuantifierKind::Forall, "a"),
            (qsl_forms::QuantifierKind::Forall, "b"),
        ]
    );
    let [parameter] = noninterference.parameters() else {
        panic!("one object parameter");
    };
    assert_eq!(
        (parameter.name.as_str(), parameter.alias.as_str()),
        ("v", "V")
    );
    assert_eq!(noninterference.mu_universal().len(), 2);
    assert!(noninterference.mu_existential().is_empty());
    let reads: Vec<_> = noninterference.mu_universal()[1]
        .reads()
        .iter()
        .map(|read| match read {
            StepRead::Argument(argument) => (argument.trace.as_str(), argument.parameter.as_str()),
            other => panic!("an argument read, not {other:?}"),
        })
        .collect();
    assert_eq!(reads, [("a", "i"), ("b", "i")]);

    let power = clause(&["S", "N"], SAVES_POWER);
    let aliases: Vec<_> = power.aliases().iter().map(|a| a.as_str()).collect();
    assert_eq!(aliases, ["S", "N"]);
    assert_eq!(power.parameters().len(), 2);

    let opaque = clause(&["V"], OPAQUE);
    assert!(opaque.mu_universal().is_empty());
    let [conjunct] = opaque.mu_existential() else {
        panic!("one existential conjunct");
    };
    assert_eq!(conjunct.variables(), ["a", "b"]);
}

/// FR-172-AC-2: each refusal is at the span the requirement names.
#[trace("TC-597", "FR-172-AC-2")]
#[test]
fn an_ill_formed_clause_refuses_at_the_named_span_with_the_named_code() {
    // A quantifier with no `of` under `behaviours`.
    let (text, cause) = hyper_cause(
        &["V"],
        "hyper H using inf over behaviours forall trace a { always holds(true) }",
    );
    assert_eq!(
        cause,
        HyperCause::QuantifierWithoutAlias {
            span: first(&text, "forall trace a")
        }
    );
    assert_eq!(
        (cause.code(), cause.tag()),
        (Code::UnsupportedConstruct, "expression-form")
    );

    // `of V` under a finite trace domain.
    let (text, cause) = hyper_cause(
        &["V"],
        "hyper H using inf over t in V::Traces bounded 4 forall trace a of V { true }",
    );
    assert_eq!(
        cause,
        HyperCause::QuantifierWithAlias {
            span: first(&text, "forall trace a of V")
        }
    );
    assert_eq!(cause.code(), Code::UnsupportedConstruct);

    // `of W` names no model.
    let (text, cause) = hyper_cause(
        &["V"],
        "hyper H using inf over behaviours forall trace a of W { always holds(true) }",
    );
    assert_eq!(
        cause,
        HyperCause::UnresolvedAlias {
            alias: "W".into(),
            span: after(&text, "forall trace a of ", "W")
        }
    );
    assert_eq!(
        (cause.code(), cause.tag()),
        (Code::MissingDeclaration, "missing-name")
    );

    // A member read with no trace variable.
    let (text, cause) = hyper_cause(
        &["V"],
        "hyper H using inf over behaviours (v: V::ConfigVersion) forall trace a of V \
         { always holds(v.versionNumber = 1) }",
    );
    assert_eq!(
        cause,
        HyperCause::UnindexedRead {
            span: first(&text, "v.versionNumber")
        }
    );
    assert_eq!(
        (cause.code(), cause.tag()),
        (Code::IllTyped, "type-mismatch")
    );

    // A read through the trace of another model.
    let (text, cause) = hyper_cause(
        &["S", "N"],
        "hyper H using inf over behaviours (d: S::ConfigVersion) \
         forall trace s of S forall trace n of N { always holds(d.versionNumber @ n = 1) }",
    );
    assert_eq!(
        cause,
        HyperCause::MismatchedTrace {
            span: first(&text, "d.versionNumber @ n")
        }
    );
    assert_eq!(cause.code(), Code::IllTyped);

    // A `match` conjunct that reads state.
    let (text, cause) = hyper_cause(
        &["V"],
        "hyper H using inf over behaviours (v: V::ConfigVersion) forall trace a of V \
         match { v.versionNumber @ a = 1 } { always holds(true) }",
    );
    assert_eq!(
        cause,
        HyperCause::MatchReadsState {
            span: first(&text, "v.versionNumber @ a")
        }
    );
    assert_eq!(
        (cause.code(), cause.tag()),
        (Code::UnsupportedConstruct, "expression-form")
    );

    // An object parameter of an alias the clause does not bind.
    let (text, cause) = hyper_cause(
        &["V", "S"],
        "hyper H using inf over behaviours (d: S::ConfigVersion) forall trace a of V \
         { always holds(true) }",
    );
    assert_eq!(
        cause,
        HyperCause::UnboundParameterAlias {
            parameter: "d".into(),
            span: first(&text, "d: S::ConfigVersion")
        }
    );
    assert_eq!(cause.code(), Code::IllTyped);

    // A fairness constraint on an operation of another model.
    let (text, refusal) = refusal(
        &["S", "N"],
        "hyper H using inf over behaviours (d: S::ConfigVersion) \
         forall trace s of S fair weak N::ConfigVersion::tick; { always holds(true) }",
    );
    let CheckCause::Temporal(cause) = refusal else {
        panic!("a temporal refusal, not {refusal:?}");
    };
    assert_eq!(
        *cause,
        TemporalCause::UnresolvedFairnessOperation {
            name: "N::ConfigVersion::tick".into(),
            span: first(&text, "N::ConfigVersion::tick")
        }
    );
    assert_eq!(
        (cause.code(), cause.tag()),
        (Code::MissingDeclaration, "missing-name")
    );
}

/// FR-172-AC-3: `align skip` checks to the operation it names; an empty list,
/// a repeated operation, an operation of no bound alias and a fairness
/// constraint each refuse `unsupported_construct`/`expression-form`.
#[trace("TC-597", "FR-172-AC-3")]
#[test]
fn an_align_skip_list_checks_and_its_ill_formed_variants_refuse() {
    let projected = |skip: &str, fairness: &str| {
        format!(
            "hyper Projected using inf over behaviours (v: V::ConfigVersion) \
             forall trace a of V {fairness} forall trace b of V align skip {skip} \
             {{ {READ_A_B} }}"
        )
    };
    let checked = clause(&["V"], &projected("{ V::ConfigVersion::mix }", ""));
    let [skipped] = checked.align_skip() else {
        panic!("one skipped operation");
    };
    assert_eq!(skipped.operation.as_str(), "mix");

    let mixed = "V::ConfigVersion::mix";
    let skips = [
        ("{ }", ""),
        ("{ V::ConfigVersion::mix, V::ConfigVersion::mix }", ""),
        ("{ N::ConfigVersion::tick }", ""),
        (
            "{ V::ConfigVersion::mix }",
            "fair weak V::ConfigVersion::step;",
        ),
    ];
    for (index, (skip, fairness)) in skips.into_iter().enumerate() {
        let declaration = projected(skip, fairness);
        let (text, cause) = hyper_cause(&["V", "N"], &declaration);
        let expected = match index {
            0 => HyperCause::AlignSkipEmpty {
                span: first(&text, "align skip { }"),
            },
            1 => {
                let start = text.rfind(mixed).expect("the second entry");
                HyperCause::AlignSkipRepeated {
                    span: Span {
                        start,
                        end: start + mixed.len(),
                    },
                }
            }
            2 => HyperCause::AlignSkipUnbound {
                span: first(&text, "N::ConfigVersion::tick"),
            },
            _ => HyperCause::AlignSkipFairness {
                span: first(&text, "fair weak V::ConfigVersion::step;"),
            },
        };
        assert_eq!(cause, expected, "{declaration}");
        assert_eq!(
            (cause.code(), cause.tag()),
            (Code::UnsupportedConstruct, "expression-form"),
            "{declaration}"
        );
    }
}

/// FR-172-AC-4: `Det` checks with two model execution bindings to `step`;
/// `NonInterference`, the clause with its fairness sets swapped between its
/// variables, and the clause with an extra `match` conjunct have three
/// identities.
#[trace("TC-597", "FR-172-AC-4")]
#[test]
fn a_relation_binds_its_executions_and_a_clause_identity_binds_its_meaning() {
    let det = relation(DET);
    let executions: Vec<_> = det
        .executions()
        .iter()
        .map(|(variable, operation)| (variable.as_str(), operation.operation.as_str()))
        .collect();
    assert_eq!(executions, [("x", "step"), ("y", "step")]);
    assert_eq!(det.executions()[0].1, det.executions()[1].1);

    let fair_a = "forall trace a of V fair weak V::ConfigVersion::step; forall trace b of V";
    let fair_b = "forall trace a of V forall trace b of V fair weak V::ConfigVersion::step;";
    let with = |prefix: &str, matched: &str| {
        format!(
            "hyper H using inf over behaviours (v: V::ConfigVersion) {prefix} {matched} \
             {{ {READ_A_B} }}"
        )
    };
    let base = clause(&["V"], &non_interference());
    let swapped_a = clause(&["V"], &with(fair_a, STEP_INPUT_MATCH));
    let swapped_b = clause(&["V"], &with(fair_b, STEP_INPUT_MATCH));
    let extra = clause(
        &["V"],
        &with(
            "forall trace a of V forall trace b of V",
            "match { a.step.op = b.step.op and \
             a.step.ConfigVersion::step.i = b.step.ConfigVersion::step.i and \
             a.step.receiver = b.step.receiver }",
        ),
    );
    assert_ne!(swapped_a.identity(), swapped_b.identity());
    assert_ne!(base.identity(), swapped_a.identity());
    assert_ne!(base.identity(), extra.identity());
    let same = clause(&["V"], &non_interference());
    assert_eq!(base.identity(), same.identity());
}

/// FR-173-AC-1: the five clauses classify into their forms, and each records
/// `temporal-satisfaction` over an unbounded extent.
#[trace("TC-598", "FR-173-AC-1")]
#[test]
fn the_adr_clauses_classify_into_their_forms() {
    let projected = "hyper Projected using inf over behaviours (v: V::ConfigVersion) \
         forall trace a of V forall trace b of V align skip { V::ConfigVersion::mix } \
         { always holds(v.versionNumber @ a = v.versionNumber @ b) }";
    let single = "hyper Possible using inf over behaviours (v: V::ConfigVersion) \
         exists trace b of V { eventually holds(v.versionNumber @ b = 1) }";
    for (declaration, form) in [
        (non_interference(), HyperForm::Universal),
        (OPAQUE.to_owned(), HyperForm::ForallExistsSafety),
        (projected.to_owned(), HyperForm::ProjectionAligned),
        (single.to_owned(), HyperForm::SingleExistential),
    ] {
        let checked = clause(&["V"], &declaration);
        assert_eq!(checked.form(), Some(&form), "{declaration}");
        assert!(
            unbounded(&CheckedHyper::Behaviours(Box::new(checked))),
            "{declaration}"
        );
    }
    let det = relation(DET);
    assert_eq!(det.form(), HyperForm::StepRelation);
    assert!(unbounded(&CheckedHyper::Relation(Box::new(det))));
}

/// Each clause outside the admitted forms is `Other` and names the rule that
/// placed it there.
#[trace("TC-598")]
#[test]
fn a_clause_outside_the_admitted_forms_names_its_rule() {
    let opaque_with = |prefix: &str, body: &str| {
        format!(
            "hyper H using inf over behaviours (v: V::ConfigVersion) {prefix} \
             match {{ a.step.ConfigVersion::step.i = b.step.ConfigVersion::step.i }} {{ {body} }}"
        )
    };
    let safe = "holds(v.versionNumber @ a != v.versionNumber @ b) and \
                always holds(v.versionNumber @ a = v.versionNumber @ b)";
    let live = "always eventually holds(v.versionNumber @ a = v.versionNumber @ b)";
    for (declaration, reason) in [
        (
            opaque_with("exists trace b of V forall trace a of V", safe),
            HyperUnsupported::ExistentialBeforeUniversal,
        ),
        (
            opaque_with("forall trace a of V exists trace b of V", live),
            HyperUnsupported::ExistentialOutsideSafety,
        ),
        (
            opaque_with(
                "forall trace a of V exists trace b of V fair weak V::ConfigVersion::step;",
                safe,
            ),
            HyperUnsupported::ExistentialOutsideSafety,
        ),
        (
            opaque_with(
                "forall trace a of V exists trace b of V align skip { V::ConfigVersion::mix }",
                safe,
            ),
            HyperUnsupported::ProjectionOutsideSafety,
        ),
        (
            "hyper H using inf over behaviours (v: V::ConfigVersion) exists trace b of V \
             align skip { V::ConfigVersion::mix } { holds(v.versionNumber @ b = 1) }"
                .to_owned(),
            HyperUnsupported::ProjectionOutsideSafety,
        ),
    ] {
        let checked = clause(&["V"], &declaration);
        assert_eq!(
            checked.form(),
            Some(&HyperForm::Other { reason }),
            "{declaration}"
        );
    }
}

/// FR-173-AC-3: a single-existential clause with a `match` block refuses at
/// the block; the same noninterference under the event-position profile
/// records its profile and no form.
#[trace("TC-598")]
#[test]
fn a_single_existential_clause_admits_no_match_and_another_profile_has_no_form() {
    let (text, cause) = hyper_cause(
        &["V"],
        "hyper Possible using inf over behaviours (v: V::ConfigVersion) \
         exists trace b of V match { b.step.op = b.step.op } \
         { eventually holds(v.versionNumber @ b = 1) }",
    );
    assert_eq!(
        cause,
        HyperCause::SingleExistentialMatch {
            span: first(&text, "match { b.step.op = b.step.op }")
        }
    );
    assert_eq!(
        (cause.code(), cause.tag()),
        (Code::UnsupportedConstruct, "expression-form")
    );

    let event = clause_under(&["V"], EVENT, &non_interference());
    assert_eq!(
        event.profile(),
        TemporalProfile::EventPositionFalseExtension
    );
    assert_eq!(event.form(), None);
}

/// FR-173-AC-4 (S3): the clause records one subject alias per distinct model
/// it names, for the request writer to supply a subject and a deadlock-freedom
/// item each.
#[trace("TC-598")]
#[test]
fn a_clause_names_one_subject_alias_per_distinct_model() {
    let aliases = |checked: &CheckedHyperClause| -> Vec<String> {
        checked
            .aliases()
            .iter()
            .map(|a| a.as_str().to_owned())
            .collect()
    };
    assert_eq!(aliases(&clause(&["S", "N"], SAVES_POWER)), ["S", "N"]);
    assert_eq!(aliases(&clause(&["V"], &non_interference())), ["V"]);
    assert_eq!(aliases(&clause(&["V"], OPAQUE)), ["V"]);
}

/// FR-174-AC-1: noninterference has the copy group `[[a, b]]`; a `<=` body, a
/// fairness set on one variable and two aliases each have an empty one; the
/// two spellings of an equality are both swappable.
#[trace("TC-599", "FR-174-AC-1")]
#[test]
fn a_symmetric_clause_has_a_copy_group_and_an_asymmetric_one_has_none() {
    let classes = |checked: &CheckedHyperClause| checked.copy_group().classes.clone();
    let expected = vec![vec!["a".to_owned(), "b".to_owned()]];
    assert_eq!(classes(&clause(&["V"], &non_interference())), expected);

    let with = |prefix: &str, body: &str| {
        format!(
            "hyper H using inf over behaviours (v: V::ConfigVersion) {prefix} {STEP_INPUT_MATCH} \
             {{ {body} }}"
        )
    };
    let both = "forall trace a of V forall trace b of V";
    for body in [
        "always holds(v.versionNumber @ b = v.versionNumber @ a)",
        "always holds(v.versionNumber @ a = v.versionNumber @ b)",
    ] {
        assert_eq!(
            classes(&clause(&["V"], &with(both, body))),
            expected,
            "{body}"
        );
    }
    let empty: Vec<Vec<String>> = Vec::new();
    assert_eq!(
        classes(&clause(
            &["V"],
            &with(
                both,
                "always holds(v.versionNumber @ a <= v.versionNumber @ b)"
            )
        )),
        empty
    );
    assert_eq!(
        classes(&clause(
            &["V"],
            &with(
                "forall trace a of V fair weak V::ConfigVersion::step; forall trace b of V",
                READ_A_B
            )
        )),
        empty
    );
    assert_eq!(classes(&clause(&["S", "N"], SAVES_POWER)), empty);
    // A symmetric body under a `match` that reads only one variable.
    assert_eq!(
        classes(&clause(
            &["V"],
            &format!(
                "hyper H using inf over behaviours (v: V::ConfigVersion) {both} \
                 match {{ a.step.op != stutter }} {{ {READ_A_B} }}"
            )
        )),
        empty
    );
    // A clause outside the universal forms has no group.
    assert_eq!(classes(&clause(&["V"], OPAQUE)), empty);
}

/// FR-174 "Canonicalisation": a copy group carries one generator for each pair
/// of consecutive variables of a class, with the atom each of the body's atoms
/// becomes under the exchange. `NonInterference` has one atom, which the swap
/// fixes; a body of one atom for each copy has two, which it exchanges; three
/// copies have two generators, of an atom for each pair of copies.
#[trace("TC-599")]
#[test]
fn a_copy_group_carries_the_exchange_of_the_body_s_atoms() {
    use qsl_semantics::check::CopyGenerator;
    let generator = |first, second, atoms: &[usize]| CopyGenerator {
        first,
        second,
        atoms: atoms.to_vec(),
    };
    assert_eq!(
        clause(&["V"], &non_interference()).copy_group().generators,
        [generator(0, 1, &[0])]
    );
    let each = "hyper H using inf over behaviours (v: V::ConfigVersion) \
         forall trace a of V forall trace b of V \
         { always (holds(v.versionNumber @ a = 0) and holds(v.versionNumber @ b = 0)) }";
    assert_eq!(
        clause(&["V"], each).copy_group().generators,
        [generator(0, 1, &[1, 0])]
    );
    let pairs = "hyper H using inf over behaviours (v: V::ConfigVersion) \
         forall trace a of V forall trace b of V forall trace c of V \
         { always (holds(v.versionNumber @ a = v.versionNumber @ b) \
         and holds(v.versionNumber @ b = v.versionNumber @ c) \
         and holds(v.versionNumber @ a = v.versionNumber @ c)) }";
    let three = clause(&["V"], pairs);
    assert_eq!(
        three.copy_group().classes,
        [["a", "b", "c"].map(str::to_owned).to_vec()]
    );
    // The atoms in arena order are `a = c`, `a = b` and `b = c`. Exchanging
    // `a` and `b` takes `a = c` to `b = c` and back, and fixes `a = b`;
    // exchanging `b` and `c` takes `a = c` to `a = b` and back, and fixes
    // `b = c`.
    assert_eq!(
        three.copy_group().generators,
        [generator(0, 1, &[2, 1, 0]), generator(1, 2, &[1, 0, 2])]
    );
    // A body one copy alone reads has none.
    let one = "hyper H using inf over behaviours (v: V::ConfigVersion) \
         forall trace a of V forall trace b of V \
         { always holds(v.versionNumber @ a = 0) }";
    assert!(clause(&["V"], one).copy_group().generators.is_empty());
}

/// FR-180-AC-1 (S3): `Det` over a vault whose `step` is bound to an
/// implementation has a code claim beside its model claim; with no binding it
/// has the model claim alone.
#[trace("TC-605")]
#[test]
fn a_bound_step_relation_has_a_code_claim_beside_its_model_claim() {
    use qsl_semantics::check::{
        AuthoredBinding, FrameBinding, RustParameter, RustPath, RustReceiver,
    };

    let det = relation(DET);
    let (_, step) = det.executions()[0].clone();
    let (_, package) = assemble(
        &["V"],
        INFINITE,
        "function noop using v(): Boolean pure { true }",
    );
    let graph = package
        .check(CheckingLimits::default())
        .expect("the vault compiles");
    let binding = AuthoredBinding::Frame {
        key: step.clone(),
        value: FrameBinding {
            function: RustPath::new("vault::step"),
            receiver: RustReceiver::SelfValue,
            parameters: [(
                quire_exact::Identifier::new("i").expect("an identifier"),
                RustParameter::new("input"),
            )]
            .into_iter()
            .collect(),
        },
    };
    let bound = graph
        .check_abstraction(&[binding])
        .expect("the binding checks");
    let code = det.code_claim(&bound).expect("every operation is bound");
    assert_eq!(code.kind(), Capability::OperationContract);
    assert_ne!(code.kind(), det.model_claim().kind());
    assert_eq!(det.model_claim().kind(), Capability::TemporalSatisfaction);

    let unbound = graph
        .check_abstraction(&[])
        .expect("the empty relation checks");
    assert!(det.code_claim(&unbound).is_none());
}

/// FR-361-AC-4: a `hyper` clause and a `relation` are gated by neither layer.
/// Under the value profile and under the model profile alike, both clauses
/// assemble without a layer refusal.
#[trace("TC-1015", "FR-361-AC-4")]
#[test]
fn hyper_clauses_are_ungated_by_the_model_profile() {
    for layer in ["quire.value.complete/v1", "quire.model.complete/v1"] {
        for declaration in [non_interference(), DET.to_owned()] {
            let (_, package) = assemble_under(layer, &["V"], INFINITE, &declaration);
            let checked = package.check(CheckingLimits::default());
            if declaration == non_interference() {
                // A hyper clause over `behaviours` is emitted (FR-488).
                checked.unwrap_or_else(|refusals| panic!("{layer}: {refusals:?}"));
                continue;
            }
            checked.unwrap_or_else(|refusals| panic!("{layer}: {declaration}: {refusals:?}"));
        }
    }
}

/// FR-361-AC-4: the bodies are examined now, so the gate is not vacuous. An
/// ill-typed relation body and an ill-typed hyper atom refuse as body faults
/// under the model profile and the value profile alike, and never as a layer
/// refusal.
#[trace("TC-1015", "FR-361-AC-4")]
#[test]
fn an_examined_body_refuses_by_its_own_fault_under_either_profile() {
    for layer in ["quire.value.complete/v1", "quire.model.complete/v1"] {
        for declaration in [
            "relation Bad using inf over (x: V::ConfigVersion::step, y: V::ConfigVersion::step) \
             { x.nope = 1 }"
                .to_owned(),
            "hyper H using inf over behaviours (v: V::ConfigVersion) forall trace a of V \
             { always holds(v.nope @ a = 1) }"
                .to_owned(),
        ] {
            let (_, package) = assemble_under(layer, &["V"], INFINITE, &declaration);
            let refusals = package
                .check(CheckingLimits::default())
                .expect_err("an ill-typed body refuses");
            let [CheckRefusal {
                cause: CheckCause::Hyper(cause),
                ..
            }] = &refusals[..]
            else {
                panic!("{layer}: {declaration}: one body fault, not {refusals:?}");
            };
            assert!(
                matches!(**cause, HyperCause::BodyMissing { .. }),
                "{layer}: {declaration}: {cause:?}"
            );
        }
    }
}

const BAD_RELATION: &str = "relation Bad using inf over \
     (x: V::ConfigVersion::step, y: V::ConfigVersion::step) { ";

/// FR-172 "Relations over model executions": the body checks under the value
/// profile. A body reading a member the operation or its receiver does not
/// declare, or a variable nothing binds, refuses `missing_declaration`; a
/// body that is not Boolean or compares unlike types refuses `ill_typed`; and
/// a body over each execution's argument, result, pre-state and post-state
/// checks and is kept.
#[trace("TC-597")]
#[test]
fn a_relation_body_is_typed_under_the_value_profile() {
    let (text, cause) = hyper_cause(
        &["V"],
        &format!("{BAD_RELATION}x.nonexistent = zzz and q.r = 3 }}"),
    );
    assert_eq!(
        cause,
        HyperCause::BodyMissing {
            name: "nonexistent".into(),
            span: first(&text, "x.nonexistent"),
        }
    );
    assert_eq!(
        (cause.code(), cause.tag()),
        (Code::MissingDeclaration, "missing-name")
    );
    for (body, needle) in [
        ("zzz = 1", "zzz"),
        ("q.r = 3", "q"),
        ("x.result = 1", "x.result"),
        ("x.self.nope = 1", "x.self.nope"),
    ] {
        let (text, cause) = hyper_cause(&["V"], &format!("{BAD_RELATION}{body} }}"));
        let start = text.find(body).expect("the body") + body.find(needle).expect("the needle");
        assert!(
            matches!(&cause, HyperCause::BodyMissing { span, .. } if span.start == start),
            "{body}: {cause:?}"
        );
    }
    for body in ["x.i", "x.i = true", "x.self = 1"] {
        let (_, cause) = hyper_cause(&["V"], &format!("{BAD_RELATION}{body} }}"));
        assert_eq!(
            (cause.code(), cause.tag()),
            (Code::IllTyped, "type-mismatch"),
            "{body}: {cause:?}"
        );
    }

    let checked = relation(DET);
    assert!(matches!(
        checked.body().root_node(),
        qsl_forms::ExprNode::Binary {
            operator: qsl_forms::BinaryOperator::Implies,
            ..
        }
    ));
}

/// FR-172 "Indexed atoms and step labels": an atom and a `match` conjunct
/// are typed, so an unknown root, an unknown member, an atom that is not
/// Boolean, a conjunct that compares an operation with a receiver key and a
/// conjunct that is not Boolean each refuse.
#[trace("TC-597")]
#[test]
fn atoms_and_match_conjuncts_are_typed() {
    let atom = |condition: &str| {
        format!(
            "hyper H using inf over behaviours (v: V::ConfigVersion) forall trace a of V \
             {{ always holds({condition}) }}"
        )
    };
    let conjunct = |condition: &str| {
        format!(
            "hyper H using inf over behaviours (v: V::ConfigVersion) forall trace a of V \
             match {{ {condition} }} {{ always holds(true) }}"
        )
    };
    for (declaration, needle, missing) in [
        (atom("w.versionNumber = 1"), "w.versionNumber", true),
        (atom("zzz"), "zzz", true),
        (atom("v.nope @ a = 1"), "v.nope", true),
        (atom("v.versionNumber @ a"), "v.versionNumber @ a", false),
        (
            atom("v.versionNumber @ a = true"),
            "v.versionNumber @ a = true",
            false,
        ),
        (
            conjunct("a.step.op = a.step.receiver"),
            "a.step.op = a.step.receiver",
            false,
        ),
        (conjunct("a.step.op"), "a.step.op", false),
    ] {
        let (text, cause) = hyper_cause(&["V"], &declaration);
        let at = first(&text, needle);
        match (&cause, missing) {
            (HyperCause::BodyMissing { span, .. }, true) => assert_eq!(span.start, at.start),
            (HyperCause::BodyIllTyped { span }, false) => assert_eq!(*span, at, "{declaration}"),
            other => panic!("{declaration}: {other:?}"),
        }
    }
    // The typed positives: an argument compared with its own type, an
    // operation with `stutter`.
    clause(
        &["V"],
        "hyper H using inf over behaviours (v: V::ConfigVersion) forall trace a of V \
             forall trace b of V match { a.step.op != stutter and \
             a.step.ConfigVersion::step.i = b.step.ConfigVersion::step.i } \
             { always holds(v.versionNumber @ a + 1 >= v.versionNumber @ b) }",
    );
}

/// ADR-023 HS-2, HS-3, HS-8: a trace variable, an object parameter and an
/// execution variable are each named once.
#[trace("TC-597")]
#[test]
fn a_name_used_twice_refuses_at_the_second() {
    let (text, cause) = hyper_cause(
        &["V"],
        "hyper H using inf over behaviours (v: V::ConfigVersion) \
         forall trace a of V exists trace a of V { always holds(v.versionNumber @ a = 1) }",
    );
    let second = text.rfind("trace a").expect("second quantifier") + "trace ".len();
    assert_eq!(
        cause,
        HyperCause::DuplicateName {
            name: "a".into(),
            span: Span {
                start: second,
                end: second + 1
            }
        }
    );
    assert_eq!(
        (cause.code(), cause.tag()),
        (Code::ConflictingDeclaration, "duplicate-declaration")
    );
    let (text, cause) = hyper_cause(
        &["V"],
        "hyper H using inf over behaviours (v: V::ConfigVersion, v: V::ConfigVersion) \
         forall trace a of V { always holds(true) }",
    );
    let second = text.rfind("v: V::ConfigVersion").expect("second parameter");
    assert_eq!(
        cause,
        HyperCause::DuplicateName {
            name: "v".into(),
            span: Span {
                start: second,
                end: second + 1
            }
        }
    );
    let (text, cause) = hyper_cause(
        &["V"],
        "relation R using inf over (x: V::ConfigVersion::step, x: V::ConfigVersion::step) \
         { true }",
    );
    let second = text.rfind("x: V::").expect("second execution");
    assert_eq!(
        cause,
        HyperCause::DuplicateName {
            name: "x".into(),
            span: Span {
                start: second,
                end: second + 1
            }
        }
    );
}

/// What the unit's check says of the clauses of `declarations` over the
/// vault: `Ok` when each is emitted, otherwise the hyper causes found.
fn body_outcome(declarations: &str) -> Result<(), Vec<HyperCause>> {
    let (_, package) = assemble(&["V"], INFINITE, declarations);
    // Checked hyper clauses and step relations are emitted.
    let Err(refusals) = package.check(CheckingLimits::default()) else {
        return Ok(());
    };
    let mut faults = Vec::new();
    for refusal in refusals {
        match refusal.cause {
            CheckCause::Temporal(cause) if matches!(*cause, TemporalCause::NotEmitted { .. }) => {}
            CheckCause::Hyper(cause) => faults.push(*cause),
            other => panic!("{declarations}: neither a body fault nor not-emitted: {other:?}"),
        }
    }
    if faults.is_empty() {
        Ok(())
    } else {
        Err(faults)
    }
}

const PHASE: &str = "enum Phase { OPEN, SHUT }\n";
const FUNCTIONS: &str = "function f using v(n: Integer): Integer pure { n }\n\
     function abs using v(n: Integer): Integer pure { if n < 0 then -n else n }\n";

fn relation_over(body: &str) -> String {
    format!(
        "relation R using inf over (x: V::ConfigVersion::step, y: V::ConfigVersion::step) \
         {{ {body} }}"
    )
}

fn atom_over(condition: &str) -> String {
    format!(
        "hyper H using inf over behaviours (v: V::ConfigVersion) forall trace a of V \
         forall trace b of V {{ always holds({condition}) }}"
    )
}

/// FR-172 "Relations over model executions", "Indexed atoms and step labels":
/// a body is typed under the value profile, so the value forms a body may
/// use check: the members of a declared enum, calls of declared functions and
/// `let`.
#[trace("TC-597")]
#[test]
fn a_body_may_use_enum_members_declared_functions_and_let() {
    for (declarations, label) in [
        (
            format!("{PHASE}{}", relation_over("Phase::OPEN != Phase::SHUT")),
            "enum member in a relation",
        ),
        (
            format!("{PHASE}{}", atom_over("Phase::OPEN = Phase::OPEN")),
            "enum member in an atom",
        ),
        (
            format!("{FUNCTIONS}{}", relation_over("f(x.i) = f(y.i)")),
            "declared function",
        ),
        (
            format!("{FUNCTIONS}{}", relation_over("abs(x.i) >= 0")),
            "declared function with a conditional body",
        ),
        (relation_over("let n = x.i in n = y.i"), "let in a relation"),
        (
            atom_over("let n = v.versionNumber @ a in n = v.versionNumber @ b"),
            "let over an indexed read",
        ),
    ] {
        assert_eq!(body_outcome(&declarations), Ok(()), "{label}");
    }
}

/// The same forms refuse when they are misused: a member of no enum, an enum
/// compared with an integer, a call of no function or with the wrong
/// arguments, and a `let` binder used past its body.
#[trace("TC-597")]
#[test]
fn misused_enum_members_calls_and_let_binders_refuse() {
    let missing = |declarations: String| match body_outcome(&declarations) {
        Err(faults) => faults,
        Ok(()) => panic!("{declarations}: a refusal"),
    };
    for (declarations, code) in [
        (
            format!("{PHASE}{}", relation_over("Phase::NONE = Phase::OPEN")),
            Code::MissingDeclaration,
        ),
        (
            format!("{PHASE}{}", relation_over("Phase::OPEN = 1")),
            Code::IllTyped,
        ),
        (
            format!("{FUNCTIONS}{}", relation_over("g(x.i) = 1")),
            Code::MissingDeclaration,
        ),
        (
            format!("{FUNCTIONS}{}", relation_over("f(x.i, y.i) = 1")),
            Code::IllTyped,
        ),
        (
            format!("{FUNCTIONS}{}", relation_over("f(true) = 1")),
            Code::IllTyped,
        ),
        (
            relation_over("(let n = x.i in n = 1) and n = 1"),
            Code::MissingDeclaration,
        ),
        (relation_over("let n = true in n = x.i"), Code::IllTyped),
    ] {
        let faults = missing(declarations.clone());
        assert_eq!(faults.len(), 1, "{declarations}: {faults:?}");
        assert_eq!(faults[0].code(), code, "{declarations}: {faults:?}");
    }
}

/// Numbers of different kinds do not compare: an integer is not a rational,
/// and a number is not a quantity.
#[trace("TC-597")]
#[test]
fn numbers_of_different_kinds_do_not_compare() {
    for body in [
        "x.i = rational(1, 2)",
        "x.i + rational(1, 2) = x.i",
        "rational(1, 2) < x.i",
        "x.i div rational(1, 2) = 1",
    ] {
        let faults = body_outcome(&relation_over(body)).expect_err(body);
        assert_eq!(faults.len(), 1, "{body}: {faults:?}");
        assert_eq!(faults[0].code(), Code::IllTyped, "{body}");
    }
    let faults = body_outcome(&relation_over(
        "rational(1, 2) < rational(3, 4) and x.i + 1 = y.i",
    ))
    .expect_err("rational literals need a declared expected range");
    assert!(matches!(
        faults.as_slice(),
        [HyperCause::BodyAmbiguousLiteral { .. }]
    ));
}

/// Whether the value typer admits `body` as the body of a Boolean function
/// over `parameters`, among `declarations`.
fn value_typer_admits(declarations: &str, parameters: &str, body: &str) -> bool {
    let (_, package) = assemble(
        &["V"],
        INFINITE,
        &format!("{declarations}function probe using v({parameters}): Boolean pure {{ {body} }}"),
    );
    package.check(CheckingLimits::default()).is_ok()
}

/// A relation body admits the same shared value forms as a Boolean function
/// body checked by the value typer.
#[trace("TC-597")]
#[test]
fn a_shared_value_form_is_admitted_as_the_value_typer_admits_it() {
    let declarations = format!("{PHASE}{FUNCTIONS}");
    for body in [
        "Phase::OPEN != Phase::SHUT",
        "Phase::NONE = Phase::OPEN",
        "Phase::OPEN = 1",
        "f(1) = f(2)",
        "abs(0 - 3) >= 0",
        "g(1) = 1",
        "f(1, 2) = 1",
        "f(true) = 1",
        "let n = 1 in n = 2",
        "(let n = 1 in n = 1) and n = 1",
        "let n = true in n = 1",
        "let n = 1 in let n = true in n = 1",
        "let n = 1 in let n = true in n",
        "let n = true in let n = 1 in n = 1",
        "(let n = 1 in n = 1) and (let n = true in n)",
        "1 = rational(1, 2)",
        "1 + rational(1, 2) = 1",
        "1 + 2 = 3",
    ] {
        let hyper = body_outcome(&format!("{declarations}{}", relation_over(body))).is_ok();
        let value = value_typer_admits(&declarations, "", body);
        assert_eq!(hyper, value, "{body}: hyper {hyper}, value {value}");
    }
}

/// FR-172-AC-6: relation and hyper atom bodies use the same value typer for
/// rational obligations, contextual literals, and root shadowing.
#[trace("TC-597", "FR-172-AC-6")]
#[test]
fn relation_and_hyper_atom_bodies_share_value_typer_refusals() {
    let half = "function half using v(n: Integer): Rational[0, 1; 1, 2] pure { rational(1, 2) }\n";
    let out_of_range = "half(x.i) < rational(3, 4)";
    let declaration = format!("{half}{}", relation_over(out_of_range));
    let (text, fault) = hyper_cause(&["V"], &declaration);
    assert!(
        matches!(
            fault,
            HyperCause::BodyUnproved {
                obligation: qsl_semantics::check::Obligation::RationalRange,
                ..
            }
        ),
        "{fault:?}"
    );
    assert_eq!(
        (fault.code(), fault.tag()),
        (Code::UndefinedExpression, "unproved-range")
    );
    assert_eq!(fault.span(), Some(first(&text, "rational(3, 4)")));

    let ambiguous = "rational(1, 2) < rational(3, 4)";
    for declaration in [relation_over(ambiguous), atom_over(ambiguous)] {
        let (text, fault) = hyper_cause(&["V"], &declaration);
        assert!(
            matches!(fault, HyperCause::BodyAmbiguousLiteral { .. }),
            "{fault:?}"
        );
        assert_eq!(
            (fault.code(), fault.tag()),
            (Code::IllTyped, "ambiguous-literal")
        );
        assert_eq!(fault.span(), Some(first(&text, "rational(1, 2)")));
    }

    for (declaration, shadowed) in [
        (atom_over("let a = 1 in a = 1"), "a"),
        (atom_over("let v = 1 in v = 1"), "v"),
        (relation_over("let x = 1 in x = 1"), "x"),
    ] {
        let faults = body_outcome(&declaration).expect_err("a local may not shadow a hyper root");
        assert!(
            matches!(faults.as_slice(), [HyperCause::BodyShadows { name, .. }] if name == shadowed),
            "{faults:?}"
        );
    }
}

/// A `let` may not rebind a binder in scope or a variable of the clause, as
/// the value typer refuses a `let` that rebinds a parameter.
#[trace("TC-597")]
#[test]
fn a_let_binder_may_not_shadow_a_binder_or_a_variable() {
    for body in [
        "let n = 1 in let n = true in n = 1",
        "let n = 1 in let n = true in n",
        "let x = 1 in x = 1",
        "let y = 1 in y = 1",
    ] {
        let faults = body_outcome(&relation_over(body)).expect_err(body);
        assert!(
            faults
                .iter()
                .any(|fault| matches!(fault, HyperCause::BodyShadows { .. })),
            "{body}: {faults:?}"
        );
        assert_eq!(
            faults[0].code(),
            Code::AmbiguousDeclaration,
            "{body}: {faults:?}"
        );
    }
    assert!(!value_typer_admits("", "x: Integer", "let x = 1 in x = 1"));
    // Two lets in separate operands do not shadow each other.
    body_outcome(&relation_over(
        "(let n = 1 in n = 1) and (let n = true in n)",
    ))
    .expect("sibling binders");
}

/// The one `holds` atom of `clause`'s body.
fn the_atom(clause: &CheckedHyperClause) -> qsl_semantics::check::FormulaNodeId {
    let nodes = clause.body().nodes();
    let atoms: Vec<_> = nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| matches!(node, qsl_semantics::check::FormulaNode::Holds { .. }))
        .collect();
    let [(index, _)] = atoms[..] else {
        panic!("one holds atom");
    };
    clause
        .body()
        .node_id(index)
        .expect("the atom is in the arena")
}

fn behaviours(quantifiers: &str, conjuncts: &str, body: &str) -> String {
    format!(
        "hyper H using inf over behaviours (v: V::ConfigVersion) {quantifiers} \
         {conjuncts} {{ {body} }}"
    )
}

/// FR-487-AC-1: the atom's slots are the (trace, parameter) pairs in
/// quantifier-then-parameter order, its condition is typed over them, each
/// model read is attributed to its component, and a second check gives the
/// same clause.
#[trace("TC-1264", "FR-487-AC-1")]
#[test]
fn an_atom_lowers_to_a_condition_over_per_trace_parameter_slots() {
    let declaration = non_interference();
    let noninterference = clause(&["V"], &declaration);
    let slots: Vec<_> = noninterference
        .atom_slots()
        .iter()
        .map(|slot| (slot.trace, slot.parameter))
        .collect();
    assert_eq!(slots, [(0, 0), (1, 0)]);
    let atom = the_atom(&noninterference);
    let (_, count) = noninterference.condition(atom).expect("a lowered atom");
    assert_eq!(count, 2);
    let components: std::collections::BTreeSet<usize> = noninterference
        .component_reads(atom)
        .expect("the atom's reads")
        .values()
        .copied()
        .collect();
    assert_eq!(components, [0, 1].into());
    // A node that is not a `holds` atom has no condition.
    let root = noninterference.body().root();
    assert!(noninterference.condition(root).is_none() || root == atom);

    // Two parameters over two models: each trace pairs with its model's parameter.
    let power = clause(&["S", "N"], SAVES_POWER);
    let slots: Vec<_> = power
        .atom_slots()
        .iter()
        .map(|slot| (slot.trace, slot.parameter))
        .collect();
    assert_eq!(slots, [(0, 0), (1, 1)]);
    assert_eq!(power.condition(the_atom(&power)).expect("lowered").1, 2);

    // The check is deterministic: a second check gives the same lowering.
    let again = clause(&["V"], &declaration);
    assert_eq!(again.identity(), noninterference.identity());
    assert_eq!(again.atom_slots(), noninterference.atom_slots());
    assert_eq!(
        again.condition(the_atom(&again)).map(|(_, slots)| slots),
        Some(2)
    );
    let (first_condition, _) = noninterference.condition(atom).expect("lowered");
    let (second_condition, _) = again.condition(the_atom(&again)).expect("lowered");
    assert_eq!(first_condition.body(), second_condition.body());
    assert_eq!(
        noninterference.component_reads(atom),
        again.component_reads(the_atom(&again))
    );

    // The condition is an equality of two reads, with no index node, and the
    // left read (`@ a`) is trace 0's component and the right read trace 1's.
    let NodeKind::Equality(..) = first_condition.kind() else {
        panic!("an equality, not {:?}", first_condition.kind());
    };
    let [left, right] = first_condition.children()[..] else {
        panic!("two operands");
    };
    assert_eq!(
        std::mem::discriminant(left.kind()),
        std::mem::discriminant(right.kind())
    );
    let reads = noninterference.component_reads(atom).expect("the reads");
    assert_eq!(reads.len(), 2);
    assert_eq!(reads.get(left.location()), Some(&0));
    assert_eq!(reads.get(right.location()), Some(&1));
}

/// FR-487 "Lowering a match conjunct": a conjunct's slots are its step reads, and a label misused
/// by the form around it refuses `ill_typed`/`type-mismatch`.
#[trace("TC-1264")]
#[test]
fn a_match_conjunct_lowers_to_slots_and_a_misused_label_refuses() {
    let noninterference = clause(&["V"], &non_interference());
    let [operations, inputs] = noninterference.mu_universal() else {
        panic!("two conjuncts");
    };
    assert_eq!(
        operations.slots(),
        [
            StepRead::Operation { trace: "a".into() },
            StepRead::Operation { trace: "b".into() }
        ]
    );
    assert_eq!(operations.condition().1, 2);
    assert_eq!(inputs.slots().len(), 2);
    assert_eq!(inputs.condition().1, 2);

    let constant = clause(
        &["V"],
        &behaviours(
            "forall trace a of V forall trace b of V",
            "match { a.step.op = V::ConfigVersion::step and b.step.op = stutter }",
            "always holds(true)",
        ),
    );
    let [step, stutter] = constant.mu_universal() else {
        panic!("two conjuncts");
    };
    assert_eq!(step.slots(), [StepRead::Operation { trace: "a".into() }]);
    assert_eq!(step.condition().1, 1);
    assert_eq!(stutter.slots(), [StepRead::Operation { trace: "b".into() }]);
    assert_eq!(stutter.condition().1, 1);

    let receivers = clause(
        &["V"],
        &behaviours(
            "forall trace a of V forall trace b of V",
            "match { a.step.receiver = b.step.receiver }",
            "always holds(true)",
        ),
    );
    let [receiver] = receivers.mu_universal() else {
        panic!("one conjunct");
    };
    assert_eq!(receiver.slots().len(), 2);
    assert_eq!(receiver.condition().1, 2);

    for (aliases, conjunct) in [
        (&["V"][..], "a.step.op = \"x\""),
        (&["V"][..], "size(a.step.op) > 0"),
    ] {
        let declaration = behaviours(
            "forall trace a of V forall trace b of V",
            &format!("match {{ {conjunct} }}"),
            "always holds(true)",
        );
        let (text, cause) = hyper_cause(aliases, &declaration);
        let at = first(&text, conjunct);
        assert!(
            matches!(cause, HyperCause::BodyIllTyped { span } if span == at),
            "{conjunct}: {cause:?}"
        );
    }
    // Labels of two models do not compare.
    let (text, cause) = hyper_cause(
        &["S", "N"],
        "hyper H using inf over behaviours (d: S::ConfigVersion) \
         forall trace s of S forall trace n of N match { s.step.op = n.step.op } \
         { always holds(true) }",
    );
    let at = first(&text, "s.step.op = n.step.op");
    assert!(matches!(cause, HyperCause::BodyIllTyped { span } if span == at));
}

/// FR-487-AC-10: presence is a fact about one component.
#[trace("TC-1264", "FR-487-AC-10")]
#[test]
fn presence_proved_in_one_component_does_not_cover_another() {
    let atom = |body: &str| {
        behaviours(
            "forall trace a of V forall trace b of V",
            "",
            &format!("always holds({body})"),
        )
    };
    clause(
        &["V"],
        &atom("present(v.parent @ a) and value(v.parent @ a) = value(v.parent @ a)"),
    );
    let (_, cause) = refusal(
        &["V"],
        &atom("present(v.parent @ a) and value(v.parent @ b) = value(v.parent @ a)"),
    );
    assert_eq!(cause.code(), Code::UndefinedExpression, "{cause:?}");
    assert_eq!(cause.cause(), Some("unproved-presence"), "{cause:?}");
}

/// FR-172-AC-5: a qualified operation name beside `t.step.op` is a label
/// constant; elsewhere or misresolved it refuses.
#[trace("TC-597", "FR-172-AC-5")]
#[test]
fn a_label_constant_checks_beside_its_trace_s_operation_and_refuses_elsewhere() {
    let conjunct = |text: &str| {
        behaviours(
            "forall trace a of V forall trace b of V",
            &format!("match {{ {text} }}"),
            "always holds(true)",
        )
    };
    for accepted in [
        "a.step.op = V::ConfigVersion::step and b.step.op = stutter",
        "V::ConfigVersion::step != a.step.op",
        "a.step.op != V::ConfigVersion::step",
    ] {
        clause(&["V", "N"], &conjunct(accepted));
    }
    for (text, refused) in [
        (
            "a.step.op = N::ConfigVersion::tick",
            "N::ConfigVersion::tick",
        ),
        (
            "a.step.receiver = V::ConfigVersion::step",
            "V::ConfigVersion::step",
        ),
        ("V::ConfigVersion::step", "V::ConfigVersion::step"),
    ] {
        let (unit, cause) = hyper_cause(&["V", "N"], &conjunct(text));
        assert!(
            matches!(cause, HyperCause::BodyIllTyped { span } if span.start <= first(&unit, refused).start && first(&unit, refused).end <= span.end),
            "{text}: {cause:?}"
        );
    }
    let (_, cause) = hyper_cause(&["V", "N"], &conjunct("a.step.op = V::ConfigVersion::nope"));
    assert!(matches!(cause, HyperCause::BodyMissing { .. }), "{cause:?}");
    let (_, cause) = hyper_cause(
        &["V", "N"],
        &conjunct("a.step.op = V::ConfigVersion::versionNumber"),
    );
    assert!(
        matches!(cause, HyperCause::MatchReadsState { .. }),
        "{cause:?}"
    );
}

/// FR-172 "Match" calls: a call in a conjunct takes step labels as its arguments, and
/// an argument that reads state refuses.
#[trace("TC-1264")]
#[test]
fn a_conjunct_may_call_a_declared_function_of_step_labels() {
    let function = "function f using v(x: Option<Int[0, 1000]>): Boolean pure { present(x) }\n";
    let declaration = |argument: &str| {
        format!(
            "{function}{}",
            behaviours(
                "forall trace a of V forall trace b of V",
                &format!("match {{ f({argument}) }}"),
                "always holds(true)",
            )
        )
    };
    let checked = clause(&["V"], &declaration("a.step.ConfigVersion::step.i"));
    assert_eq!(checked.mu_universal().len(), 1);
    let declaration = declaration("v.versionNumber @ a");
    let (unit, cause) = hyper_cause(&["V"], &declaration);
    assert!(
        matches!(cause, HyperCause::MatchReadsState { span } if span == first(&unit, "v.versionNumber @ a")),
        "{cause:?}"
    );
}

/// FR-172-AC-8: `@ a` scopes over the compound operand it follows, so the
/// reads inside carry `a`; a read no `@` encloses refuses.
#[trace("TC-1264", "FR-172-AC-8")]
#[test]
fn an_at_scopes_over_a_compound_operand() {
    let atom = |body: &str| {
        behaviours(
            "forall trace a of V forall trace b of V",
            "",
            &format!("always holds({body})"),
        )
    };
    clause(
        &["V"],
        &atom("(let r = v.parent in present(r) and value(r) = value(r)) @ a"),
    );
    let (unit, cause) = hyper_cause(&["V"], &atom("present(v.parent @ a) and present(v.parent)"));
    assert!(
        matches!(cause, HyperCause::UnindexedRead { span } if span == after(&unit, "present(v.parent @ a) and present(", "v.parent")),
        "{cause:?}"
    );
}

/// FR-487 "Lowering a match conjunct": the slots of a conjunct carry the types the labels have: an
/// operation is a `String`, an argument an option of its parameter's type.
#[trace("TC-1264")]
#[test]
fn a_conjunct_s_slots_have_the_types_of_their_labels() {
    let checked = clause(&["V"], &non_interference());
    let [operations, inputs] = checked.mu_universal() else {
        panic!("two conjuncts");
    };
    assert!(operations
        .slots()
        .iter()
        .all(|slot| matches!(slot, StepRead::Operation { .. })));
    for slot in inputs.slots() {
        let StepRead::Argument(read) = slot else {
            panic!("an argument read, not {slot:?}");
        };
        assert_eq!(
            (read.operation.operation.as_str(), read.parameter.as_str()),
            ("step", "i")
        );
    }
    let receivers = clause(
        &["V"],
        &behaviours(
            "forall trace a of V forall trace b of V",
            "match { a.step.receiver = b.step.receiver }",
            "always holds(true)",
        ),
    );
    for slot in receivers.mu_universal()[0].slots() {
        assert!(matches!(slot, StepRead::Receiver { .. }), "{slot:?}");
    }
}

/// FR-487 Outputs: a conjunct refused by the value typer or the definedness
/// check is refused with a source region.
#[trace("TC-1264")]
#[test]
fn a_refused_conjunct_has_a_source_region() {
    let declaration = behaviours(
        "forall trace a of V forall trace b of V",
        "match { value(a.step.ConfigVersion::step.i) = 1 }",
        "always holds(true)",
    );
    let (text, package) = assemble(&["V"], INFINITE, &declaration);
    let outcomes = check_hyper_clauses(&package);
    let [Err(StageFailure::Refused(refusal))] = &outcomes[..] else {
        panic!("one refusal, not {outcomes:?}");
    };
    assert_eq!(
        refusal.cause.cause(),
        Some("unproved-presence"),
        "{refusal:?}"
    );
    let region = package
        .region(&refusal.location)
        .expect("the refusal has a region");
    let conjunct = first(&text, "value(a.step.ConfigVersion::step.i) = 1");
    assert_eq!(
        (region.start(), region.end()),
        (conjunct.start as u64, conjunct.end as u64)
    );
}

/// The checking half of FR-487-AC-9 (its evaluation half is QSL-1085) and
/// FR-172-AC-8: `@ a` scopes over the `let`
/// and the `deref`, so every model read of the atom reads component `a`, and
/// an `@ b` moves them all to `b`; a read no `@` encloses refuses.
#[trace("TC-1264", "FR-487-AC-9")]
#[trace("TC-1264", "FR-172-AC-8")]
#[test]
fn an_indexed_let_and_deref_read_one_component() {
    let atom = |body: &str| {
        behaviours(
            "forall trace a of V forall trace b of V",
            "",
            &format!("always holds({body})"),
        )
    };
    let components = |trace: &str| {
        let linked = clause(
            &["V"],
            &atom(&format!(
                "(let r = v.parent in present(r) and deref(value(r)).versionNumber = 1) @ {trace}"
            )),
        );
        let reads = linked
            .component_reads(the_atom(&linked))
            .expect("the reads")
            .values()
            .copied()
            .collect::<Vec<_>>();
        assert!(
            reads.len() >= 2,
            "the read of `parent` and of `versionNumber`"
        );
        reads
    };
    assert!(components("a").iter().all(|component| *component == 0));
    assert!(components("b").iter().all(|component| *component == 1));
    // The `.versionNumber` read is enclosed by no `@`.
    let (text, cause) = hyper_cause(
        &["V"],
        &atom("deref(value(v.parent @ a)).versionNumber = 1"),
    );
    assert!(
        matches!(cause, HyperCause::UnindexedRead { span } if text[span.start..span.end].ends_with("versionNumber")),
        "{cause:?}"
    );
}

/// FR-172-AC-7: a receiver label whose operation set spans two object types
/// is refused at the read rather than being assigned one trace's type.
#[trace("TC-1264", "FR-172-AC-7")]
#[test]
fn receiver_read_over_two_object_types_is_refused_at_the_read() {
    let declaration = behaviours(
        "forall trace a of V forall trace b of S",
        "match { a.step.receiver = b.tick.receiver }",
        "always holds(true)",
    );
    let (text, cause) = hyper_cause(&["V", "S"], &declaration);
    let code = cause.code();
    let tag = cause.tag();
    assert_eq!(
        (code, tag),
        (Code::UnsupportedConstruct, "expression-form"),
        "{cause:?}"
    );
    let (start, end) = match cause {
        HyperCause::MatchReadsState { span } => (span.start, span.end),
        HyperCause::BodyUnsupported { span } => (span.start, span.end),
        HyperCause::BodyIllTyped { span } => (span.start, span.end),
        other => panic!("the receiver read is the refusal locus: {other:?}"),
    };
    assert!(
        text[start..end].contains("receiver"),
        "the refusal region {start}..{end} names the receiver read"
    );
}

/// FR-487-AC-10: definedness is per component. A reference presence proved in
/// component `a` does not prove the same reference in component `b`.
#[trace("TC-1264", "FR-487-AC-10")]
#[test]
fn a_proof_about_a_read_in_one_component_does_not_cover_it_in_another() {
    let atom = |body: &str| {
        behaviours(
            "forall trace a of V forall trace b of V",
            "",
            &format!("always holds({body})"),
        )
    };
    clause(
        &["V"],
        &atom(
            "present(v.parent @ a) and present(deref(value(v.parent @ a)).parent @ a) \
             and value(deref(value(v.parent @ a)).parent @ a) = value(v.parent @ a)",
        ),
    );
    // The same reference's attribute, proved present in `a` and read in `b`.
    let (_, cause) = refusal(
        &["V"],
        &atom(
            "present(v.parent @ a) and present(deref(value(v.parent @ a)).parent @ a) \
             and value(deref(value(v.parent @ a)).parent @ b) = value(v.parent @ a)",
        ),
    );
    assert_eq!(cause.cause(), Some("unproved-presence"), "{cause:?}");
}

/// FR-487 "Dependencies": the value typer decides the forms of an atom, so a
/// call of a declared function checks in an atom.
#[trace("TC-1264")]
#[test]
fn an_atom_may_call_a_declared_function() {
    let declaration = format!(
        "function bump using v(n: Int[0, 1000]): Integer pure {{ n + 1 }}\n{}",
        behaviours(
            "forall trace a of V forall trace b of V",
            "",
            "always holds(bump(v.versionNumber @ a) = bump(v.versionNumber @ b))",
        )
    );
    clause(&["V"], &declaration);
}
