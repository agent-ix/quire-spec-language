---
id: SR-13659
title: "base review of STD-177 FR-150-AC-2 against FR-145 (PR #1143)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@5a71fd301e44e5072238efd8a3308780b5279b55; spec/functional/FR-150-check-the-symmetric-population-annotation.md, spec/test-cases/TC-565-s3-accepts-and-refuses-the-symmetric-annotation.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-150
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-565
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-021
    type: references
  - target: ix://agent-ix/quire-spec-language/US-019
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-566
    type: references
---
# SR-13659: base review of STD-177 (PR #1143)

## Summary

Ticket: STD-177 (consumers QSL-539, QSL-871). Base checklist over
`git diff origin/main...5a71fd301` (2 files, +12/-3). The diff adds a
precedence bullet to FR-150's Behavior, restates FR-150-AC-2 and updates
TC-565 step 2. Spec-only diff, so code-review and gap-analysis do not apply.
Sub-analyses run: ears-conformance (SR-13660), integrity (SR-13661),
failure-domain (SR-13662).

Checks made by hand:

- QSpec main (`30c9301`) FR-145 "Fold and reduce admission": a set or bag
  step is in the catalog only when its body is `acc OP t` or `t OP acc`,
  `acc` is not in `t`, `t` has type `A`, and `OP` is `+`/`*` on `Integer` or
  `and`/`or` on `Boolean`; any other step is
  `refused { code: ill_typed, cause: operator-ineligible }`. Typing is
  checked first (`type-mismatch` wins). `A` is a `qualified-name`, and
  because `Integer` and `Boolean` are keywords they are named through an
  alias such as `type Total = Integer;`. The PR quotes the code and cause
  correctly.
- Code at the PR head: `qsl-semantics/src/check/check/typing.rs` `fold`
  (lines 2570-2602) refuses an unordered source with a non-catalogued step
  with `identity_observing(...)` when the source type contains a reference
  into a `symmetric` population, and otherwise with `ineligible`
  (`operator-ineligible`). `catalogued_step` (`check.rs` 3678) implements
  FR-145's catalog with `t`'s type equal to `A`. The author's claim holds.
- Probe (scratch test over TC-458's ConfigVersion fixture with `peers`,
  uncommitted), Set and Bag `peers`, annotated and not:
  non-catalog Boolean fold (`LAST_PEER`), the AC-2 "choose the parent" fold
  `fold<Peer>(acc, p in self.peers: p, identity: self)` in a post clause and
  an invariant, `reduce<Peer>`, `acc + acc`, and both nestings of a catalog
  and a non-catalog fold over `self.peers` all refuse
  `identity-observing-form` annotated and `operator-ineligible` unannotated.
  `fold<Tally>(acc, p in self.peers: acc + 1, identity: 0)` and
  `fold<Flag>(acc, p in self.peers: acc or p = self, identity: false)` check
  in all four documents. Non-reference sources keep FR-145's outcome in the
  annotated unit. A `Sequence` of references with any step checks, which
  matches form 1's `Set`/`Bag` scope. A `Bag` function parameter's `reduce`
  of references refuses `identity-observing-form` annotated and
  `operator-ineligible` unannotated.
- The AC-2 fixture as written, `fold<Integer>(...)`, fails at S1 (FND-001).
- AC numbering: no AC added or renumbered; no collision.
- Wording sweep of the FR-150 relatives (ADR-021, US-019, TC-565, TC-566,
  QSpec FR-381) for other statements of the same rule (FND-002, FND-004 to
  FND-006; SYM-3 in SR-13661).
- Gates: `make check-spec-validation` exits 0 with 17 warnings, identical to
  main and none on a touched file; `tools/check-index-completeness.sh`
  exits 0 on both. Tools: quire 0.36.2 (engine 0.50.2), quoin 0.28.3.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-150-AC-2's and TC-565 step 2's counting fold `fold<Integer>(acc, p in self.peers: acc + 1, identity: 0)` cannot be written: `Integer` is a keyword and FR-145 makes `A` a qualified name, so S1 refuses it `invalid_syntax`/`unexpected-token` ("expected identifier") at `Integer`. Use `type Tally = Integer;` and `fold<Tally>(acc, p in self.peers: acc + 1, identity: 0)`; that form checks in both units, Set and Bag. | spec/functional/FR-150-check-the-symmetric-population-annotation.md:102; spec/test-cases/TC-565-s3-accepts-and-refuses-the-symmetric-annotation.md:22 |
| FND-002 | high | ADR-021 §7.1 "Broken symmetry" still says that the `peers` unit with the post clause that folds to choose the parent checks without the annotation ("Without the annotation the unit checks"). That is the claim this PR removes from FR-150-AC-2, and FR-150-AC-1/AC-2 build on §7.1's unit. Under FR-145 that unit refuses `operator-ineligible`. Restate it in this PR. | spec/decisions/ADR-021-state-space-reduction.md:452-457 |
| FND-003 | medium | FR-150-AC-4 and TC-566 step 2 say a `fold` over a `Set<Int[0, 3]>` field checks but name no step. Under the catalog rule this PR now states in FR-150, the natural sum `fold<Tally>(acc, x in s: acc + x, identity: 0)` refuses `operator-ineligible` in both units (`t` is `Int[0, 3]`, not `A`), while `acc + 1` checks. Name a catalog step in AC-4 and TC-566. | spec/functional/FR-150-check-the-symmetric-population-annotation.md:104; spec/test-cases/TC-566-every-identity-observing-form-is-refused-in-every-clause-kind.md:22 |
| FND-004 | medium | QSpec FR-381 still says the same forms over an unannotated population are admitted (Behavior), and FR-381-AC-2 says the unannotated `fold` unit is admitted. Both contradict QSpec's own FR-145 and the restated FR-150-AC-2. FR-381 also has no rule for which cause wins in an annotated unit. FR-150 says the code and cause are QSpec's and that QSpec owns the forms. Needs a paired QSpec change, or an owner ruling recorded on STD-177 that QSL owns the precedence (the author flagged this as owner-level). | spec/functional/FR-150-check-the-symmetric-population-annotation.md:74-85; quire-specification spec/functional/analysis/FR-381-check-symmetric-populations.md (Behavior "Scope of the check", FR-381-AC-2) |
| FND-005 | low | Several relatives still say any fold over a set of `ConfigVersion` references is refused, but the counting fold over the same set now checks. These are FR-150's Use case, US-019-EX-2, TC-565's title and Description, and TC-566 step 1's "a state constraint with the `fold`". Qualify each, for example "a fold whose step is outside FR-145's catalog". | spec/functional/FR-150-check-the-symmetric-population-annotation.md:35-37; spec/usecase/US-019-check-a-larger-model-with-state-space-reduction.md:79-82; spec/test-cases/TC-565-s3-accepts-and-refuses-the-symmetric-annotation.md:3,13 |
| FND-006 | low | "A form listed above over a population that is not annotated SHALL check as it does today." "Today" is time-relative status wording. A non-catalog fold over an unannotated population now refuses `operator-ineligible`, so "SHALL check" reads as "is admitted", which contradicts the new bullet. Restate it as "is checked by the rules that apply without the annotation". | spec/functional/FR-150-check-the-symmetric-population-annotation.md:91-92 |

## Verdict

Not mergeable as is: two high findings, two medium and two low. The rule
the PR states is correct, and the code does exactly what it says in every
case probed: annotated or not, catalog step or not, Set or Bag, fold or
reduce, nested, and reference or non-reference source. FR-145's catalog and
cause are quoted correctly. The gates pass with no added warnings.

FND-001 is the new fixture failing to parse, a one-token fix. FND-002 is
the same false claim left in ADR-021 §7.1, which FR-150's unit is drawn
from. FND-003 and FND-005 are relatives that the new rule makes inaccurate.
FND-004 needs either a paired QSpec PR or an owner ruling. The author asked
for an owner decision on the precedence, so it may be deferred with a STD
ticket.

Examined and clean: FR-150-AC-2's annotated half and its "choose the
parent" unannotated half; the precedence bullet's code and cause; TC-565
step 2's expected results apart from the fixture spelling.

The consumer test
`symmetric_population::a_fold_over_references_into_an_annotated_population_refuses`
is still tagged `TC-565` without `FR-150-AC-2` and does not yet exercise the
counting fold. That is a follow-up for the code lane and not a finding on
this spec diff.
