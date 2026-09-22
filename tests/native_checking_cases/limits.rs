// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-051: measured inclusive budgets and bounded expansion of shared aliases.

use super::*;
use quire_spec_language::checking::CheckUsage;

#[derive(Clone, Copy, Debug)]
enum Budget {
    Nodes,
    Depth,
    Values,
    Graph,
    Presence,
    Materialized,
    Goal,
}

impl Budget {
    fn used(self, usage: &CheckUsage) -> usize {
        match self {
            Self::Nodes => usage.native_nodes,
            Self::Depth => usage.max_native_depth.max(usage.max_proof_depth),
            Self::Values => usage.proof_values,
            Self::Graph => usage.proof_graph_nodes,
            Self::Presence => usage.presence_work,
            Self::Materialized => usage.materialized_nodes,
            Self::Goal => usage.max_goal_nodes,
        }
    }
    fn set(self, limits: &mut CheckLimits, value: usize) {
        match self {
            Self::Nodes => limits.nodes = value,
            Self::Depth => limits.depth = value,
            Self::Values => limits.proof_values = value,
            Self::Graph => limits.proof_graph_nodes = value,
            Self::Presence => limits.presence_work = value,
            Self::Materialized => limits.materialized_nodes = value,
            Self::Goal => limits.goal_nodes = value,
        }
    }
}

fn bounded<'a>(
    models: &'a [NativeModel],
    expression: &str,
    limits: CheckLimits,
) -> Result<CheckedPackage<'a>, Box<CheckingError>> {
    let (linked, bindings) = prepared(models, expression, ClauseKind::Invariant).unwrap();
    check(linked, bindings, limits)
}

fn exhaustion(result: Result<CheckedPackage<'_>, Box<CheckingError>>) -> Box<CheckingError> {
    let Err(error) = result else {
        panic!("must refuse without a partial checked package");
    };
    assert_eq!(
        (error.diagnostic.phase, error.diagnostic.code),
        (Phase::Check, Code::ResourceExhausted)
    );
    assert!(error.diagnostic.is_incomplete());
    assert!(
        error.upstream.is_none(),
        "native budgets precede IR execution"
    );
    error
}

fn shared_aliases(levels: usize) -> String {
    let mut expression = String::from("let g0 = present(self.parent) in ");
    for level in 1..=levels {
        expression.push_str(&format!(
            "let g{level} = g{} and g{} in ",
            level - 1,
            level - 1
        ));
    }
    expression.push_str(&format!(
        "g{levels} implies deref(value(self.parent)).n < self.n"
    ));
    expression
}

#[test]
#[trace("TC-051", "FR-016-AC-8")]
fn tc_051_every_budget_accepts_observed_exact_work_and_refuses_one_less_or_zero() {
    let models = [native_rule_model::parts().model()];
    for levels in 0..=3 {
        let expression = shared_aliases(levels);
        let measured = *bounded(&models, &expression, CheckLimits::default())
            .unwrap()
            .usage();
        if levels == 3 {
            assert!(
                measured.materialized_nodes > measured.proof_graph_nodes,
                "shared graph must actually expand"
            );
        }
        for dimension in [
            Budget::Nodes,
            Budget::Depth,
            Budget::Values,
            Budget::Graph,
            Budget::Presence,
            Budget::Materialized,
            Budget::Goal,
        ] {
            let exact = dimension.used(&measured);
            assert!(exact > 0, "test must exercise {dimension:?}");
            let mut limits = CheckLimits::default();
            dimension.set(&mut limits, exact);
            let checked = bounded(&models, &expression, limits).unwrap();
            assert_eq!(*checked.usage(), measured, "{dimension:?}");
            for lower in [exact - 1, 0] {
                dimension.set(&mut limits, lower);
                exhaustion(bounded(&models, &expression, limits));
            }
        }
        println!("inclusive checker budgets, alias levels {levels}: {measured:?}");
    }
}

#[test]
#[trace("TC-051", "FR-016-AC-8")]
fn tc_051_shared_graph_expansion_cannot_elevate_hard_limits_or_reuse_prior_success() {
    let models = [native_rule_model::parts().model()];
    let elevated = CheckLimits {
        nodes: usize::MAX,
        depth: usize::MAX,
        proof_values: usize::MAX,
        proof_graph_nodes: usize::MAX,
        presence_work: usize::MAX,
        materialized_nodes: usize::MAX,
        goal_nodes: usize::MAX,
    };
    let expression = shared_aliases(3);
    let usage = *bounded(&models, &expression, elevated).unwrap().usage();
    exhaustion(bounded(
        &models,
        &expression,
        CheckLimits {
            materialized_nodes: usage.materialized_nodes - 1,
            ..elevated
        },
    ));
    assert_eq!(
        *bounded(&models, &expression, CheckLimits::default())
            .unwrap()
            .usage(),
        usage
    );
    // Only a linear number of native let nodes, but exponential IR expansion.
    let expression = shared_aliases(14);
    let ordinary = exhaustion(bounded(&models, &expression, CheckLimits::default()));
    let attempted = exhaustion(bounded(&models, &expression, elevated));
    assert_eq!(ordinary.diagnostic.message, attempted.diagnostic.message);
    assert_eq!(ordinary.diagnostic.span, attempted.diagnostic.span);
    assert!(
        ordinary.diagnostic.message.contains("per-goal nodes"),
        "{ordinary:?}"
    );
}

#[test]
#[trace("TC-051", "FR-016-AC-8")]
fn tc_051_expanded_proof_depth_is_bounded_even_when_native_depth_is_small() {
    let models = [native_rule_model::parts().model()];
    let mut expression = String::from("let g0 = present(self.parent) in ");
    for level in 1..=30 {
        expression.push_str(&format!("let g{level} = not not g{} in ", level - 1));
    }
    let exact = format!("{expression}true and g30 implies deref(value(self.parent)).n < self.n");
    let usage = *bounded(&models, &exact, CheckLimits::default())
        .unwrap()
        .usage();
    assert_eq!(usage.max_proof_depth, 64);
    assert!(usage.max_native_depth < 64);
    exhaustion(bounded(
        &models,
        &exact,
        CheckLimits {
            depth: 63,
            ..CheckLimits::default()
        },
    ));
    expression.push_str("true and (true and g30) implies deref(value(self.parent)).n < self.n");
    let error = exhaustion(bounded(
        &models,
        &expression,
        CheckLimits {
            depth: usize::MAX,
            ..CheckLimits::default()
        },
    ));
    assert!(
        error.diagnostic.message.contains("expanded proof depth"),
        "{error:?}"
    );
}

#[test]
#[trace("TC-051", "FR-016-AC-8")]
fn tc_051_materialization_counts_accumulate_across_distinct_goals() {
    let models = [native_rule_model::parts().model()];
    let expression = "self.n < 999 implies self.n + 1 < 1000 and self.n + 2 <= 1000";
    let checked = bounded(&models, expression, CheckLimits::default()).unwrap();
    assert_eq!(checked.clauses()[0].proofs().len(), 2);
    let usage = *checked.usage();
    assert!(usage.materialized_nodes > usage.max_goal_nodes);
    let limits = CheckLimits {
        materialized_nodes: usage.materialized_nodes,
        goal_nodes: usage.max_goal_nodes,
        ..CheckLimits::default()
    };
    assert_eq!(
        *bounded(&models, expression, limits).unwrap().usage(),
        usage
    );
    let error = exhaustion(bounded(
        &models,
        expression,
        CheckLimits {
            materialized_nodes: usage.materialized_nodes - 1,
            ..limits
        },
    ));
    assert!(
        error.diagnostic.message.contains("materialized nodes"),
        "{error:?}"
    );
}
