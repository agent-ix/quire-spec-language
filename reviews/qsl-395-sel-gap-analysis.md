---
id: SR-1270
title: "QSL-395 gap analysis of PR #621 (FR-093-AC-16, FR-027-AC-9, FR-094, FR-099-AC-1, TC-442, TC-446, ADR-011)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@56049f4cfcfc43c1d5b91c9aaab291a7b91497cb; PR #621 diff 8244dd2b4...56049f4c; spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md; spec/functional/FR-027-export-compiled-native-package.md; spec/functional/FR-094-key-model-owned-reference-population-and-quantity-nodes.md; spec/functional/FR-099-compile-against-supplied-libraries.md; spec/test-cases/TC-442-spine-compile-admits-a-domain-package-and-locks-its-selection.md; spec/test-cases/TC-446-spine-compile-resolves-imports-against-supplied-libraries.md; spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md; context: spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-027
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-094
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-099
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: reviews
---
# QSL-395 gap analysis of PR #621

## Summary

Ticket: QSL-395 (IR-535). PR quire-spec-language#621 at 56049f4cf. Contract:
QSpec FR-322-AC-35 `DependencySelection {identity, package_id}`, and IR-534
`ModelRef {identity, digest_domain, digest}`, which refuses a version member.

Examined, with the test that backs each row:

| Row | Amended text | Backing test | Oracle |
| --- | --- | --- | --- |
| FR-093-AC-16 | `{identity, package_id}` entries, no `version` | `the_dependency_closure_is_written_in_the_lock_and_the_preimage` (emit/tests.rs:2639); the conformance clause by `conformance_dependency_selection_vectors` (checked_v2/tests.rs:1731), tagged FR-087-AC-3 (FND-003) | whole-array `assert_eq!` on lock and preimage; QSpec main 396493c's recorded `package_id` recomputed; 9 entry-mutation cases and 3 order vectors |
| FR-027-AC-9 | `model_selections` hold identity, `digest_domain`, digest, no version | `a_complete_v1_request_with_a_domain_package_locks_its_model_selection` (compile_command.rs:693), `a_model_bearing_unit_emits_its_model_selection_and_reads_back_verified` (emit/tests.rs:2268) | whole-array `assert_eq!` on lock and preimage |
| FR-099-AC-1 | one `DependencySelection` `{test/geometry, d}` | `an_import_binds_the_library_compiled_from_source` (dependency_tests.rs:127) | whole-array `assert_eq!`; `d` is the library's recompiled `package_id` |
| TC-442 step 1 | expected `model_selections` without `version` | same tests as FR-027-AC-9 | as above |
| TC-446 step 1 | `{test/geometry, d}` | same test as FR-099-AC-1 | as above |
| ADR-011 §2.4 table, prose, OQ-5, QSpec list | `{identity, package_id}` | n/a (decision) | matches emit.rs:837-846 |
| FR-094 (prose, :89-94) | digest, not version, is the lock evidence | n/a (prose) | matches emit.rs:974-982 |

The focused logs (`qsl-395-test.log`, `qsl-395-test2.log`,
`qsl-395-conformance.log`, `qsl-395-clippy.log`) all exit 0 against IR
d771c6a. IR main c5fa773 is the merge of that head. The commits differ, but
the struct shapes QSL builds against are the ones in the logs.

No expected `package_id` or digest is a hand-edited literal (see SR-1269).
A returning `version` fails every row's oracle on the emit side, and it
fails to compile against IR c5fa773's structs.

Out of scope by ruling, not counted as gaps: the source `import ... version
... digest` syntax (HL1, QSL-471); `SuppliedLibrary.version`, FR-027-AC-10's
empty-version refusal and the replay request's dependency versions
(ADR-013:934, ADR-015 D-4, FR-071; A2); native protocol `domain_package`
version members; and FR-321 intake `ModelSelection` (IT-012).

Not a QSL gap: QSpec's `dependency-selection-vectors.json` has no
"entry carries `version`" mutation, so QSL's I2 read refusing one is
enforced only by IR's `deny_unknown_fields` and tested in IR (IR-534).

## Verdict

CONDITIONAL. Every scoped AC and test-case row is backed by a passing tagged
test with a strong oracle, apart from one tag that predates this PR
(FND-003). Two prose statements outside the edited lines
still say the lock carries a version: one in FR-094, which this PR edited,
and one in ADR-013 O-04. Fix both in this PR.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | ADR-013 O-04's Equality row still says "the selected version is lock evidence in `model_selections`, so a version-only change of a domain package keeps its model-owned node ids". That contradicts amended FR-027-AC-9, ADR-011 §2.4 and the emitter, which write no version into `model_selections`. | spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md:197 |
| FND-002 | medium | FR-094's Model declaration nodes section still says "the lock selects the domain package's bytes by version and digest". This PR corrected the matching sentence at :91 of the same file but left this one. The lock now selects by identity and `sha256-jcs` digest. | spec/functional/FR-094-key-model-owned-reference-population-and-quantity-nodes.md:115-116 |
| FND-003 | low | `conformance_dependency_selection_vectors` backs FR-093-AC-16's conformance clause (QSpec's `dependency-selection-vectors.json`, `make conformance`), but it is tagged only `#[trace("TC-253", "FR-087-AC-3")]`. FR-087-AC-3 is about `library` verified binding, and FR-093-AC-16 and TC-416 do not reach that clause through it. This predates the PR, but the PR amends FR-093-AC-16. | qsl-package/src/checked_v2/tests.rs:1729 |

## Dispositions

Round 1, reviewed at 536d437865d523ababa33941d8363bfbcfead278 (fix commit
536d43786 over 56049f4cf).

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 536d43786 |
| FND-002 | fixed | 536d43786 |
| FND-003 | fixed | 536d43786 |
