---
id: SR-20261104231201
title: "PR 2312 (batch T-S1) spec review: numeric, quantity and collection forms"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language PR 2312 diff only; FR-091, FR-093, FR-397, FR-495, TC-483, TC-1091, TC-1284, TC-1334, TC-1335"
review_set: subset
---

## Summary
Spec-only batch for QSL-1655/1656/1654/1454/1485/1488/1601/1602. Re-measured against QSpec origin/main (by ID). These match QSpec: the dropFirst/removeOne typing, lowering, leaves and charges (FR-145-AC-10/11, FR-322-AC-117); the quantity power semantics and the U12/U13/U21 outcomes (FR-142-AC-6, TC-187); the FR-147-AC-3/7/8 mapping and FR-335 `finite_bound_available` semantics; the TC-193 F17/F24/F25/F27/F28/F31/F32/F33 outcomes quoted in FR-397-AC-27/28; and the topology retained fields (FR-142 row table, step (-1), base-dimension rule, U11n/o/p/w/x/z3). The PR names a TC for every new AC, and its trace lines and TC scopes are consistent. No existing AC is weakened. The vague "charges as before" wording becomes explicit. The PR adds no ceremony and no compatibility layer, and cites QSpec by ID only. One medium contradiction: the new Meter bullet conflicts with the existing "Evaluate once" ordering-charge rule and with QSpec FR-148's composition-charge sentence.

## Verdict
CONDITIONAL. FND-001 must be resolved in-PR, either with a carve-out and an OQ row or by aligning with FR-148. The low findings are optional polish.

## Scope
- FR-091 mapping rows, "Integer power spelling", "Topology error fields", AC-71, AC-72 and OQ-15. FR-093 "Integer power", "One-occurrence collection forms", the lowering rows, AC-58 and AC-59. FR-397 "Evaluate once" edit, Meter bullet, AC-27 and AC-28. FR-495 negotiation bullets and AC-7. TC-483 steps 16-17, TC-1091 steps 22-23, TC-1284 step 6, TC-1334 and TC-1335.
- QSpec cross-checks: FR-142, FR-145, FR-147, FR-148, FR-322 (prose and AC-117), the checked-package-v2 schema, the shared grammar, TC-187, TC-190 and TC-193.
- `quire validate` (0.36.2) passes on all 9 files. The one warning is FR-397:156 `ears:non-singular`, which is also on main. `git diff --check` is clean.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The new Meter bullet (FR-397:264-277) says every IEEE node is evaluated through `quire_exact`, that QSL "keeps no QSL copy of an IEEE operation or of its charge schedule", and that the charges are FR-148's "and no other charge". The existing "Evaluate once" rule (FR-397:146-161, FR-397-AC-26) computes the `numericEqual` parts of an ordering inline in QSL, uncharged, and charges at most one `compare_ieee`. That is a QSL copy of numericEqual and a QSL charge schedule. QSpec FR-148 says "The comparison operators are defined by these compositions, so their charges are the charges of the operations composed", so the QSL ordering charge differs from QSpec. Only the word "standalone" separates the two rules. Fix (pick one): (a) add a sentence to the Meter bullet naming "Evaluate once" as the one stated exception (inline numericEqual parts, at most one compare_ieee for totalOrder), and record it as an FR-397 OQ lead default whose reopen condition is a QSpec FR-148 ruling on ordering charges; or (b) align with FR-148 by charging each composed part through `compare_ieee` and amending AC-26. Either way, the deviation must be stated, not implicit. | FR-397:146-161, FR-397:264-277, FR-397-AC-26, QSpec FR-148 |
| FND-002 | low | FR-495-AC-7's two scenarios are the same configuration. "Only a test descriptor advertising (value-validity, bounded)" already has no SMT provider, so "With a lock that also holds no SMT provider, each item still settles requires-bound" tests nothing new. QSpec FR-147-AC-8's clause compares a locked run with an SMT backend registered to one without. Fix: run the first scenario with a cvc5 provider registered beside the bounded descriptor (the finite backend's item is still `requires-bound`), then without one. Mirror this in TC-1284 step 6. | FR-495:164, TC-1284 step 6 |
| FND-003 | low | FR-093:251-259 routes a power with a negative exponent through FR-495's Linked/Kernel zero-divisor observations. FR-495's "Zero divisor, two observations" bullet lists only integer div/rem/mod, decimal/rational `/` and quantity `/`. Fix: add "a quantity `^` with a negative exponent (FR-093)" to that list, so FR-495 stays the single owner of the two observations. | FR-495:101, FR-093:251-259 |
| FND-004 | low | The "Topology error fields" table restates QSpec FR-142's Retains column, and its row 6 drops FR-142's tie-break "when the dimension has several cycles, the one that contains the least unit". Fix: say the fields are FR-142's Retains column as values, and keep only the QSL additions: value representation, FCD identity, and package root first in row 4. Otherwise, restore the tie-break. | FR-091:636 |
| FND-005 | low | FR-091-AC-71 cites U11x but asserts only its first vector. TC-187 U11x also requires that adding `unit e : E = 0;` refuses `invalid-value` at `e`. Fix: add that vector or drop U11x's second half from the claim. | FR-091:1231 |
| FND-006 | low | FR-091-AC-72 does not say whether each refused unit (`bad`, `r2`, `z`, `g`) is assembled alone with the base declarations or alongside `tick`/`tock`. With first-defect order, one combined unit reports one refusal. Fix: "each in its own unit beside the base declarations". | FR-091:1232, TC-483 step 17 |

## Author questions

- Q1, the `e ^ n` spelling (FR-091-OQ-15): acceptable as a lead default. It is recorded with a veto and a reopen condition, uses no new token, and is consistent across FR-091, FR-093, TC-1334 and the shared grammar (`signed-int`, `postfix`/`unary`). It extends a QSpec-owned grammar, so raise it with QSpec as a production request. It does not block this spec PR.
- Q2, the `quire.op.quantity.pow` operand contract: needs a QSpec ruling. Confirmed: FR-322 prose says nothing about it, and only the v2 schema enum names the identity. The QSL choice `[ref(e), ref(n)]` is a sound lead default. FR-093-AC-58's "IR's v2 reader admits each package" cannot be met until QSpec and IR state the contract, so file the QSpec ask now.
- Q3, IEEE negate in `quire_exact`: not a ruling. It is a dependency on the quire-exact lane, which must add an IEEE negate operation. File a blocking ticket for QSL-1655's negation path.
- FND-001 (the ordering charge vs FR-148) needs a QSpec or user ruling, or explicit lead-default recording in-PR.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Option (a): the FR-397 Meter bullet now names the "Evaluate once" ordering composition as its one stated exception (inline uncharged `numericEqual` parts, at most one `compare_ieee` for `totalOrder`), recorded as the FR-397-OQ-1 lead default in a new Rulings table that reopens on a QSpec FR-148 ruling about composed-comparison charges. |
| FND-002 | fixed | FR-495-AC-7 now runs two distinct locked runs: cvc5 registered beside the bounded descriptor, then the bounded descriptor alone with no SMT provider. TC-1284 step 6 and its expected result mirror this. |
| FND-003 | fixed | FR-495 "Zero divisor, two observations" now lists a quantity `^` with a negative exponent, whose base is its divisor (FR-093 "Integer power"). |
| FND-004 | fixed | The FR-091 row 6 field restores FR-142's tie-break: when the dimension has several cycles, the one that contains the least unit. |
| FND-005 | fixed | FR-091-AC-71 adds the U11x second vector (`unit e : E = rational(0, 1);` refuses row 1 `invalid-value` at `e`, carrying scale `0`); TC-483 step 16 assembles it. |
| FND-006 | fixed | FR-091-AC-72 says each refused unit is assembled in its own unit of compilation beside the base declarations, without `tick` or `tock`; TC-483 step 17 matches. |
