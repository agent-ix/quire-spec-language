---
id: SR-16325
title: "spec-review of FR-327 InsufficientData outcome (quire-spec-language PR 1340)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@83273d13; spec/functional/FR-327-evaluate-a-temporal-clause-over-a-finite-trace.md; spec/functional/FR-330-run-a-temporal-clause-through-the-spine.md; spec/functional/FR-194-evaluate-window-aggregates-in-monitors.md; spec/functional/FR-267-write-run-results-as-native-run-result-2.md; spec/test-cases/TC-847-an-undefined-letter-fails-a-supplied-trace.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-327
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-330
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-194
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-267
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-847
    type: references
---

# Spec review of FR-327 InsufficientData outcome (quire-spec-language PR 1340)

## Summary

Ticket: STD-378. One spec-review pass over `git diff origin/main...83273d13`, checked for the no-drop/no-narrowing/no-deferral rule, testable vectors (key vectors derived by hand), id uniqueness across all open PR heads, cross-repo names, mergeability and gates.

Gate: `QSPEC_DIR=<QSpec origin/main 0eef131f> make check-spec-validation`: exit 0, 19 warnings (none in the touched files).

Examined, clean:

- FR-327-AC-7 hand-derived over `Counter` 0, 1, 2: `A` is true at 0 and 1 and short (0, 1) at 2; `B` is short (1, 2) at 0 and true at 1, 2. Results: `always[0,2] A` on origin gives `InsufficientData{where 2, A, 0, 1}`; the `holds(!= 1) and A` clause is false at 1; `holds(!= 2) and B` is no-truth at 0, true at 1, false at 2 so false at 2; the `3 mod (1 - c.value)` clause is undefined at 1; the `3 mod (2 - c.value)` clause with `B` is undefined at 2; `holds(<= 2) and B` gives `InsufficientData{0, B, 1, 2}`; the on-origin `always[0,2] (... and B)` gives `InsufficientData` at 0; over the one-position trace with `c.value` 2 the undefined atom wins over the short atom in both clause-node orders. All match the AC and TC-847 step 6.
- FR-330-AC-8 matches FR-285 (undefined exits 10) and FR-267's `insufficient-data` row. No deferral language; the precedence choice is a stated lead default, not a narrowing.
- Ids: FR-327-AC-7 and FR-330-AC-8 appear on no other open PR head.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | AC-7 writes the atom `A` as `count where holds(c.value != 2) over past[0, 0] >= 1` with no `min_count`. On main FR-194 requires a declared `min_count`, so the vector does not check on main. It checks only after open #1334 (STD-377, optional `min_count` defaulting to 1). The PR names no dependency on #1334. Fix: write `min_count 1` in the vector, or record the merge order after #1334. | spec/functional/FR-327-evaluate-a-temporal-clause-over-a-finite-trace.md:188 | wrong-requirement |
| FND-002 | medium | The precedence rules cover an undefined or refused atom against a short atom at one position, and a false activation at another position against a short one. They do not say what a false sibling atom does against a short atom inside one activation at one position (`holds(c.value != 2) and B` at position 0 with a short `B` and a false `holds`). Under Boolean conjunction it is false; under "has no truth value" it is not. AC-7 has no vector for it. Fix: state the same-position, same-activation rule and add the vector. | spec/functional/FR-327-evaluate-a-temporal-clause-over-a-finite-trace.md:120-134 | correct-requirement-no-evidence |

## Verdict

Mergeable after FND-001, FND-002 (medium) are fixed or dispositioned by the lead.

Conflicts with main: none. Clean merge-tree with open #1303, #1326, #1332 and #1334.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | Fix 89f7c65b. Atom `A` is now written `count where holds(c.value != 2) over past[0, 0] min_count 1 >= 1`, which checks on main, so the vector no longer depends on #1334. |
| FND-002 | fixed | The rule now says an activation has no truth value even when another atom of the same letter at that position is false (ADR-018 UE-1: the letter is the value of every atom there). AC-7 and TC-847 step 6 add `always[0,0] (holds(c.value != 0) and B)` on each: at position 0 `holds` is false and `B` short, so the activation has no truth value; positions 1 and 2 are true; the result is `InsufficientData{where 0, B, 1, 2}`. Hand-checked. |

Round 1 verdict at 89f7c65b: Mergeable: both findings fixed, no new defect. Gate: exit 0, 19 warnings, none in touched files. Merge-tree clean with main and with #1334.
