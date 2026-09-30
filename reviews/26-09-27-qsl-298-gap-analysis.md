---
id: SR-762
title: "QSL-298 gap analysis of PR 503 (FR-113; TC-511, TC-512)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; spec/functional/FR-113-resolve-scoped-anchors-in-nested-control-scopes.md; spec/test-cases/TC-511-s3-resolves-scoped-anchors-in-nested-scopes.md; spec/test-cases/TC-512-s3-refuses-ambiguous-and-shadowing-names.md; qsl-semantics/src/check/protocol_clause.rs; qsl-forms/src/protocol_clause.rs; qsl-semantics/src/check/mod.rs; qsl-semantics/src/check/assemble.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-113
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-511
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-512
    type: reviews
---
## Summary

Ticket: QSL-298. PR: quire-spec-language#503. There is no plan
bundle for QSL-298. This analysis checks each FR-113 acceptance criterion
and TC-511/TC-512 against the tests and the code. The team-leader decision
on QSL-298 moves binder no-shadowing (FR-113-AC-5, and the shadowing half of
AC-6) and FR-114's check half to QSL-306. Those parts are out of scope here.

Coverage by criterion (the mutations are listed in SR-761):

- AC-1: built. The test asserts only that no refusal is raised. The
  resolved identities are never checked (partial).
- AC-2: built correctly. The test is not an oracle: outermost-first
  resolution passes it.
- AC-3: built and tested for all three cases. The empty scope is asserted
  only for the `Absent` case.
- AC-4: built and tested (both declarations plus the anchor, in source
  order). The swap step and the loci order are not tested.
- AC-5: not built. Deferred to QSL-306.
- AC-6: the non-stopping and repeat-determinism parts are tested with a
  missing + wrong-kind pair. The ordering assertion is vacuous. The
  shadowing half is QSL-306.
- AC-7: built and tested for all six sites and for the channel
  mismatch/match. The `of Recovered` (an `event`) variant is replaced by
  `of Applied`.

Trace. The 7 tests carry `#[trace("TC-511"|"TC-512", "FR-113-AC-n")]`.
Underspecified code: none. Every new branch maps to an FR-113 clause. The
exception is the removed `UnimplementedProtocol` refusal: no requirement
permits a protocol to compile with unchecked content (SR-761 FND-001).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-113 Description says "A protocol with any refusal emits no checked node". This PR goes further and lets a protocol through with every non-anchor part unchecked, then drops it from the emitted package. No requirement permits this, and it undoes the QSL-297 guard (SR-753 FND-002). See SR-761 FND-001 for the probe. | qsl-semantics/src/check/assemble.rs:474; qsl-semantics/src/check/mod.rs:1036-1049 |
| FND-002 | medium | FR-113-AC-1 ("each recorded by the node's identity") and FR-113-AC-2 (inner resolves to inner, `Main::Applied` to outer) have no test that asserts which node an anchor resolved to. Both are claimed as covered, but their oracle is only "no refusal". | qsl-semantics/src/check/protocol_clause.rs:366-393 |
| FND-003 | medium | FR-113-AC-6 is tagged as verified, but its shadowing half is not built (QSL-306). The substitute test's ordering check does not fail when the sort is removed. The matrix must not count AC-6 as covered until QSL-306 lands. | qsl-semantics/src/check/protocol_clause.rs:489-508 |
| FND-004 | low | Parts of TC-511/TC-512's procedures are missing. AC-3 does not assert the empty scope for two of its three cases. AC-4 leaves out the swap step. AC-7 has no case whose actual kind is `event`. Channels `C`/`D` are not declared. | qsl-semantics/src/check/protocol_clause.rs:399-417; qsl-semantics/src/check/protocol_clause.rs:443-478; qsl-semantics/src/check/protocol_clause.rs:517-670 |

## Verdict

Request changes. FND-001 blocks merge. FND-002 and FND-003 need the tests
tightened in this PR. FND-004 is low. AC-5 and the shadowing half of AC-6
are correctly deferred to QSL-306.
