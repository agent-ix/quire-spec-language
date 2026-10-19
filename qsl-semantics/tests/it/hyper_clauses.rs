// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-172, FR-173, FR-174 and FR-180 (S3): the `TemporalTrace` family's check
//! of a `hyper` clause over `behaviours` and of a `relation` over model
//! executions (TC-597, TC-598, TC-599, TC-605). The vault is the
//! `ConfigVersion` fixture package: `versionNumber` stands for the vault's
//! low field and `step(i)` and `mix()` for its operations; each device model is
//! the same shape under its own package identity, with `tick(load)`.

use ix_trace_rs::trace;
use qsl_foundation::diagnostic::{Code, StageFailure};
use qsl_foundation::digest::WireNodeId;
use qsl_foundation::Span;
use qsl_semantics::check::{
    check_hyper_clauses, Capability, CheckCause, CheckRefusal, CheckedHyper, CheckedHyperClause,
    CheckedModelRelation, HyperCause, HyperForm, HyperTraceFamily, HyperUnsupported,
    PackageDeclarations, StepRead, TemporalCause, TemporalProfile,
};
use qsl_semantics::family::{ClaimExtent, FamilyContract};
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::intake::{admit_unit, package_input};
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
        let input = package_input([document.as_slice()]);
        let [(digest, _)] = input.iter().collect::<Vec<_>>()[..] else {
            panic!("one document");
        };
        selections.push_str(&format!(
            "model {alias} = {identity:?} version \"1.0.0\" digest \"sha256-jcs:{}\";\n",
            qsl_semantics::model::key::hex(digest)
        ));
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
    let package = PackageDeclarations::assemble(raw, built, models, Vec::new())
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
        (_, Ok(CheckedHyper::Relation(relation))) => relation,
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

fn unbounded(clause: &CheckedHyper) -> bool {
    let [claim] = HyperTraceFamily::requirements(clause)
        .try_into()
        .expect("one claim");
    let params: Vec<WireNodeId> = (0..4_u8)
        .map(|index| WireNodeId::from_digest([index + 2; 32]))
        .collect();
    let record = claim
        .requirements(WireNodeId::from_digest([1; 32]), &params)
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
    assert!(unbounded(&CheckedHyper::Relation(det)));
}

/// FR-173-AC-2: each clause outside the admitted forms is `Other` and names
/// the rule that placed it there.
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
/// Under the value profile and under the model profile alike, the unit
/// assembles and its check refuses only that the clause is not emitted, with
/// no layer refusal and no model-construct family.
#[trace("TC-1015", "FR-361-AC-4")]
#[test]
fn hyper_clauses_are_ungated_by_the_model_profile() {
    for layer in ["quire.value.complete/v1", "quire.model.complete/v1"] {
        for declaration in [non_interference(), DET.to_owned()] {
            let (_, package) = assemble_under(layer, &["V"], INFINITE, &declaration);
            let refusals = package
                .check(CheckingLimits::default())
                .expect_err("a checked clause is refused as not emitted");
            let [CheckRefusal {
                cause: CheckCause::Temporal(cause),
                ..
            }] = &refusals[..]
            else {
                panic!("{layer}: {declaration}: one temporal refusal, not {refusals:?}");
            };
            assert!(
                matches!(**cause, TemporalCause::NotEmitted { .. }),
                "{layer}: {declaration}: {cause:?}"
            );
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
/// vault: `Ok` when each is refused only as not emitted (every body typed),
/// otherwise the hyper causes found.
fn body_outcome(declarations: &str) -> Result<(), Vec<HyperCause>> {
    let (_, package) = assemble(&["V"], INFINITE, declarations);
    let refusals = package
        .check(CheckingLimits::default())
        .expect_err("a checked clause is refused as not emitted");
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
    body_outcome(&relation_over(
        "rational(1, 2) < rational(3, 4) and x.i + 1 = y.i",
    ))
    .expect("same kinds compare");
}
