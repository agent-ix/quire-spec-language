---
id: SR-1266
title: "A1b gap analysis of PR #616 (FR-001-AC-5, FR-010-AC-11, FR-093-AC-6/7/17, FR-095-AC-1/2/3, FR-110; TC-416, TC-420, TC-421, TC-424, TC-425)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@7afb98e17cc1862e1504079021b48163a0fcf2fb; PR #616 diff against origin/main (merge base fbf69cb9); spec/functional/FR-001-read-exact-source.md; spec/functional/FR-010-report-native-outcomes.md; spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md; spec/functional/FR-095-occurrence-keyed-source-map-and-locus.md; spec/functional/FR-110-resolve-header-profile-selections-at-e3.md; spec/test-cases/TC-416, TC-420, TC-421; spec/decisions/ADR-011, ADR-013; spec/tests.md (rows TC-416, TC-420, TC-421, TC-424, TC-425, TC-490); the tests the diff adds or changes"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-110
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-095
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-001
    type: reviews
---
# A1b gap analysis of PR #616

## Summary

Ticket: QSL-470 (slice A1b). PR: quire-spec-language#616. No plan bundle
names this slice, so this is an AC-to-test check using the quoin
gap-analysis method, with the semantic review (intent, test and code) the
brief asked for. I ran `quire coverage --scope . --json` at the head. It
reports no unbacked row and no status lie for any AC or TC the diff touches.
Its three status lies (TC-199, TC-201, TC-246) are outside the diff.

Trace, per unit:

- **FR-001-AC-5 (TC-424).** The AC now says the `RawSourceRef` has authority,
  identity and the `quire.source.bytes/v1` digest "and no other member", and
  that new bytes change only the digest.
  `admission_mints_the_caller_named_source_reference` checks the three
  members, and that `b'` keeps authority and identity and gets
  `ByteDigest::of(b"b'")`. The type has no fourth member. FR-001 Status
  correctly leaves the four `SourceIdentity` labels to QSL-381.
- **FR-010-AC-11 (TC-425).** `parse_and_format_take_the_four_source_labels`
  compares the exact `RawSourceRef` JSON with no `revision`. The two cli tests
  assert `source` has no `revision`. The two-label grammar stays QSL-381's
  remaining work, as FR-010 Status and the TC-425 row say.
- **FR-093-AC-6 and the law `definition` rule.** FR-093 now says a law's
  `definition` is the `DefinitionLock` row of its role, "exactly
  `{authority, identity}`, with no revision and no digest".
  `a_law_names_its_definition_by_authority_and_identity` checks exactly that,
  but no AC states it, and the test traces AC-7 (FND-001).
- **FR-093-AC-7 (TC-416).** Key recomputation is unchanged in kind. The E14
  to E17 keys in FR-093 changed because the law `DefinitionRef` is in the
  application-node preimage. `recursive_text_leaf_vectors_check_and_key`
  reads them from FR-093 and recomputes them, so the new keys are measured,
  not typed.
- **FR-093-AC-17 (TC-416).** `the_lock_selects_the_catalog_definitions`
  compares `diagnostics.catalog` with
  `{"authority": "agent-ix", "identity": "quire.native.diagnostics/v1"}` as
  exact JSON and reads the package back verified. TC-416 step 9 matches.
  `an_old_reference_shape_refuses_unknown_member` adds the negative side.
- **FR-095-AC-1, AC-2 (TC-420).** Both ACs drop `Revision`.
  `raw_source_refs_and_regions_refuse_malformed_members` and
  `region_equality_is_over_digest_start_and_end_only` match the new text.
- **FR-095-AC-3 (TC-421).** `a_verified_read_carries_the_wire_source_map`
  checks authority, identity and digest. The QSpec-fixture half,
  `conformance_c14_...`, fails at this head because IR #253 refuses QSpec's
  positive fixture. That is routed to the plan session and recorded nowhere
  here.
- **FR-110 (TC-490).** The Description, Inputs and Status now say
  `DefinitionLock` names each definition by `{authority, identity}`, holds no
  revision, digest or QSpec document, and is compared with QSpec's lock by
  `make conformance`. The code matches. The comparison
  (`conformance_catalog_matches_qspec_complete_value_lock`) has no QSL AC
  (FND-002).
- **ADR-011 and ADR-013 edits.** These are wording changes to
  `{authority, identity}` and to `RawSourceRef` without a revision. They agree
  with the code.

Underspecified code (code to spec):

- `CatalogRole::identity()` and `selection_rule()` are owned by ADR-011 §2.4
  and FR-110 (the catalog), and by QSpec FR-001's package-selection rules.
- `NATIVE_DIAGNOSTICS_IDENTITY` is owned by FR-093-AC-17.
- The `UnsupportedConstruct` arm in `map_refusal_code` has no owning QSL
  requirement and no test. It is recorded as SR-1265 FND-001, not repeated
  here.

Semantic review (intent, test and code):

- Every changed test would fail if the behaviour it names broke; SR-1265
  lists each one.
- One weak spot: `the_catalog_covers_every_role_exactly_once` asserts
  `roles == CatalogRole::ALL`, and `pinned()` builds the catalog from
  `CatalogRole::ALL`, so that line holds by construction. Its other checks
  (unique identities, the `agent-ix` authority, the exact two-member
  reference JSON) are real. This is not a finding.
- The catalog's hand-written identities can only be checked against QSpec
  under `make conformance`. That `make conformance` run stopped at c14, so I
  ran the two catalog tests myself against QSpec main b1da9c8. Both pass: all 20 rows (role, authority and identity, in order), the
  package-selection rules, the trigger vocabulary and the refusal codes
  match, and the 7 accepted and 12 refused selection vectors give their
  recorded outcomes (~/dev/worktrees/logs/qsl-616-review-catalog-conformance.log).

## Verdict

CONDITIONAL: one medium and one low finding, both trace gaps in the spec. The
tests that back the behaviour exist and are strong. What is missing is an AC
that states the behaviour, which is the main point of the PR. No AC or TC the
diff touches is unbacked.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No FR-093 AC states that the lock's edition and `definition_selections` rows and every law `definition` are exactly `{authority, identity}`, which is this PR's central rule. FR-093 Status says the emitter writes "each lock definition and each law `definition` as the identity-only `DefinitionRef` AC-17 states", but AC-17 names only `diagnostics.catalog`. The new test `a_law_names_its_definition_by_authority_and_identity` traces `FR-093-AC-7`, which is about key recomputation; its own doc cites "FR-093-AC-6's law" instead. So the matrix shows the law-shape rule as untested, and AC-7 as backed by a test that does not recompute keys. Fix: extend AC-17, or add an AC, to say that the lock's edition and definition selections and each law `definition` are exactly `{authority, identity}` and IR's reader admits the package. Then retag `a_law_names_its_definition_by_authority_and_identity` (and the lock-row half of `the_lock_selects_the_catalog_definitions`) to it, and add it to TC-416's scope. | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:710; spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:796-800; qsl-package/src/emit/tests.rs:343-349 |
| FND-002 | low | `conformance_catalog_matches_qspec_complete_value_lock` is now the only check that the `DefinitionLock` catalog's hand-written identities, which every emitted definition ref comes from, match QSpec's `complete-value-lock.json`. Yet no QSL AC states that comparison. The test traces only `QSpec-TC-192`, QSpec's integer-division profile test. FR-110's Inputs promise that "`make conformance` compares it with QSpec's lock", but no FR-110 AC or TC-490 step says so. Fix: add an FR-110 AC saying that the catalog's (role, authority, identity) rows, in order, and its package-selection rules, trigger vocabulary and selection refusal codes equal QSpec's `complete-value-lock.json`, read under `make conformance`. Add a TC-490 step for it and trace the test to that AC. | qsl-semantics/tests/it/complete_value_lock.rs:69-75; spec/functional/FR-110-resolve-header-profile-selections-at-e3.md:53-58 |

## New findings (disposition pass 1)

Found at 199093fbbe0e4860ab60a4243d98f348e0933606.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | low | The new TC-490 step 8 sits after the procedure's closing tag instruction, outside the numbered list. That instruction still reads `#[trace("FR-110-AC-1", …, "FR-110-AC-8", "TC-490")]`, so it never names FR-110-AC-9. Step 8 also reads only `complete-value-lock.json`, but its expected result, like AC-9, also requires that "QSpec's selection vectors give their recorded outcomes". `complete-value-selection-vectors.json` appears nowhere in the procedure. Fix: move step 8 above the tag instruction, extend the range to FR-110-AC-9, and have step 8 also read the selection vectors and run each through `admit_selection`. | spec/test-cases/TC-490-e3-resolves-header-profiles-against-the-definition-lock.md:50-53; spec/test-cases/TC-490-e3-resolves-header-profiles-against-the-definition-lock.md:79-82 |

## Dispositions

Round 1, reviewed at 199093fbbe0e4860ab60a4243d98f348e0933606 (fix commits 6dc0d0ff8 and 199093fbb, on top of 7afb98e17). I ran no build. I read the coder's qsl-616-74f0b99 focused-test and conformance logs: `a_law_names_its_definition_by_authority_and_identity`, `the_lock_selects_the_catalog_definitions` and both `complete_value_lock` conformance tests pass.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 6dc0d0ff8: the new FR-093-AC-20 (FR-093:713) states that the lock's edition and `definition_selections` rows and a `Float64` addition's `ieee_profile` law `definition` are exactly `{authority, identity}`, and that IR admits both packages. `a_law_names_its_definition_by_authority_and_identity` now traces `TC-416`, `FR-093-AC-20` (emit/tests.rs:346), and its doc no longer cites AC-6. `the_lock_selects_the_catalog_definitions` adds AC-20. FR-093 Status now credits AC-17 with `diagnostics.catalog` and AC-20 with the lock rows and law. TC-416 adds AC-20 to its scope and step 9, and the tests.md TC-416 row lists AC-20. |
| FND-002 | fixed | 6dc0d0ff8: the new FR-110-AC-9 (FR-110:181) states the `make conformance` comparison, covering catalog rows in order, selection rules, trigger vocabulary, refusal codes and the selection vectors. `conformance_catalog_matches_qspec_complete_value_lock` now traces `TC-490`, `FR-110-AC-9`, and the selection-vector test adds both. TC-490 has a step 8, its scope reaches AC-9, and the tests.md TC-490 row lists AC-9. FR-110 Status cites AC-9. The step 8 placement defect is FND-003. |

Round 2, reviewed at c65ba790bac245d0e9025f4ac7a4bd00559f5c70 (fix commit c65ba790b). I ran no build.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-003 | fixed | c65ba790b: step 8 now sits inside the numbered procedure, before the tag instruction. It reads both `complete-value-lock.json` and `complete-value-selection-vectors.json` from `QSPEC_DIR`, compares the lock with `DefinitionLock::pinned()` and admits each selection vector through it, which matches its expected result and FR-110-AC-9. The tag instruction now reads `#[trace("FR-110-AC-1", …, "FR-110-AC-9", "TC-490")]` (TC-490:50-56). |
