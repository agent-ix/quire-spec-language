---
id: SR-772
title: "QSL-312 code review of PR 512 (FR-105 AC-2/AC-4 test remainder)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@5247a61078b8ee67283cbaf82af46c561dad71ca; qsl-replay/src/spine/clause/tests.rs; qsl-package/src/emit.rs (read, unchanged); qsl-semantics/src/check/lowering.rs (read, unchanged)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-105
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-462
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-463
    type: reviews
---
## Summary

Ticket: QSL-312. PR: quire-spec-language#512 at 5247a610. This review covers
code and Rust (the rust-review lane is folded in here). The diff is test-only.
It touches `qsl-replay/src/spine/clause/tests.rs` and four spec files, and
changes no production code.

What was checked:

- **Exact dependency sets.** `VersionUnchanged` {self, result, anchor,
  condition}, `ParentOrder`/`NoCycle` {self, ConfigVersion, condition} and the
  anchor {ConfigVersion, frame} are compared with `assert_eq!` over sorted
  `Vec`s. That is exact multiset equality, so a missing, extra or duplicate
  entry fails. The expected sets agree with FR-105's Outputs table
  (dependencies = every body reference) and with `BodyNames::of`
  (`qsl-package/src/emit.rs:279-334`). ConfigVersion and the anchor are found
  independently of the body under test, through `resolve_declaration` and a
  single-node scan.
- **Following references.** Only `qsl-semantics/src/check/lowering.rs:2640`
  creates `NodeTag::Expression` nodes. `resolve_expression` follows a
  `Reference` into one of them. `find_application` calls it on every term it
  visits, so the pre, reaches and field-read searches all follow references,
  and so does the direct `pre` operand check. No helper assumes a flat body.
  `state_clause_condition` and `clause_condition_node` return the raw
  reference on purpose, and every caller resolves it.
- **AC-4 tests can fail.** Rename compares the full node-key sets, and the
  renamed clause's identity with ParentOrder's. The `<=` test uses `assert_ne`
  on the clause id and the package_id, and `assert_eq` on the other two ids.
  `edit_once` asserts that exactly one match is edited. The ParentOrder2 test
  checks that the node-key sets are equal and the claim ordinals are [0] then
  [0, 1]. The second-post test checks 3 then 4 clause nodes, that the anchor
  and frame keys are equal (`single_state_node_key` panics on a second one) and
  that the anchor ordinals are [0, 1]. The Sub test relies on one anchor, which
  panics if Sub gets its own anchor. It also checks that the context and
  `semantic_type` are ConfigVersion and that the ordinals are [0, 1, 2]. Each
  test fails if the behaviour it names is broken.
- **Fixture.** `config_version_domain_document_with_sub` parses the base
  document and pushes one type entry. `config_version_unit_and_packages_for`
  is a plain extraction, and the original function delegates to it. Nothing is
  duplicated.
- **Dropped `"kind": "dependency_reference"` assertion.** The wire spells that
  term `"term": "dependency_reference"`
  (`qsl-replay/src/spine/dependency_tests.rs:798`), never `"kind"`. It is
  never listed in `dependencies` (`emit.rs:312-314`), and this unit imports
  nothing. Dropping the assertion was correct.
- **Rust idioms.** The fully qualified `qsl_semantics::check::` paths match
  the file's existing convention (23 uses on main). Test-only
  `panic!`/`unreachable!` are fine. `&dyn Fn` is fine for a recursive
  predicate. The file has no `unsafe`, no integer casts and no locks.
  `#[trace]` tags are on all 11 new tests.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The dependencies of the `versionNumber` field-read node are checked with `.contains(ConfigVersion)`, a subset check. The clause and anchor sets use exact equality. A surplus dependency on the field read would pass. AC-2 says "each emitted node's dependencies", so compare the exact set: ConfigVersion plus the field read's argument reference (the `deref(self)` expression node). | qsl-replay/src/spine/clause/tests.rs:3754-3758 |
| FND-002 | low | `occurrence_ordinals` filters by role, and the occurrences test asserts only the expected role. A `state_clause`, anchor or frame that picked up an extra occurrence in another role (for example a stray `generated` on a clause node) would pass. Assert each node's full (role, ordinal) list instead. | qsl-replay/src/spine/clause/tests.rs:3562-3574, 3770-3795 |
| FND-003 | low | The new section banner names "QSL-308c/QSL-312". QSL-308 is canceled, and the spec files in this PR no longer cite it. Say "QSL-312" alone. | qsl-replay/src/spine/clause/tests.rs:3348 |

## Verdict

PASS with three low findings. No production code changed. The three exact
dependency sets asked for are exact. Following references works everywhere it
is needed. Each AC-4 test fails if its behaviour regresses.
