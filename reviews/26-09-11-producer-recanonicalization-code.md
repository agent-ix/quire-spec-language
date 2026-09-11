---
id: SR-381
title: "Code review of producer recanonicalization refusal"
type: SpecReview
analysis: code-review
scope: "tests/native_protocol_emission.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-042
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-121
    type: references
---

## Summary

Code/Rust review examines the test-only follow-up to the existing digest-domain
controls. An authored producer configuration and its literal JCS spelling denote
the same JSON object but have different bytes and exact-byte references. Each
matched reference/byte pair admits; crossing either pair refuses during native
dependency intake. No production code, wire contract or public cause changes.

## Verdict

**PASS** for this test-only increment: focused boundary/mutation checks, both
full suites, both strict Clippy lanes, formatting and scoped validation pass.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings in the reviewed test-only increment. | - |

## Evidence and limits

The public test uses the real composed proof and native admission path, retaining
one source identity and all non-digest selectors across its four cells. The
negative controls explicitly establish that only the configuration dependency
has mismatched bytes. Invalid(Seal), zero visited sources/models and no source
locus identify early native metadata intake, not a later reader/source refusal.
This belongs in native_protocol_emission rather than the reader test suite:
native::admit is the boundary exercised, and a failed admission exposes no
package for emission.

The normal focused test passes. Temporarily removing only the dependency digest
comparison in native/metadata.rs makes the same test fail: authored selection
over recanonicalized bytes is admitted, yielding no error instead of
Invalid(Seal). The production guard was restored byte-for-byte, confirmed by an
empty production-file diff. This is executed mutation evidence, not an inferred
claim that any seal error would detect this regression.

The JCS bytes are a literal fixture for a small object of ASCII keys and an
integer. No canonicalization implementation or second semantic frontend was
introduced. Tests use ordinary Rust assertions and existing fixture APIs; there
are no mock seams, unsafe additions, new dependencies or production panics.

The change was reviewed against 1c96fc5, the already merged digest-domain
increment. Full minimal/all-feature suites pass 609/625 tests respectively,
with zero failures and four inherited ignored in each; both strict Clippy lanes
pass with warnings denied. Formatting and diff checks pass. Pinned Quire with
both selected process/ISO modules validates 436/436 specification/review
documents grammar-clean with zero grammar findings. No applicable assurance
profile or cargo-deny configuration was found, and hosted workflows are unchanged.

Tracking tags bind the public control to TC-121 and FR-042-AC-1/AC-3. This control
does not establish actual B consumer acceptance or the complete artifact
campaign. No plan bundle covers this stage, and issue #66 retains issue-based
planning; the gap-analysis plan-completion gate is not claimed. Remaining work:
#40.
