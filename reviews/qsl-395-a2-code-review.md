---
id: SR-1281
title: "Code review of quire-spec-language A2: a library carries no version"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@e3c96833abd7e79ca31e84acedd8948253ad39e3; A2's own commits only, git diff ad18594412cb4cdd849104a7643491b231cd79ee..e3c96833a (ad1859441 is HL1's head): qsl-semantics/src/library/mod.rs, qsl-package/src/checked.rs, qsl-package/src/checked_v2.rs, qsl-replay/src/request.rs, qsl-replay/src/spine.rs, qsl-replay/src/spine/lifecycle.rs, qsl-replay/src/execute.rs, src/command.rs, src/command/wire.rs, and their tests"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-087
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-099
    type: reviews
---
# Code review of quire-spec-language A2

## Summary

Ticket: QSL-395 (A2, L2). There is no PR yet: A2 is stacked on HL1
(ad1859441), which is stacked on LC1 (#624). Reviewed A2's four commits
(cf347d77a, 85e390201, a488bd8b1, e3c96833a) on
`task/395-a2-dependency-versions`. The Rust lane (rust-review) is folded into
this file.

Rulings applied, not raised: `SuppliedLibrary.version`, the replay-request
dependency versions and the library revision labels are deleted; a library
is selected by identity and bound by `package_id`; ADR-015 D-4 rules 5 to 7
stay; no compat; the profile `model` declaration's version and digest are
untouched.

What the change does, checked against the code:

- `LibraryPackage.version`, `Selection.version`, `Import.version`,
  `SuppliedLibrary.version`, `DependencyEntry{,Wire}.version`,
  `ResolvedLibrary.version`, `wire::Library.version` and
  `VerifiedPackage::version` are deleted, with every `version` parameter of
  `read_v2`, `read_checked_package_v2`, `read_import_view` and
  `ClosureReader`.
- The empty-version refusals are deleted: `LinkRefusal::EmptySelection`,
  `DependencyInputRefusal::EmptyVersion`, `LibrarySelection::EmptyVersion`,
  and the version half of `ReplayRequestRefusal::EmptyDependencySelection`.
  Each one existed only to reject an empty value of the deleted field.
- `StaleCause` and `LibraryCause::RevisionMismatch` are deleted.
  `verify_binding` now compares only `package_id`; `StaleDependency` keeps
  its I2 rule 1 classification and `byte-digest-mismatch` cause.
- `HostCause::SelectionVersion` survives, correctly: its only remaining
  caller is the profile `model` declaration's version
  (`qsl-cst/src/parser.rs:356`), which the ruling leaves alone.
- `wire::Library` keeps `#[serde(deny_unknown_fields)]`, so a library object
  carrying `version` now refuses at deserialization as `invalid-request`,
  exit 20. That is what FR-027-AC-10 and TC-446 step 7 now say, and
  `malformed_libraries_refuse_as_invalid_request` backs it with a request
  that is otherwise the one `a_complete_v1_request_supplies_its_libraries_to_the_spine`
  compiles to exit 0, so the version member is the only difference.

Test oracles checked:

- `duplicate_package_id_is_the_named_exception_outside_all_four` now makes
  the two packages differ in `imports` instead of `version`. `resolve_libraries`
  compares whole `LibraryPackage`s at the `by_id` index
  (`qsl-semantics/src/library/mod.rs:853`), so the refusal is still reached
  for the stated reason, and TC-282 and FR-087 name `imports` as the
  excluded field.
- `a_resolved_library_lock_is_a_pinned_request` now builds a second package
  with a different preimage instead of a second version, and asserts
  `ByteDigestMismatch`. Real oracle.
- `refuses_when_the_pinned_version_disagrees` and the version half of
  `e4_refuses_a_conflicting_diamond` are deleted with the behaviour they
  tested. The `package_id` half of the diamond case stays.
- The FR-071 bound test now oversizes `identity` instead of `version`. It
  still proves the bound measures `dependencies` entries.

Rust lane: no new `unwrap`, `expect`, panic or `unsafe` on a production
path, no wildcard arm, no new pin, limit or compatibility layer. The
`Ok::<_, ReplayRequestRefusal>` turbofish in `ReplayRequest::decode` is
needed now that the closure's only early return is a `?`.

Focused tests run at e3c96833a through `locked-build.sh` (`qsl-semantics`
filtered to `library`; `qsl-package` to `checked_v2`, `e4_`, `diamond`,
`closure`; `qsl-replay` to `request::`, `dependency_tests`, `tc_444`,
`spine::tests`; root `it` to `malformed_libraries`,
`a_complete_v1_request_supplies`, `tc_450_step_5`, `tc_450_step_6`): 164
passed, 0 failed.

## Verdict

Approve with low findings. The deletions are the version ceremony the ruling
names, and none drops a behaviour a QSL requirement still states: every
removed refusal guarded only the deleted field, and the content check
(`package_id` recompute and compare) is untouched at every site. The three
findings are wording and one now-empty wrapper; none blocks merge. The
QSpec FR-307 divergence is a spec finding, SR-1283 FND-001.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `refuses_when_the_pinned_id_disagrees` is still documented as "TC-253 step 8", but it now fails one condition only (the pin's `package_id`). Step 8 is a doubly-failing input; `refuses_when_conditions_2_and_3_both_fail` and the checked_v2 wire-path test already back it. The test now duplicates checked_v2's `refuses_when_the_pinned_package_id_disagrees`. Relabel it as condition 3 (step 4, adverse) or delete it. | qsl-semantics/src/library/binding_tests.rs:161-177 |
| FND-002 | low | `LibraryRefusal::class` and `RefusalClass::BindingCondition` still document "condition 2 or 3". After this change no variant classifies to `BindingCondition(3)`: the only one was `StaleDependency{RevisionMismatch}`. The doc now describes a class nothing produces. | qsl-semantics/src/library/mod.rs:479-482, :513-522 |
| FND-003 | low | `Selection` is now a one-field struct around `PackageId`, which is already the domain identity newtype. Every site spells `Selection { package_id }`. Replace it with `PackageId` in `PinnedRequest`, `LibraryLock::selections`, `ResolvedDependency` and `ConflictingPin`, and keep `PinMismatch` as `{pinned: PackageId, presented: PackageId}`, whose two field names still say which side is which. | qsl-semantics/src/library/mod.rs:241-245, :296-301 |

## Dispositions

Disposition pass 1 at 04e61dc192a1c46eed563dc747afe8e8f18d7200 (fix commits
168aa7ecf and 04e61dc19, still stacked on HL1 ad1859441).

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 168aa7ecf: `refuses_when_the_pinned_id_disagrees` deleted; condition 3's stale pin stays backed by checked_v2 `refuses_when_the_pinned_package_id_disagrees`, step 8 by `refuses_when_conditions_2_and_3_both_fail`. |
| FND-002 | fixed | 168aa7ecf: `class()` and `RefusalClass` docs now say condition 2 only, and `BindingCondition` says a stale pin is I2's first rule. |
| FND-003 | fixed | 168aa7ecf: `Selection` deleted; `PinnedRequest`, `LibraryLock::selections`, `ResolvedDependency`, `ConflictingPin` and `single_pin` hold `PackageId`; `PinMismatch` is `{pinned: PackageId, presented: PackageId}`. |
