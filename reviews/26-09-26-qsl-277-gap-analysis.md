---
id: SR-737
title: "QSL-277 gap analysis of PR 491 against FR-104 and TC-459 to TC-461"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@e278a3bdbde466895dc438b93138b2897028ee5d; FR-104-AC-1 to FR-104-AC-8; TC-459; TC-460; TC-461; qsl-semantics/tests/it/state_clauses.rs; qsl-semantics/src/check/assemble/tests.rs; spec/functional/FR-104-check-state-clauses.md; spec/tests.md; spec/spec.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-104
    type: reviews
---
## Summary

Ticket: QSL-277. PR: quire-spec-language#491 at e278a3bd. Each FR-104 AC is
mapped to the tests that back it. The spec is the authority.

| AC | Test(s) | Coverage |
| --- | --- | --- |
| AC-1 | `the_configversion_state_clauses_check` | Partial. Kinds, the `self` type, `ParentOrder` all `current` and `VersionUnchanged` {pre, post} are asserted. No field type is asserted (FND-001). |
| AC-2 | `a_postcondition_reads_result_typed_as_the_operations_result`, `result_outside_a_postcondition_refuses_wrong_anchor` | Codes only. The locus and the clause/operation payload are not asserted (FND-004). |
| AC-3 | `missing_and_ambiguous_names_refuse_at_their_locus`, `ill_typed_and_operator_ineligible_clauses_refuse` (rows 2, 3), `a_state_clause_context_resolves_or_refuses_at_its_name` | Codes only, except the assembler unit test, which also asserts the row-1 span (FND-004). |
| AC-4 | `ill_typed_and_operator_ineligible_clauses_refuse` | Codes only (FND-004). |
| AC-5 | `one_requirement_record_per_clause_and_frame`, `two_no_maximum_populations_of_one_type_refuse_ambiguous_name` | Counts and capability kind only (FND-002). Step 5a is not representable (FND-003). |
| AC-6 | `clause_identity_and_requirement_keys_are_stable`, `two_clauses_of_equal_kind_anchor_and_body_share_identity` | Step 3 is covered. For step 4, the claim ordinals are not asserted (FND-005). |
| AC-7 | `postconditions_of_an_operation_that_modifies_parent` | Covered. |
| AC-8 | `clauses_of_an_operation_with_a_reference_typed_parameter` | Covered. |

The coder's four spec questions:

1. The alias, context and operation are resolved in the S2 assembler (FR-091
   precedent). This is fine as behaviour. FR-104's Behavior says "exactly as
   it does for a function (FR-091)", and the codes and loci match. The spec
   text still needs changing in this PR, though. The Description says the
   family `check` resolves these names. FR-091's catalog table does not list
   the three new assembler causes. The ambiguous-population refusal has
   moved out of "Requirements". This is part of FND-006.
2. FR-104 assumes a population maximum that `PopulationRecord` does not
   have. This is a spec defect that this PR must fix, as FND-003. FR-104
   (this lane) owns the fix.
3. `Int[0, 1000]` vs `Integer`. The deferral to QSL-289 (Backlog, "FR-056:
   read ... bound scalars") is valid. The claim that "the test asserts
   `Integer`" is wrong: no field type is asserted at all (FND-001).
4. The separate `UnanchoredResult` variant is the right design. The existing
   doc still has to be widened, as SR-736 FND-005.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-104-AC-1's typing claims are untested. Nothing asserts the type of `self.versionNumber`: neither `Int[0, 1000]` nor the `Integer` stand-in that the module note claims. Nothing asserts `self.parent: Option<Reference<Config::ConfigVersion>>`, that `self.versionNumber` is an `Attribute` node, that `pre(self.versionNumber)` is a `Pre` over it (TC-459 step 1), or `NoCycle`'s reads. Fix: walk the checked bodies and assert each node's kind and type, with `Integer` standing in for `Int[0, 1000]`. Mark the `Int[0, 1000]` half of the FR-104-AC-1 row "Unverified (QSL-289)", as FR-103-AC-1 already does. | qsl-semantics/tests/it/state_clauses.rs:1-10, 108-156; spec/functional/FR-104-check-state-clauses.md:208 |
| FND-002 | medium | FR-104-AC-5 is checked only by record count and capability kind. Nothing asserts extent `Unbounded`, exactly one domain, domain kind `Population` boundable by `Cardinality`, `DomainKey{ConfigVersion object type node, [config_history ordinal]}`, clause records keyed by `claim` occurrences, or the frame record keyed by the frame node's occurrence. Step 5 (no maximum) does not assert that the refusal names `archive` and `config_history`, or that it is at the `on`. | qsl-semantics/tests/it/state_clauses.rs:445-470, 547-563 |
| FND-003 | medium | FR-104-AC-5 and TC-461 step 5a need a population that "declares a maximum". The domain-package `PopulationRecord` is `{key, member_types, extent}` and has no maximum. This matches QSpec FR-153's record. The maximum lives on the binding and on `Population<T>[N]` (FR-084, FR-097-AC-8). The branch cannot be represented, so half of step 5 is untestable. Fix in this PR: amend FR-104 ("Requirements" :175-199, AC-5 :212) and TC-461 step 5 so that every domain-package population is unbounded at S3, and drop the bounded-`archive` branch. If a declared maximum is wanted, that is a QSpec FR-153 change needing its own ticket; it does not belong here. | spec/functional/FR-104-check-state-clauses.md:175-199, 212; spec/test-cases/TC-461-s3-records-state-clause-requirements.md:376-378; qsl-semantics/src/model/domain_package.rs:465-479 |
| FND-004 | low | The test helper `check` throws away every refusal's location and payload, so no test asserts a locus that FR-104 names: at `op`, at both declarations, at the `pre`, at `value(self.parent)`, at `result`, or at the `reaches`. Only the assembler unit test checks the row-1 span. Fix: return (code, cause, region) and assert the region for each TC-459 step 3 and TC-460 row. | qsl-semantics/tests/it/state_clauses.rs:76-95 |
| FND-005 | low | TC-461 step 4 expects the shared node to carry two `claim` occurrences, ordinals 0 (`ParentOrder`) and 1 (`ParentOrder2`). The test asserts only that the identities are equal and that there are five records. | qsl-semantics/tests/it/state_clauses.rs:511-528 |
| FND-006 | low | The spec text is stale after this PR. FR-104's Description (:29-49) describes the code as it was before the PR, and says the family `check` resolves names that the assembler now resolves (question 1). The Dependencies line "waits for the other lane" (:224-225) is out of date. FR-091's catalog table lacks `UnresolvedOperation`, `AmbiguousOperation` and `AmbiguousPopulation`. TC-459 to TC-461 still say "Planned (QSL-273)", as do spec/tests.md:241-243 and spec/spec.md:518. The tests for TC-460 rows 2 and 3 (AC-3) are tagged `FR-104-AC-4` only. | spec/functional/FR-104-check-state-clauses.md:29-49, 224-225; spec/tests.md:241-243; spec/spec.md:518; qsl-semantics/tests/it/state_clauses.rs:364-366 |

## Verdict

Request changes. AC-7 and AC-8 are fully backed. AC-1 and AC-5 are
materially under-tested, and the AC-5 spec needs amending (FND-003). All of
these can be fixed inside this PR. Code defects are in SR-736.

## Dispositions

Round 2, checked against ab6a987c on 2026-09-26.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed ab6a987c | The test now asserts `Attribute` nodes of `Integer` and `Option<Reference<ConfigVersion>>`, a `Pre` over an `Attribute`, `NoCycle`'s `Reaches` node, and all-`current` reads (state_clauses.rs:163-219). Still open, low: the FR-104-AC-1 row (FR-104:221) has no "Unverified: `Int[0, 1000]` pending QSL-289" note like FR-103-AC-1's, and spec/tests.md:241 marks TC-459 "Passed locally" without qualification. |
| FND-002 | still-open | `one_requirement_record_per_clause_and_frame` is unchanged (state_clauses.rs:598-621). It checks the count and capability kind only. The new tests check only `to_wire()` "unbounded"/"bounded". Nothing asserts exactly one domain, kind `Population`, `DomainKey{ConfigVersion node, [config_history ordinal]}`, or that the clause and frame records are keyed by the `claim` and frame occurrences. Step 5 does not assert the names `archive`/`config_history` or the `on` span. |
| FND-003 | fixed ab6a987c | FR-104 "Requirements", AC-5 and TC-461 step 5 now say every domain-package population is unbounded. The bounded-`archive` branch is removed. |
| FND-004 | still-open (low) | This is partly addressed. Rows 3, 5, 6, 7 and 8 assert that the path is empty or non-empty, and rows 4 and 9 assert two distinct loci. That does not meet the finding. A refusal moved to the wrong subexpression, for example `deref(...)` instead of `value(self.parent)` in row 8, or the `=` operand instead of the `pre` in row 5, still passes a non-empty-path check. Asserting the exact `location.path` (a `Vec` of child indices) is one line per row and needs no mapping from nodes to source, so "disproportionate" does not apply. The `result` locus (AC-2) and the assembler spans for rows 2 and 4-at-`on` are also still unasserted in the integration tests. Non-blocking. |
| FND-005 | fixed ab6a987c | Claim role and ordinals 0 and 1 are asserted (state_clauses.rs:680-685). |
| FND-006 | fixed ab6a987c | The FR-104 Description and Dependencies are updated, the FR-091 catalog has three new rows, TC-459 to TC-461, spec/tests.md and spec/spec.md statuses are updated, and the trace tags now name FR-104-AC-3 and AC-4. |

### Round 3 dispositions

Checked against e2e5ffdc on 2026-09-26.

Two of the coder's claims were checked against the tree with `git show
ab6a987c:<path>`:

- "FND-004 already done in round 1": wrong. At ab6a987c,
  `state_clauses.rs` had 5 `location.path.is_empty()` checks and no
  exact-path assertion. The exact paths first appear in 6b144f75.
- "FR-104-AC-1 and tests.md:241 already carry the QSL-289 note": wrong. At
  ab6a987c, FR-104 and the TC-459 row of tests.md had no "QSL-289". The note
  first appears in 6b144f75.

The round-2 dispositions stand as written.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed 6b144f75 | The FR-104-AC-1 row now has an "Unverified ... QSL-289" note (FR-104:229). The TC-459 row at tests.md:241 says "AC-1's `Int[0, 1000]` half unverified, QSL-289". |
| FND-002 | fixed 6b144f75 | Step 1 asserts, for each record, `Unbounded` with exactly one domain of kind `Population` and finite kind `Cardinality`, one `DomainKey` equal across all four records, and occurrence roles of 3 `claim` and 1 `generated` (state_clauses.rs:626-686). Step 5 asserts `AmbiguousPopulation{context: "Config::ConfigVersion", populations: [archive, config_history]}` in order, with a span that slices to `Config::ConfigVersion` (state_clauses.rs:769-803). |
| FND-004 | fixed 6b144f75 | Exact `location.path` is asserted for `result` (root, :309-314) and for rows 3 `[]`, 5 `[0]`, 6 `[]`, 7 `[]` and 8 `[0, 0, 0]` (:521-613). This meets the finding: a refusal at a wrong subexpression now fails. A trivial leftover remains, which is low and not reopened: the integration test does not assert row 2's `op` span, row 10's path, or the exact loci of rows 4 and 9, which are only checked as distinct. |

### Round 4 dispositions

Checked against 18e70b23 on 2026-09-26.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-004 | fixed 18e70b23 | The round-3 leftovers are closed. Rows 4 and 9 assert exact `Location`s: `StateClause{ParentOrder, 0/1}`, and `Body{ParentOrder, 0}` plus `StateClause{ParentOrder, 0}`, each with an empty path (state_clauses.rs:468-525). Row 2 asserts `UnresolvedOperation{Config::ConfigVersion, missing}` with a span that slices to `missing` (:539-560). Row 10 asserts `Body{r, 0}` with an empty path (:664-684). Every TC-460 row now has an exact locus. |
