//! FR-114 attempt binding: the `contracts` list against the unit's state
//! clauses, and the assembler's operation resolution.

use ix_trace_rs::trace;

use super::attempt_fixture::{attempt_flow, HEADER};
use super::{
    assemble, assembly_errors, checks, empty_scope_and_signatures, only_cause, protocol_causes,
    recovery_flow, refusals,
};
use crate::check::refusal::ProtocolAnchorCause;
use crate::check::CheckCause;

/// SR-770 FND-005: a `contracts` entry naming two state clauses of one
/// name never binds the first silently. The whole pipeline already
/// refuses the duplicate clause names themselves (`AmbiguousName` at
/// both clauses, before any protocol is checked); `check`'s own binding
/// refuses `ambiguous_declaration`/`ambiguous-name` at the entry too,
/// naming both clauses, so it never relies on that earlier stage.
#[trace("TC-513", "FR-114-AC-3")]
#[test]
fn a_contract_entry_naming_two_clauses_of_one_name_refuses() {
    let source = format!(
        "post Dup using v on M::Actor::op {{ true }}\n\
         post Dup using v on M::Actor::op {{ false }}\n{}",
        attempt_flow("Flow", "Dup")
    );
    let (_, found) = refusals(&source);
    assert!(
        !found.is_empty()
            && found
                .iter()
                .all(|refusal| matches!(&refusal.cause, CheckCause::AmbiguousName { name, .. } if name == "Dup")),
        "{found:?}"
    );

    let (_, assembled) = assemble(&source);
    let (scope, signatures) = empty_scope_and_signatures();
    let refused = super::super::check(
        &assembled.protocols[0],
        &std::collections::BTreeMap::new(),
        &std::collections::BTreeSet::new(),
        &scope,
        &signatures,
        &assembled.protocol_attempts[0],
        &assembled.state_clauses,
    )
    .map(|_| ())
    .expect_err("an ambiguous contracts entry refuses");
    let [cause] = protocol_causes(&refused)[..] else {
        panic!("one refusal: {refused:?}");
    };
    match cause {
        ProtocolAnchorCause::Ambiguous { name, loci, .. } => {
            assert_eq!(name, "Dup");
            assert_eq!(loci.len(), 2, "{loci:?}");
        }
        other => panic!("an Ambiguous cause, got {other:?}"),
    }
}

/// FR-114-AC-3 (TC-513): a `contracts` entry naming no state clause of
/// the unit refuses `missing_declaration`/`missing-name` at the entry.
#[trace("TC-513", "FR-114-AC-3")]
#[test]
fn a_missing_contract_entry_refuses_naming_the_entry() {
    let source = recovery_flow("Main::Committed", "").replacen(
        "attempt Tried by R on M::Actor::op contracts []",
        "attempt Tried by R on M::Actor::op contracts [Absent]",
        1,
    );
    let (_, found) = refusals(&source);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].cause.code().as_str(), "missing_declaration");
    assert_eq!(found[0].cause.cause(), Some("missing-name"));
    match protocol_causes(&found)[0] {
        ProtocolAnchorCause::MissingContract { entry, .. } => assert_eq!(entry, "Absent"),
        other => panic!("a MissingContract cause, got {other:?}"),
    }
}

/// FR-114-AC-3 (TC-513): a `contracts` entry naming an invariant (which
/// names no operation at all) refuses `wrong_snapshot`/`wrong-anchor`,
/// naming both the attempt's own anchor and the invariant's.
#[trace("TC-513", "FR-114-AC-3")]
#[test]
fn a_contract_entry_naming_an_invariant_refuses() {
    let source = format!(
        "invariant ParentOrder using v on M::Actor at current {{ true }}\n{}",
        recovery_flow("Main::Committed", "").replacen(
            "attempt Tried by R on M::Actor::op contracts []",
            "attempt Tried by R on M::Actor::op contracts [ParentOrder]",
            1,
        )
    );
    let (_, found) = refusals(&source);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].cause.code().as_str(), "wrong_snapshot");
    assert_eq!(found[0].cause.cause(), Some("wrong-anchor"));
    match protocol_causes(&found)[0] {
        ProtocolAnchorCause::WrongContractAnchor {
            entry,
            attempt_anchor,
            clause_anchor,
            ..
        } => {
            assert_eq!(entry, "ParentOrder");
            assert_eq!(attempt_anchor, "M::Actor::op");
            assert_eq!(clause_anchor, "M::Actor");
        }
        other => panic!("a WrongContractAnchor cause, got {other:?}"),
    }
}

/// FR-114-AC-3 (TC-513, SR-771 FND-003): a `contracts` entry naming a
/// `pre` clause of a *different* operation (`M::Actor::other`, not the
/// attempt's own `M::Actor::op`) refuses `wrong_snapshot`/`wrong-anchor`,
/// naming both operation anchors.
#[trace("TC-513", "FR-114-AC-3")]
#[test]
fn a_contract_entry_anchored_at_a_different_operation_refuses() {
    let source = format!(
        "pre ProbePre using v on M::Actor::other {{ true }}\n{}",
        attempt_flow("Flow", "ProbePre")
    );
    let (refusal, cause) = only_cause(&source);
    assert_eq!(refusal.cause.code().as_str(), "wrong_snapshot");
    assert_eq!(refusal.cause.cause(), Some("wrong-anchor"));
    match cause {
        ProtocolAnchorCause::WrongContractAnchor {
            entry,
            attempt_anchor,
            clause_anchor,
            ..
        } => {
            assert_eq!(entry, "ProbePre");
            assert_eq!(attempt_anchor, "M::Actor::op");
            assert_eq!(clause_anchor, "M::Actor::other");
        }
        other => panic!("a WrongContractAnchor cause, got {other:?}"),
    }
}

/// FR-114-AC-2 (TC-513, SR-771 FND-002): `contracts []` still binds the
/// attempt's operation -- its checked attempt carries an anchor and a
/// frame -- with an empty `contracts` list and no refusal, while the
/// same protocol naming a `post` clause of that operation binds exactly
/// that clause's identity.
#[trace("TC-513", "FR-114-AC-2")]
#[test]
fn an_empty_contracts_list_binds_with_no_refusal() {
    let empty = checks(&attempt_flow("Flow", ""));
    let attempt = &empty.protocols()[0].attempts[0];
    assert!(attempt.contracts.is_empty(), "{attempt:?}");

    let named = checks(&format!(
        "post Done using v on M::Actor::op {{ true }}\n{}",
        attempt_flow("Flow", "Done")
    ));
    let attempt = &named.protocols()[0].attempts[0];
    assert_eq!(attempt.contracts, [named.state_clauses()[0].identity()]);
}

/// FR-114-AC-3 (TC-513, SR-771 FND-003): an attempt `on
/// M::Actor::missing`, an operation `M::Actor` does not declare, refuses
/// `missing_declaration`/`missing-name` at `missing` at assembly, the
/// same resolution (and refusal) a `pre`/`post` clause's own operation
/// gets (FR-104).
#[trace("TC-513", "FR-114-AC-3")]
#[test]
fn an_attempt_naming_a_missing_operation_refuses_at_its_name() {
    let source = attempt_flow("Flow", "").replacen("M::Actor::op", "M::Actor::missing", 1);
    let errors = assembly_errors(&source);
    let [error] = errors.as_slice() else {
        panic!("one error: {errors:?}");
    };
    match &error.cause {
        crate::check::AssemblyCause::UnresolvedOperation { context, operation } => {
            assert_eq!(context, "M::Actor");
            assert_eq!(operation, "missing");
        }
        other => panic!("an UnresolvedOperation cause, got {other:?}"),
    }
    let text = format!("{HEADER}{source}\n");
    assert_eq!(&text[error.span.start..error.span.end], "missing");
    assert_eq!(error.cause.code().as_str(), "missing_declaration");
}

/// SR-770 FND-006: a unit with a bad state clause and a bad attempt
/// reports both in one assembly refusal, not only the clause.
#[trace("TC-513", "FR-114")]
#[test]
fn a_bad_clause_and_a_bad_attempt_refuse_together_at_assembly() {
    let source = format!(
        "post Broken using v on M::Actor::absent {{ true }}\n{}",
        attempt_flow("Flow", "").replacen("M::Actor::op", "M::Actor::missing", 1)
    );
    let errors = assembly_errors(&source);
    let operations: Vec<&str> = errors
        .iter()
        .filter_map(|error| match &error.cause {
            crate::check::AssemblyCause::UnresolvedOperation { operation, .. } => {
                Some(operation.as_str())
            }
            _ => None,
        })
        .collect();
    assert_eq!(operations, ["absent", "missing"], "{errors:?}");
}
