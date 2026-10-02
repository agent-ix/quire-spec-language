---
id: SR-1080
title: "Spec review of PR #578 (state forall separating witness, ADR-031, FR-265 to FR-269, native run retirement)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@fdc332355886c1c1687c653d1cd6c9cbb637d1af; spec/decisions/ADR-031-state-forall-separating-witness.md, spec/functional/FR-265 to FR-269, spec/functional/FR-001, FR-026, FR-027, FR-028, FR-031, FR-032, FR-072, FR-098, FR-100, FR-108, FR-122, spec/test-cases/TC-740 to TC-744, TC-103, TC-104, TC-106, TC-109, TC-110, TC-430, TC-435, TC-469, TC-517, spec/spec.md, spec/tests.md, spec/native-workflow/tests.md"
review_set: subset
---

## Summary

Ticket: QSL-389. Review of `git diff origin/main...HEAD` at fdc33235 against
the four owner rulings (R-1 `/2` replaces `/1` in one change with no compat
path; R-2 QSpec owns the value-path vocabulary; R-3 a basis label; R-4 the
deciding quantifier named by its node key), the wave-B/wave-C owner rulings,
and the QSpec counterparts FR-207 AC-9 to AC-11, FR-351-AC-7 and FR-352
AC-2, AC-6, AC-7 (quire-specification#176, open).

Lenses applied: integrity (cross-FR consistency), QSpec consistency, EARS
phrasing, AC-to-TC coverage, caps and limits, version tracking, compat paths,
undefined-is-refuted.

What is right: the derivation (FR-265) is one function shared by producer
and replay; every fixture case in FR-265's criteria evaluates to the stated
result when traced by hand (AllBelow, SomeAtLeast, NotAllBelow, GuardedAll,
EqualsAll, LetAll, OrAll, NestedAll, FilteredAll, MappedAll, BuiltAll,
NoneSelected); `/2` replaces `/1` with no reader for `/1` and no dual
production (R-1 is honoured in FR-267 and FR-072-CON-1); the basis label is
on every clause-run report (R-3); the deciding quantifier is the occurrence
key (R-4); the separation check re-evaluates rather than trusts; undefined
S6a outcomes keep their O-16 category and basis `unavailable`, consistent
with the QSL-366 ruling, which applies to proof engines; every AC has a
behaviour TC (TC-740 to TC-744). `quire validate` on the changed files exits
0 and `tools/check-index-completeness.sh` passes.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-267 says the `run` command SHALL write every clause-run result document as `native-run-result/2`, and ADR-031 SW-10 calls `/2` "the CLI `run` serialization of an FR-109 clause run report". With native `run` retired, the only `run` route is FR-100, which refuses `selection`, `snapshots` and `invocations`, and FR-109 says its CLI request form "is not specified here ... a follow-up". No CLI path produces a clause-run document, so FR-267's clause-run writer is unreachable from `run` and the retirement removes the only CLI clause route. Specify the clause-run CLI request form in this change, or state FR-267's clause-run writer as the library serializer of `ClauseRunReport`. | spec/functional/FR-267-write-run-results-as-native-run-result-2.md:49; spec/functional/FR-100-run-a-named-function-through-the-spine.md:245; spec/functional/FR-109-run-a-state-clause-through-the-spine.md:171 |
| FND-002 | high | FR-100 Inputs now states fixed intake caps: "The request is at most 1 MiB; it names at most 64 dependent files of at most 8 MiB together". This diff moves them from FR-026 into FR-100 as new normative text. Owner rule: no fixed caps; a bound is a caller-configurable resource limit with a published default, and reaching it names the limit, its value and how to raise it. | spec/functional/FR-100-run-a-named-function-through-the-spine.md:50-51 |
| FND-003 | medium | The intake limits and request-envelope rules moved into FR-100 (closed envelope, duplicate members, per-file digest check, relative path resolution, byte and file limits) have no AC in FR-100. Their only AC was FR-026-AC-4, which this diff retires, so the behaviour is now untested. | spec/functional/FR-100-run-a-named-function-through-the-spine.md:48-55 |
| FND-004 | medium | FR-267-AC-3 tests a source "whose source declares an edition other than `0-draft` or `1-draft`", but FR-100 now refuses `0-draft` and no edition with `unknown_edition` too (FR-100-AC-2). The AC carries the retired route's edition set. | spec/functional/FR-267-write-run-results-as-native-run-result-2.md:85 |
| FND-005 | medium | FR-265 Behavior restates the value-path segment vocabulary as QSL SHALLs ("Each later segment SHALL be one member, object reference, option value or enclosing-collection position"), and ADR-031 SW-5 lists the root and step kinds, although R-2 says "QSL cites it and restates none of it". The restated list already drifts from QSpec FR-207: it has no map-key step (FR-207-AC-10) and no `built` subject among the roots (FR-207-AC-9). Cite QSpec FR-207 instead. | spec/functional/FR-265-derive-a-state-clause-separating-witness.md:81-85; spec/decisions/ADR-031-state-forall-separating-witness.md:98 |
| FND-006 | medium | FR-268 and FR-269 disagree on a failed step 2. FR-268: "If any step fails, the executor SHALL settle `inconclusive` with `DisagreementCause::Witness` naming the step that failed". FR-269: "step 2's domain evaluation failing to complete is `NoValue`", and its `Separation` enum has no step-2 value. FR-268 also states no outcome for a domain or body evaluation that ends `undefined` or refused, only for an exhausted meter. | spec/functional/FR-268-check-a-state-clause-witness-on-replay.md:68-83; spec/functional/FR-269-settle-a-witness-disagreement-as-a-typed-cause.md:38-41 |
| FND-007 | medium | FR-267 defines `/2`'s command-error envelope by reference to a retired requirement: Inputs "a command-error envelope (FR-026's members, ...)" and Dependencies "FR-026 (the members `/2` keeps)". The members of a live wire format depend on text kept only as a record of a retired route. State the envelope members in FR-267, or cite QSpec FR-352 for them. | spec/functional/FR-267-write-run-results-as-native-run-result-2.md:37, :94 |
| FND-008 | medium | The retired FR-026, FR-028, FR-031 and FR-032 keep their full SHALL text and AC tables under a banner ("The text below records the retired route"), their Status sections keep "Before retirement ..." history, and the test indexes keep 22 "Retired (ADR-031 R-1)" rows for TC-103, TC-104, TC-106, TC-109 and TC-110. That is a history record kept as normative text. It breaks "state what is" and the no-tracking rule. Delete the retired FRs, TCs and their index rows, and point the remaining references at FR-100, FR-109 and FR-267. | spec/functional/FR-026-run-standalone-native-workflow.md:21-24, :129; spec/functional/FR-028-run-selected-native-package.md:15, :74; spec/functional/FR-031-run-extracted-native-source.md:15-19, :90; spec/functional/FR-032-realize-config-version-workflow.md:17; spec/native-workflow/tests.md:44-97 |
| FND-009 | low | Stale statements about the retired route remain. ADR-012 still says the "`0-draft` route of CLI `run`, which FR-100 sends to native run (FR-026)". FR-109's relationships still target FR-028. NFR-010 still relates to FR-032. ADR-031 R-1 overrides ADR-011 §7.3 M-6c and ADR-012 §15.8 but amends neither. | spec/decisions/ADR-012-semantic-family-extension-contracts.md:1371-1372; spec/functional/FR-109-run-a-state-clause-through-the-spine.md:22; spec/non-functional/NFR-010-source-release-integrity.md:7 |
| FND-010 | low | FR-265-AC-5's `BuiltAll` record has "a value path rooted at the list expression's occurrence key" and `index` 1. QSpec FR-207-AC-9 gives a built-collection path "with an index step for the element's position". ADR-031 SW-5 makes the value path end at the domain collection, with the index as a separate component. The AC does not say whether the path carries the index step. | spec/functional/FR-265-derive-a-state-clause-separating-witness.md:138 |
| FND-011 | low | FR-265 Dependencies says the QSpec value-path vocabulary and the deciding-quantifier component "are being specified", which is future tense, not a statement of what is; they are QSpec FR-207 AC-9 to AC-11 and FR-351-AC-7. ADR-031 Status still reads "Proposed" although every question has a recorded ruling. | spec/functional/FR-265-derive-a-state-clause-separating-witness.md:151-153; spec/decisions/ADR-031-state-forall-separating-witness.md:39 |

## Verdict

Not mergeable as it stands. FND-001 leaves FR-267's clause-run document
with no producer once native `run` is retired. FND-002 adds fixed caps,
which the owner rules forbid. FND-003 to FND-008 are real gaps or
inconsistencies an implementer would trip on. The witness design itself
(FR-265, FR-266, FR-268's re-derivation, FR-269) is sound and matches the
rulings.

## New findings (disposition pass 1)

Delta reviewed: fdc33235..46ce4ed1 (FR-034, IT-002 and NFR-010 repointed from FR-032 to FR-108).

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-012 | low | IT-002 now says FR-108 runs the ConfigVersion corpus "through the spine", but the next sentence still calls them "these native runs". | spec/integration/IT-002-native-state-workflow.md:29 |

## New findings (disposition pass 2)

Delta reviewed: 46ce4ed1..69a8830c (8b4d5ad0 fix round; efa77d95 and 3f1301e2 identity-only headers; 69a8830c plan-lead C-items C-26, C-27, C-34 to C-37 and C-47, which restore native `run` and FR-026 to FR-032 until FR-100's clause runner lands). TC-452's fixture `F` was recomputed from its three lines: 136 bytes, `sha256:0c84cc4af8de870892c28bae81b32ab5fb9e28c46fd5f41e2d3f4db50eca36eb`, and the literal `5` at byte 132, line 3, column 54 (end byte 133, column 55). All of these match the TC.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-013 | high | FR-108's unit selects `profile m = "quire.model.complete/v1"`, and every clause is `using m`. FR-110 resolves a header profile only against the `root` row, `quire.value.complete/v1`, which is the one header-selectable row, and refuses any other identity with `unknown_profile` (`wrong-selection-role` or `unsupported-selection`). As written, the corpus unit refuses at E3, so none of FR-108's spine runs or its parity test reach evaluation. Either FR-110 admits the `quire.model.complete/v1` layer in this change (QSpec's `complete-value-lock.json` lists it under `header_selectable_layers`), or FR-108 keeps `quire.value.complete/v1`. | spec/functional/FR-108-run-the-configversion-spine-corpus.md:47, :56; spec/functional/FR-110-resolve-header-profile-selections-at-e3.md:33, :76, :86-87 |
| FND-014 | high | FR-108's and TC-456's `model` lines drop the version string: `model Config = "example/config-version" digest "sha256-jcs:..."`. QSpec's shared grammar requires it (`model = 'model', ident, '=', string, 'version', string, 'digest', string, ';'`), and FR-056 states the same form, so both units fail to parse at S1. This is a grammar production, not a version check. Keep the version string, or cite a QSpec grammar change that makes it optional. | spec/functional/FR-108-run-the-configversion-spine-corpus.md:57; spec/test-cases/TC-456-s2-builds-state-clause-forms.md:22; spec/functional/FR-056-admit-domain-package-model-declarations.md:226-227 |
| FND-015 | medium | FR-108 says a domain-package digest mismatch refuses `content-mismatch`. At this head, no QSL requirement and no QSpec catalog entry defines that cause. FR-056 refuses the same mismatch with `stale_dependency`/`byte-digest-mismatch`. Cite the cause FR-056 uses. | spec/functional/FR-108-run-the-configversion-spine-corpus.md:51; spec/functional/FR-056-admit-domain-package-model-declarations.md:95 |
| FND-016 | medium | FR-267, FR-109, ADR-031 SW-11 and R-1, and ADR-013's #186 row each tie the landing of `/2` and the deletion of native `run` to "FR-100's clause runner". No requirement specifies that runner: FR-100 runs a named function, and FR-109's `run_clause` is a library entry. FR-108 names the same deletion "the M-6c PR", and ADR-011:701 says native clause execution stays "until M-6c lands one". That gives one event two names, and neither names a specified deliverable. Name the requirement that specifies the CLI clause runner, or state the condition as ADR-011 M-6c in every place. | spec/functional/FR-267-write-run-results-as-native-run-result-2.md:35; spec/functional/FR-109-run-a-state-clause-through-the-spine.md:176; spec/decisions/ADR-031-state-forall-separating-witness.md:109, :137; spec/functional/FR-108-run-the-configversion-spine-corpus.md:116 |
| FND-017 | medium | FR-267 Behavior says "The `run` command SHALL write every command-error envelope with that format" (`native-run-result/2`), and FR-267-AC-3 expects a `/2` envelope for an `edition "9-draft"` request. At the same head, FR-100 says every refusal before S6a and every internal failure "writes FR-026's native-run-result/1 command-error envelope". These are present-tense SHALLs that contradict each other for the same refusal. They are reconciled only by FR-267's Description sentence, which defers it to a later change. Scope FR-267's command-error bullet and AC-3 to that change in the same words, or move them into the requirement that lands it. | spec/functional/FR-267-write-run-results-as-native-run-result-2.md:57, :105; spec/functional/FR-100-run-a-named-function-through-the-spine.md:219, :234-235 |

## New findings (disposition pass 3)

Delta reviewed: 69a8830c..b91af8ee (0a20f351 FR-312 clause-run request form and ADR-013 OQ-2; c61bfbcc no per-file digests on clause-run and `1-draft` requests; b91af8ee round-2 fixes, `request_digest` and `locus.source_digest` deleted). Checked: `request_digest` is gone from FR-100, FR-267, TC-450 and TC-742; the locus is `{file, span}` in FR-100 and TC-452, with TC-452's span unchanged (byte 132 to 133); FR-312's `observations` map keys to files only; FR-312 is the request form and defines "FR-100's clause runner" once; ADR-013 OQ-2 and the Serialized authority row put the replay result in QSpec FR-323's envelope. `quire validate` on every changed spec file exits 0 and `tools/check-index-completeness.sh` passes.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-018 | low | FR-108 ("this requirement merges after #176") and TC-456 ("this TC merges after #176") put a merge-order note into normative text. Once merged, the note is stale history. State the grammar form by citing QSpec's shared-grammar `model` production, and keep the merge order on the PR. | spec/functional/FR-108-run-the-configversion-spine-corpus.md:52-53; spec/test-cases/TC-456-s2-builds-state-clause-forms.md:23-24 |
| FND-019 | low | FR-312's `observations` map each key to a file only, with no digest. FR-109 still puts "the identity and digest of every snapshot and invocation admission read" into provenance, and FR-109-AC-5 expects `stale_dependency`/`byte-digest-mismatch` when "snapshot bytes change after the selection digest was taken". Through FR-312, that digest is the reader's own digest of the bytes it just read, so AC-5's check cannot fail. Delete the per-file observation digest from FR-109's provenance and AC-5, or say which digest the reader puts into FR-106's provisions. | spec/functional/FR-109-run-a-state-clause-through-the-spine.md:82-85, :161; spec/functional/FR-312-read-a-clause-run-request-and-run-it.md:54 |
| FND-020 | low | FR-100 Behavior now refuses a `1-draft` request "carrying neither `call` nor `clause`" and runs one carrying `clause` through FR-312. FR-100-AC-3 still says "a `1-draft` request with no `call` refuses". FR-100 also does not say what a `1-draft` request carrying `clause` gets before FR-312 lands, although its SHALL is in the present tense. Align AC-3, and scope the `clause` bullet the way FR-267's command-error bullet is scoped. | spec/functional/FR-100-run-a-named-function-through-the-spine.md:273-279, :342 |

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-009 | still-open | 46ce4ed1 repoints NFR-010 to FR-108. ADR-012:1372 still says FR-100 sends `0-draft` to native run (FR-026), and FR-109:22 still targets FR-028. |
| FND-001 | still-open | 46ce4ed1 does not touch it; the cited text is unchanged. |
| FND-002 | still-open | 46ce4ed1 does not touch it; the cited text is unchanged. |
| FND-003 | still-open | 46ce4ed1 does not touch it; the cited text is unchanged. |
| FND-004 | still-open | 46ce4ed1 does not touch it; the cited text is unchanged. |
| FND-005 | still-open | 46ce4ed1 does not touch it; the cited text is unchanged. |
| FND-006 | still-open | 46ce4ed1 does not touch it; the cited text is unchanged. |
| FND-007 | still-open | 46ce4ed1 does not touch it; the cited text is unchanged. |
| FND-008 | still-open | 46ce4ed1 does not touch it; the cited text is unchanged. |
| FND-010 | still-open | 46ce4ed1 does not touch it; the cited text is unchanged. |
| FND-011 | still-open | 46ce4ed1 does not touch it; the cited text is unchanged. |
| FND-001 | fixed | 8b4d5ad0: FR-267 makes the clause-run document the library serialization of `run_clause`'s `ClauseRunReport`, and FR-109 says the CLI clause run is native `run` until the clause runner lands, so the writer has a producer (the envelope conflict is FND-017). |
| FND-002 | fixed | 69a8830c: FR-026 now reads the request under `request_bytes` and dependent files under `dependent_bytes`, each with a published default, a CLI option and no ceiling. The fixed file-count cap is gone, FR-027 shares the limits, and FR-100 defers to FR-026. |
| FND-003 | fixed | 69a8830c: The intake rules are back in FR-026, and FR-026-AC-4 tests both limits, naming the limit, its value and the option. FR-026-AC-3 and AC-5 cover malformed requests and path resolution. |
| FND-004 | fixed | 8b4d5ad0: FR-267-AC-3 uses `edition "9-draft"`, which is unknown under both routes FR-100 states. |
| FND-005 | fixed | 8b4d5ad0: FR-265 and ADR-031 SW-5 cite the QSpec FR-207 runtime value path. 69a8830c also makes SW-6 cite it for `filter`, `map` and built elements. |
| FND-006 | fixed | 8b4d5ad0: FR-268 and FR-269 agree: `Separation { step, reason }` covers steps 1 to 4, undefined results refute with `UndefinedEvaluation`, a refusal carries its record, and an exhausted meter settles `NoValue`. |
| FND-007 | fixed | 8b4d5ad0: FR-267 states the command-error envelope members itself and no longer cites FR-026 for them. |
| FND-008 | fixed | 69a8830c: C-34 restores FR-026, FR-028, FR-031 and FR-032, TC-103, TC-104, TC-106, TC-109 and TC-110 as live requirements, with no retired banner and no "Before retirement" history. The retired-text record no longer exists. |
| FND-009 | fixed | 69a8830c: ADR-012 section 15.8, FR-109's FR-028 edge and ADR-031 R-1 now agree that native `run` stands until the clause runner lands. NFR-010 is deleted (C-37) and has no remaining references. |
| FND-010 | fixed | 8b4d5ad0: FR-265-AC-5's `BuiltAll` path is rooted at a `built` subject and ends in an index step for position 1 (QSpec FR-207-AC-9). |
| FND-011 | fixed | 8b4d5ad0: FR-265 Dependencies cites QSpec FR-207 AC-9 to AC-11 and FR-351-AC-7, and ADR-031 Status reads Accepted. |
| FND-012 | fixed | 69a8830c: IT-002 is restored to say FR-032 supplies the native runs. That is true again because FR-032 stands, so "these native runs" is accurate. |
| FND-013 | fixed | b91af8ee: FR-108's unit selects `profile v = "quire.value.complete/v1"`, the `root` row FR-110 resolves, and every clause is `using v`. |
| FND-014 | still-open | FR-108 and TC-456 now cite quire-specification#176, whose diff drops `'version', string` from the `model` production. But #176 is OPEN at d11f789c, so QSpec main still requires the version. QSL FR-056 also still states the model selection as "identity, version and `sha256-jcs` digest" and the form `model M = "<identity>" version "<version>" digest ...`. #578 can merge only after #176 lands and FR-056 drops the version too. |
| FND-015 | still-open | The cause now exists: quire-specification#174 is merged and adds `stale_dependency`/`content-mismatch` to QSpec FR-272. But FR-056, whose I1 admission runs this check, still refuses a digest mismatch with `stale_dependency`/`byte-digest-mismatch`, so two QSL requirements give two causes for one refusal. This resolves when FR-056 adopts `content-mismatch` (#586, SR-1040 FND-003) or FR-108 cites FR-056's cause. |
| FND-016 | fixed | b91af8ee: FR-312 defines "FR-100's clause runner" once (its reader plus `run_clause`) and identifies the change as ADR-011 M-6c's deletion of native `run`. FR-100, FR-108, FR-109, FR-267, ADR-011:701, ADR-013 #186 and ADR-031 SW-11 and R-1 all use that wording. |
| FND-017 | fixed | b91af8ee: FR-267's command-error bullet and AC-3 are scoped to the change that lands FR-100's clause runner, and FR-100 says `/1` until that change and `/2` from it. |
