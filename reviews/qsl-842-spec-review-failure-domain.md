---
id: SR-12098
title: "failure-domain review of PR 944 (FR-308, FR-218, FR-381 protocol S2/S3 spec)"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quire-spec-language@8fdb69ac17e5f7c148c7317db892d127b3299ec7; spec/functional/FR-218-*.md, spec/functional/FR-381-*.md; porting reference TC-132..TC-135, TC-142, src/protocol_artifact/**, qsl-cst/src/grammar.rs"
review_set: subset
---

## Summary

Ticket: QSL-842. Unstated failure modes in the new S3 rules, and porting completeness against the old
native path (TC-132..135, TC-142) for the author's "not ported" list. Three medium findings; nothing
blocking in this lane. Seven of the eight "not ported" items are outside G1-G3 and owned by another gap;
the proof-budget item is owned by no gap.

## Scope

Examined: FR-381 "Retry, commit and recovery" commit rule and AC-11; FR-381 stage table for decidability;
FR-218 partition proof (AC-19); the "not ported" list. Context: ADR-027 CP-1, CP-4, FO-1, PS-6; QSpec
FR-053, FR-057; the plan's gaps G1-G9; QSL-558's stated scope.

"Not ported" list, checked one at a time:
- Channel delivery interval rows (TC-132): out of scope. The spine grammar's `Channel` (qsl-cst/src/grammar.rs) and QSpec's complete channel production use delivery-policy words, so no delivery interval can be written.
- `Unsupported::Export` scalar payload: G5 / SP-B (binding requirements). Out of scope.
- TC-142 `Invalid::Owner`/`Cycle`: out of scope. The old variants are artifact-intake integrity errors (src/protocol_artifact intake/validate/recovery), so they belong to the reader side (SP-B/G7). The source-level case is ported as FR-381-AC-2 (`foreign`, `forward`), and a capture cycle cannot be written.
- Evaluation rows (TC-142, TC-134 evaluate half): G8 / SP-D. Out of scope.
- TC-133 repeat maximum rows: runtime meaning is in ADR-027 FO-1 (counter, `exhausted` at the maximum); the static part is the grammar's `uint`. Out of scope.
- TC-132/TC-134 occurrence and binding requirements: G5. Out of scope.
- Clock selection: QSL-509 / TemporalTrace. Out of scope.
- Independent-atom proof limit (TC-133 proof budgets): no gap owns it. See FND-002.

The stage table is decidable: `forward`, `foreign`, `stage` and `unbound` are disjoint by declaring
template, stage and source position.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The commit refusal is defined two ways. It applies to "a `commit` node that precedes the forward effect in the compiled causal edge relation", and the stated reason is "so that every execution takes it before the effect". These differ when the commit sits in one `case` of a `choice` before the effect: the commit can reach the effect through edges, yet not every execution takes it. A reachability reading refuses a template that a dominance reading admits, and TC-1058 has no choice row to settle which applies. QSpec FR-057 and ADR-027 CP-1 treat registration after commit as a runtime fact. | spec/functional/FR-381-check-protocol-captures-activation-and-compensation-recovery-at-s3.md:204-210,239; TC-1058 rows 19-20 |
| FND-002 | medium | FR-218 proves a `choice` partition "over the visible facts as independent Booleans", which takes 2^n valuations, but sets no bound and no typed refusal when the bound is reached. FR-218's only limit clause covers nesting. The old TC-133 proof-budget rows (zero, exact and one-short budgets; exhaustion not reported as a control violation) have no owner among G1-G9, so they are lost silently unless a ticket or an AC takes them. | spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md:129-131,255-260; spec/test-cases/TC-133-admit-bounded-choreography-control.md |
| FND-003 | medium | Tracker gap. QSL-558's Spec section implements FR-218 plus the S3 parts of FR-211/212/219/220 and excludes everything else. This PR moves the capture, activation and compensation checks into the new FR-381, so no ticket implements FR-381. | Linear QSL-558; spec/functional/FR-381-check-protocol-captures-activation-and-compensation-recovery-at-s3.md |

## Verdict

Nothing blocking in this lane. The "not ported" list holds except for the proof-budget item (FND-002).

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | The commit rule is dominance: a commit on every path to the effect is refused; a commit in one `case` is admitted (TC-1058 row 20a). |
| FND-002 | recorded | The partition proof bound and the old proof-budget rows are owned by no slice; recorded in the PR body. |
| FND-003 | recorded | Tracker gap: no ticket implements FR-381; recorded for the planner. |
