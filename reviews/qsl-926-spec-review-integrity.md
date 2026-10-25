---
id: SR-17191
title: "QSL-926 PR #1524 integrity review of FR-337, FR-125-AC-18 and their test cases"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language; spec/functional/FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md, spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md, spec/test-cases/TC-1265-an-on-each-clause-emits-its-trigger-and-guard.md, spec/test-cases/TC-520-model-behaviours-read-as-temporal-traces.md, spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md, spec/functional/FR-338-check-an-en-1-closure-certificate.md, spec/functional/FR-339-check-an-en-1-component-certificate.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-337
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-125
    type: reviews
---

## Summary

Ticket: QSL-926. PR: quire-spec-language#1524 (head a8b073ce6). This lens checks completeness, consistency and atomicity inside the change, and whether the TCs cover each criterion. criterion-strength could not run (no Jev client), so each AC was also read for whether it can fail.

Examined units: FR-337-AC-6, FR-337-AC-7, FR-125-AC-18, FR-126-AC-16, FR-338-AC-10, FR-339-AC-5, TC-1265 (steps 1 to 4 and expected results), TC-520 step 17, TC-521 step 9, TC-525 step 9, TC-1201 and TC-1215 fixture notes.

Coverage map: FR-337-AC-6 is TC-1265 steps 1 and 2. FR-337-AC-7 is steps 3 and 4. FR-125-AC-18 is TC-520 step 17. Every case of each AC has a procedure step and an expected result.

The fixture wording ("test-built", "in the shape S4 emits (FR-337-AC-6)", "trigger the `on each` parameter's node") now agrees across FR-126-AC-16, FR-338-AC-10, FR-339-AC-5, TC-521, TC-525, TC-1201 and TC-1215. None still says "the `over` parameter's node" or "S4 refuses an `on each` clause".

The Counter vectors were checked by hand. `= 1` gives `Violated` to 2 and `>= 2` gives `Holds`. `>= 3` violates at 3. The `mod` guard is defined at 0 and 1 and divides by zero at 2. The AC-7 refusals agree with FR-104-AC-12 and FR-381-AC-10, confirmed by an S3 probe (SR-17190).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-125-AC-18's `G` case is vacuous as tested. `always holds(true)` `on each (y: M::Dog) when (G)` settles `proved` under any defined reading of `G`, so a seam that read `G` false everywhere, or true everywhere, would pass. TC-520 step 17 expects only "`proved` for `G`". The AC's "reads `G` false at `t` and true at `k` and `d`" is never observed. Either assert the seam's `activation` reading per instance directly in step 17, or add a body that fails where the clause activates, for example `always holds(false)` `when (G)` returns `Violated` bound to `k`. | FR-125-AC-18, TC-520 step 17 |
| FND-002 | low | FR-337-AC-6 says the I2 reader "admits each package and returns each clause's triple and `over` node", but TC-1265 reads back only step 1's package. Step 2's guard-less, `when (true)` and `on origin` packages are not read back. Add the read-back to step 2 and its expected result. | FR-337-AC-6, TC-1265 step 2 |

## Verdict

Acceptable on integrity once FND-001 is fixed. The fixture statements are now consistent, and each AC has a step.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-125-AC-18 now asserts the seam's `G` reading per instance at the initial state (false for `t`, true for `k` and `d`), and that `always holds(false)` `when (G)` refutes naming `k` at position 0. TC-520 step 17 and its expected result read both. |
| FND-002 | fixed | TC-1265 step 2 and its expected result now read each package back through the I2 reader. |
