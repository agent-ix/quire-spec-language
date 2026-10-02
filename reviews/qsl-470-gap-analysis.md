---
id: SR-1204
title: "QSL-470 gap analysis of PR #592 (FR-110, FR-111 identity resolution)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@d1ae0de9c9a333984de53443350ddd214aec2387; PR #592 diff against origin/main; spec/functional/FR-110-resolve-header-profile-selections-at-e3.md; spec/functional/FR-111-link-a-complete-v1-definition-bundle.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-110
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-111
    type: reviews
---
## Summary

Ticket: QSL-470. PR: quire-spec-language#592.

Trace:
- FR-110-AC-2, AC-3 and AC-6 are covered by
  `a_header_profile_resolves_by_identity_against_the_root_row`
  (qsl-replay/src/spine.rs), with exact code, cause, alias, role and source
  order.
- FR-110-AC-5 is covered by `a_token_after_a_header_profile_identity_is_a_syntax_error`.
  It covers `version`, `digest` and an extra string, with stage `Source`, code
  `invalid_syntax` and the offset of the offending token.
- FR-110-AC-1 is partial, as the Status says: the layer-closure rows are
  remaining work. See FND-003.
- FR-111-AC-2 is covered by
  `roots_resolve_by_identity_and_an_unknown_identity_refuses_naming_its_index`
  (resolving under other bytes) and by
  `a_dependency_edge_to_an_absent_definition_refuses_missing_selection`.
- FR-111-AC-3 is covered by
  `one_identity_selected_under_two_authorities_refuses_naming_both`, for a root
  plus a reached definition and for two roots.
- FR-111-AC-5 is covered by `semantic_identity_binds_typed_definition_interpretation`:
  a change of bytes still changes the identity.
- FR-111-AC-7 is covered by `resolution_causes_match_the_complete_cause_catalog`
  and `conformance_fr111_resolution_causes_are_listed_by_qspec_native_diagnostics`.
  The conformance test ran against local QSpec: 12 pairs.
- Status text: the FR-110 Status matches the code (test names, the remaining
  layer work, the compiled-in lock bytes). FR-111's Status matches, but its
  Behavior identity paragraph does not (FND-001).
- "Nothing left compares a revision": true for header profiles, `link_bundle`,
  the editor, the sampler, the observation evidence and the temporal support
  table. It is not true for other definition and profile selection paths
  (FND-002).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The FR-111 Behavior text on the bundle identity no longer matches the code. The preimage is now each definition's `{authority, identity}` plus `digest` (the SHA-256 of its bytes), role, dependencies and capabilities. FR-111:87 lists "exact reference, role, dependencies, capabilities", with no byte digest, and FR-111:90 says "The label stays `/2`, so the identity of an existing bundle is unchanged". That is false now: every non-empty bundle's identity changed, because the reference shape changed and `digest` was added. Only the empty-definitions golden vector is the same. Fix: list the byte digest and the `{authority, identity}` reference, and delete the "unchanged" sentence. | spec/functional/FR-111-link-a-complete-v1-definition-bundle.md:85-90; qsl-semantics/src/library/bundle.rs:878-900 |
| FND-002 | medium | Revision comparisons remain in definition and profile selection paths that this PR does not touch and the rulings do not cover. `src/linking/composed/definitions.rs:472-481` resolves a composed definition by identity plus `revision`, and also checks its byte digest (`Cause::DefinitionDigest`). `src/temporal.rs:398,588` refuses `Dimension::ProfileRevision` when a trace's profile revision differs from the selected one. `qsl-semantics/src/model/observation/document.rs:174-175` matches documents on `revision_namespace` and `revision`. These may belong to other slices or to the IR-blocked `CheckedArtifactRef` move, so the team leader should place each one: either this PR or a named ticket. As it stands, the claim that nothing still compares a revision is false repo-wide. | src/linking/composed/definitions.rs:472-481; src/temporal.rs:398; src/temporal.rs:588; qsl-semantics/src/model/observation/document.rs:174-175 |
| FND-003 | low | Step 1 of `a_header_profile_resolves_by_identity_against_the_root_row` (tagged FR-110-AC-1) was weakened. It used to assert that the emitted root lock row equals `root.reference()`; it now asserts only `selected.len() == 1` over rows filtered by identity. `DefinitionLock` and its `reference()` are unchanged in this PR, so the full-row oracle still holds and still catches an emitter writing a wrong row. Restore it, or assert at least `authority` and `identity`. | qsl-replay/src/spine.rs:1381-1390 |
| FND-004 | medium | FR-111 never compares a selection's authority with the authority of the definition it resolves to. A single root, or a lone dependency edge, `{other-authority, X}` resolves to the catalog's `{agent-ix, X}`. `LinkedBundle::definitions` then holds the key `{other-authority, X}` whose value's `exact()` is `{agent-ix, X}`. `conflicting-authority` fires only between two selections. So the authority half of the new `DefinitionRef` is decorative unless a second selection exists. The spec is silent on this case. Decide it: either refuse a selection whose authority differs from the resolved definition's (`conflicting-authority`), or state that resolution ignores the selection's authority. Then add the test. | spec/functional/FR-111-link-a-complete-v1-definition-bundle.md:66-69; qsl-semantics/src/library/bundle.rs:307-309 |

## Verdict

Changes requested: FND-001, FND-002 and FND-004 are medium. FND-003 is low.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | high | The fix round deleted the temporal profile-revision refusal and the `Dimension::ProfileRevision` cases from the FR-043-AC-3 tests, but FR-043 still requires that refusal. FR-043:138 says "The evaluator SHALL refuse a trace whose asserted profile identity or revision differs", and FR-043-AC-3 says "a trace whose asserted profile identity, profile revision or clock binding name differs ... refuses ... and names that dimension". TC-122 step 2 (line 49) still lists the asserted profile revision. The code now admits such a trace. Amend FR-043's Behavior and AC-3, and TC-122, to identity and clock only. Also decide whether FR-043-AC-2's retained "revisions" premise stays. | spec/functional/FR-043-evaluate-bounded-native-temporal.md:138; spec/functional/FR-043-evaluate-bounded-native-temporal.md:259; spec/test-cases/TC-122-evaluate-bounded-native-temporal.md:49 |
| FND-006 | medium | One definition selection path still compares a revision, and it is not among the three sites FND-002 named or the ruling placed. Protocol-artifact intake recognizes a compiled artifact's definition only when `definition.identity == candidate.identity() && definition.revision.value == candidate.revision()`. This is the same identity-plus-revision match the round removed from `composed/definitions.rs`. Place it, either in this PR or in a named ticket, as FND-002 asked for each site. | src/protocol_artifact/intake.rs:453-455 |

## Dispositions

Round 1, reviewed at `cd999fc8badbaad0450286a92e2b90838091357b` (diff `d1ae0de9..cd999fc8`).

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 9d20823cf0eb43b49e0ee20c63c224cb9d13671d |
| FND-002 | fixed | 9d20823cf0eb43b49e0ee20c63c224cb9d13671d |
| FND-003 | fixed | 9d20823cf0eb43b49e0ee20c63c224cb9d13671d |
| FND-004 | fixed | 9d20823cf0eb43b49e0ee20c63c224cb9d13671d |
