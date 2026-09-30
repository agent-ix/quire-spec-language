---
id: SR-765
title: "QSL-304 code review (with rust-review lane) of PR 506"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; qsl-semantics/tests/it/model_operations.rs; qsl-semantics/tests/it/state_clauses.rs; qsl-semantics/src/model/observation/document.rs (admit_scalar; unchanged); quire-exact/src/integer.rs (Integer::from_str; unchanged); qsl-semantics/tests/it/type_environment_model.rs (RedefinitionWidens tests; unchanged); qsl-replay/src/spine/clause/tests.rs (separate ConfigVersion builder; unchanged)"
review_set: subset
---
## Summary

Ticket: QSL-304. PR: quire-spec-language#506.
Methods: code-review with the rust-review lane folded in.

The PR flips the shared `ConfigVersion` fixture's `versionNumber` from
`ix://quire/native/Integer` to a package-declared `VersionNumber` value type
(`Int[0, 1000]`) through a new `version_number_value_type()` helper. It also
updates the tests whose outcome changed and adds TC-465 rows 20 and 30.

What I checked, and what I found:

1. **Retyping is complete.** All five builders that declare
   `ConfigVersion::versionNumber` now use `&version_number`:
   `config_version_document`, `config_version_document_with_operations`,
   `subtype_document`, `subtype_document_with_two_member_population`, and the
   inline document in `admission_is_deterministic_regardless_of_document_order`.
   The FR-103-AC-3 `delta` parameter is typed `VersionNumber` too. The
   remaining `native/Integer` typeRefs are all unrelated to `versionNumber`:
   `Note.text` (model_operations.rs:330), `AuditLog.note` (:1415), the
   `versionTotal` operation results (state_clauses.rs:899, :957), and
   `Tag::versionNumber` (:1877). `Tag` is an unrelated type in a test about a
   same-named field on a different type, so its field's type does not matter
   there. `qsl-replay` has its own separate builder that already uses
   `VersionNumber`, and the PR does not touch it. The builders are
   `pub(super)`, so nothing outside `qsl-semantics/tests/it` can call them.
2. **The Sub::version change is a real fix.**
   `a_write_to_a_field_that_redefines_a_modifies_grant_admits` tests
   authorization through a `redefines` chain; it is not about widening. Once
   the parent field is `Int[0, 1000]`, a native `Integer` redefinition widens
   the type and refuses at assembly (declaration.rs:1698), before the test
   reaches what it checks. Retyping `Sub::version` to the same `VersionNumber`
   makes it a valid, non-widening redefinition. The widening refusal itself is
   already covered by three tests in type_environment_model.rs:890-1005, so no
   coverage is lost.
3. **TC-465 rows 20/30 are not vacuous.** `admit_scalar`
   (document.rs:890-903) sends parse failures and out-of-interval values to
   the same `invalid-value`. So the key question is whether `"-1"` and
   `"1001"` pass the parser. `Integer::from_str` (integer.rs:315-330) strips
   a leading `-` and accepts canonical digits, so both parse. They therefore
   reach `interval.contains`, and under the native `Integer` arm they would
   be admitted. `tc465_document` is built on the flipped builder
   (`config_version_document_with_operations`), and the snapshot's
   `objects[0]`/`objects[1]` are `root`/`child` (state_clauses.rs:2199-2201).
   The tests also assert the `object` and `field` locus. I tried a mutation
   run (forcing `interval.contains` to accept), but the permission layer
   blocked the edit, so non-vacuity rests on this reading, not on a red run.
4. **The find-by-identity lookups are correct.** Both use
   `.find(|entry| entry["identity"] == config_version).expect(...)`.
   `Value == &str` compares string contents, and the identity string matches
   `wire_type`'s `"identity"`. A miss panics; it cannot match nothing
   silently. The chained `["operations"][0]["frame"]["modifies"] =` write
   could silently create keys through IndexMut, but `"frame"` is the real key
   (model_operations.rs:146/443), and row 40 asserts `frame_violation`, which
   would fail if the narrowing had not happened. The `tags` mutation is
   guarded the same way by row 17's `unsupported-feature` assertion. The old
   hardcoded `types[0]` now points at the `VersionNumber` type, so the lookup
   change was necessary. No other positional `types[...]` access remains in
   `tests/it`.

Rust-review lane: test-only diff. The `unwrap()`/`expect()` calls are in
tests, and there are no new panics or casts in production code. The findings
below are stale documentation and duplicated fixture code.

Gates, run fresh by me in the worktree (not taken from the PR's log):
`make ci` exited 0. The log has 93 `test result: ok` lines and 0
FAILED, and the `it` binary ran 229 tests.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The `state_clauses.rs` module header is stale and now says the opposite of the code. It still says "`versionNumber` is typed as native `Integer`, not the bound `Int[0, 1000]` scalar", that the shared fixture "still substitutes the native `Integer` field", and that "Every assertion below that would otherwise read `Int[0, 1000]` reads `Integer` instead". After this PR every such assertion reads `Int[0, 1000]`. | qsl-semantics/tests/it/state_clauses.rs:6-16 |
| FND-002 | medium | `model_operations.rs` docs contradict the tests. (a) The rewritten module doc says "AC-3's second parameter type, `delta`, stays the native `Integer`", but `a_version_number_typed_parameter_admits_and_is_typed_bound_integer` types `delta` as `VersionNumber` and asserts `Int[0, 1000]`. (b) The doc on `bound_integer_value_type_admits_and_assembles` still refers to "this module's own scope note" (removed) and says `Int[0, 1000]` is "not the native `Integer` this file's other tests substitute". | qsl-semantics/tests/it/model_operations.rs:41-43; qsl-semantics/tests/it/model_operations.rs:943-951 |
| FND-003 | low | The new comment on `Sub::version`'s typeRef says "a `redefines` that widens the type it redefines refuses `RedefinitionWidens`". Read in place, it suggests this fixture refuses, but it is meant to admit. It should say the field matches the redefined type so that the redefinition does not widen it. | qsl-semantics/tests/it/state_clauses.rs:2050-2053 |
| FND-004 | low | The fixture code is duplicated. (a) `IntegerInterval::new(Integer::from(0_i64), Integer::from(1000_i64)).unwrap()` is repeated at four sites in two files. (b) The `VersionNumber` identity is re-spelled as a `format!` in the AC-3 test and in `bound_integer_value_type_admits_and_assembles`, and as a string literal in state_clauses.rs. (c) `bound_integer_value_type_admits_and_assembles` keeps its own copy of the constraint closure that `version_number_value_type` now provides. A single `version_number_interval()` helper plus reuse of `version_number_value_type()` would give each value one source. | qsl-semantics/tests/it/model_operations.rs:182-203; qsl-semantics/tests/it/model_operations.rs:912; qsl-semantics/tests/it/model_operations.rs:955-988; qsl-semantics/tests/it/model_operations.rs:1052; qsl-semantics/tests/it/model_operations.rs:1317; qsl-semantics/tests/it/model_operations.rs:1355; qsl-semantics/tests/it/state_clauses.rs:193; qsl-semantics/tests/it/state_clauses.rs:2054 |

## Verdict

Approve with findings. There are no high findings. The fixture flip is
complete and correct. The Sub::version change is a real fix, not a silencer.
Rows 20/30 go through the `ValueType::Int` interval arm, and the identity
lookups cannot match nothing silently. FND-001 to FND-004 are documentation
and duplication fixes that belong in this PR.

## Dispositions

| FND | Outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | header now states `versionNumber` is the bound `VersionNumber` scalar and out-of-bound values refuse at admission (TC-465 rows 20/30) |
| FND-002 | fixed | module doc now says `delta` is typed by the same `VersionNumber` declaration; the dedicated test's doc no longer cites a removed scope note or a substitution |
| FND-003 | fixed | the comment says the redefinition matches the redefined type and admits; widening is covered in `type_environment_model.rs` |
| FND-004 | fixed | `version_number_identity()` and `version_number_bound()` replace the four interval constructions and three identity re-spellings; `bound_integer_value_type_admits_and_assembles` reuses `version_number_value_type()`, and the `appliesTo` explanation moved into that helper |
