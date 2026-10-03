// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-725 step 2 (FR-258-AC-5): every body lowering writes for the TC-415
//! nested forms is in the stratified v2 body grammar. The same walk runs
//! over the 100,000-deep checked packages of step 1 in `qsl-package`'s
//! `emit::tests::deep_bodies`, where those packages are checked and emitted.

use super::expression_depth::{check_on_small_stack as check_form_on_small_stack, Form, FORMS};

/// The TC-415 nestings each form is lowered at.
const LEVELS: [usize; 2] = [2, 1_000];
use super::*;
use crate::check::stratum::is_stratified_body;

/// Every node body of `graph` that is not in the stratified v2 body
/// grammar.
fn unstratified_bodies(graph: &CheckedGraph) -> Vec<Json> {
    graph
        .semantic_graph()
        .nodes()
        .map(|node| preimage(node)["body"].clone())
        .filter(|body| !is_stratified_body(body))
        .collect()
}

/// Every node body lowering writes for every TC-415 nested form, at 2 and
/// at 1,000 levels, is in the stratified grammar.
#[trace("TC-725", "FR-258-AC-5")]
#[test]
fn every_lowered_body_is_in_the_stratified_grammar() {
    for form in FORMS {
        // An unguarded `value(….next)` refuses on its unproved presence, so
        // it lowers no body.
        if matches!(form, Form::Field) {
            continue;
        }
        for levels in LEVELS {
            let graph = check_form_on_small_stack(form, levels, CheckingLimits::default())
                .unwrap_or_else(|refusals| panic!("{form:?} at {levels} checks: {refusals:?}"));
            assert_eq!(
                unstratified_bodies(&graph),
                Vec::<Json>::new(),
                "{form:?} at {levels}"
            );
        }
    }
}
