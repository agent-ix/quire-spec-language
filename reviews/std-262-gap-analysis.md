---
id: SR-15493
title: "gap analysis of STD-262 FR-290-AC-5 to AC-14 and TC-774 (QSL PR #1217)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@8e62c04067d50955f5388df4071fb2077f0a16b2; spec/functional/FR-290-settle-plugin-results-as-typed-terminal-records.md, spec/test-cases/TC-774-plugin-results-settle-as-typed-records-and-plugin-proofs-carry.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-290
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-774
    type: reviews
---
# SR-15493: gap analysis of STD-262 FR-290-AC-5 to AC-14 and TC-774 (QSL PR #1217)

## Summary

Ticket: STD-262. This is a spec-only PR. Plan completion: not assessed. AC-to-TC trace: TC-774's scope
is now AC-1 to AC-14. Its steps 5 and 6 cover AC-5 to AC-10, step 7 covers AC-11, steps 8 to 10 cover
AC-12 to AC-14, and each step has a matching expected result. The code is not implemented yet: there are
no readers, no `PluginReplayContext` and no tagged tests. The PR body names QSL-977 as the code half.
Gate: `QSPEC_DIR=/Users/peter/dev/quire-specification make check-spec-validation` exits 0
(quire 0.36.2, quoin 0.28.3), with no warning on the changed files. AC-12's semantics are recorded in
SR-15490 FND-001.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | minor: some stated behaviour has no vector, and some vectors leave out what the AC promises. No AC covers the rule that the certification check runs before `read_plugin_basis` (for example, an entry with a bad `certification` and a core-only basis), `request` refusal 3, or step 3 across members (SR-15492 FND-001). AC-6 promises "each with its refusal and pointer", but the `copy-swap` class `["a","a"]` vector names no pointer. AC-9's two `Malformed` vectors say "at the member" where the pointers are `/cause/boundary_states` and `/cause/limit`. Add the vectors and the exact pointers. | spec/functional/FR-290-settle-plugin-results-as-typed-terminal-records.md:356,359 |

## Verdict

Minor only. All ten new ACs are traced by TC-774. Coverage gaps are FND-001.

## Dispositions

Round 1, reviewed QSL #1217 at 6ce0503e2.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | - |
