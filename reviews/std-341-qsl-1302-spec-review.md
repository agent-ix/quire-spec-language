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

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-005 | medium | QSpec FR-306 at the paired head (09371106) adds two cache rules: every `tool-unavailable` result is never stored, and for a backend that fronts an external tool the cache is read only after the item's FR-290 probe passes, so a hit is returned only for the tool that would run. ADR-029 CA-4 (store and read) and CA-5 (never stored) state neither, and FR-292 says the driver's `quire-cache` implements FR-306 "(ADR-029 CA-1 to CA-4)". A driver built from the ADR reads the cache before the probe and returns a stored result after a mid-invocation tool replacement, the stale hit STD-341 refuses. Fix: CA-4 reads only after a passing probe for an external-tool backend; CA-5 adds `tool-unavailable` results. | spec/decisions/ADR-029-lifecycle-cli-provider-plugin-cache-codegen-boundaries.md:745-757; spec/functional/FR-292-key-cached-results-by-content-identity.md:38 | wrong-requirement |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-006 | medium | ADR-029 CA-2 now keys a pure in-process engine on "the crate that implements a pure in-process engine, at its version in the producer's build", so an engine change moves the key only when the crate's declared version changes. QSL engine crates stay at `0.1.0` across main builds whose code differs (`qsl-analyze/Cargo.toml` is `0.1.0`), so two such builds share every `analyze` key and the later build returns the earlier build's result. Same root cause as QSpec SR-16207 FND-008. Fix: change CA-2 together with QSpec FR-331 "Backend tool" (bind the engine's code identity, or require a version that changes with every engine code change). | spec/decisions/ADR-029-lifecycle-cli-provider-plugin-cache-codegen-boundaries.md:722-730 | missing-requirement |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | At eca1e752 TC-772 step 4 converts "two manifests that differ only in the QSpec FR-331 descriptor member `tool.version`" and expects "the same `BackendId` and different `ManifestDigest`s, each the QSpec FR-331 `ManifestDigest` of its descriptor". |
| FND-002 | fixed | FR-292 restates the key as FR-306's closed object `{operation, package_id, request, backend, manifest, target}` with the provider's `BackendId` and its QSpec FR-331 `ManifestDigest` record. |
| FND-003 | fixed | FR-288 Behavior: "The conversion shall set the descriptor's `ManifestDigest` to the QSpec FR-331 `ManifestDigest` of the manifest's descriptor: SHA-256 in the QSpec FR-201 domain `quire.tool-manifest.jcs/v1` over the RFC 8785 bytes of that complete descriptor, `tool` included." |
| FND-004 | fixed | ADR-029 CA-2 names FR-306's closed object `{operation, package_id, request, backend, manifest, target}`, "and no other member", with one bullet per member. |
| FND-005 | fixed | At 45cc93f67 ADR-029 CA-4: "For an item routed to a backend that fronts an external tool, the cache is read only after that item's QSpec FR-290 tool probe passes"; CA-5: "A `tool-unavailable` result, whose outcome depends on the host's tool and not on the key, is not stored." FR-292 cites ADR-029 CA-1 to CA-5 with both rules. |
| FND-006 | fixed | At 45e68bc44 ADR-029 CA-2: a pure engine's `tool` carries "`build`, the `quire.build-inputs/v1` digest of the provider code's build inputs that the producer's build computes. A tool upgrade, an engine release, and an engine code change under an unchanged crate version therefore each change the `ManifestDigest` and the key"; FR-288 and FR-292 say the same. |
