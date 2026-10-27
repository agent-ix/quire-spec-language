---
id: SR-60049
title: "review of the accepted execution point carried in V2 state_clause/operation_anchor identities (quire-spec-language PR 1665, QSL-1082)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@23d308939 and 4dad159c6 (empty tree diff from merged 1348ba086); qsl-semantics/src/check/lowering/state.rs (check_execution_point, operation_anchor), check/node_key/mod.rs, value/member.rs (AcceptedPoint), check/refusal.rs, check/mod.rs gate; qsl-package/src/emit/tests/receiver_frames.rs; against QSpec aaedd51c FR-322/341/342/464 and QSL FR-105-AC-12"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-105
    type: references
---

# Review of PR 1665 (QSL-1082), merged as 1348ba086

## Summary

Verdict PASS with findings; no HIGH. Key computation is correct: 25 of 25 QSpec operation vectors match at aaedd51c, and the four execution-point vectors re-hash (canonical JSON sha256) to their published digests. The gated emission is honest. The main weakness is that the lowering that attaches a point to the member and the anchor has no test: three of my mutations of it survive (FND-001, MEDIUM). Note: the PR merged while this review ran, so the SR cannot ride the PR branch.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | Lowering with a non-None point is untested. `lower_state_clause`/`operation_anchor` are only ever called with `None` in production and tests; key tests build bodies by hand. Surviving mutations: dropping the anchor `execution_point` binding push; dropping `execution_point` from the lowered `Member::StateClause`; not passing the clause point to `operation_anchor`; moving `frame` after `execution_point` in the lowered anchor. All pass every test. K15 (point-bearing anchor vs point-free attempt anchor are distinct nodes) is covered only at the hand-built key level, not through lowering. Fix: one lowering test with a point on a precondition and an attempt on the same operation, asserting the two anchor keys differ and the clause member and anchor carry the same point. | FR-105-AC-12; FR-341; FR-342 | correct-requirement-no-evidence |
| FND-002 | low | QSL is stricter than QSpec FR-341 step 4 text. The text says precondition and postcondition "take pre or post"; QSL (matching src/checking/bindings.rs and ClauseBinding semantics: precondition pairs with Pre only, postcondition with Post only) refuses a precondition carrying post. QSL is right (the accepted binding can never produce that pair; accepting it would let the IR read a clause at the wrong point). Divergence should be closed in QSpec (tighten step 4 to precondition=pre, postcondition=post), not by loosening QSL. | FR-105-AC-12; FR-341; FR-342 | wrong-requirement |
| FND-003 | low | `AcceptedPoint` Hash: variant tags 0..3 plus name; Eq is the IR type's derived Eq, so Hash is consistent with Eq. Mutating tag 1 to 0 survives, which is legal (collision) and only costs hash quality. No identity inconsistency. | FR-105-AC-12; FR-341; FR-342 | correct-requirement-no-evidence |
| FND-004 | low | The refusal rides `CheckCause::InternalFault(KeyFault::ExecutionPoint)`. Acceptable: the accepted binding is already validated at src/checking/bindings.rs before lowering, so a mismatch here is an invariant breach, and with the gate no production path reaches it. | FR-105-AC-12; FR-341; FR-342 | correct-requirement-no-evidence |
| FND-005 | low | Gate is honest: production passes `execution_point: None` (single construction site, check/mod.rs, with a comment), no other constructor of StateClauseInput exists, and `the_pinned_ir_reader_refuses_an_execution_point_at_either_carrier` fails when IR admits the carrier (ticket IR-1042). The ungate commit must also add the FND-001 test. | FR-105-AC-12; FR-341; FR-342 | correct-requirement-no-evidence |

## Mutation results (qsl-semantics lib)

Killed: member wire drops the point (3 tests); swap pre/post applicability; skip operation-name check; anchor point serialized as unit. Survived: FND-001 set; Hash tag collision (FND-003). My "none operation admitted" mutation did not compile and is not counted.

## Conventions

No `#[allow]`, no new wildcard arms in production (the one `_ => {}` is in a test), refusals typed, string compare under `#[ix_trace_rs::string_edge]`. New tests carry no `#[trace]` (LOW, consistent with FND-001's follow-up).

## Gate

`cargo fmt --all --check` clean. `make ci`, own target dir, on head 23d308939: with QSPEC_DIR at aaedd51c the graph stopped at `conformance_c14_source_map_lookup_over_qspec_positive_fixtures` (owner_model unknown member, QSL-1147 class); preceding checks, including 25 of 25 operation vectors, passed, later lanes not reached. Pristine main at the same QSpec: the qsl-package conformance filter fails the same 13 tests (identical sets) and the 25-vector check fails (expected; fixed by this PR). With QSPEC_DIR at ccad669c: exit 0, every lane green (21 operation vectors). `cargo test --test it layer`: 14 passed. Not re-run on 4dad159c6, whose tree equals merged main.
