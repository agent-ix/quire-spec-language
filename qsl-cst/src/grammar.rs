// SPDX-License-Identifier: AGPL-3.0-or-later
//! Declarative transcription of the frozen complete-V1 EBNF.
use std::collections::BTreeMap;

use super::Production as P;

#[derive(Clone, Debug)]
pub(super) enum Terminal {
    Exact(&'static str),
    Literal(&'static str),
    Identifier,
    MemberName,
    Text,
    TextValue(&'static str),
    UnsignedInteger,
    End,
}

#[derive(Clone, Debug)]
pub(super) enum Rule {
    Terminal(Terminal),
    Production(P),
    Sequence(Vec<Rule>),
    Choice(Vec<Rule>),
    Optional(Box<Rule>),
    Repeat {
        rule: Box<Rule>,
        minimum: usize,
        commit_on_progress: bool,
    },
}

pub(super) type Grammar = BTreeMap<P, Rule>;

#[derive(Clone, Copy, Debug)]
pub(super) enum SelectionPart {
    Exact(&'static str),
    Alias,
    Identity,
    Version,
    Digest,
    OptionalAlias,
}

pub(super) fn selection_schema(production: P) -> Option<&'static [SelectionPart]> {
    use SelectionPart::{Alias, Digest, Exact, Identity, OptionalAlias, Version};
    match production {
        P::Profile => Some(&[
            Exact("profile"),
            Alias,
            Exact("="),
            Identity,
            Exact("version"),
            Version,
            Exact("digest"),
            Digest,
            Exact(";"),
        ]),
        P::ImportDeclaration => Some(&[
            Exact("import"),
            Identity,
            Exact("version"),
            Version,
            Exact("digest"),
            Digest,
            OptionalAlias,
            Exact(";"),
        ]),
        P::Model => Some(&[
            Exact("model"),
            Alias,
            Exact("="),
            Identity,
            Exact("version"),
            Version,
            Exact("digest"),
            Digest,
            Exact(";"),
        ]),
        _ => None,
    }
}

fn selection_rule(production: P) -> Rule {
    let rules = selection_schema(production)
        .expect("selection production has a schema")
        .iter()
        .map(|part| match part {
            SelectionPart::Exact(value) => x(value),
            SelectionPart::Alias => ident(),
            SelectionPart::Identity | SelectionPart::Version | SelectionPart::Digest => text(),
            SelectionPart::OptionalAlias => opt(s(vec![x("as"), ident()])),
        })
        .collect();
    s(rules)
}

fn x(value: &'static str) -> Rule {
    Rule::Terminal(Terminal::Exact(value))
}
fn literal(value: &'static str) -> Rule {
    Rule::Terminal(Terminal::Literal(value))
}
fn ident() -> Rule {
    Rule::Terminal(Terminal::Identifier)
}
fn member() -> Rule {
    Rule::Terminal(Terminal::MemberName)
}
fn text() -> Rule {
    Rule::Terminal(Terminal::Text)
}
fn uint() -> Rule {
    Rule::Terminal(Terminal::UnsignedInteger)
}
fn r(production: P) -> Rule {
    Rule::Production(production)
}
fn s(rules: Vec<Rule>) -> Rule {
    Rule::Sequence(rules)
}
fn c(rules: Vec<Rule>) -> Rule {
    Rule::Choice(rules)
}
fn opt(rule: Rule) -> Rule {
    Rule::Optional(Box::new(rule))
}
fn star(rule: Rule) -> Rule {
    Rule::Repeat {
        rule: Box::new(rule),
        minimum: 0,
        commit_on_progress: true,
    }
}
fn backtracking_star(rule: Rule) -> Rule {
    Rule::Repeat {
        rule: Box::new(rule),
        minimum: 0,
        commit_on_progress: false,
    }
}
fn plus(rule: Rule) -> Rule {
    Rule::Repeat {
        rule: Box::new(rule),
        minimum: 1,
        commit_on_progress: true,
    }
}
fn list(item: Rule) -> Rule {
    // The caller may admit one trailing comma after this list. Retain the
    // delimiter when it is not followed by an item so that outer grammar owns it.
    s(vec![item.clone(), backtracking_star(s(vec![x(","), item]))])
}

pub(super) fn complete_v1() -> Grammar {
    let mut grammar = Grammar::new();
    source_and_types(&mut grammar);
    expressions(&mut grammar);
    temporal(&mut grammar);
    protocol(&mut grammar);
    analysis(&mut grammar);
    grammar
}

/// Every reserved keyword and symbol spelling in the base (non-complete)
/// grammar, for editor completion candidates.
// Widened to `pub`: the root crate's `complete::editor` calls it across the
// crate boundary (ADR-011 §7.3 X-3).
pub fn base_reserved_spellings() -> std::collections::BTreeSet<&'static str> {
    fn collect(rule: &Rule, output: &mut std::collections::BTreeSet<&'static str>) {
        match rule {
            Rule::Terminal(Terminal::Exact(value) | Terminal::Literal(value)) => {
                output.insert(value);
            }
            Rule::Sequence(rules) | Rule::Choice(rules) => {
                for rule in rules {
                    collect(rule, output);
                }
            }
            Rule::Optional(rule) | Rule::Repeat { rule, .. } => collect(rule, output),
            Rule::Terminal(_) | Rule::Production(_) => {}
        }
    }
    let grammar = complete_v1();
    let mut output = std::collections::BTreeSet::new();
    for rule in grammar.values() {
        collect(rule, &mut output);
    }
    output
}

pub(super) fn complete_reserved_words(
    grammar: &Grammar,
) -> std::collections::BTreeSet<&'static str> {
    fn collect(rule: &Rule, output: &mut std::collections::BTreeSet<&'static str>) {
        match rule {
            Rule::Terminal(Terminal::Exact(value)) if crate::token::identifier_spelling(value) => {
                output.insert(value);
            }
            Rule::Sequence(rules) | Rule::Choice(rules) => {
                for rule in rules {
                    collect(rule, output);
                }
            }
            Rule::Optional(rule) | Rule::Repeat { rule, .. } => collect(rule, output),
            Rule::Terminal(_) | Rule::Production(_) => {}
        }
    }

    let mut output = std::collections::BTreeSet::new();
    for rule in grammar.values() {
        collect(rule, &mut output);
    }
    output
}

pub(super) fn complete_compound_spellings(grammar: &Grammar) -> Vec<&'static str> {
    fn is_compound(value: &str) -> bool {
        let mut parts = value.split('-');
        let Some(first) = parts.next() else {
            return false;
        };
        let Some(second) = parts.next() else {
            return false;
        };
        crate::token::identifier_spelling(first)
            && crate::token::identifier_spelling(second)
            && parts.all(crate::token::identifier_spelling)
    }

    fn collect(rule: &Rule, output: &mut std::collections::BTreeSet<&'static str>) {
        match rule {
            Rule::Terminal(Terminal::Exact(value) | Terminal::Literal(value))
                if is_compound(value) =>
            {
                output.insert(value);
            }
            Rule::Sequence(rules) | Rule::Choice(rules) => {
                for rule in rules {
                    collect(rule, output);
                }
            }
            Rule::Optional(rule) | Rule::Repeat { rule, .. } => collect(rule, output),
            Rule::Terminal(_) | Rule::Production(_) => {}
        }
    }

    let mut output = std::collections::BTreeSet::new();
    for rule in grammar.values() {
        collect(rule, &mut output);
    }
    output.into_iter().collect()
}

fn source_and_types(g: &mut Grammar) {
    g.insert(
        P::CompleteUnit,
        s(vec![
            r(P::Header),
            plus(r(P::Profile)),
            star(r(P::ImportDeclaration)),
            star(r(P::Model)),
            plus(r(P::Declaration)),
            Rule::Terminal(Terminal::End),
        ]),
    );
    g.insert(
        P::Header,
        s(vec![
            x("language"),
            Rule::Terminal(Terminal::TextValue("ix:native")),
            x("edition"),
            Rule::Terminal(Terminal::TextValue("1-draft")),
            x(";"),
        ]),
    );
    g.insert(P::Profile, selection_rule(P::Profile));
    g.insert(P::ImportDeclaration, selection_rule(P::ImportDeclaration));
    g.insert(P::Model, selection_rule(P::Model));
    let declarations = vec![
        r(P::DimensionDeclaration),
        r(P::UnitDeclaration),
        r(P::EnumDeclaration),
        r(P::RecordDeclaration),
        r(P::TupleDeclaration),
        r(P::AliasDeclaration),
        r(P::FunctionDeclaration),
        r(P::Predicate),
        r(P::StateClause),
        r(P::TemporalClause),
        r(P::ProtocolClause),
        r(P::RelationClause),
        r(P::HyperClause),
        r(P::HybridDeclaration),
        r(P::SynthesisDeclaration),
        r(P::VerificationPlan),
    ];
    g.insert(P::Declaration, c(declarations));
    g.insert(
        P::QualifiedName,
        s(vec![ident(), star(s(vec![x("::"), member()]))]),
    );
    g.insert(P::ModelName, s(vec![ident(), x("::"), member()]));
    g.insert(P::TypeName, r(P::ModelName));
    g.insert(P::OperationName, s(vec![r(P::TypeName), x("::"), member()]));
    g.insert(P::SignedInteger, s(vec![opt(x("-")), uint()]));
    g.insert(
        P::RoundingMode,
        c(vec![
            literal("exact"),
            x("toward-zero"),
            x("toward-positive"),
            x("toward-negative"),
            x("nearest-even"),
            x("nearest-away"),
        ]),
    );
    g.insert(
        P::TextProfile,
        c(vec![
            x("unicode-scalars"),
            x("nfc"),
            x("nfd"),
            x("nfkc"),
            x("nfkd"),
            x("binary-utf8"),
        ]),
    );
    g.insert(
        P::TypeReference,
        c(vec![
            x("Boolean"),
            x("Integer"),
            s(vec![
                x("Int"),
                x("["),
                r(P::SignedInteger),
                x(","),
                r(P::SignedInteger),
                x("]"),
            ]),
            s(vec![
                x("Rational"),
                x("["),
                r(P::SignedInteger),
                x(","),
                r(P::SignedInteger),
                x(";"),
                uint(),
                x(","),
                uint(),
                x("]"),
            ]),
            s(vec![
                x("Decimal"),
                x("["),
                r(P::SignedInteger),
                x(","),
                r(P::SignedInteger),
                x(";"),
                uint(),
                x(","),
                uint(),
                x(";"),
                r(P::RoundingMode),
                x("]"),
            ]),
            s(vec![
                c(vec![x("Float32"), x("Float64")]),
                x("["),
                r(P::RoundingMode),
                x("]"),
            ]),
            s(vec![
                x("Text"),
                x("["),
                uint(),
                x(","),
                uint(),
                x(";"),
                r(P::TextProfile),
                x("]"),
            ]),
            s(vec![x("Option"), x("<"), r(P::TypeReference), x(">")]),
            s(vec![
                c(vec![x("Sequence"), x("Set"), x("Bag"), x("OrderedSet")]),
                x("<"),
                r(P::TypeReference),
                x(">"),
                x("["),
                uint(),
                x(","),
                uint(),
                x("]"),
            ]),
            s(vec![x("Reference"), x("<"), r(P::QualifiedName), x(">")]),
            r(P::QualifiedName),
        ]),
    );
    g.insert(P::ParameterType, r(P::TypeReference));
    g.insert(
        P::DimensionTerm,
        s(vec![
            r(P::QualifiedName),
            opt(s(vec![x("^"), r(P::SignedInteger)])),
        ]),
    );
    g.insert(
        P::DimensionDeclaration,
        s(vec![
            x("dimension"),
            ident(),
            opt(s(vec![
                x("="),
                r(P::DimensionTerm),
                star(s(vec![c(vec![x("*"), x("/")]), r(P::DimensionTerm)])),
            ])),
            x(";"),
        ]),
    );
    g.insert(
        P::UnitDeclaration,
        s(vec![
            x("unit"),
            ident(),
            x(":"),
            r(P::QualifiedName),
            x("="),
            r(P::ExactNumber),
            opt(s(vec![x("*"), r(P::QualifiedName)])),
            opt(s(vec![x("+"), r(P::ExactNumber)])),
            x(";"),
        ]),
    );
    g.insert(
        P::EnumMember,
        s(vec![ident(), opt(s(vec![x("="), text()]))]),
    );
    g.insert(
        P::EnumDeclaration,
        s(vec![
            opt(x("ordered")),
            x("enum"),
            ident(),
            x("{"),
            list(r(P::EnumMember)),
            opt(x(",")),
            x("}"),
        ]),
    );
    g.insert(
        P::Field,
        s(vec![
            ident(),
            x(":"),
            r(P::TypeReference),
            opt(x("?")),
            x(";"),
        ]),
    );
    g.insert(
        P::RecordDeclaration,
        s(vec![
            x("record"),
            ident(),
            x("{"),
            plus(r(P::Field)),
            x("}"),
        ]),
    );
    g.insert(
        P::TupleDeclaration,
        s(vec![
            x("tuple"),
            ident(),
            x("("),
            list(r(P::TypeReference)),
            x(")"),
            x(";"),
        ]),
    );
    g.insert(
        P::AliasDeclaration,
        s(vec![
            x("type"),
            ident(),
            x("="),
            r(P::TypeReference),
            x(";"),
        ]),
    );
    g.insert(P::Parameter, s(vec![ident(), x(":"), r(P::ParameterType)]));
    let parameters = s(vec![x("("), opt(list(r(P::Parameter))), x(")")]);
    g.insert(
        P::FunctionDeclaration,
        s(vec![
            x("function"),
            ident(),
            x("using"),
            ident(),
            parameters.clone(),
            x(":"),
            r(P::TypeReference),
            x("pure"),
            opt(s(vec![x("decreases"), x("("), r(P::Expression), x(")")])),
            r(P::Block),
        ]),
    );
    g.insert(
        P::Predicate,
        s(vec![
            x("predicate"),
            ident(),
            x("using"),
            ident(),
            parameters,
            x(":"),
            x("Boolean"),
            r(P::Block),
        ]),
    );
    g.insert(
        P::StateClause,
        c(vec![
            s(vec![
                x("invariant"),
                ident(),
                x("using"),
                ident(),
                x("on"),
                r(P::TypeName),
                x("at"),
                x("current"),
                r(P::Block),
            ]),
            s(vec![
                c(vec![x("pre"), x("post")]),
                ident(),
                x("using"),
                ident(),
                x("on"),
                r(P::OperationName),
                r(P::Block),
            ]),
        ]),
    );
    g.insert(P::Block, s(vec![x("{"), r(P::Expression), x("}")]));
}

fn expressions(g: &mut Grammar) {
    g.insert(
        P::HexDigit,
        c(vec![
            literal("0"),
            literal("1"),
            literal("2"),
            literal("3"),
            literal("4"),
            literal("5"),
            literal("6"),
            literal("7"),
            literal("8"),
            literal("9"),
            literal("a"),
            literal("b"),
            literal("c"),
            literal("d"),
            literal("e"),
            literal("f"),
        ]),
    );
    g.insert(
        P::Hex32,
        s(std::iter::once(x("0x"))
            .chain((0..8).map(|_| r(P::HexDigit)))
            .collect()),
    );
    g.insert(
        P::Hex64,
        s(std::iter::once(x("0x"))
            .chain((0..16).map(|_| r(P::HexDigit)))
            .collect()),
    );
    g.insert(
        P::Expression,
        c(vec![
            s(vec![
                x("let"),
                ident(),
                x("="),
                r(P::Expression),
                x("in"),
                r(P::Expression),
            ]),
            s(vec![
                x("if"),
                r(P::Expression),
                x("then"),
                r(P::Expression),
                x("else"),
                r(P::Expression),
            ]),
            r(P::Implication),
        ]),
    );
    g.insert(
        P::Implication,
        s(vec![
            r(P::Disjunction),
            opt(s(vec![x("implies"), r(P::Implication)])),
        ]),
    );
    g.insert(
        P::Disjunction,
        s(vec![
            r(P::Conjunction),
            star(s(vec![x("or"), r(P::Conjunction)])),
        ]),
    );
    g.insert(
        P::Conjunction,
        s(vec![
            r(P::Comparison),
            star(s(vec![x("and"), r(P::Comparison)])),
        ]),
    );
    g.insert(
        P::Comparison,
        s(vec![
            r(P::Sum),
            opt(s(vec![
                c(vec![x("="), x("!="), x("<"), x("<="), x(">"), x(">=")]),
                r(P::Sum),
            ])),
        ]),
    );
    g.insert(
        P::Sum,
        s(vec![
            r(P::Product),
            star(s(vec![c(vec![x("+"), x("-")]), r(P::Product)])),
        ]),
    );
    g.insert(
        P::Product,
        s(vec![
            r(P::Unary),
            star(s(vec![
                c(vec![x("*"), x("/"), x("div"), x("rem"), x("mod")]),
                r(P::Unary),
            ])),
        ]),
    );
    g.insert(
        P::Unary,
        c(vec![
            s(vec![c(vec![x("not"), x("-")]), r(P::Unary)]),
            r(P::Postfix),
        ]),
    );
    g.insert(
        P::Postfix,
        s(vec![
            r(P::Primary),
            star(c(vec![
                s(vec![x("."), member()]),
                s(vec![x("["), r(P::Expression), x("]")]),
            ])),
        ]),
    );
    g.insert(
        P::ExactNumber,
        c(vec![
            s(vec![
                x("rational"),
                x("("),
                r(P::SignedInteger),
                x(","),
                r(P::SignedInteger),
                x(")"),
            ]),
            s(vec![
                x("decimal"),
                x("("),
                r(P::SignedInteger),
                x(","),
                uint(),
                x(")"),
            ]),
        ]),
    );
    g.insert(
        P::FloatValue,
        c(vec![
            s(vec![
                x("float32"),
                x("("),
                x("bits"),
                x(":"),
                r(P::Hex32),
                x(")"),
            ]),
            s(vec![
                x("float64"),
                x("("),
                x("bits"),
                x(":"),
                r(P::Hex64),
                x(")"),
            ]),
        ]),
    );
    g.insert(
        P::EnumValue,
        s(vec![
            ident(),
            x("::"),
            member(),
            star(s(vec![x("::"), member()])),
        ]),
    );
    g.insert(
        P::CollectionValue,
        s(vec![
            c(vec![
                x("sequence"),
                literal("set"),
                literal("bag"),
                literal("orderedSet"),
            ]),
            x("["),
            opt(list(r(P::Expression))),
            x("]"),
        ]),
    );
    g.insert(P::FieldValue, s(vec![ident(), x(":"), r(P::Expression)]));
    g.insert(
        P::RecordValue,
        s(vec![
            r(P::QualifiedName),
            x("{"),
            r(P::FieldValue),
            star(s(vec![x(","), r(P::FieldValue)])),
            x("}"),
        ]),
    );
    g.insert(
        P::TupleValue,
        s(vec![
            r(P::QualifiedName),
            x("("),
            r(P::Expression),
            star(s(vec![x(","), r(P::Expression)])),
            x(")"),
        ]),
    );
    g.insert(
        P::CollectionCall,
        c(vec![
            s(vec![
                c(vec![x("map"), x("collect"), x("filter"), x("flatMap")]),
                x("("),
                ident(),
                x("in"),
                r(P::Expression),
                x(":"),
                r(P::Expression),
                x(")"),
            ]),
            s(vec![x("flatten"), x("("), r(P::Expression), x(")")]),
            s(vec![
                c(vec![x("fold"), x("reduce")]),
                x("<"),
                r(P::QualifiedName),
                x(">"),
                x("("),
                ident(),
                x(","),
                ident(),
                x("in"),
                r(P::Expression),
                x(":"),
                r(P::Expression),
                opt(s(vec![x(","), x("identity"), x(":"), r(P::Expression)])),
                x(")"),
            ]),
            s(vec![
                c(vec![x("forall"), x("exists")]),
                x("("),
                ident(),
                x("in"),
                r(P::Expression),
                x(":"),
                r(P::Expression),
                x(")"),
            ]),
            s(vec![
                c(vec![x("count"), x("sum")]),
                x("<"),
                r(P::QualifiedName),
                x(">"),
                x("("),
                ident(),
                x("in"),
                r(P::Expression),
                x(":"),
                r(P::Expression),
                x(")"),
            ]),
        ]),
    );
    let args = s(vec![x("("), opt(list(r(P::Expression))), x(")")]);
    g.insert(
        P::Primary,
        c(vec![
            x("true"),
            x("false"),
            x("null"),
            x("none"),
            x("self"),
            x("result"),
            uint(),
            text(),
            r(P::ExactNumber),
            r(P::FloatValue),
            r(P::CollectionValue),
            r(P::RecordValue),
            r(P::TupleValue),
            r(P::EnumValue),
            s(vec![
                c(vec![x("present"), x("value"), x("deref"), x("pre")]),
                x("("),
                r(P::Expression),
                x(")"),
            ]),
            s(vec![
                c(vec![x("convert"), x("allInstances")]),
                x("<"),
                r(P::TypeReference),
                x(">"),
                x("("),
                r(P::Expression),
                x(")"),
            ]),
            r(P::CollectionCall),
            s(vec![
                x("size"),
                opt(s(vec![x("<"), r(P::TypeReference), x(">")])),
                x("("),
                r(P::Expression),
                x(")"),
            ]),
            s(vec![
                x("contains"),
                x("("),
                r(P::Expression),
                x(","),
                r(P::Expression),
                x(")"),
            ]),
            s(vec![
                x("reaches"),
                x("("),
                r(P::Expression),
                x(","),
                r(P::Expression),
                x(","),
                r(P::QualifiedName),
                x(")"),
            ]),
            s(vec![x("("), r(P::Expression), x(")")]),
            s(vec![ident(), args]),
            r(P::QualifiedName),
        ]),
    );
}

fn temporal(g: &mut Grammar) {
    let bound_parameter = s(vec![x("("), r(P::Parameter), x(")")]);
    g.insert(
        P::Activation,
        c(vec![
            s(vec![x("on"), x("origin")]),
            s(vec![
                x("on"),
                x("each"),
                bound_parameter.clone(),
                opt(s(vec![x("when"), x("("), r(P::Expression), x(")")])),
            ]),
        ]),
    );
    g.insert(
        P::Capture,
        s(vec![
            x("capture"),
            r(P::Parameter),
            x("="),
            r(P::Expression),
            x(";"),
        ]),
    );
    g.insert(
        P::Interval,
        s(vec![
            x("["),
            uint(),
            x(","),
            c(vec![uint(), x("*")]),
            x("]"),
        ]),
    );
    g.insert(
        P::TemporalClause,
        s(vec![
            x("temporal"),
            ident(),
            x("using"),
            ident(),
            x("over"),
            bound_parameter.clone(),
            x("clock"),
            text(),
            r(P::Activation),
            x("{"),
            star(r(P::Capture)),
            r(P::TemporalExpression),
            x("}"),
        ]),
    );
    g.insert(P::TemporalExpression, r(P::TemporalImplication));
    g.insert(
        P::TemporalImplication,
        s(vec![
            r(P::TemporalDisjunction),
            opt(s(vec![x("implies"), r(P::TemporalImplication)])),
        ]),
    );
    g.insert(
        P::TemporalDisjunction,
        s(vec![
            r(P::TemporalConjunction),
            star(s(vec![x("or"), r(P::TemporalConjunction)])),
        ]),
    );
    g.insert(
        P::TemporalConjunction,
        s(vec![
            r(P::TemporalRelation),
            star(s(vec![x("and"), r(P::TemporalRelation)])),
        ]),
    );
    g.insert(
        P::TemporalRelation,
        s(vec![
            r(P::TemporalUnary),
            opt(s(vec![
                c(vec![x("until"), x("release"), x("since"), x("triggered")]),
                r(P::Interval),
                r(P::TemporalUnary),
            ])),
        ]),
    );
    g.insert(
        P::TemporalUnary,
        c(vec![
            s(vec![x("not"), r(P::TemporalUnary)]),
            s(vec![
                c(vec![
                    x("eventually"),
                    x("always"),
                    x("once"),
                    x("historically"),
                ]),
                r(P::Interval),
                r(P::TemporalUnary),
            ]),
            r(P::TemporalPrimary),
        ]),
    );
    g.insert(
        P::TemporalPrimary,
        c(vec![
            x("true"),
            x("false"),
            s(vec![x("("), r(P::TemporalExpression), x(")")]),
            s(vec![x("holds"), x("("), r(P::Expression), x(")")]),
        ]),
    );
}

fn protocol(g: &mut Grammar) {
    let bound_parameter = s(vec![x("("), r(P::Parameter), x(")")]);
    g.insert(
        P::RoleLifetime,
        c(vec![
            x("workflow"),
            x("scope"),
            s(vec![x("until"), x("("), r(P::Expression), x(")")]),
        ]),
    );
    g.insert(
        P::Role,
        c(vec![
            s(vec![
                x("role"),
                ident(),
                x("on"),
                r(P::QualifiedName),
                x(";"),
            ]),
            s(vec![
                x("role"),
                ident(),
                x("each"),
                r(P::QualifiedName),
                x("from"),
                ident(),
                x("max"),
                uint(),
                x("lifetime"),
                r(P::RoleLifetime),
                x(";"),
            ]),
        ]),
    );
    g.insert(
        P::Relationship,
        s(vec![
            x("relationship"),
            ident(),
            x("="),
            r(P::ModelName),
            x(";"),
        ]),
    );
    g.insert(
        P::Ordering,
        c(vec![
            x("unordered"),
            s(vec![
                x("fifo"),
                x("by"),
                bound_parameter.clone(),
                r(P::Block),
            ]),
        ]),
    );
    g.insert(
        P::DeliveryPolicy,
        c(vec![
            literal("unknown"),
            x("at-most-once"),
            x("at-least-once"),
            x("exactly-once-premise"),
        ]),
    );
    g.insert(
        P::Capacity,
        c(vec![
            uint(),
            s(vec![x("symbolic"), x("("), ident(), x(")")]),
        ]),
    );
    g.insert(
        P::OverflowPolicy,
        c(vec![x("reject"), x("block"), x("loss")]),
    );
    g.insert(
        P::Channel,
        s(vec![
            x("channel"),
            ident(),
            x("from"),
            ident(),
            x("to"),
            ident(),
            x("carries"),
            r(P::TypeReference),
            x("ordering"),
            r(P::Ordering),
            x("delivery"),
            r(P::DeliveryPolicy),
            x("capacity"),
            r(P::Capacity),
            x("overflow"),
            r(P::OverflowPolicy),
            x(";"),
        ]),
    );
    g.insert(
        P::ProtocolRequirement,
        s(vec![x("requires"), x("temporal"), ident(), x(";")]),
    );
    g.insert(
        P::NodeReference,
        s(vec![ident(), star(s(vec![x("::"), ident()]))]),
    );
    g.insert(
        P::Compensation,
        s(vec![
            x("compensate"),
            ident(),
            x("for"),
            r(P::NodeReference),
            x("as"),
            bound_parameter.clone(),
            x("by"),
            ident(),
            x("on"),
            r(P::OperationName),
            x("using"),
            ident(),
            x("clock"),
            text(),
            x("{"),
            star(r(P::Capture)),
            x("activate"),
            x("first"),
            bound_parameter.clone(),
            x("when"),
            r(P::Block),
            x("{"),
            star(r(P::Capture)),
            x("}"),
            x("within"),
            r(P::Interval),
            x(";"),
            x("attempts"),
            uint(),
            x("of"),
            r(P::TypeName),
            x(";"),
            x("retry"),
            x("("),
            r(P::Parameter),
            x(","),
            r(P::Parameter),
            x(")"),
            r(P::Block),
            x(";"),
            x("commit"),
            c(vec![r(P::NodeReference), x("never")]),
            x(";"),
            x("recover"),
            bound_parameter.clone(),
            r(P::Block),
            x(";"),
            x("}"),
        ]),
    );
    g.insert(
        P::Visibility,
        s(vec![
            x("visible"),
            x("("),
            opt(list(r(P::Expression))),
            x(")"),
        ]),
    );
    g.insert(
        P::Sequence,
        s(vec![
            x("sequence"),
            ident(),
            x("{"),
            star(r(P::Control)),
            x("}"),
        ]),
    );
    g.insert(
        P::Case,
        s(vec![
            x("case"),
            ident(),
            x("when"),
            r(P::Block),
            r(P::Control),
        ]),
    );
    g.insert(
        P::Choice,
        s(vec![
            x("choice"),
            ident(),
            x("by"),
            ident(),
            r(P::Visibility),
            x("{"),
            r(P::Case),
            r(P::Case),
            star(r(P::Case)),
            x("}"),
        ]),
    );
    g.insert(P::Branch, s(vec![x("branch"), ident(), r(P::Control)]));
    g.insert(
        P::JoinPolicy,
        c(vec![
            x("all"),
            x("any"),
            s(vec![x("quorum"), x("("), uint(), x(")")]),
            s(vec![x("predicate"), x("("), ident(), x(")")]),
        ]),
    );
    g.insert(
        P::Parallel,
        s(vec![
            x("parallel"),
            ident(),
            x("{"),
            r(P::Branch),
            r(P::Branch),
            star(r(P::Branch)),
            x("}"),
            x("join"),
            r(P::JoinPolicy),
            x("["),
            list(ident()),
            x("]"),
            opt(s(vec![
                x("outstanding"),
                c(vec![x("continue"), x("cancel")]),
            ])),
            x(";"),
        ]),
    );
    g.insert(
        P::Repetition,
        s(vec![
            x("repeat"),
            ident(),
            x("by"),
            ident(),
            r(P::Visibility),
            x("max"),
            uint(),
            x("invariant"),
            r(P::Block),
            x("variant"),
            r(P::Block),
            x("while"),
            r(P::Block),
            r(P::Control),
            x("exhausted"),
            r(P::Control),
        ]),
    );
    g.insert(
        P::AwaitControl,
        s(vec![
            x("await"),
            ident(),
            x("after"),
            r(P::NodeReference),
            x("using"),
            ident(),
            x("clock"),
            text(),
            x("within"),
            r(P::Interval),
            x("match"),
            r(P::EventNode),
            x("then"),
            r(P::Control),
            x("timeout"),
            r(P::Control),
        ]),
    );
    g.insert(
        P::Related,
        s(vec![
            x("related"),
            x("by"),
            ident(),
            x("("),
            r(P::Expression),
            x(","),
            r(P::Expression),
            x(")"),
        ]),
    );
    let event_tail = s(vec![
        x("as"),
        bound_parameter.clone(),
        star(r(P::Related)),
        r(P::Block),
        x(";"),
    ]);
    g.insert(
        P::EventNode,
        c(vec![
            s(vec![
                x("send"),
                ident(),
                x("via"),
                ident(),
                event_tail.clone(),
            ]),
            s(vec![
                x("receive"),
                ident(),
                x("via"),
                ident(),
                x("of"),
                r(P::NodeReference),
                event_tail.clone(),
            ]),
            s(vec![
                x("attempt"),
                ident(),
                x("by"),
                ident(),
                x("on"),
                r(P::OperationName),
                x("contracts"),
                x("["),
                opt(list(ident())),
                x("]"),
                event_tail.clone(),
            ]),
            s(vec![
                x("effect"),
                ident(),
                x("of"),
                r(P::NodeReference),
                event_tail.clone(),
            ]),
            s(vec![
                x("event"),
                ident(),
                x("by"),
                ident(),
                opt(s(vec![x("for"), r(P::NodeReference)])),
                event_tail,
            ]),
        ]),
    );
    g.insert(
        P::Check,
        s(vec![
            x("check"),
            ident(),
            x("using"),
            ident(),
            r(P::Block),
            x(";"),
        ]),
    );
    g.insert(
        P::Commit,
        s(vec![
            x("commit"),
            ident(),
            x("by"),
            ident(),
            x("as"),
            bound_parameter.clone(),
            r(P::Block),
            x(";"),
        ]),
    );
    g.insert(
        P::Control,
        c(vec![
            r(P::Sequence),
            r(P::Choice),
            r(P::Parallel),
            r(P::Repetition),
            r(P::AwaitControl),
            r(P::EventNode),
            r(P::Check),
            r(P::Commit),
        ]),
    );
    g.insert(
        P::Finish,
        s(vec![
            x("finish"),
            ident(),
            x("as"),
            bound_parameter.clone(),
            r(P::Block),
            x(";"),
        ]),
    );
    g.insert(
        P::ProtocolClause,
        s(vec![
            x("protocol"),
            ident(),
            x("using"),
            ident(),
            x("over"),
            bound_parameter,
            r(P::Activation),
            x("{"),
            star(r(P::Capture)),
            plus(r(P::Role)),
            star(r(P::Relationship)),
            star(r(P::Channel)),
            star(c(vec![r(P::ProtocolRequirement), r(P::Compensation)])),
            x("run"),
            r(P::Control),
            r(P::Finish),
            x("}"),
        ]),
    );
}

fn analysis(g: &mut Grammar) {
    g.insert(
        P::ExecutionBinding,
        s(vec![ident(), x(":"), r(P::QualifiedName)]),
    );
    g.insert(
        P::RelationClause,
        s(vec![
            x("relation"),
            ident(),
            x("using"),
            ident(),
            x("over"),
            x("("),
            r(P::ExecutionBinding),
            x(","),
            r(P::ExecutionBinding),
            star(s(vec![x(","), r(P::ExecutionBinding)])),
            x(")"),
            r(P::Block),
        ]),
    );
    g.insert(
        P::TraceDomain,
        s(vec![
            ident(),
            x("in"),
            r(P::QualifiedName),
            x("bounded"),
            uint(),
        ]),
    );
    g.insert(
        P::Quantifier,
        s(vec![c(vec![x("forall"), x("exists")]), x("trace"), ident()]),
    );
    g.insert(
        P::HyperClause,
        s(vec![
            x("hyper"),
            ident(),
            x("using"),
            ident(),
            x("over"),
            r(P::TraceDomain),
            plus(r(P::Quantifier)),
            r(P::Block),
        ]),
    );
    g.insert(
        P::Equation,
        s(vec![
            r(P::QualifiedName),
            x("'"),
            x("="),
            r(P::Expression),
            x(";"),
        ]),
    );
    g.insert(
        P::HybridMode,
        s(vec![
            x("mode"),
            ident(),
            x("invariant"),
            r(P::Block),
            x("flow"),
            x("{"),
            plus(r(P::Equation)),
            x("}"),
            star(s(vec![
                x("transition"),
                x("to"),
                ident(),
                x("when"),
                r(P::Block),
                x("reset"),
                r(P::Block),
                x(";"),
            ])),
        ]),
    );
    g.insert(
        P::HybridDeclaration,
        s(vec![
            x("hybrid"),
            ident(),
            x("using"),
            ident(),
            x("{"),
            plus(r(P::HybridMode)),
            x("}"),
        ]),
    );
    g.insert(
        P::SynthesisDeclaration,
        s(vec![
            x("synthesis"),
            ident(),
            x("using"),
            ident(),
            x("grammar"),
            r(P::QualifiedName),
            x("domain"),
            r(P::QualifiedName),
            x("satisfies"),
            r(P::Block),
            x(";"),
        ]),
    );
    g.insert(
        P::VerificationStep,
        s(vec![
            x("check"),
            ident(),
            x("claim"),
            r(P::QualifiedName),
            x("method"),
            r(P::QualifiedName),
            x("domain"),
            r(P::QualifiedName),
            opt(s(vec![x("depends"), x("["), list(ident()), x("]")])),
            opt(s(vec![x("bound"), uint()])),
            x(";"),
        ]),
    );
    g.insert(
        P::VerificationPlan,
        s(vec![
            x("verify"),
            ident(),
            x("using"),
            ident(),
            x("{"),
            plus(r(P::VerificationStep)),
            x("}"),
        ]),
    );
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;

    use super::{complete_compound_spellings, complete_reserved_words, complete_v1};
    use crate::{parse, Limits};
    use qsl_foundation::SourceIdentity;

    #[trace("TC-180", "FR-339-AC-1", "FR-302-AC-1")]
    #[test]
    fn every_declarative_compound_is_one_budgeted_cst_leaf() {
        let compounds = complete_compound_spellings(&complete_v1());
        assert!(!compounds.is_empty());
        for compound in compounds {
            let parsed = parse(
                SourceIdentity {
                    identity: "test:compound-authority".into(),
                    revision: compound.into(),
                },
                "compound.native",
                compound.as_bytes(),
                Limits {
                    tokens: 1,
                    ..Limits::default()
                },
            )
            .unwrap();
            assert_eq!(parsed.cst().tokens().len(), 1, "`{compound}`");
            assert_eq!(parsed.cst().tokens()[0].spelling(), compound.as_bytes());

            let refusal = parse(
                SourceIdentity {
                    identity: "test:compound-authority".into(),
                    revision: format!("{compound}:below"),
                },
                "compound.native",
                compound.as_bytes(),
                Limits {
                    tokens: 0,
                    ..Limits::default()
                },
            )
            .unwrap_err();
            assert_eq!(refusal.code, qsl_foundation::Code::ResourceExhausted);
            assert_eq!(refusal.span.start.byte, 0);
            assert_eq!(refusal.span.end.byte, compound.len());
        }
    }

    #[trace("TC-180", "FR-339-AC-1")]
    #[test]
    fn every_declarative_reserved_word_is_refused_as_an_identifier() {
        let prefix = concat!(
            "language \"ix:native\" edition \"1-draft\";\n",
            "profile Complete = \"quire.value.complete/v1\" version \"1\" digest ",
            "\"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\";\n",
            "record "
        );
        for word in complete_reserved_words(&complete_v1()) {
            let text = format!("{prefix}{word} {{ datum: Integer; }}");
            let parsed = parse(
                SourceIdentity {
                    identity: "test:reserved-word-authority".into(),
                    revision: word.into(),
                },
                "reserved.native",
                text.as_bytes(),
                Limits::default(),
            )
            .unwrap();
            assert!(
                !parsed.is_admissible(),
                "reserved word `{word}` was admitted"
            );
            assert_eq!(
                parsed.diagnostics()[0].span.start.byte,
                prefix.len(),
                "reserved word `{word}` refused at the wrong token"
            );
        }
    }
}
