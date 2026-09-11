---
id: SR-332
title: "Gap analysis of the delivered compiled protocol reader against FR-042 and TC-121"
type: SpecReview
analysis: gap-analysis
scope: "FR-042; TC-121; TM-003 (spec/model-linking/tests.md); src/protocol_artifact/; tests/protocol_artifact.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-042
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TM-003
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-121
    type: references
---

## Summary

Gap analysis of the FR-042 reader slice at 51506ed. `quire coverage --scope`
reports **366/376 rows backed, zero status lies**, and all six unbacked rows
predate this branch. TM-003 is honest: every FR-042 row stays 🚧 Planned, the new
text says the manually authored transport fixtures establish neither native
family admission nor source-to-consumer emission, and no tag claims FR-042-AC-1
or AC-10. The delivered reader/encoder is real and gated. The full ticket is
far from closed: two of ten criteria have no test at all, two more carry tags
whose tests do not reach the criterion, and the choreography half of the
implementation is unexercised.

## Verdict

**FAIL** — matrix rows for TC-121 have no backing tagged test for AC-1 and
AC-10, and FND-002 is high. This is the expected state for a planned row; it
blocks TC-121 closure, not the reader merge.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | FR-042-AC-1 (production emission from admitted family evidence) and FR-042-AC-10 (real source-to-`quire-protocol` handoff) are named by no `#[trace]` tag and no test; TC-121 procedure steps 1 and 10 are unexecuted | tests/protocol_artifact.rs:25; spec/test-cases/TC-121-publish-compiled-protocol-artifacts.md:31 | missing-requirement |
| FND-002 | high | AC-5 and AC-6 are tagged by tests that never construct a choice, repeat, await, channel, delivery, attempt, effect, registration, retry, commit or recovery record; the tagged assertions cover causal-edge omission/duplication, a closure binding and a subject owner only | tests/protocol_artifact.rs:580; tests/protocol_artifact.rs:829 | correct-requirement-no-evidence |
| FND-003 | medium | AC-9 requires zero, exact and one-short per dimension; exact/one-short exist for 8 of 13, while `ContentBytes`, `Entries`, `References`, `ByteWork` and `Depth` are tested at zero only | tests/protocol_artifact.rs:631-646 | correct-requirement-no-evidence |
| FND-004 | medium | Reverse gap: `Unsupported::ProducerCorrespondence`, `Unsupported::Export` for relationship/population/component/endpoint and `Type::Reference`, and the hard-coded `edition != "1-draft"` refusal are shipped behaviors that no requirement or contract clause names as this version's scope | src/protocol_artifact/models.rs:125; src/protocol_artifact/models.rs:577; src/protocol_artifact/intake.rs:480 | missing-requirement |
| FND-005 | low | FR-042 has no plan bundle, so the skill's plan-completion step has no target; this follows the AGENTS.md directive to keep prototype bookkeeping minimal and is recorded, not proposed for change | plan/; AGENTS.md | missing-requirement |

## Coverage

`quire coverage --scope /home/peter/dev/worktrees/quire-language-artifact-contract --json`
(quire 0.31.0, engine 0.46.0): 366/376 backed, 258 criteria, 0 status lies, 0
untracked tests. The six unbacked rows are TC-115, TC-010, FR-017-AC-2 and
FR-036-AC-5/6/8 — all pre-existing FR-036/NFR-005 work. The three unmatched tags
are the pre-existing `IT-004` tags in `tests/fixture_audit.rs`. **No new
regression is attributable to this branch.** TC-117 and TC-121 rows are backed by
`tests/protocol_number.rs` (9 tests, FR-038-AC-1..5) and
`tests/protocol_artifact.rs` (20 tests, FR-042-AC-2..AC-9).

## Delivered versus full acceptance

Delivered and verified: bounded parser-free `read`, canonical `encode_candidate`,
closed wire records, external seal checking, independent source/dependency/
definition/model selection against actual `NativeModel` exports and registered
immutable definition bytes, typed handle/scope/type/profile/binding checks, the
structural causal-edge expansion for sequence/parallel/check, thirteen limit
dimensions with clamping and fresh retry, and byte-exact canonical comparison.

Not delivered, and correctly not claimed: native family admission, production
emission, the B handoff, choice coverage and non-overlap, decision visibility,
guarded definedness at the choreography level, repeat progress, runtime
conformance. No positive wire fixture substitutes for any of these, and a
`#[trace]` tag on a partial control does not establish its criterion. Under #35,
#36, #39 and #40 the full FR-042/TC-121 acceptance stays open.
