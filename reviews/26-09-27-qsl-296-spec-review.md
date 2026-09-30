---
id: SR-752
title: "Spec review of QSL-21a protocol frame and scoped anchor FRs"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; spec/functional/FR-112-build-protocol-scoped-anchor-forms.md; spec/functional/FR-113-resolve-scoped-anchors-in-nested-control-scopes.md; spec/functional/FR-114-bind-a-protocol-attempt-to-its-operation-frame.md; spec/functional/FR-115-run-an-operation-frame-over-an-invocation.md; spec/functional/FR-116-replay-a-frame-counterexample.md; spec/functional/FR-088-clause-name-and-type-identity.md; spec/functional/FR-104-check-state-clauses.md; spec/functional/FR-105-emit-state-nodes.md; spec/functional/FR-109-run-a-state-clause-through-the-spine.md; spec/test-cases/TC-510 to TC-515; spec/spec.md; spec/tests.md; spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md; context read: ADR-012 §12.2, FR-072, FR-098, FR-106, src/linking/composed/scopes/protocol.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-112
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-113
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-114
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-115
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-116
    type: reviews
---
## Summary

Ticket: QSL-296 (QSL-21a). PR: quire-spec-language#497. The review
covers integrity, scope and failure domain, interfaces, and evidence. It also
checks each AC against its TC by hand. The TCs are Planned, which is expected
for a spec-only PR.

Sound:
- FR-114 amends FR-104 and FR-105 and adds no second frame node. FR-115
  reuses FR-106 check 11. There is no duplication.
- All six ADR-011 edits (:866, :944-945, :1113, :1187, :1390, :1425) match
  the M-6a and M-6c rows (:1070, :1072).
- FR-088-CON-2 still excludes frame semantics and points at FR-114 to FR-116.
- The FR bodies apply ruling 4 (ambiguous_declaration) and add no FrameForm,
  no Frame/ScopedAnchor CheckedClauseKind and no finish-write rule.
- The FR and TC ids do not collide with origin/main.
- The ACs are concrete and testable.
- `quire validate` over the 16 changed files exits 0; its one EARS warning is
  at FR-104:33, which this PR does not change.
- `make check-index-completeness check-no-committed-binaries` exits 0.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Rulings 1 to 3 (2026-09-27) are not recorded in the repo. ADR-012 §12.2 still specifies what they strike or hold: the Frame and ScopedAnchor clause kinds (:960, :961), a v2 scoped anchor node (:962, :970), the `frame` source production (:963), `FrameForm` (:964), the `finish` postcondition-write check (:965), and Frame/ScopedAnchor checked clause subnodes (:967). FR-112 cites "§12.2 Parse and Form rows" as its source (FR-112:22, :76). Nothing in FR-112 or FR-113 says ScopedAnchor is check-internal. Failure: a QSL-297 coder follows FR-112 to the §12.2 Form row and builds FrameForm and a frame production, or a QSL-299 coder adds a ScopedAnchor CheckedClauseKind and wire node, contradicting FR-105 and FR-088-AC-4. Fix: amend the §12.2 rows with the rulings. Then add one line to FR-112 or FR-113 saying ScopedAnchor is a check-internal representation with no CheckedClauseKind variant and no v2 node. | spec/decisions/ADR-012-semantic-family-extension-contracts.md:960-970; spec/functional/FR-112-build-protocol-scoped-anchor-forms.md:22,76 |
| FND-002 | high | FR-116's `reproduced-with-evaluated-witness` path contradicts merged FR-072 and FR-098. FR-115 reports frame_violation as an `evaluate` refusal. FR-072:80-85 says a refused S6a outcome or a family result completes no value, so it SHALL settle `inconclusive` with `NoValue` whatever the verdicts are. FR-098:133-137 says a verdict other than `violation` settles `Verdicts`. FR-116 also settles `Verdicts` for "same violation at another member". Under FR-072 the two verdicts there are equal, and FR-072's `Verdicts` cause holds verdicts, not changes. Failure: FR-116-AC-1 (TC-515 step 1) and AC-2 cannot pass while FR-072 (TC-189/190) holds. Fix: either say how a frame check maps to a verdict and value on the replay arm (for example, the frame check as a Boolean completed value), or amend FR-072 and FR-098 in this PR with the frame case. | spec/functional/FR-116-replay-a-frame-counterexample.md:78-87; spec/functional/FR-072-implement-typed-replay-result.md:80-85; spec/functional/FR-098-execute-a-replay-request.md:133-137 |
| FND-003 | medium | FR-113 resolves each anchor by name only. It never checks the target's node kind. QSpec names the kind each site takes: `receive ... of SendNode`, `effect ... of AttemptNode`, `event ... for Compensation`, and `compensate ... for` a successful effect node (choreography-surface.md:24, :58-62). The composed checker FR-113 replaces refuses `WrongTargetKind` (protocol.rs:388-391, `StructuralKind::accepts` :631-653) and a receive on a channel other than its send's (`IncompatibleReference`, :595-606). Failure: `effect Applied of Committed` or `compensate Undo for Main::Tried` checks with no refusal. When M-6d deletes the composed checker (QSL-303), that refusal is lost. Fix: add a target-kind rule per FR-112 site, and the channel rule, each with a catalog code (the nearest existing row looks like `ill_typed`/`type-mismatch`, which keeps the expected and actual kind), plus an AC and a TC-511/512 step. | spec/functional/FR-113-resolve-scoped-anchors-in-nested-control-scopes.md:48-85 |
| FND-004 | low | FR-109's amendment adds the `Frame` selection (:55-56). It leaves "The entry SHALL resolve the selected name in the compiled package's one name table of state clauses and functions" (:98-102) unchanged. A Frame selection names an operation, which is not in that table, so the two now conflict. Fix: scope that bullet to Clause/Function and cite FR-115 for Frame. | spec/functional/FR-109-run-a-state-clause-through-the-spine.md:98-105 |
| FND-005 | low | FR-115 reuses FR-106 check 4, which compares a document's model to "the package's model selection for the clause's alias". A Frame run has no clause. Fix: say the operation's declaring type's alias. | spec/functional/FR-115-run-an-operation-frame-over-an-invocation.md:56-61; spec/functional/FR-106-admit-snapshots-and-invocations.md:191-193 |
| FND-006 | low | ADR-011:1262, in the dated 2026-09-23 OQ-2 ruling, still says "`lowering` and IT-010 are deleted in M-6a" with no amendment note. The 2026-09-22 ruling at :1327 carries "Amended 2026-09-24" for the same change. The coder's six edits are correct; this is a seventh line of the same kind. Fix: append the same amendment note. | spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md:1262 |

## Verdict

Request changes. FND-001 and FND-002 must be fixed in this PR. FND-003 should
be fixed in this PR, because the checker it specifies replaces one that
already refuses this case. FND-004 to FND-006 are small edits to make in the
same round.
