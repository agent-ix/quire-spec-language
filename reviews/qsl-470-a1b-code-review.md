---
id: SR-1265
title: "Code review of PR #616: definition refs are authority and identity (QSL-470 A1b)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@7afb98e17cc1862e1504079021b48163a0fcf2fb; PR #616 diff against origin/main (merge base fbf69cb9, 45 files): qsl-semantics/src/value/{definition,diagnostics_catalog,mod}.rs, quire-semantic-value/src/definition.rs, qsl-foundation/src/source.rs, qsl-foundation/src/source/provenance.rs, qsl-foundation/src/diagnostic/locus.rs, qsl-package/src/{emit,checked,checked_v2}.rs and their tests, qsl-replay/src/result.rs, src/linking/composed/definition_source.rs, qsl-semantics and qsl-eval test updates, tests/it/{cli,composed_definition_source}.rs, tests/fixtures/parser-differential/baseline.txt, Cargo.toml, Cargo.lock, qsl-package/Cargo.toml, qsl-semantics/Cargo.toml, Makefile"
review_set: subset
---
# Code review of PR #616

## Summary

Ticket: QSL-470 (slice A1b). PR: quire-spec-language#616, branch
task/470-a1b-artifact-refs, draft. The PR makes every definition reference
`{authority, identity}` and drops the revision from `RawSourceRef`, while
source rows keep their `quire.source.bytes/v1` digest. It deletes the
`quire-specification` dependency, the lock reader (`DefinitionLock::read`,
`LockReadError`) and the diagnostics-header reader. `DefinitionLock::pinned()`
now builds the catalog from `CatalogRole::identity()` and
`CatalogRole::selection_rule()`. The catalog-versus-QSpec checks move to
`make conformance`, which reads QSpec at run time from `QSPEC_DIR`. The
rust-review lane is folded into this file.

The PR builds against IR branch `feat/artifact-ref-authority-identity` at
e8db0876 through the `TEMP:` commit (995c91aeb). The brief says that commit is
replaced by a repoint to IR main before merge, so it is not reviewed here.

What the brief asked me to check:

- **No revision or digest left on a definition ref.** Verified.
  `DefinitionReference` is `{authority, identity}` with
  `deny_unknown_fields` (qsl-semantics/src/value/definition.rs:267-279).
  `CatalogEntry` has no revision, path or digest. `DefinitionRevision`,
  `DefinitionLock::revision()`, `PackageCause::RevisionMismatch` and
  `PackageCause::DigestDomainMismatch` are gone. The emitter's `artifact()`
  writes `CheckedArtifactRef {authority, identity}` (qsl-package/src/emit.rs:663-669).
  A grep of the tree at this head finds no `DefinitionRevision`,
  `LockReadError`, `quire_specification`, `native_diagnostics_identity` or
  `rule_manifest`. The `revision-mismatch` uses that remain belong to the
  dependency, library and frame paths, not to definition refs.
- **Source rows keep their digest.** Verified. `RawSourceRef` keeps
  `DigestRecord` and refuses a non-`quire.source.bytes/v1` digest
  (provenance.rs). The emitter's `source_artifact()` writes
  `CheckedSourceRef {authority, identity, digest_domain, digest}`
  (emit.rs:671-681). `the_lock_selects_the_catalog_definitions` asserts the
  exact source row JSON, digest included, on `lock.sources` and on every
  source-map region.
- **No compatibility layer.** Verified. There is no `serde(default)`, no
  optional field and no second reader in the diff.
  `an_old_reference_shape_refuses_unknown_member` checks that a `revision`,
  `digest_domain` or `digest` on the edition, a definition selection or the
  diagnostics catalog refuses `unknown_member`, and so does a `revision` on a
  source row or a region's source.
- **IR's `unsupported_construct`.** The new arm maps it to
  `Code::UnsupportedConstruct` (qsl-package/src/checked_v2.rs:500). IR raises
  it with cause `expression-form` for `case`, `temporal_formula` and
  `temporal_fairness` operators, and QSpec's native catalog lists
  `unsupported_construct` with that cause. So the mapping is right. It has no
  test (FND-001).
- **Parser-differential baseline.** All 100 `historical` and all 100
  `composed` buckets were re-recorded, and the 100 `complete` buckets are
  unchanged. `render_native` hashes `format!("{unit:?}")`, and a parsed unit's
  `Debug` includes its `RawSourceRef`. Dropping `revision` therefore changes
  every successful native parse, and so every native bucket. The complete-V1
  rendering prints tokens, CST nodes and diagnostics, not the source ref, so
  its buckets cannot change. No parser code is in the diff: under `src/` the
  diff touches only `src/linking/composed/definition_source.rs`, and it touches
  nothing in `qsl-cst`. So the re-record follows from the `RawSourceRef`
  change and cannot be hiding a parser regression. I could not recompute the
  old hashes with `revision` stripped without a second build of the base, so
  this rests on the diff, not on a measurement.
- **New ceremony.** None in the diff. The two new Makefile `grep` checks only
  confirm that each conformance test ran rather than printing `skipped`. They
  match `[1-9][0-9]*` and pin no count. `package_id_matches_its_golden_vector`
  and the FR-093 E14-E17 keys are canonical identity digests, and
  `recursive_text_leaf_vectors_check_and_key` recomputes the spec's E14-E17
  keys from the checked graph.
- **Routed item, for the plan session.** Production revision labels in the
  composed registry are routed to the plan session and are not findings here.
  Note that this diff adds one of them. `RegisteredDefinition::Diagnostics`
  now hard-codes revision `"1-draft.8"`
  (src/linking/composed/definition_source.rs:327-338), where it used to read
  the value from QSpec's document header. QSpec main's `native-diagnostics.md`
  header (b1da9c8) no longer declares a revision
  ("Interpretation identity: `quire.native.diagnostics/v1`."). So the literal
  is a copy of a value that QSpec has since dropped, and it feeds the handoff
  writer.

Rust lane:

- `pinned()` builds a fixed `[CatalogEntry; CatalogRole::ALL.len()]` from
  `CatalogRole::ALL`, and groups the `exactly_one` roles by trigger. It
  cannot panic. `resolve_division` and `admit_ieee_profile` now match on
  authority and identity together, and the refusal order is unchanged for
  missing, conflicting and reserved-intrinsic cases.
- The new public items (`CatalogRole::identity`,
  `NATIVE_DIAGNOSTICS_IDENTITY`) have docs. `SelectionRule` and
  `selection_rule` are private.
- No new `unsafe`, `#[allow]` or `as` cast. The test-only `unwrap`s in
  `complete_value_lock.rs` read QSpec JSON under `make conformance` and panic
  with the path, which is fine for a test.
- `DefinitionLock::entry` still returns `Option` over a catalog that is now
  total by construction (FND-002).

Test-oracle strength, for each changed test:

- `the_lock_selects_the_catalog_definitions` compares whole JSON values for
  the lock rows, the source rows, each region source and the diagnostics
  catalog. An extra member, a dropped digest or a reintroduced revision each
  fails it.
- `a_law_names_its_definition_by_authority_and_identity` asserts that laws
  exist, then compares each law's `definition` with the `ieee_profile` row as
  exact JSON. Its trace is wrong (SR-1266 FND-001).
- `an_old_reference_shape_refuses_unknown_member` checks the admitted
  envelope first, so a refusal cannot come from a broken fixture, and then
  asserts `UnknownMember` for each of the 11 mutations.
- `div_09_...` and `semantic_admission_refuses_...` substitute another
  authority under the catalogued identity. If the authority comparison were
  dropped, the substituted reference would be admitted and both tests would
  fail.
- `admission_mints_the_caller_named_source_reference` checks that `b'` changes
  only the digest and that the digest is `ByteDigest::of(b"b'")`.
- The cli tests assert `source` has no `revision`, and TC-425's test compares
  the exact `RawSourceRef` JSON.
- The depth-limit fixture moving from 8 to 7 matches the removed
  `source.revision.{namespace,value}` level, which was the envelope's deepest
  path (`source_map[0].regions[0].source.revision.value`).
- The two `conformance_*` tests in `complete_value_lock.rs` are strong when
  `QSPEC_DIR` is set. The coder's `make conformance` run
  (qsl-616-e8db087-conformance.log) stopped at the routed
  `conformance_c14_...` failure, before the Makefile reaches them, so I ran
  them myself against QSpec main b1da9c8 (see Gates).

Gates: the coder's `make ci` log (qsl-616-e8db087-ci.log) was written at
02:09, after the 01:36 head commit, and ends `exit=0`. I ran
`cargo test -p qsl-semantics --test it -- complete_value_lock::` with
`QSPEC_DIR` set to QSpec main b1da9c8's two definition documents, through
locked-build. Result: 3 passed, 0 failed. It printed "conformance: 20 catalog rows match
QSpec's lock" and "conformance: 7 accepted and 12 refused selection
vectors". The target was already built at this head, so nothing was
rebuilt (log: ~/dev/worktrees/logs/qsl-616-review-catalog-conformance.log).

## Verdict

Changes requested: one medium and one low finding. Both are small: one new
test, and one signature tightening. The contract holds. Definition refs are
`{authority, identity}` everywhere in the diff's reach, source rows keep
their digest, there is no compatibility layer, the `quire-specification`
dependency is gone, the `unsupported_construct` mapping is correct and the
baseline re-record follows from the change. Setting aside the routed
conformance item (c14) and the routed registry revisions, the PR is
mergeable once these findings and SR-1266's are fixed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The new `CheckedPackageRefusalCode::UnsupportedConstruct => Code::UnsupportedConstruct` arm has no test. No QSL test reads a v2 wire whose application node uses a `case`, `temporal_formula` or `temporal_fairness` operator and then checks the I2 refusal's `code()`. Other envelope arms have one: the `AmbiguousDeclaration`, `MissingDeclaration`, `InvalidSourceMap` and `InvalidModelBinding` envelope refusals are each asserted through `code()` in checked_v2/tests.rs. The path is live: the routed c14 failure is exactly this refusal on QSpec's positive fixture. If the arm were changed to `Code::InvalidPackage`, as the neighbouring `UnsupportedNodeTag` arm maps, every gate would stay green. Fix: add a checked_v2 test that reads an envelope with one `temporal_formula` application node, valid `package_id` included, and asserts `Code::UnsupportedConstruct` plus IR's `expression-form` cause and its `/body/operator` locus. | qsl-package/src/checked_v2.rs:500; qsl-package/src/checked_v2.rs:423-433 |
| FND-002 | low | `DefinitionLock::entry` still returns `Option<&CatalogEntry>` and does a linear search. The catalog is now `CatalogRole::ALL.map(..)`, so it has a row for every role by construction and `None` cannot happen. Callers therefore keep dead failure handling: `admit_ieee_profile` keeps an unreachable `MissingMember` refusal (definition.rs:584-586), `emit::catalog_entry` keeps an `expect` (emit.rs:685-687), and `check::resolve_profiles` keeps another (check/profile.rs:67-69). Fix: return `&CatalogEntry` from `entry`, for example by indexing the array with the role's position in `CatalogRole::ALL`, and delete the dead branches and `expect`s. | qsl-semantics/src/value/definition.rs:352-354; qsl-semantics/src/value/definition.rs:584-586; qsl-package/src/emit.rs:685-687 |

## New findings (disposition pass 1)

Found at 199093fbbe0e4860ab60a4243d98f348e0933606.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | medium | The IR #269 lockstep replaces the deleted arm with `CheckedPackageRefusalCode::UnknownProfile => Code::UnknownProfile`. That arm has no test either, so FND-001's problem is back on the same line with a new variant. The mapping itself is correct: IR raises `unknown_profile` when a temporal application's `temporal_profile` law names no QSpec FR-250 profile, with cause `unsupported-selection` or `wrong-selection-role` (IR 74f0b99, checked_package/v2/temporal.rs:533-551). QSpec's native catalog lists that code with those causes, and `Code::UnknownProfile` is the code FR-110 header refusals already use. But no QSL test reads a wire that IR refuses this way, and no QSpec adverse mutation reaches it: `conformance_i2_read_...` maps only `unknown_contract_version`, `digest_domain_mismatch`, `unsupported_node_tag`, `invalid_semantic_graph` and `invalid_package`. If the arm were changed to `Code::InvalidPackage`, every gate would stay green. Fix: add a checked_v2 test that reads an envelope holding one temporal application whose `temporal_profile` law names an unknown identity. Assert `Code::UnknownProfile`, IR's `unsupported-selection` cause and the `/operation/laws/0/definition` locus. | qsl-package/src/checked_v2.rs:500 |

## Dispositions

Round 1, reviewed at 199093fbbe0e4860ab60a4243d98f348e0933606 (fix commits 6dc0d0ff8 and 199093fbb, on top of 7afb98e17). `git range-diff` shows the earlier commits are unchanged, apart from the TEMP commit, which now points at IR 74f0b99. I ran no build. The coder's qsl-616-74f0b99 logs show the focused tests, clippy, string-edge and the conformance steps passing, with c14 now green. The only conformance failure is the known `conformance_dependency_selection_vectors`.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 6dc0d0ff8: the `UnsupportedConstruct` arm is gone, because IR 74f0b99 no longer has the variant (`CheckedPackageRefusalCode` ends at `MissingImport`, `UnknownProfile`). There is nothing left to test. The replacement `UnknownProfile` arm has the same gap and is recorded as FND-003. |
| FND-002 | fixed | 6dc0d0ff8: `DefinitionLock::entry` now returns `&CatalogEntry` as `&self.catalog[role as usize]` (definition.rs:351-355). `admit_ieee_profile` no longer has the `MissingMember` branch, and the `expect`s are gone from `emit::catalog_entry`, `check::resolve_profiles`, qsl-bench and the tests. The index depends on `CatalogRole`'s declaration order matching `CatalogRole::ALL`. It does today, and `the_catalog_covers_every_role_exactly_once` asserts `lock.entry(entry.role) == entry` for every role, so a reordering fails `make ci`. `emit::catalog_entry` is now a one-line pass-through; that is harmless. |

Round 2, reviewed at c65ba790bac245d0e9025f4ac7a4bd00559f5c70 (fix commit c65ba790b, on top of 199093fbb; the TEMP pin is still IR 74f0b99). I ran no build. I read the coder's qsl-616-fix2 red, green and clippy logs. I ran `rustfmt --check` on the changed files, which modifies nothing; it passed and the tree is clean.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-003 | fixed | c65ba790b: `unknown_temporal_profile_is_unknown_profile` (checked_v2/tests.rs:1606-1697) takes QSpec's all-families fixture and checks first that its own key recomputation reproduces every published application key. It then renames the temporal clause's `temporal_profile` law to `quire.fixture.temporal-profile/v1`, rekeys, rebuilds the identity preimage and `package_id`, and asserts IR `UnknownProfile`, QSL `Code::UnknownProfile` (line 1689) and the `/semantic_graph/nodes/{clause}/body/operation/laws/0/definition` locus. With the arm remapped to `InvalidPackage`, the red log (qsl-616-fix2-red.log, exit 101) fails at line 1689, `left: InvalidPackage, right: UnknownProfile`. The green log passes. The check runs only under `make conformance`, because the only temporal application fixture is QSpec's and QSL reads it by reference. `make ci` alone would still pass a remap. That closes the finding: `make conformance` is one of the repo's full gates, and the finding was that no gate catches a remap, which is no longer true. In the Makefile, the I2 step runs before the known-red `conformance_dependency_selection_vectors` step, so the check executes today. |
