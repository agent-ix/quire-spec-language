---
id: SR-12815
title: "Gap analysis of FR-135, FR-136, FR-137 refinement checks (QSL-543, PR #970)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@3d0b0a27672bd3af0661f9c502be871147439284; PR #970; spec/functional/FR-135, FR-136, FR-137, FR-310; qsl-semantics/tests/it/refinement_declarations.rs, qsl-forms/tests/it/refinement_forms.rs, qsl-cst/tests/it/refinement_declaration.rs"
review_set: subset
evaluated_revision: "3d0b0a27672bd3af0661f9c502be871147439284"
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-135
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-136
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-137
    type: reviews
---
# Gap analysis of FR-135, FR-136, FR-137 (PR #970)

## Summary

Ticket: QSL-543. Planless. Plan completion: not assessed.
`quoin matrix --json` (quoin 0.28.3) shows these as tagged: FR-135-AC-1..4,
FR-136-AC-1..3, FR-137-AC-1, AC-2 and AC-4, FR-310-AC-1 and AC-3. These are
untagged: FR-136-AC-4, FR-137-AC-3 and FR-310-AC-2. All tagged tests ran
green.

Author-reported deferrals, and my judgement of each:

- FR-136-AC-4 and FR-137-AC-3 (protocol subject). Legitimate. No requirement
  says how a refinement header selects a concrete protocol subject, so this
  is a spec gap for the owner. Leaving them untagged is correct.
- The node-identity halves of FR-135-AC-4 and FR-137-AC-1, and the
  `AbstractSide::Dependency` half of FR-135-AC-1. The deferral is
  legitimate, since they need S4 emission. The tags are not (FND-001).
- FR-361-AC-3/AC-4 wait on PR #964. Legitimate.
- FR-138, FR-139 and FR-147 rows are not parsed, and FR-310-AC-2 is
  untagged. Legitimate, since each is its own FR.

## Verdict

The coverage of what was built is good. Each refusal test asserts the code,
the cause tag, the exact source text of the refusal region, and that it is
the only refusal. The positive tests assert the resolved keys. There are no
stubs. Every new production symbol is reached by a tagged test.
`a_checked_refinement_refuses_not_emitted_at_package_build` runs
`PackageDeclarations::check` without the test flag, and that is the
package-build gate.

One medium finding: three AC tags claim more than their tests cover. The
other finding is low.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Three tests carry a criterion tag although part of the criterion is untested: FR-135-AC-1 (the `AbstractSide::Dependency` half), FR-135-AC-4 (the "changes the node identity and the `package_id`" half) and FR-137-AC-1 (the "node identity differs" half). The doc comments say so, but the matrix shows all three ACs as covered, so nothing would flag them when S4 lands. The repo's own precedent for an S4-gated half tags only the TC: `terminal_declarations.rs:51-54` (`#[trace("TC-519")]`) and `temporal_fairness.rs:115-121` (`#[trace("TC-518")]`). Fix: drop the AC id from these three `trace` attributes and keep `TC-540`/`TC-542`, or split each AC. | qsl-semantics/tests/it/refinement_declarations.rs:492-496, 691-694, 930-934 |
| FND-002 | low | Some test oracles are weaker than their ACs. For `value` mapped twice (FR-135-AC-2) and `peek` written twice (FR-136-AC-2), the AC says the refusal names both rows, but the tests assert only the second row's text and never `Conflicting.first`. The population case does assert it. FR-135-AC-2's "each at its span" is not asserted for the two `MissingRow` refusals (removed population and object rows). FR-136-AC-1's "receiver self" is never asserted, and for `put` the test checks only that one argument exists (`is_some`), not that it is `x`. | qsl-semantics/tests/it/refinement_declarations.rs:554-564, 600-619, 718-768, 820-830 |

## Dispositions

Round 1, reviewed at `84309fc22546a5adec7ac758a9dfb2ed5513d3ec`.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 84309fc22546a5adec7ac758a9dfb2ed5513d3ec |
| FND-002 | fixed | 84309fc22546a5adec7ac758a9dfb2ed5513d3ec |

FND-001: the three tests now carry `#[trace("TC-540")]` / `#[trace("TC-542")]` only. No test in the tree tags FR-135-AC-1, FR-135-AC-4 or FR-137-AC-1, so the matrix reports them untagged until S4 lands.

FND-002: the value-twice and peek-twice tests assert `Conflicting.first` slicing to the first row. The three `MissingRow` refusals assert their span (`CasRefinesCounter`). FR-136-AC-1 asserts `self` as the receiver's only parameter, with a reference type, and asserts `put`'s argument as an `Integer` body over `[self, x]`.

Round 2, reviewed at `bf517a3ed4ea53bbba03ffb6573e046cade6b192`: no findings were open and none are new. The new tests carry `TC-541` only and claim no AC.
