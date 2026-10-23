---
id: SR-16208
title: "spec-review of ADR-029 CA-2 and FR-288-AC-4 naming the FR-331 tool.version member (QSL PR 1302)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@b6198650; spec/decisions/ADR-029-lifecycle-cli-provider-plugin-cache-codegen-boundaries.md; spec/functional/FR-288-build-the-registry-from-provider-manifests.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-029
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-288
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-772
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-292
    type: references
---

# Spec review of ADR-029 CA-2 and FR-288-AC-4 naming tool.version (QSL PR 1302)

## Summary

Ticket: STD-341. One spec-review pass over `git diff origin/main...b6198650`,
paired with QSpec PR 391 (head d6b07993). Oracles: QSpec FR-331 and FR-306
at that head, QSL ADR-013 (`ManifestDigest` kept on the `qsl-route`
`BackendDescriptor`), QSL FR-292 and TC-772.

Gates: `QSPEC_DIR=<QSpec PR 391 worktree> make check-spec-validation` exits
0 (grammar warnings only, none in the changed files). origin/main
(71a080048) is an ancestor of the head. No other open QSL PR touches
ADR-029, FR-288, FR-292 or TC-772. The PR adds no ids.

Examined, clean:

- ADR-029 CA-2 now says the `ManifestDigest` is over the provider's complete
  FR-331 descriptor, the tool version is `tool.version`, and a tool-only
  upgrade changes the digest and the key and leaves the `BackendId`. That
  matches QSpec FR-331 and FR-306 at the paired head and resolves the
  STD-341 contradiction with FR-288-AC-4.
- FR-288-AC-4 names the member and adds the digest check; it can fail.
- FR-288 body: the descriptor holds no other identity of the tool, and
  `tool` reaches it only through the digest, consistent with ADR-013.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | FR-288-AC-4 now requires "different manifest digests", but TC-772, the case it names, was not updated: step 4 still converts "two manifests that differ only in a tool version member" and its expected result is only "the two manifests give the same `BackendId`". The new half of the AC has no test-case expectation, so a test written from TC-772 passes with equal digests. Fix: in TC-772 step 4 name `tool.version` and expect "the same `BackendId` and different `ManifestDigest`s". | spec/test-cases/TC-772-one-manifest-conversion-serves-compile-time-providers-and-plugins.md:22,36; spec/functional/FR-288-build-the-registry-from-provider-manifests.md:85 | correct-requirement-no-evidence |
| FND-002 | medium | FR-292 still restates QSpec FR-306's key as "the operation, the subject's `package_id`, the canonical request, the provider's backend identity and, for `generate`, the target", without the `manifest` member. After this pair, FR-292 contradicts ADR-029 CA-2 and QSpec FR-306 on the key preimage, which is the same kind of contradiction STD-341 was filed for. Fix: add "the provider's `ManifestDigest`" to FR-292's restatement, or replace the list with a reference to FR-306's closed object. | spec/functional/FR-292-key-cached-results-by-content-identity.md:30-36 | wrong-requirement |
| FND-003 | low | FR-288 says `tool` reaches the descriptor "only through the manifest digest", and AC-4 checks that digest, but no FR-288 sentence states that the conversion computes the descriptor's `ManifestDigest` or over what (only ADR-029 CA-2 says layer R computes it over the complete FR-331 descriptor in `quire.tool-manifest.jcs/v1`). The existing tests build `ManifestDigest::from_digest` from arbitrary bytes, so the conversion's computation is unstated in the requirement AC-4 tests. Fix: add a Behavior bullet: the conversion sets the descriptor's `ManifestDigest` to the QSpec FR-331 `ManifestDigest` of the manifest's descriptor. | spec/functional/FR-288-build-the-registry-from-provider-manifests.md:41-44,55-60 | missing-requirement |
| FND-004 | low | ADR-029 CA-2 names only the `manifest` member of the key; it does not give the other member names of QSpec FR-306's closed object `{operation, package_id, request, backend, manifest, target}` or that no other member enters it, so the ADR and FR-306 can drift on member spelling. Fix: cite FR-306's closed object by its member names. | spec/decisions/ADR-029-lifecycle-cli-provider-plugin-cache-codegen-boundaries.md:707-726 | wrong-requirement |

## Verdict

Not mergeable yet: FND-001 leaves the new AC-4 clause without a test-case
expectation, and FND-002 leaves FR-292 contradicting the new key. Both are
small edits inside this PR. FND-003 and FND-004 are low. The PR should merge
after, or together with, QSpec PR 391, whose open findings (SR-16207) may
change the FR-331 wording these sections cite.
