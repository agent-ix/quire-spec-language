---
id: SR-20261105101074
title: "Code and Rust review of QSL-604 slice 1 extension definition validation"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; QSL-604 slice 1; PR #2463 head 01ec946f0; qsl-semantics/src/library/extension.rs, qsl-semantics/src/library/extension_tests.rs, qsl-semantics/src/library/mod.rs; FR-354 Definition validation and Refusals, FR-111-CON-1, QSpec FR-467 Members and Validation rules 1-3"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-354
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-111
    type: reviews
---

## Summary

Ticket: QSL-604. Code review with the rust-review lane folded in, scoped to the PR #2463 diff (three files, +1012). QSpec FR-467 was read from a temporary worktree of quire-specification origin/main (b09800f7).

## Verdict

**CHANGES REQUESTED (not mergeable as is).** Two medium findings, both small fixes. FND-001 breaks the "first violated rule" ordering the spec requires: a duplicate node entry's own members are never checked by rule 1, so a definition that violates rule 1 is refused under rule 2 (reproduced with a throwaway probe test). FND-002 is the `MissingDefinition` arm, which refuses with `missing_import`/`missing-selection`. That pair is not in FR-354's table, and it differs from FR-111's codes for a root that is not in the catalog and for a conflicting authority (reproduced).

Sound: the three catalog-stage refusals match the FR-354 table (code `invalid_package`; causes `missing-member`, `conflicting-definition`, `definition-cycle`; the names of the definition and member, of the definitions or core grammar and the keyword or node kind, and of the cycle in path order, first and last equal, as in FR-111). Rules run in order over all selected definitions. Only definitions that pass are held, because the result is all-or-nothing. Unselected definitions are never parsed or validated: the cycle walk only follows their edges. Within-definition collisions name production indices, and entry-keyword collisions name entry indices (FR-467-AC-8, AC-10). The module names no check, package, model, emitter or backend type (FR-111-CON-1). CoreVocabulary is caller-supplied, which is acceptable for this slice (QSL-1786). There are no placeholder or `todo!` arms. Clippy `-D warnings` is clean on `qsl-semantics --all-targets`, and the 8 PR tests pass.

## Scope

- FR-354 Definition validation (examined): validate each selected extension definition by FR-467 before S1 reads it, report the first violated rule and no later one, and hold only the definitions that pass.
- FR-354 Refusals, rows 1-3 (examined): the catalog-stage `invalid_package` rows with the causes `missing-member`, `conflicting-definition` and `definition-cycle`.
- FR-111-CON-1 (examined): library stays free of check, package, model, emitter and backend types.
- QSpec FR-467 Members (examined): the `grammar_schema` and `typed_node_schema` JSON shapes.
- QSpec FR-467 Validation rules 1-3 (examined): first violated rule, and what each refusal names.
- FR-111 Closure and Refusals (context_only): root and edge resolution codes, conflicting authority, cycle path order.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Rule 1 validates only the first node entry for each keyword (`entries.iter().find`), and the final loop over entries checks only that each keyword names a production. A second node entry with the same keyword is never checked for `node_kind`, clause entries, `type` or unlisted clauses. So a definition whose duplicate entry lacks `node_kind` (a rule-1 violation) is refused `conflicting-definition`/EntryKeyword (rule 2), against FR-467 "report the first violated rule". Probe reproduced: `ConflictingDefinition { kind: EntryKeyword, name: "alpha", with: SameDefinition{0,1} }`. Fix: validate every entry's members in rule 1, not only the matched one. | qsl-semantics/src/library/extension.rs:534-544; qsl-semantics/src/library/extension.rs:583-590 |
| FND-002 | medium | `ExtensionRefusal::MissingDefinition` refuses a selected ref the catalog lacks with `missing_import`/`missing-selection`. FR-354's table has no such row. FR-111 refuses a selection root not in the catalog as `unknown_profile`/`unsupported-selection`, and two selections of one identity under different authorities as `ambiguous_declaration`/`conflicting-authority`. Here both give `missing_import` (probe: selecting `agent-ix/ext.a` and `other-auth/ext.a` gives `MissingDefinition(other-auth/ext.a)`). Use FR-111's codes, or take selections already resolved by FR-110/FR-111 so that this arm cannot occur. The test asserts the unspecified code. | qsl-semantics/src/library/extension.rs:215-237; qsl-semantics/src/library/extension.rs:303-311; qsl-semantics/src/library/extension_tests.rs:415-416 |
| FND-003 | low | `check_acyclic` silently skips a dependency the catalog does not resolve (`let Some(..) else continue`), so `hold_extensions` returns Ok and holds an extension whose dependency is missing (probe: dependency `ext.zzz` absent gives ok). The skip is neither documented nor tested. Leaving this to `link_bundle` may be intended, but the doc comment should say so and a test should pin it. | qsl-semantics/src/library/extension.rs:361-363 |
| FND-004 | low | `entry_keywords` parses the exact bytes a second time, with a silent `Vec::new()` fallback. That makes rule 2 correct only because of an ordering stated in a comment ("called only after read_productions"). Return the entry keywords from `read_productions`, so each definition is parsed once and the fallback arm goes away. | qsl-semantics/src/library/extension.rs:470-484 |
| FND-005 | low | `CoreVocabulary` derives `Default`. An empty vocabulary silently turns off the core-keyword and core-node-kind half of rule 2, so a caller that writes `CoreVocabulary::default()` gets no core-collision refusal. Drop the derive and keep `new` as the only constructor until QSL-1786 supplies the edition vocabulary. | qsl-semantics/src/library/extension.rs:25-29 |
| FND-006 | low | Payload ambiguity. A node entry with no `keyword` is reported as `ProductionForEntry { keyword: "" }`. `MissingMember::ClauseName { index }` carries a production-clause index at one site and an entry-clause index at the other, so a reader cannot tell which list the index refers to. | qsl-semantics/src/library/extension.rs:564-568; qsl-semantics/src/library/extension.rs:583-588 |

## Evidence and limits

Built only through `locked-build.sh` with the shared target qsl-shared-target, jobs 4 and debug 0. `cargo test -p qsl-semantics --lib library::extension_tests`: 8 passed. `cargo clippy -p qsl-semantics --all-targets -- -D warnings`: clean. FND-001, FND-002 and FND-003 were confirmed by a local, uncommitted probe test, which was then removed with `git checkout`. No full gate was run.
