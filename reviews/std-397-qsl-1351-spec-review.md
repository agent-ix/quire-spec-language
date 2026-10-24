---
id: SR-16296
title: "Spec review of quire-spec-language PR 1351: STD-397"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@b65f16ce; spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md; spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md; spec/functional/FR-107-evaluate-state-clauses-at-s6a.md; spec/functional/FR-397-select-the-ieee-profile-of-a-package-that-declares-or-evaluates-a-float.md; spec/test-cases/TC-1091-a-package-that-declares-or-evaluates-a-float-selects-the-ieee-profile.md"
review_set: subset
date: 2026-10-23
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-107
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-397
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-1091
    type: references
---

# SR-16296: Spec review of quire-spec-language PR 1351 (STD-397)

## Summary

Ticket: STD-397. Head b65f16ce (PR 1351). The head merges an earlier head of PR 1347 (21290dff), not its current head edbe57f5; the two differ only by main merges, and the own delta is taken from the non-merge commits ad0c494f and a3b21ef6 (five files). origin/main is not an ancestor; merge with main is clean.

The AC vectors were derived by hand and hold: comparison compositions (`a < b` = N(a) and N(b) and totalOrder(a,b) and not numericEqual(a,b); `-0 < +0` false, `-0 <= +0` true), sqrt(2) at nearest-even 0x3ff6a09e667f3bcd and toward-zero ...bcc, the fma example 0x3c9ffffffffffffe against 0 for multiply-then-add, 2^53+1 conversions (0x4340000000000000, toward-positive ...01), float32 1/3 (0x3eaaaaab, 0x3eaaaaaa), NaN payload keeping (0x7ff8000000000001 to 0x7fc00001, 0x7ff8000040000000 refused), negate bit flips, and the structural-equality vectors (numericEqual per leaf, bitIdentical for set members). Names (`float32(bits: ..)`, `quire.op.ieee.*`, `ieee_not_exact`, `ieee_nan_payload_not_representable`, `ieee_rational_out_of_domain`) exist in the QSpec stack. Stale refusal text for float constants, comparisons and unary minus is gone from the tip.

Also checked:

- FR-397-AC-1..22 and FR-091-AC-48/49 are unique across the 72 other open PR heads. Gate: exit 0, 19 warnings, same as main. Merge-tree conflicts with open PRs 1335, 1333 (FR-093) and 706.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | FR-397-AC-14 and FR-397-AC-22 require `Set<Float64[nearest-even]>` to check and lower, but at this head FR-098 says S3 admits no set type whose element type bears a float (QSpec FR-144-AC-6, FR-323-AC-3), and FR-323-AC-3 and ADR-012 still refuse it. Only PR 1357 (STD-396, later in the chain) lifts that refusal. The two ACs are unrealizable at this PR's own head and the PR body names no dependency on 1357. Fix: move the Set rows of AC-14 and AC-22 into PR 1357, or record that 1351 and 1357 land together. | spec/functional/FR-397-select-the-ieee-profile-of-a-package-that-declares-or-evaluates-a-float.md:269 | wrong-requirement |
| FND-002 | low | Lead default 4 refuses a same-width `convert` that only changes the rounding mode (`type-mismatch`); the stated route through `Rational[..]` is Undefined for a NaN or an infinity, so those values cannot change mode. QSpec FR-148 defines no such operation, so the refusal follows the oracle and the capability did not exist before. Recorded for the lead's ruling under the standing no-narrowing rule; if the lead treats it as a narrowing it is high. | spec/functional/FR-397-select-the-ieee-profile-of-a-package-that-declares-or-evaluates-a-float.md:225 | missing-requirement |

## Verdict

**Not mergeable.** FND-001 (high) block the PR.
