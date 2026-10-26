---
id: SR-60019
title: "Rust and code review of PR #1592: release declarations at S1-S3 (QSL-1004)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@8b97d4816473c92a229589e91342c12b547c15eb; PR #1592 (branch qsl-1004-release-decl); qsl-cst/src/{cst,grammar}.rs; qsl-forms/src/{dispatch,syntax,task_set,value,lib}.rs; qsl-semantics/src/check/task_set/release.rs; qsl-semantics/src/check/{refusal,task_set,mod,assemble}.rs; qsl-replay/src/spine.rs; src/complete/editor.rs; tests qsl-cst taskset/complete_grammar, qsl-forms task_set_forms, qsl-semantics task_set_releases"
review_set: subset
date: '2026-10-25'
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-249
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-704
    type: reviews
---
## Summary

Ticket: QSL-1004. Rust and code review of the S1-S3 half of the `release-decl`
change. Verdict PASS with findings; no blocker.

Grammar fidelity: `release-decl = 'release' ident '.' ident 'on' qualified-name ';'`
(QSpec shared-grammar.md:971) is `release`, ident, `.`, ident, `on`, qualified-name, `;`
in `qsl-cst/src/grammar.rs`, and `ReleaseDeclaration` is in the declaration
alternatives beside `ResponseDeclaration`. Probed on the head with a throwaway
qsl-forms test (reverted): `release Ctl . a on Config :: ConfigVersion :: sample ;`,
the same split over newlines, a trailing `// c`, and a one-segment operation all
build one `DeclarationForm::Release`. A three-segment qualified name builds in the
PR's own test. The word `release` was already reserved for the infix temporal
operator; the declaration head and the operator never compete for a position
(operator follows an operand). `a_release_declaration_parses_beside_the_temporal_release_operator`
puts both in one unit and asserts one TemporalClause and two ReleaseDeclarations, and the
malformed-form test rejects `release holds(true);`.

Production tables: every table that lists `ResponseDeclaration` also lists
`ReleaseDeclaration`: cst.rs, grammar.rs, complete_grammar.rs (accepted list and
corpus), editor.rs `is_symbol`, forms value.rs (both match arms), dispatch.rs
(`LeadingTokenKind::Release`, `from_spelling`), `DeclarationForm::Release`, the
three exhaustive matches in forms tests, TaskSetDeclaration and TimedConstruct, and
the spine message match. No table was missed (grep of `ResponseDeclaration` and
`schedulable` across the tree).

Semantics: `check_releases` reports every unresolved name (task set, task, operation
in one pass; the operation is still resolved when the set or task is not), skips the
duplicate check for an unresolved declaration, and keys the duplicate on
(operation key, task set, task), so two tasks per operation and the same task on two
operations are allowed. Operation resolution is the shared
`ProbabilisticInput::operation`, the one `delay`, `reward` and `random` use, so a
three-segment `alias::Type::op` and a two-segment `Type::op` resolve the same way.
The refusal spans are the name (task set, task, operation) and the whole second
declaration for the duplicate. No `unwrap`, `unsafe` or lossy cast in non-test code;
accessors only; `Release` has no public constructor.

## Checks run

Head 8b97d4816, QSPEC_DIR set to the PR B branch (2e81f694). Each exit code 0:
`cargo test -p qsl-cst`, `-p qsl-forms`,
`-p qsl-semantics --features qsl-semantics/test-support` (1258 pass), `-p qsl-replay`,
`cargo clippy --workspace --all-targets -- -D warnings`, `cargo test -p xtask`,
`-p qsl-string-edge`, `cargo run -p xtask -- conformance` (7 accepted and 12 refused
selection vectors; same output against QSpec origin/main). The aggregate `make ci` is
recorded in the Linear comment.

## Findings

| ID | Severity | Summary | Refs |
|----|----------|---------|------|
| FND-001 | low | `an_operation_that_releases_nothing_records_no_release` closes with the comment "A unit that declares no task set and no release records none", but the fixture always carries the `Ctl` taskset, so it asserts a unit with a task set and no release. The test is sound; the comment states a case it does not run. Fix the comment, or add a fixture with no `taskset` (the facet gate then still admits the unit). | task_set_releases.rs:108-110 |
| FND-002 | low | `a_release_is_refused_unless_the_timed_facet_is_selected` is tagged `FR-249-AC-3`, but AC-3's text says nothing about the facet gate. It is a real and useful test of FR-230's gate; tag it `TC-704` only, or add the clause to AC-3. | task_set_releases.rs:188-189 |
| FND-003 | medium | `AmbiguousReleaseOperation` (`ambiguous_declaration`/`ambiguous-name`) has no test at any stratum and is not in AC-3, although FR-249 Behavior and the release.rs header both promise it. The branch is dead to the suite: deleting the `Unresolved::Ambiguous` arm, or mapping it to missing-name, leaves every test green. See gap-analysis SR-60020 FND-002 for the fixture and wording fix. | release.rs:98-110; refusal.rs |
| FND-004 | low | The new spine messages (`check_message` arms for the five release causes) have no test; the sibling `UnknownResponseTask` arms are equally untested. Not a regression; noted so QSL-585 does not assume coverage. | spine.rs:1236-1258 |
| FND-005 | low | The duplicate check is a linear scan of `model.releases` per declaration, quadratic in the number of releases of one unit. The input is bounded by the unit source limit, as for `DuplicateTask`; no action. | release.rs:116 |

## Verdict

PASS with findings. FND-003 should be fixed in this PR (a test plus the AC wording);
FND-001 and FND-002 are one-line edits.
