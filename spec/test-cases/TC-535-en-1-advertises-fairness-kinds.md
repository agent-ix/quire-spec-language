---
id: TC-535
title: "Candidates carry and are filtered by the fairness kinds their backends advertise"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-134
    type: verifies
---
# TC-535: Candidates carry and are filtered by the fairness kinds their backends advertise

## Description

Verify that EN-1 registers advertising weak and strong fairness, that the
registry carries each backend's fairness kinds to its candidates, and that
it leaves out a backend lacking a fairness kind the item uses.

Scope: FR-134-AC-1, FR-134-AC-2, FR-134-AC-3, FR-134-AC-4, FR-134-AC-5, FR-134-AC-6.

## Test Procedure

1. Register EN-1 and a test temporal backend advertising only `Weak`;
   compute the candidates for an (`temporal-satisfaction`, `unbounded`)
   item with a weak constraint, then with a `strong` constraint.
2. Register only the `Weak` test backend; compute the candidates for the
   item with a `strong` constraint and settle it.
3. Admit each manifest below, advertising (`temporal-satisfaction`,
   `unbounded`) unless stated, with the `fairness_kinds` member as given, and
   record the outcome:

   | Vector | `fairness_kinds` | Outcome |
   |--------|------------------|---------|
   | V1 | `["weak"]` | admitted, `{Weak}` |
   | V2 | `["weak","strong"]` | admitted, `{Weak, Strong}` |
   | V3 | `[]` | admitted, no kind |
   | V4 | `["Weak"]` | refused `invalid_capability`/`unknown-fairness-kind`, names `Weak` |
   | V5 | `["weak","eventual"]` | refused `unknown-fairness-kind`, names `eventual` |
   | V6 | `[""]` | refused `unknown-fairness-kind`, names the empty label |
   | V7 | `["weak","weak"]` | refused `duplicate-fairness-kind`, names `weak` |
   | V8 | `["weak","strong","weak"]` | refused `duplicate-fairness-kind`, names `weak` |
   | V9 | member absent | refused `absent-fairness-kinds` |
   | V10 | `["weak","weak","eventual"]` | refused `unknown-fairness-kind` (unknown before repeat) |
   | V11 | `["weak",1]` | refused `unknown-fairness-kind`, names `1` |
   | V12 | `null` | refused `unknown-fairness-kind`, names `null` |
   | V13 | `"weak"` (a string, not a list) | refused `unknown-fairness-kind`, names `"weak"` |
   | V14 | `["x","y"]` | refused `unknown-fairness-kind`, names `x` (first in member order) |
   | V15 | `["strong","weak","weak","strong"]` | refused `duplicate-fairness-kind`, names `weak` (the first element that repeats an earlier element) |
   | V16 | `["Weak"]`, advertising (`temporal-satisfaction`, `unbounded`) and (`Foo`, `unbounded`) | refused `invalid_capability`/`unknown-kind` (the pair cause wins) |
   | V17 | member absent, with a (`temporal-satisfaction`, `bounded`) pair and no `domains` | refused `invalid-domains` (the `domains` cause wins) |

4. For each of V4, V7 and V9, register the manifest under an identity that
   already holds an admitted registration, and compute that identity's
   candidates afterward.
5. Admit a manifest advertising only (`local-projection`, `unbounded`), which
   FR-057 admits without `domains`, with `fairness_kinds` absent, then
   `["Weak"]`, then `null`.

Tag the tests `#[trace("TC-535", "FR-134-AC-n")]`.

## Expected Results

- Step 1: EN-1 with `{Weak, Strong}` and the test backend with `{Weak}`;
  then EN-1 alone.
- Step 2: an empty candidate set reported as FR-075 reports one, naming the
  missing `strong` capability; `Unsupported`,
  `unsupported-requested-capability`.
- Step 3: the outcomes in the vector table, each refusal keyed by backend
  identity; V3 is a candidate for an item with no fairness constraint and for
  none with a `weak` or `strong` constraint.
- Step 4: each refusal leaves the earlier registration standing, and its
  candidates are unchanged.
- Step 5: all three manifests are admitted and the member is not validated.
