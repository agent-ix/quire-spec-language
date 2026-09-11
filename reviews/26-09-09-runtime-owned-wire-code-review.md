---
id: SR-273
title: "Code and Rust review of owned runtime wire decoding"
type: SpecReview
analysis: code-review
scope: "3c6a0e6 against 8866239; runtime input/wire/reading, digest, runtime_reading tests"
review_set: subset
---
## Summary

Author PR-readiness review using the actual agent-skills/code-review,
rust-review and rust-style skills, with repository trace attributes and local
serial gates. PR #16's decoder-ownership findings 3/8 and digest finding 5 are
resolved in this slice; its standalone schema follow-up remains under #27.

## Verdict

**PASS** — no remaining findings in Task-035's implementation scope. This is
author review; the new PR still needs its first independent review.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings | - |

## Checks

Closed wire records own validated scalar wrappers; public Deserialize entries
own their object guard. Conversion constructs every public field and matches
all value variants exhaustively. Ordinary vectors therefore inherit the nested
type's refusal rules. Required result presence is represented separately from
optional result content; the empty absent variant refuses extra fields.
Serialize keeps the existing public field order and names. Bare-hex decoding
uses the byte codec, while general FromStr still requires sha256:.

The three new tests reach real public APIs, with eleven record types and all ten
value variants, positive objects, field-ordered arrays, malformed/missing fields,
raw duplicate keys, and invalid digest/result scalars. Two tests failed before
the guard relocation. No conditional assertions, internal mocks or new skips.
Existing byte-selection, typed stage precedence, limits/retries and execution
tests pass. The scalar decoder's single text error is a Serde explanation, not
a public message discriminant or a value later parsed for classification.

Panic/conversion inspection: digest slicing follows exact 64-byte ASCII checks;
no unchecked casts, new recursion, unsafe code, discarded errors or catch-all
operator conversions. No async, shared mutable state, callbacks, I/O, dependency
or license changes. The CI diff is empty; both feature lanes and dispatch-only
triggers remain. Reverse-gap discovery found no unowned changed behavior or
source/test stub. Full native profile conformance is still #30.

## Local verification

At implementation 3c6a0e6, with locked/offline Cargo, target cache, nice 10,
one job and one test thread:

| Gate | Result |
| --- | --- |
| Strict Clippy, all targets, all/minimal features | Both pass |
| Tests, all features | 353 ordinary + 3 compile-fail doctests pass; 4 existing ignored |
| Tests, minimal features | 337 ordinary + 3 compile-fail doctests pass; 4 existing ignored |
| Focused runtime reader | 8 pass |
| Cached minimal binaries/config_version_fixtures build | Pass |
| Warnings-denied all-feature rustdoc; fmt | Pass |
| Rust fixture-audit self-test/model-bytes; CLI parse example | Pass; historical producer bytes only |

No deny.toml is installed. Logs are /tmp/agent-a-owned-wire-*. Hosted CI was
not dispatched. SR-265–272 carry the required QUOIN all-set review; #28 and the
advisor's version-detection failure limit assurance claims, not these run results.
