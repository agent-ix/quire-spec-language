---
id: SR-924
title: "QSL-346 gap analysis of PR 546: TC-166, FR-062-AC-10, FR-065-AC-6 against the tests"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@6260464f3408e75aea2126a681f638b933690474; spec/test-cases/TC-166-replay-executor-typed-name-selection.md; spec/functional/FR-062-implement-checked-family-contract.md (AC-10); spec/functional/FR-065-migrate-function-application-to-checked-family.md (AC-6); qsl-replay/src/execute.rs; qsl-replay/src/execute/tests.rs; qsl-replay/src/call_site.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/TC-166
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-062
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-065
    type: reviews
---
## Summary

Ticket: QSL-346. PR: quire-spec-language#546 at 6260464f.

FR-062-AC-10 has two clauses. The first: a call to the `replay` facade's entry point with a bare `&str` fails
to compile. The `compile_fail` doctest on `replay` backs it (E0308 on
`selected_function: "small"`). The second: an unresolvable `QualifiedName` returns a typed
refusal and does not compare against a display name. `tc_166_an_unresolvable_qualified_name_refuses_unknown_function` backs it, tagged
`TC-166`, `FR-062-AC-10` and `FR-065-AC-6`. The "when it calls a family's widened
`evaluate` hook" clause also holds: `replay` -> `select` -> `CheckedPackage::call` ->
`ValueFunctionFamily::evaluate`.

FR-065-AC-6 says the same about the executor entry: resolution against the recompiled
package's declarations (`select(&compiled, ..)`), `&str` refused at compile time, and
a typed refusal, not display-name equality. The same doctest and test back it.

The call_site test is tagged `TC-166` only. That is correct: both ACs name the
`replay` facade's entry point, and `call_site` is a separate entry. Doctests cannot
carry `#[trace]`. The `replay` doc names both ACs in prose, which is the repo's existing
precedent for `compile_fail` backing.

Bindings checked: execute test -> TC-166, FR-062-AC-10, FR-065-AC-6 (correct);
call_site test -> TC-166 (correct).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | TC-166's new Status says steps 2 and 3 are backed, but step 2 is never run as written. Step 2 is: "Build a checked package with two functions whose display names differ only by case ... construct a replay request naming one of them by its exact `QualifiedName`". Its expected result: the executor "calls exactly the named function, distinguishing it from the cosmetically similar one". The fixture `proved()` declares `small` and no `Small`. The test only checks that `small` replays and that `Small` is refused when it is not declared. The refusal oracle does catch case-folding (mutation-checked), but no test resolves one of two case-variant declarations. Fix: add a fixture that declares both `small` and `Small` with different bodies, and assert each selection gets its own body's verdict. Or reword step 2 and the Status to what the test does. | spec/test-cases/TC-166-replay-executor-typed-name-selection.md:25-29,38-40,54-60; qsl-replay/src/execute/tests.rs:536-560 |

## Verdict

Both ACs are backed on every clause, and the traces match each AC's text. One medium
finding: TC-166's Status claims step 2, which the test does not run. Fix it in this PR
with either a two-declaration fixture or a reworded step.
