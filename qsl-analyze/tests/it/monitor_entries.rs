// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-251 "Transitions" (TC-1081 steps 4, 6-8): the state-passing entries
//! generated as complete-V1 source are checked by the one checker and run by
//! `qsl-eval`'s function evaluator against the tick-trace reference.

use qsl_analyze::monitor_entries::{
    render, BoolExpr, EntrySpec, IntervalNode, Thresholds,
};
use qsl_package::CheckedPackage;
use qsl_semantics::check::PackageDeclarations;
use quire_semantic_value::checking::CheckingLimits;

/// Eventually within [0, 3000] ticks at uncertainty 0 (cover width 1).
pub(crate) fn deadline_spec() -> EntrySpec {
    EntrySpec {
        atoms: vec![0, 1],
        node: IntervalNode {
            future: true,
            dual: false,
            phi: None,
            psi: BoolExpr::Atom(1),
            thresholds: Thresholds {
                all_lo: 1,
                all_hi: 2999,
                none_lo: -1,
                none_hi: 3001,
                zero_in: true,
            },
            capacity: 8,
            subformula: 3,
        },
        subject: "C".to_owned(),
        max_reading_gap: 100,
        activation_capacity: 4,
    }
}

pub(crate) fn package_of(declarations: &str) -> CheckedPackage {
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         {declarations}\n"
    );
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    if !parsed.is_admissible() {
        let at = parsed
            .diagnostics()
            .iter()
            .filter_map(|d| d.region.as_ref().map(|r| (r.start() as usize, r.end() as usize)))
            .next()
            .unwrap_or((0, 0));
        let from = at.0.saturating_sub(60);
        panic!(
            "{:?}\n...{}[[{}]]{}",
            parsed.diagnostics().iter().map(|d| d.message.clone()).collect::<Vec<_>>(),
            &text[from..at.0],
            &text[at.0..at.1],
            &text[at.1..(at.1 + 60).min(text.len())]
        );
    }
    let unit = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "fixture".to_owned(),
        unit,
        Vec::new(),
        Vec::new(),
    )
    .expect("the assembler builds the package declarations");
    CheckedPackage::link(
        declarations
            .check(CheckingLimits::default())
            .unwrap_or_else(|refusals| panic!("the package checks: {refusals:?}\n{text}")),
    )
}

#[test]
fn the_generated_declarations_check() {
    let _ = package_of(&render(&deadline_spec()));
}
