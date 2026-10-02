---
id: SR-1040
title: "QSL-395 spec review (integrity) of PR 586: delete pin, version and provenance ceremony"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@4dffb8b83bd8c32bdba78f39b536fc9ed716ffe8; git diff origin/main...HEAD (66 files under spec/): ADR-011..ADR-017, FR-001, FR-005, FR-010, FR-018, FR-035, FR-049, FR-056, FR-057, FR-058 (deleted), FR-061, FR-062, FR-075, FR-079 (deleted), FR-087, FR-091, FR-096..FR-101, FR-103, FR-106, FR-110, FR-111, FR-115, FR-116, FR-120, FR-122, IT-006, IT-012, IT-013 (deleted), NFR-001, NFR-011, TC-077, TC-113, TC-158, TC-159/203/204 (deleted), TC-282, TC-425, TC-431, TC-440, TC-444, TC-446, TC-448, TC-452, TC-454, TC-465, TC-469, TC-490, TC-491, TC-514, TC-515, TC-517, US-011, spec.md, tests.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-110
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-116
    type: reviews
---
## Summary

Ticket: QSL-395. PR: quire-spec-language#586 (draft) at 4dffb8b8. Checked
against the QSL-395 rulings recorded on the ticket (2026-10-01) and the
wave B / wave C briefs.

Load-bearing content identity survives:

- `package_id` recompute and compare at replay (FR-098) is unchanged.
- `DependencyIdentityMismatch` at the E4 link step and at compile against
  supplied libraries (FR-087-AC-14, FR-099-AC-2, ADR-015 steps 5 and D-4)
  is unchanged.
- The I1 domain-package digest recompute (FR-056 Admission, FR-154 check 3)
  is unchanged. This is the content check the ruling keeps for MS-03.
- Snapshot and invocation byte checks (FR-106 check 1.3) are unchanged.

Dangling references: FR-058, IT-013, TC-159 and TC-203 have no remaining
reference outside historical review files. FR-079 and TC-204 are still
named by a test trace tag (FND-007). `FR-058` hits in `src/` and `tests/`
are QSpec's FR-058, a different artifact. ADR-013 R-08, which this PR
deletes, is still cited twice (FND-002).

`content-mismatch` is not applied one way. It replaces `revision-mismatch`
for recompiled node-identity compares (FR-116, FR-122, ADR-014, ADR-017,
TC-515, TC-517). Every canonical-digest compare keeps
`byte-digest-mismatch`, including the one MS-03 check the ruling names
(FND-003). QSpec's catalog has no `content-mismatch` cause (FND-004).

`quire validate` on the 59 changed files exited 0 (warnings only; the
FR-061 EARS warning is SR-1041). `tools/check-index-completeness.sh`
exited 0.

## Verdict

Not mergeable as it stands. FND-001 is high: ADR-011 still specifies the
header-profile revision and digest refusal that FR-110 deletes. The rest
are medium or low and can be fixed in the same PR.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | ADR-011 §2.4 still resolves the Value header profile by identity, revision value and digest and refuses `stale_dependency`/`byte-digest-mismatch`, and says FR-110 reads the `root` row's digest. FR-110 now resolves by identity alone and FR-110-AC-5 compiles a 64-`a` digest. | spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md:503, :516, :544-547 |
| FND-002 | medium | ADR-017 RF-6 and Alternatives cite ADR-013 R-08, which this PR deletes, and state that a header pinning an old `root` revision refuses at compile. Under the new FR-110 that refusal does not exist. | spec/decisions/ADR-017-protocol-refinement-abstraction-boundaries.md:545, :810 |
| FND-003 | medium | The one content check the QSL-395 ruling keeps (recompute the domain-package document digest and compare it with the selection, refusing `content-mismatch`) still refuses `stale_dependency`/`byte-digest-mismatch`. `content-mismatch` went to node-identity compares instead. | spec/functional/FR-056-admit-domain-package-model-declarations.md:116; ADR-013:153; ADR-016:230 |
| FND-004 | medium | `content-mismatch` is not a `stale_dependency` cause in QSpec `native-diagnostics.md` on origin/main, and no QSpec FR or STD ticket is cited. FR-116 Status admits the gap. FR-122, ADR-014, ADR-017, TC-515 and TC-517 state it as a catalog cause, and ADR-013 O-17 forbids inventing one. | spec/functional/FR-122-replay-a-state-clause-counterexample.md:130, :134; ADR-014:457; ADR-017:266, :300 |
| FND-005 | medium | FR-087-AC-14 still refuses two selections of one identity that "differ in version or `package_id`". FR-099 and ADR-015 step 2 now refuse a diamond only when the digest differs. | spec/functional/FR-087-typestate-and-cross-package-node-key.md:604 |
| FND-006 | medium | FR-110 says the emitted lock row "binds the definition bytes", and ADR-011 says the edition row carries "the digest that file records". Both rely on the stored rule-document digests in `complete-value-lock.json`, which the ruling deletes. Neither says where the `definition_selections` digest comes from once they are gone. | spec/functional/FR-110-resolve-header-profile-selections-at-e3.md:78-80; ADR-011:502 |
| FND-007 | medium | FR-079 and TC-204 are deleted, but `tests/it/lowering_registry_isolation.rs:146` still carries `#[trace("TC-204", "FR-079-AC-2")]`, and its module doc cites FR-079 and TC-203. The trace now names deleted artifacts. | tests/it/lowering_registry_isolation.rs:2, :146 |
| FND-008 | low | The catalog-revision sweep is incomplete. `1-draft.N` claims remain in FR-096, FR-062, FR-093, FR-092, FR-026, TC-430, five spec.md rows and ADR-014. The amended ADR-013 O-17 says code claims no catalog revision. | spec/functional/FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md:128; FR-062:539; FR-093:125; FR-092:120; FR-026:124; spec/spec.md:391, :400, :425, :435, :508; ADR-014:70 |
| FND-009 | low | FR-099 Inputs and ADR-015 still require a per-library `version` in the dependency input. With `revision-mismatch` deleted, no rule compares it; it is refused only when empty. Either delete it or state what reads it. | spec/functional/FR-099-compile-against-supplied-libraries.md:43-45; ADR-015:66-67 |
| FND-010 | low | FR-110-AC-5's id is reused for the opposite behaviour: it was a `byte-digest-mismatch` refusal and is now a successful compile. The existing test tagged FR-110-AC-5 asserts the old refusal. | spec/functional/FR-110-resolve-header-profile-selections-at-e3.md:141 |

## New findings (disposition pass 1)

Disposition pass 1 at 8bd562a038838f9564e903e4262a77e7477ee601. Scope: the
PR diff against origin/main. New findings are raised only on lines the diff
changed, or on references the diff's deletions left dangling.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-011 | medium | The PR deletes FR-061, TC-158, FR-069-AC-2, FR-070-AC-8, FR-071-AC-4, FR-071-AC-8, TC-188 and TC-445, but code still traces to them and `make ci` still runs the FR-061 check. Round 3 reverted all code (spec only), and no surviving FR Status or tests.md row records the code removal, unlike FR-042's SHA256SUMS note. Fix: record each removal as remaining work in a surviving artifact or a named ticket, or delete the code here. | qsl-replay/src/proof_result.rs:283, :466-469; qsl-replay/src/request.rs:197, :466, :1064-1068, :1113-1120; qsl-replay/src/witness.rs:1241-1246; tools/arch-lint/duplicate_revisions.rs:2, :148-283; tools/arch-lint/main.rs:2-7, :317; tools/arch-lint/graph.rs:44, :355-357; Makefile:201, :282-285 |
| FND-012 | medium | The `BackendDescriptor` shape disagrees. ADR-012 §7.1 and its Descriptor row now drop `tool`, so a descriptor is the identity plus the advertised pairs. FR-075 Inputs cites ADR-012 §7.1 for a descriptor that carries `tool`, and FR-075-AC-7, FR-057 and ADR-013 O-19/C-28 compare `tool` for descriptor equality. Two implementers would read the equal-descriptor rule differently. | spec/decisions/ADR-012-semantic-family-extension-contracts.md:673, :1017; spec/functional/FR-075-compute-candidates-from-registered-backends.md:42-44, :225; spec/functional/FR-057-admit-shared-capability-kinds.md:239; spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md:718, :967 |
| FND-013 | low | ADR-013 C-27 now says the `backend` member is the identity alone, but its evidence column still cites the `backend_digest_in_any_other_fr201_domain_refuses` tests as the digest-domain half of FR-070-AC-6, for a backend digest that no longer exists. | spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md:966 |

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 564d2b74 |
| FND-002 | fixed | 564d2b74 |
| FND-003 | fixed | 564d2b74 |
| FND-004 | fixed | 564d2b74 (citations); `content-mismatch` is now on QSpec main through quire-specification#174 (merged) |
| FND-005 | fixed | 564d2b74 |
| FND-006 | fixed | 564d2b74 |
| FND-007 | still-open | 564d2b74 dropped the dead traces, but b052250b reverted all code to main (spec only). `tests/it/lowering_registry_isolation.rs:2,146` still traces FR-079 and TC-204, and nothing on the branch records the removal now that ADR-011's FR-079 retirement line is deleted |
| FND-008 | fixed | 564d2b74 |
| FND-009 | fixed | 564d2b74 |
| FND-010 | fixed | b052250b |

Round 2, reviewed at 08500770878eec974bdf6f15801d969df7b5de7f (`git diff 8bd562a0...08500770`).

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-007 | fixed | 08500770 |
| FND-011 | fixed | 08500770 |
| FND-012 | fixed | 08500770 |
| FND-013 | fixed | 08500770 |

## New findings (disposition pass 2)

Reviewed at 08500770, lines `git diff 8bd562a0...08500770` changed only.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-014 | low | TC-447 still says its fixtures are "matching TC-282's own", but the rewrite makes `A2` differ from `A1` by an extra advertised pair, (`value-validity`, `unbounded`). QSpec TC-282's `A2` advertises the same single pair as `A1` and differs only in one option, which QSL's `{id, advertises}` descriptor has no member for. State that TC-447 departs from TC-282 here and why, or drop "matching TC-282's own". | spec/test-cases/TC-447-duplicate-backend-identity-matches-qspec-tc282.md:23-27 |
