// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-171: S2 builds the forms of a `hyper` clause over `behaviours` and of a
//! `relation` over named executions, run on real complete-V1 source through
//! `qsl_cst::parse` and `build_unit` (TC-596).

use ix_trace_rs::trace;
use qsl_forms::{
    build_unit, BinaryOperator, DeclarationForm, ExprNode, ExpressionForm, FairnessKind,
    FormsCause, HyperBehavioursForm, HyperFiniteForm, ModelRelationForm, ParsedUnit,
    QuantifierKind, StepLabelForm, TemporalNodeForm,
};
use qsl_foundation::{SourceIdentity, Span};

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile inf = \"quire.temporal.infinite-trace/v1\";\n\
     model V = \"example/vault\" version \"1\" digest \
     \"sha256-jcs:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\";\n\
     model S = \"example/device-saving\" version \"1\" digest \
     \"sha256-jcs:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\";\n\
     model N = \"example/device-normal\" version \"1\" digest \
     \"sha256-jcs:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc\";\n";

/// ADR-023 section 1's noninterference clause.
pub const NON_INTERFERENCE: &str =
    "hyper NonInterference using inf over behaviours (v: V::Vault)\n\
     forall trace a of V\n\
     forall trace b of V\n\
     match { a.step.op = b.step.op and a.step.Vault::step.i = b.step.Vault::step.i }\n\
     {\n  always holds(v.l @ a = v.l @ b)\n}";

/// ADR-023 section 1's power comparison over two models.
const SAVES_POWER: &str =
    "hyper SavesPower using inf over behaviours (d: S::Device, e: N::Device)\n\
     forall trace s of S\n\
     forall trace n of N\n\
     match { s.step.Device::tick.load = n.step.Device::tick.load }\n\
     {\n  always holds(d.used @ s <= e.used @ n)\n}";

fn parse(text: &str) -> qsl_cst::ParsedSource {
    qsl_cst::parse(
        SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit")
}

fn build(declaration: &str) -> (String, ParsedUnit) {
    let text = format!("{HEADER}{declaration}\n");
    let parsed = parse(&text);
    assert!(parsed.is_admissible(), "{text}: {:?}", parsed.diagnostics());
    let unit = build_unit(&parsed).unwrap_or_else(|failure| panic!("{text}: {failure:?}"));
    (text, unit)
}

fn hyper(declaration: &str) -> (String, HyperBehavioursForm) {
    let (text, unit) = build(declaration);
    let [form] = unit.forms() else {
        panic!("one declaration: {:?}", unit.forms());
    };
    let DeclarationForm::HyperBehaviours(clause) = form.form().clone() else {
        panic!("a hyper clause over behaviours: {:?}", form.form());
    };
    (text, *clause)
}

fn slice(text: &str, span: Span) -> &str {
    &text[span.start..span.end]
}

/// Each indexed read of `condition` as `object.field @ trace`, sorted.
fn indexed_reads(condition: &ExpressionForm) -> Vec<String> {
    let mut reads: Vec<String> = condition
        .expression
        .iter()
        .filter_map(|node| match node.node() {
            ExprNode::Indexed { read, trace } => {
                let ExprNode::Field { operand, field } = node.at(*read).node() else {
                    panic!("a field read: {:?}", node.at(*read).node());
                };
                let ExprNode::Name(object) = node.at(*operand).node() else {
                    panic!("a name: {:?}", node.at(*operand).node());
                };
                Some(format!("{object}.{field} @ {trace}"))
            }
            _ => None,
        })
        .collect();
    reads.sort();
    reads
}

/// The `holds` conditions of a body, in arena order.
fn atoms(clause: &HyperBehavioursForm) -> Vec<&ExpressionForm> {
    clause
        .body
        .iter()
        .filter_map(|(_, node)| match node {
            TemporalNodeForm::Holds { condition, .. } => Some(condition),
            _ => None,
        })
        .collect()
}

/// FR-171-AC-1: `NonInterference` builds one object parameter, the two
/// quantifiers in order with empty fairness sets, a `match` form holding both
/// step-label equalities, no `align skip` and a body holding the indexed
/// atoms; `SavesPower` builds two parameters and the aliases `S` and `N`.
#[trace("TC-596", "FR-171-AC-1")]
#[test]
fn noninterference_and_saves_power_build_their_parameters_quantifiers_match_and_atoms() {
    let (text, clause) = hyper(NON_INTERFERENCE);
    assert_eq!(clause.name.name, "NonInterference");
    assert_eq!(clause.profile.alias, "inf");
    let [parameter] = &clause.parameters[..] else {
        panic!("one object parameter: {:?}", clause.parameters);
    };
    assert_eq!(parameter.name.name, "v");
    assert_eq!(slice(&text, parameter.name.span), "v");

    let quantifiers: Vec<_> = clause
        .quantifiers
        .iter()
        .map(|quantifier| {
            (
                quantifier.kind,
                quantifier.variable.name.as_str(),
                quantifier.alias.as_ref().map(|alias| alias.name.as_str()),
                quantifier.fairness.len(),
            )
        })
        .collect();
    assert_eq!(
        quantifiers,
        [
            (QuantifierKind::Forall, "a", Some("V"), 0),
            (QuantifierKind::Forall, "b", Some("V"), 0),
        ]
    );
    assert!(clause.align_skip.is_none());

    let matched = clause.match_block.as_ref().expect("a match block");
    let ExprNode::Binary {
        operator: BinaryOperator::And,
        left,
        right,
    } = matched.expression.root_node()
    else {
        panic!("a conjunction: {:?}", matched.expression.root_node());
    };
    let step_labels = |side| {
        let ExprNode::Binary {
            operator: BinaryOperator::Equal,
            left,
            right,
        } = matched.expression.at(side).node()
        else {
            panic!("an equality: {:?}", matched.expression.at(side).node());
        };
        [left, right].map(|operand| match matched.expression.at(*operand).node() {
            ExprNode::StepLabel { trace, label } => (trace.clone(), label.clone()),
            other => panic!("a step label: {other:?}"),
        })
    };
    assert_eq!(
        step_labels(*left),
        [
            ("a".to_owned(), StepLabelForm::Operation),
            ("b".to_owned(), StepLabelForm::Operation),
        ]
    );
    let argument = |trace: &str| {
        (
            trace.to_owned(),
            StepLabelForm::Argument {
                operation: "Vault::step".to_owned(),
                parameter: "i".to_owned(),
            },
        )
    };
    assert_eq!(step_labels(*right), [argument("a"), argument("b")]);

    let [atom] = atoms(&clause)[..] else {
        panic!("one atom: {:?}", atoms(&clause));
    };
    assert_eq!(indexed_reads(atom), ["v.l @ a", "v.l @ b"]);

    let (_, power) = hyper(SAVES_POWER);
    let parameters: Vec<_> = power
        .parameters
        .iter()
        .map(|parameter| parameter.name.name.as_str())
        .collect();
    assert_eq!(parameters, ["d", "e"]);
    let aliases: Vec<_> = power
        .quantifiers
        .iter()
        .map(|quantifier| quantifier.alias.as_ref().map(|alias| alias.name.as_str()))
        .collect();
    assert_eq!(aliases, [Some("S"), Some("N")]);
}

/// FR-171-AC-2: `forall trace a of V fair weak V::Vault::step;` builds a
/// quantifier holding one fairness constraint form, and ADR-023 section 13's
/// `align skip { V::Vault::mix }` builds an `align_skip` holding `mix`.
#[trace("TC-596", "FR-171-AC-2")]
#[test]
fn a_fairness_constraint_and_an_align_skip_list_build_as_written() {
    let (_, clause) = hyper(
        "hyper Fair using inf over behaviours (v: V::Vault)\n\
         forall trace a of V fair weak V::Vault::step;\n\
         forall trace b of V\n\
         align skip { V::Vault::mix }\n\
         { always holds(v.l @ a = v.l @ b) }",
    );
    let [first, second] = &clause.quantifiers[..] else {
        panic!("two quantifiers: {:?}", clause.quantifiers);
    };
    let [constraint] = &first.fairness[..] else {
        panic!("one constraint: {:?}", first.fairness);
    };
    assert_eq!(constraint.kind, Some(FairnessKind::Weak));
    assert_eq!(constraint.operation.name, "V::Vault::step");
    assert!(second.fairness.is_empty());

    let skip = clause.align_skip.expect("an align skip list");
    let operations: Vec<_> = skip
        .operations
        .iter()
        .map(|operation| operation.name.as_str())
        .collect();
    assert_eq!(operations, ["V::Vault::mix"]);
    assert!(clause.match_block.is_none());
}

/// FR-171-AC-3: `Det` builds a `ModelRelationForm` with the executions `x` and
/// `y` in order and the body's `pre` reads as written, and none of the three
/// clauses refuses `UnrepresentedConstruct`.
#[trace("TC-596", "FR-171-AC-3")]
#[test]
fn a_relation_over_two_model_executions_builds_its_bindings_and_body() {
    let (text, unit) = build(
        "relation Det using inf over (x: V::Vault::step, y: V::Vault::step) \
         { x.i = y.i and pre(x.self.l) = pre(y.self.l) implies x.self.l = y.self.l }",
    );
    let [form] = unit.forms() else {
        panic!("one declaration");
    };
    let DeclarationForm::ModelRelation(relation) = form.form() else {
        panic!("a relation: {:?}", form.form());
    };
    let ModelRelationForm {
        name,
        executions,
        body,
        ..
    } = relation.as_ref();
    assert_eq!(name.name, "Det");
    let bindings: Vec<_> = executions
        .iter()
        .map(|binding| {
            (
                binding.variable.name.as_str(),
                binding.operation.name.as_str(),
                slice(&text, binding.operation.span),
            )
        })
        .collect();
    assert_eq!(
        bindings,
        [
            ("x", "V::Vault::step", "V::Vault::step"),
            ("y", "V::Vault::step", "V::Vault::step"),
        ]
    );
    let pre_reads = body
        .expression
        .iter()
        .filter(|node| matches!(node.node(), ExprNode::Pre(_)))
        .count();
    assert_eq!(pre_reads, 2);
}

/// FR-171 "Behavior": a `hyper` clause over a declared finite trace domain
/// builds its own form, so S3 can judge a quantifier's `of`; S2 resolves
/// nothing.
#[trace("TC-596", "FR-171-AC-1")]
#[test]
fn a_finite_trace_domain_clause_builds_its_domain_and_quantifiers() {
    let (_, unit) = build(
        "hyper Finite using inf over t in V::Traces bounded 4 forall trace a exists trace b of V { true }",
    );
    let [form] = unit.forms() else {
        panic!("one declaration");
    };
    let DeclarationForm::HyperFinite(clause) = form.form() else {
        panic!("a finite hyper clause: {:?}", form.form());
    };
    let HyperFiniteForm {
        domain,
        quantifiers,
        ..
    } = clause.as_ref();
    assert_eq!(domain.variable.name, "t");
    assert_eq!(domain.population.name, "V::Traces");
    assert_eq!(domain.bound, 4);
    let aliases: Vec<_> = quantifiers
        .iter()
        .map(|quantifier| quantifier.alias.as_ref().map(|alias| alias.name.as_str()))
        .collect();
    assert_eq!(aliases, [None, Some("V")]);
}

/// An indexed read is a syntax of a hyper body: the same `holds` condition in
/// a `temporal` clause is unrepresented, located at the condition.
#[trace("TC-596", "FR-171-AC-1")]
#[test]
fn an_indexed_read_in_a_temporal_clause_is_unrepresented() {
    let text = format!(
        "{HEADER}temporal C using inf over (p: V::Vault) clock \"steps\" on origin \
         {{ always holds(p.l @ a = 1) }}\n"
    );
    let parsed = parse(&text);
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let refusal = build_unit(&parsed).expect_err("a temporal clause admits no indexed read");
    assert!(matches!(
        refusal.cause,
        FormsCause::UnrepresentedConstruct { .. }
    ));
    assert_eq!(slice(&text, refusal.span.expect("located")), "p.l @ a = 1");
}
