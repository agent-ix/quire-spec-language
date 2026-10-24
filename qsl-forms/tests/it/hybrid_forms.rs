// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-447 (TC-1187, TC-1220): S1 parses and S2 builds the `hybrid`
//! declaration with its optional `initial` clause, run on real complete-V1
//! source through `qsl_cst::parse` and `build_unit`.

use ix_trace_rs::trace;
use qsl_forms::{build_unit, DeclarationForm, HybridForm};
use qsl_foundation::{SourceIdentity, Span};

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile p = \"quire.temporal.infinite-trace/v1\";\n";

/// FR-447's `Braking`.
const BRAKING: &str = "hybrid Braking using p {\n\
       mode Moving invariant { v >= 0 and v <= 20 and a >= 1 and a <= 2 }\n\
         flow { v' = -a; a' = 0; t' = 1; }\n\
         transition to Stopped when { v = 0 } reset { v = 0 };\n\
       mode Stopped invariant { v = 0 }\n\
         flow { v' = 0; a' = 0; t' = 1; }\n\
     }";

/// FR-447's `Braking2`: two initial entries.
const BRAKING2: &str = "hybrid Braking2 using p {\n\
       initial {\n\
         mode Moving when { v = 20 and a >= 1 and a <= 2 and t = 0 };\n\
         mode Stopped when { v = 0 and t = 0 };\n\
       }\n\
       mode Moving invariant { v >= 0 }\n\
         flow { v' = -a; a' = 0; t' = 1; }\n\
         transition to Stopped when { v = 0 } reset { v = 0 and a >= 1 };\n\
       mode Stopped invariant { v = 0 }\n\
         flow { v' = 0; a' = 0; t' = 1; }\n\
     }";

fn parse(text: &str) -> qsl_cst::ParsedSource {
    qsl_cst::parse(
        SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit")
}

fn build(declarations: &str) -> (String, HybridForm) {
    let text = format!("{HEADER}{declarations}\n");
    let parsed = parse(&text);
    assert!(parsed.is_admissible(), "{text}: {:?}", parsed.diagnostics());
    let unit = build_unit(&parsed).unwrap_or_else(|failure| panic!("{text}: {failure:?}"));
    let [form] = unit.forms() else {
        panic!("one declaration: {:?}", unit.forms());
    };
    let DeclarationForm::Hybrid(hybrid) = form.form() else {
        panic!("a hybrid declaration: {:?}", form.form());
    };
    (text.clone(), (**hybrid).clone())
}

fn slice(text: &str, span: Span) -> &str {
    &text[span.start..span.end]
}

/// FR-447-AC-1 (S2 half): `Braking` builds one `HybridForm` with modes
/// `Moving` and `Stopped` in that order, `Moving` holding three equations and
/// one transition to `Stopped`, each with its span; no `initial` is written.
#[trace("TC-1187", "FR-447-AC-1")]
#[test]
fn braking_builds_a_hybrid_form_with_its_modes_equations_and_transition() {
    let (text, hybrid) = build(BRAKING);
    assert_eq!(hybrid.name.name, "Braking");
    assert_eq!(hybrid.profile.alias, "p");
    assert!(hybrid.initial.is_none());
    let names: Vec<_> = hybrid.modes.iter().map(|m| m.name.name.as_str()).collect();
    assert_eq!(names, ["Moving", "Stopped"]);
    let moving = &hybrid.modes[0];
    let equations: Vec<_> = moving
        .equations
        .iter()
        .map(|e| slice(&text, e.span))
        .collect();
    assert_eq!(equations, ["v' = -a;", "a' = 0;", "t' = 1;"]);
    assert_eq!(moving.equations[0].variable.name, "v");
    let [transition] = moving.transitions.as_slice() else {
        panic!("one transition: {:?}", moving.transitions);
    };
    assert_eq!(transition.target.name, "Stopped");
    assert_eq!(slice(&text, transition.target.span), "Stopped");
    assert_eq!(hybrid.modes[1].transitions.len(), 0);
    assert_eq!(hybrid.modes[1].equations.len(), 3);
}

/// FR-447-AC-6 (S2 half): `Braking2` builds a `HybridForm` whose `initial`
/// holds two entries with their spans, in source order.
#[trace("TC-1220", "FR-447-AC-6")]
#[test]
fn braking2_builds_two_initial_entries_with_their_spans() {
    let (text, hybrid) = build(BRAKING2);
    let initial = hybrid.initial.expect("an initial clause");
    let entries: Vec<_> = initial
        .entries
        .iter()
        .map(|e| (e.mode.name.as_str(), slice(&text, e.span)))
        .collect();
    assert_eq!(
        entries,
        [
            (
                "Moving",
                "mode Moving when { v = 20 and a >= 1 and a <= 2 and t = 0 };"
            ),
            ("Stopped", "mode Stopped when { v = 0 and t = 0 };"),
        ]
    );
}

/// FR-447-AC-6: `initial` is a keyword only as the first word of a hybrid
/// body; as a variable name elsewhere in the unit it stays an identifier.
#[trace("TC-1220", "FR-447-AC-6")]
#[test]
fn initial_stays_an_identifier_outside_the_clause() {
    let (_, hybrid) =
        build("hybrid H using p { mode M invariant { initial >= 0 } flow { initial' = 1; } }");
    assert!(hybrid.initial.is_none());
    assert_eq!(hybrid.modes[0].equations[0].variable.name, "initial");
}

/// An `initial` clause after a mode is not the grammar's first member: S1
/// does not admit it.
#[trace("TC-1220", "FR-447-AC-6")]
#[test]
fn an_initial_clause_after_a_mode_is_not_admitted() {
    let text = format!(
        "{HEADER}hybrid H using p {{ mode M invariant {{ true }} flow {{ x' = 1; }} initial {{ mode M when {{ true }}; }} }}\n"
    );
    let parsed = parse(&text);
    assert!(!parsed.is_admissible());
}

#[test]
fn a_hybrid_declaration_has_a_dispatch_entry() {
    let text = format!("{HEADER}{BRAKING}\n");
    let parsed = parse(&text);
    let unit = build_unit(&parsed).expect("S2 builds the unit");
    assert_eq!(unit.forms().len(), 1);
}

fn claim_clause(declarations: &str) -> qsl_forms::TemporalClauseForm {
    let text = format!("{HEADER}{declarations}\n");
    let parsed = parse(&text);
    assert!(parsed.is_admissible(), "{text}: {:?}", parsed.diagnostics());
    let unit = build_unit(&parsed).expect("S2 builds the unit");
    let DeclarationForm::Temporal(clause) = unit.forms()[0].form() else {
        panic!("a temporal clause");
    };
    (**clause).clone()
}

/// FR-447: a `temporal` clause may write no `clock` member, and a timed
/// interval may write `*` for its upper end.
#[trace("TC-1220", "FR-447-AC-9")]
#[test]
fn a_claim_builds_with_no_clock_member_and_a_timed_interval_with_no_upper_bound() {
    let clause = claim_clause(
        "temporal C using p over (h: Braking) on origin { always [rational(1, 4) s, *] holds(h.v < 30) }",
    );
    assert!(clause.clock.is_none());
    let with_clock = claim_clause(
        "temporal C using p over (h: Braking) clock \"model-steps\" on origin { always [0, 1] holds(h.v < 30) }",
    );
    assert_eq!(with_clock.clock.expect("a clock").text, "model-steps");
    let Some(qsl_forms::TemporalNodeForm::Operator(always)) =
        clause.formula.node(clause.formula.root())
    else {
        panic!("an operator at the root");
    };
    let Some(qsl_forms::OperatorInterval::Timed(timed)) = &always.interval else {
        panic!("a timed interval: {:?}", always.interval);
    };
    assert_eq!(timed.upper, qsl_forms::TimedUpper::Open);
}
