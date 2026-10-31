---
id: SR-1931002
title: "gap analysis of PR #1931 (QSL-1337): rank node keys by structure instead of comparing deep keys in check"
type: SpecReview
analysis: gap-analysis
date: 2026-10-31
scope: "agent-ix/quire-spec-language; PR #1931 git diff origin/main...a24b23b59 (qsl-semantics check/key_order.rs, lowering, family, claims, mod); FR-092-AC-7, FR-258-AC-2, FR-062-AC-3"
review_set: subset
relationships:
- target: ix://agent-ix/quire-spec-language/FR-092
  type: reviews
---
# SR-1931002: gap analysis of PR #1931 (QSL-1337)

## Summary

Ticket: QSL-1337. A manual criterion-to-test check over the diff. Plan completion: not assessed. Tests were read, not run. The author reports 707 qsl-semantics and 431 qsl-package lib tests passing.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-092-AC-7 | spec/functional/FR-092-key-type-parameter-and-declared-nodes.md:153 | examined |
| FR-258-AC-2 | spec/functional/FR-258-check-and-lower-expressions-at-any-depth.md:81 | context_only |
| FR-062-AC-3 | spec/functional/FR-062-implement-checked-family-contract.md:369 | examined |
| key_order tests | qsl-semantics/src/check/key_order.rs | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

**Clean for major gaps.** The rewritten TC-413/TC-726 test still asserts what FR-092-AC-7 claims: the same type keys on a 512 KiB stack as on an 8 MiB stack, now as an ordered, injective shape list. key_order.rs has an oracle test against `NodeKey::cmp`: 400 generated keys over two packages, prefix texts, two ordinals and nested earlier keys, plus separately rebuilt equal keys. It also has a stability test and a 50,000-deep chain test. No production behavior is left without a test. The new key_order unit tests carry no `#[trace]` tag. That is below the major-only bar, because they test an internal sort and not an AC.
