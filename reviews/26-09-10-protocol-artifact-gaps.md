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

Gap analysis of the FR-042 reader slice rechecked at 23a5892. `quire coverage
--scope` still reports **366/376 rows backed, zero status lies**, with the same
six unbacked rows that predate this branch. TM-003 remains honest: every FR-042
row stays 🚧 Planned and no tag claims FR-042-AC-1 or AC-10. The corrections
substantially closed the AC-5 evidence gap and the reverse-gap finding, and the
suite grew from 20 to 24 tests. The full ticket stays far from closed: two of ten
criteria still have no test at all.

## Verdict

**FAIL** — FR-042-AC-1 and FR-042-AC-10 have no backing tagged test, and FND-001
is high. This is the expected state for a 🚧 Planned row; it blocks TC-121
closure, not the reader merge.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | FR-042-AC-1 (production emission from admitted family evidence) and FR-042-AC-10 (real source-to-`quire-protocol` handoff) are still named by no `#[trace]` tag and no test; TC-121 procedure steps 1 and 10 are unexecuted. Every fixture is a hand-authored wire package, which by the contract's own wording establishes neither family admission nor source-to-consumer emission | tests/protocol_artifact.rs:25; spec/test-cases/TC-121-publish-compiled-protocol-artifacts.md:31 | missing-requirement |
| FND-002 | medium | AC-6 is tagged but materially unreached: no fixture constructs a channel, send, receive or delivery, so "one send / two deliveries / one effect remains 1/2/1" never runs, and empty `compensations` leaves effect-dependent registration, retries and full-versus-partial recovery untested. The commit half is now covered. AC-4's "all eight query forms" and graph-export operand authority (`Query`, `Size`, `Contains`, `Parent`, `Reaches`) are likewise tagged but never constructed | tests/protocol_artifact.rs:1115; tests/support/protocol_artifact/mod.rs:1343-1344 | correct-requirement-no-evidence |
| FND-003 | medium | AC-9 requires zero, exact and one-short per dimension; unchanged at exact/one-short for 8 of 13, with `ContentBytes`, `Entries`, `References`, `ByteWork` and `Depth` still tested at zero only | tests/protocol_artifact.rs:655-672; tests/protocol_artifact.rs:704-717 | correct-requirement-no-evidence |
| FND-004 | low | FR-042 still has no plan bundle, so the skill's plan-completion step has no target. This follows the AGENTS.md directive to keep prototype bookkeeping minimal and is recorded, not proposed for change | plan/; AGENTS.md | missing-requirement |

## Resolved since the original review

**AC-5 evidence (was part of a high finding).** `Fixture::controlled()` now builds
one protocol declaration with a role, 14 controls and 33 independently listed
causal edges covering all eight control operations — sequence, owned labeled
choice, parallel/join, bounded repeat at maximum zero, await with an inclusive
interval, event, check and commit — plus `Event::Attempt`, `Event::Effect` and
domain `Event::Event`, and `Attempt`/`Effect`/`Commit`/`Progress`/`Clock` binding
kinds. `Fixture::families()` adds five declarations across predicate, state and
temporal bodies with eleven value operations including non-Boolean numbers, text
and a supplementary-plane scalar. The two new tests carry ten strong one-axis
typed refusals (duplicate choice label, foreign case body, negative repeat
maximum, wrong-kind await anchor, missing clock binding, cross-wired effect
attempt, foreign commit instance, tampered `RepeatProgress` maximum) plus the
three await-authority negatives that TC-121 step 5 now names.

**Reverse gap (was medium).** `Unsupported::ProducerCorrespondence`,
`Unsupported::Export` for relationship/population/component/endpoint and
`Type::Reference`, and the `edition != "1-draft"` refusal are now named by the
contract's "Bounded read and emission" section as the current native adapter's
explicit supported boundary, with the refusals stated to keep full FR-042
emission and handoff acceptance open. They are shipped behaviors with an owning
clause rather than undocumented scope.

## Coverage

`quire coverage --scope /home/peter/dev/worktrees/quire-language-artifact-contract --json`
(quire 0.31.0, cli 4f6ed024, engine 0.46.0@ca7362d4): **366/376 backed, 258
criteria, 0 status lies, 6 unbacked rows, 3 unmatched tags, 2 no-symbol rows.**
The six unbacked rows are TC-115, TC-010, FR-017-AC-2 and FR-036-AC-5/6/8 — all
pre-existing FR-036/NFR-005 work, two of them Manual/Inspection by declared type.
The three unmatched tags are the pre-existing `IT-004` tags in
`tests/fixture_audit.rs`. **No new regression is attributable to this branch, and
the rollup is unchanged from the original review.** TC-117 and TC-121 rows are
backed by `tests/protocol_number.rs` (9 tests, FR-038-AC-1..5) and
`tests/protocol_artifact.rs` (24 tests, FR-042-AC-2..AC-9). Semantic review was
not run — the optional extension was declined.

## Delivered versus full acceptance

**Delivered and verified.** Bounded parser-free `read` with published pass
precedence, canonical `encode_candidate`, closed wire records, external seal
checking, independent source/dependency/definition/model selection against actual
`NativeModel` exports and registered immutable definition bytes, typed
handle/scope/type/profile/binding checks, the exhaustive local-table kind/body
lookup, static await timeout association, the self-provenance marker with owner,
range and cycle checks retained, the structural causal-edge expansion, thirteen
limit dimensions with clamping and fresh retry, locus-free package-wide output
work, and byte-exact canonical comparison.

**Not delivered, and correctly not claimed.** Native family admission, production
emission, the `quire-protocol` B handoff, choice coverage and non-overlap,
decision visibility, guarded definedness at the choreography level, repeat
progress and runtime conformance. The new fixtures do not change this: exercising
`Choice` and `Repeat` records proves the reader's structural checks, not the
family-admission obligations the contract assigns to the compiler under FR-042
plus the accepted standard's FR-050–059. No wire fixture establishes native
compilation, and a `#[trace]` tag on a partial control does not establish its
criterion. Under #35, #36, #39 and #40 the **full FR-042/TC-121 acceptance stays
open**, with AC-1 and AC-10 wholly unexecuted and AC-4/AC-6/AC-9 partially
evidenced. Per the owner directive, that later assurance is tracked separately and
does not block unblocked prototype engineering.
