---
id: SR-916
title: "QSL-345 gap analysis of PR 545, items 1 and 2 (FR-121-AC-14, FR-121-AC-15, TC-516 steps 14-15)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@17895e2ca8666075a7b8d9ec9884fcc1e089f48a; spec/functional/FR-121-locate-a-function-call-site-through-the-replay-facade.md (AC-14, AC-15, Outputs, re-export paragraph); spec/test-cases/TC-516-locate-a-function-call-site.md (steps 14-15); qsl-replay/src/call_site.rs; qsl-replay/src/identity.rs; qsl-replay/src/lib.rs. AC-12/AC-13 and TC-516 steps 12-13 excluded (item 4 leaves this PR)."
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-121
    type: reviews
---
## Summary

Ticket: QSL-345 (PR 1 of 2). PR: quire-spec-language#545 at 17895e2c.

- FR-121-AC-14 / TC-516 step 14 is covered by
  `call_site_returns_the_checked_package_bytes_its_package_id_names`, tagged
  `#[trace("TC-516", "FR-121-AC-14")]`. The test compares the returned `package`
  with a fresh compile's `emitted.bytes()`. It parses the bytes and checks
  `contract_version`. It RFC 8785-digests `identity_preimage` with
  `quire_canonical::sha256` and mints the digest under `PackageSemanticV2`, and
  checks that this equals `package_id`. It also checks the bytes' own
  `package_id.digest` member. Every assertion fails on wrong bytes or a wrong
  digest. Binding correct.
- FR-121-AC-15 / TC-516 step 15 is covered by
  `a_declared_domain_is_built_through_the_facade_alone` on `p`'s parameter `x`
  (the step 1 unit), with `[0, 9]` and the inverted range refused as
  `InvertedIntegerRange`. Binding correct. Its root-path oracle is weak: see
  SR-915 FND-001, not repeated here.
- The ticket's items 1 and 2 map onto these ACs. The Outputs `package` member and the
  re-export list are implemented as specified.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

AC-14 and AC-15 are traced and tested. There are no gaps beyond SR-915 FND-001.
When item 4 is removed, either keep AC-14/AC-15 and TC-516 steps 14/15 under their
current numbers (leaving 12-13 unused), or renumber them and update both trace tags
in call_site.rs and the TC-516 scope line at the same time.
