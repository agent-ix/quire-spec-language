---
id: SR-13277
title: "Gap analysis of QSL-548 E13e against FR-146, TC-551, ADR-020 RE-2 and ADR-013 O-09 (PR #1040)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@fab2b74c3; spec/functional/FR-146-*.md (AC-1..AC-4), spec/test-cases/TC-551-*.md, spec/decisions/ADR-020-*.md (RM-1, RM-5, RS-2..RS-4, RE-2), spec/decisions/ADR-018-*.md (V-3, V-6, V-8, PC-2, PC-6), spec/decisions/ADR-013-*.md (O-09), spec/functional/FR-144-*.md (context); QSpec spec/functional/analysis/FR-375-*.md and FR-377-*.md (context, quire-specification@ad2b37e); qsl-semantics/src/check/step_simulation.rs, qsl-replay/src/model_check/step_settle.rs, qsl-semantics/tests/it/refinement_declarations.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-146
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-551
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-020
    type: reviews
---
# SR-13277: Gap analysis of QSL-548 E13e against FR-146 (PR #1040)

## Summary

Ticket: QSL-548 (E13e). Tools: `quoin matrix --repo . --json` (quoin
0.28.3, quire 0.36.2, engine 0.50.2), exit 0. Plan completion: not assessed.

The matrix shows all four FR-146 ACs as `tagged`:

- AC-1 by `the_counter_refinement_writes_an_initial_and_a_step_record_per_row`
  and `an_any_row_writes_no_simulation_record`.
- AC-2 by `a_hidden_field_writes_no_simulation_record`.
- AC-3 by `a_concrete_invariant_is_a_hypothesis_of_every_record`.
- AC-4 by the two `step_settle` unit tests.

The PR body calls AC-2 and AC-4 "TC-551 only", but the matrix binds them
anyway: each test's doc comment names its AC.

What was judged:

- **Hypotheses.** Keeping only the invariant clauses of concrete-model
  object types is right. RM-1 makes the declaring package the concrete
  subject, and it may also hold the abstract model, so "every invariant
  clause of the concrete package" read literally would admit abstract
  invariants as induction hypotheses over concrete state. This is a spec
  wording gap (FND-006). The code reads it correctly.
- **No records** when the refinement has a hidden field or an `any` row:
  matches FR-146 and RE-2. History rows are refused before a
  `CheckedRefinement` exists (FR-138 is not built), so they need no check
  here.
- **Settlement.**
  - Holds: `Proved{Inductive{1}}`, `decisive-witness`, `Certified` only
    when the certificate is verified.
  - Counterexample: `Inconclusive(InductionNotClosed{1})`, `unsettled`,
    never refuted.
  - No candidate: `Unsupported(BackendAbsent)`, `unavailable`.

  All three match FR-146 and ADR-018 V-3, V-6 and V-8. The certificate
  input misses PC-2's rejected case (SR-13276 FND-001, and FND-009 below).
- **Declared remainder.** The PR declares four items not built: the
  `operation-contract` requirement record with its extent and occurrence
  key, the refinement node key, RE-3's SMT unrolling with `check_step`, and
  the FR-314 verifier. Declaring them is acceptable, and the code claims
  none of them. But the AC tags and the PR's "Built and tagged: AC-1,
  AC-3" claim more than is built (FND-002 to FND-005), and two of the
  four items have no owning ticket (FND-008).
- **The coder's question.** Does the occurrence-key and refinement-node-key
  scheme need a spec first? Yes for the occurrence key and the obligation
  identity preimage (FND-007). No for the refinement node key: QSpec
  FR-375 specifies the `relation`/`refinement` checked node, and FR-144
  (QSL-544) mints it.
- **SR-13116's order assumption.** QSpec FR-375 does specify an order for
  the checked node's `steps` array, and it is not declaration order
  (FND-001).

## Verdict

**FAIL at FR-146 scope, and not mergeable as tagged.**

- High, FND-001: the `StepRowRef` position disagrees with the order QSpec
  FR-375 gives the checked node's `steps` array. The planner should rule on
  the order before CG reads these records.
- Medium, fix in this PR, FND-002 to FND-005: AC-1 and AC-3 are tagged
  although their requirement-record halves are not built, and AC-2 and
  AC-4 show as `tagged` through doc-comment AC mentions despite the PR's
  "TC-only" intent. Each test should either assert the AC in full or name
  only the TC, with no AC id in its tag or its doc comment.
- Spec and plan gaps for the planner, FND-006 to FND-009, posted as a
  Linear planner note.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `StepRowRef` is the zero-based position in `CheckedRefinement::steps()`, which is source order. QSpec FR-375's checked-package node, the artifact CG reads, holds the step rows in a `steps` array that is "sorted ascending by the node-key digest of its first named declaration, then by member name". That node enters the identity projection. A record's `Step{row: k}` therefore names a different row in the node CG reads than the one QSL meant, unless source order happens to match digest order. Moving a row in the source also changes the record identities, although the node does not change. SR-13116 called the order unspecified; the order is specified in QSpec, just not as declaration order. Planner: rule that `StepRowRef` indexes FR-375's sorted `steps` array (or name the row by its concrete operation's node key), and state it in FR-146 and QSV's `StepRowRef` docs. The PR then computes positions in that order. | qsl-semantics/src/check/step_simulation.rs:19-21, 75-82 |
| FND-002 | medium | The two AC-1 tests carry `FR-146-AC-1`, and the PR body lists AC-1 as "built and tagged". AC-1 says the declaration "writes eight `operation-contract` records beside its refinement record", but the PR builds neither the `operation-contract` requirement record nor the refinement record (FR-144), and lists the former under "Remainder". The tests assert the eight `StepSimulationRecord` values and their empty hypotheses, which is the part that is built. Either narrow the tag to `TC-551` and remove the AC id from the doc comment, or get AC-1 split so that its record-value half can be tagged alone. | qsl-semantics/tests/it/refinement_declarations.rs:1591-1622 |
| FND-003 | medium | `a_concrete_invariant_is_a_hypothesis_of_every_record` is tagged `FR-146-AC-3`, and the PR lists AC-3 as built. AC-3 also requires "the request also holds `CasInv`'s own `operation-contract` record", and nothing builds or tests that. "The `Initial` record's obligation identity differs" is tested only through the record's canonical bytes, and the obligation identity it names is not built (FND-007). Two of AC-3's three conjuncts are not covered. Narrow the tag to `TC-551` as for FND-002. | qsl-semantics/tests/it/refinement_declarations.rs:1636-1661 |
| FND-004 | medium | The PR says AC-2 is "TC-551 only (partly built)". But the matrix reports FR-146-AC-2 as `tagged`, bound to `a_hidden_field_writes_no_simulation_record`, whose doc comment names `FR-146-AC-2`. The `RegisterHistory` half and "each still writes its refinement record" are untested. Coverage is inflated against the stated intent. Remove the AC id from the doc comment so that the binding is TC-only. | qsl-semantics/tests/it/refinement_declarations.rs:1625-1634 |
| FND-005 | medium | The PR says AC-4 is TC-only. But the matrix reports FR-146-AC-4 as `tagged`, bound to the two `step_settle` unit tests, whose doc comments name `FR-146-AC-4`. They settle synthetic enum values. They do not settle `CasRefinesCounter`'s `commitA` record, the lost-update `commitB` record from `(1, 0, f, 0, t)`, or each record with no SMT candidate registered, and no FR-314 verifier stands behind `Verified`. Remove the AC id from the doc comments so that the binding is TC-only. | qsl-replay/src/model_check/step_settle.rs:104-141 |
| FND-006 | medium | FR-146 says "`hypotheses` SHALL list every invariant clause of the concrete package", and RE-2 says "the concrete package's invariant clauses are the induction hypothesis". Under ADR-020 RM-1 the concrete subject is the declaring package, which can also hold the abstract model, so the literal text admits abstract-model invariants as hypotheses over concrete state. The PR keeps only the invariants whose context is a concrete-model object type, which is the sound reading. Planner: change the text to "every invariant clause whose context is an object type of the concrete model", and say whether invariants of a concrete model compiled into another package count. | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md:105-106 |
| FND-007 | medium | The RE-2 occurrence key and obligation identity have no specified scheme. FR-146 keys each record "by an occurrence key derived from the refinement node and the obligation", and says its identity "SHALL bind the refinement node, the obligation and the hypotheses (ADR-013 O-09)". O-07's occurrence key is (node id, role, ordinal), and no role or ordinal is defined for `Initial` or `Step{row}`. O-09's preimage covers four subject kinds (clause, operation application, function contract, parity claim), none of them a refinement step obligation, and it has no `hypotheses` member. This answers the coder's question: the occurrence key and identity preimage need a spec (an O-09 subject kind or an FR-146 preimage) before the requirement record is built. The refinement node key needs none: QSpec FR-375 specifies the `relation`/`refinement` node, and FR-144 (QSL-544) mints it. | spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md:266-276 |
| FND-008 | medium | Two of the declared remainder items have no owning ticket. The FR-314 certificate check belongs to QSL-662 (E-SMTc), and the refinement node key to FR-144 in QSL-544. Nothing found by `ix-board search` owns (a) writing each record as an `operation-contract` requirement record and offering it to negotiation (FR-075, FR-146 "Routing"), or (b) offering the refinement record to SMT unrolling with `check_step` (RE-3, FR-146 "Routing"). QSL-548's "Delivers" says the records "are emitted for SMT discharge", which this PR does not do. Planner: give (a) and (b) tickets, or re-scope QSL-548's Delivers to the record values and the settlement map. | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md:108-115 |
| FND-009 | low | FR-146's settlement bullet gives `Certified` when FR-314 verifies the certificate and `Uncertified` when it is "absent or unverifiable". It does not mention a certificate the checker shows wrong, which ADR-018 PC-2 and PC-6 settle `inconclusive`, `Inconclusive(CertificateRejected{rule, at})`, and never `proved`. Because the FR omits that case, the PR's certificate input has no way to express it (SR-13276 FND-001). Planner: add the rejected case to FR-146's settlement bullet and AC-4. | spec/functional/FR-146-write-per-step-simulation-records-for-a-refinement.md:116-123 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-010 | medium | `step_row_refs` orders rows correctly only if `operation_key` returns the node key that FR-375's lowered refinement node gives each concrete operation. Nothing in QSL mints that key yet (FR-144 and the refinement-node lowering belong to QSL-544). The test's stand-in, `sha256(DeclarationKey.node)`, is the digest of the IR identity string, not the checked-package node key. So the tests show that the order does not depend on source order, but no test shows that the positions match FR-375's `steps` array. Also, the "then by member name" tiebreak compares `DeclarationKey.node`, an `ix://` IR identity rather than a member name. RM-5's one row per concrete operation means the tiebreak never fires, but the docs overstate it. Plan: QSL-544 or the requirements-hook ticket must pass the operation keys from the lowered refinement node (better: read the order from that node), and add a test against a lowered node. Until then, the docs should name the closure as a placeholder. | qsl-semantics/src/check/step_simulation.rs:21-26, 99-131; qsl-semantics/tests/it/refinement_declarations.rs:1583-1588 |
| FND-011 | low | quire-semantic-value's `StepRowRef` docs still say "zero-based position in the refinement's step rows in declaration order (QSL FR-135 `steps`)" (quire-semantic-value@48e1bc3, `src/step_simulation.rs`). This PR now uses FR-375's node-key order, so the shared type's docs contradict the only producer. A QSV follow-up should change the docs once the planner confirms the order (planner note item 1). | quire-semantic-value src/step_simulation.rs:14-19 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | acdf936ce's `step_row_refs` gives each row its position in QSpec FR-375's order: ascending by the concrete operation's node key, then name. `reordering_step_rows_does_not_change_the_row_a_reference_names` swaps the first and last row and asserts that every operation keeps its `StepRowRef`, with positions exactly 0..7. The node key itself is caller-supplied until QSL-544 mints it, and that dependency is FND-010. |
| FND-002 | fixed | acdf936ce tags both AC-1 tests `#[trace("TC-551")]` only and removes the AC ids from their doc comments ("TC-551 step 1, record values only"). The PR body no longer lists AC-1 as built. |
| FND-003 | fixed | acdf936ce tags the AC-3 test `TC-551` only. Its doc comment says that `CasInv`'s own record and the obligation identity are not built. |
| FND-004 | fixed | acdf936ce removes `FR-146-AC-2` from the hidden-field test's doc comment ("TC-551 step 2, the hidden-field case only"), so its binding is TC-only. |
| FND-005 | fixed | acdf936ce rewrites both `step_settle` doc comments as "TC-551 step 4, settlement only", with no AC id. No `FR-146-AC` id is left in qsl-semantics or qsl-replay. |
| FND-006 | deferred | A spec wording change for the planner: planner note item 3 on QSL-548 (comment 53367127). The code already reads the text the sound way, and an abstract-invariant test now pins that reading. |
| FND-007 | deferred | A spec-first item for the planner: planner note item 2 on QSL-548. The occurrence key and obligation-identity preimage are not built in this PR, and the PR says so. |
| FND-008 | deferred | A plan item for the planner: planner note item 5 on QSL-548. Tickets for the `operation-contract` record with its FR-075 offer and for RE-3, or a re-scope of QSL-548's Delivers. |
| FND-009 | deferred | A spec change for the planner: planner note item 4 on QSL-548. The code half landed as SR-13276 FND-001 (`BackendCertificate::Rejected`). |
| FND-010 | deferred | Out of scope for this PR. The concrete operation's checked-node key comes from the refinement-node lowering (QSpec FR-375 node, minted under FR-144), which QSL-544 owns, and `step_row_refs` has no production caller yet. The open work is to pass the lowered node's keys, or read its `steps` order directly, and to test against a lowered node. Recorded for the planner in the round-2 dispositions comment, to be attached to QSL-544 or the requirements-hook ticket. Round 2 (b5b653131) did not change the "caller supplies each concrete operation's node key" docs. |
| FND-011 | deferred | Deferred to quire-semantic-value, which this PR cannot edit. Two places there still say "declaration order": SV FR-123 (`spec/functional/FR-123-step-simulation-record-and-obligation.md:32-35`) and `StepRowRef`'s docs (`src/step_simulation.rs:14-19`, at QSV 48e1bc3). Both need to change to QSpec FR-375's node-key order once the planner confirms it (planner note item 1). |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-012 | low | The main merge brought in STD-212 (#1050), which rewrites FR-146 and adds FR-146-AC-5 to AC-7 and TC-551 steps 5 to 7. AC-7 covers the settlement map given the checker's `Rejected` and `Unverifiable` results directly, and it is fully built: `a_rejected_certificate_settles_inconclusive_never_proved` and the `Unverifiable` case of `a_holding_record_settles_proved_with_the_certificates_standing` assert exactly what it states. The tests carry only `TC-551`, so `quoin matrix` reports AC-7 `untagged`. Tag both with `FR-146-AC-7`. AC-5 should stay TC-only: the test reverses only the first and last rows, not all seven as AC-5 states, and it orders by the `sha256(DeclarationKey.node)` stand-in (FND-010). AC-6 should also stay TC-only: the clause-order test swaps two concrete invariants, not `CasInv` and `AbsInv`. Both could be tagged after small test changes. | qsl-replay/src/model_check/step_settle.rs:124-139, 162-180 |
