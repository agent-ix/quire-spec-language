---
id: SR-933
title: "Integrity review of PR #550 (driver owns Kani run, replay and terminal record)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@6d60826c4fb5e9e1f7acebcaf349965cd4cdf43d; spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md, spec/decisions/ADR-012-semantic-family-extension-contracts.md, spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md, spec/decisions/ADR-016-state-model-finite-execution-mapping.md, spec/functional/FR-069-implement-typed-proof-result-envelope.md"
review_set: subset
---

## Summary

Ticket: QSL-354. Consistency of every edited sentence with the rest of each
ADR, and dangling references from deleted rows. One direct contradiction
remains between the new terminal mapping and older E9 / O-26 text, one
pin-equality statement survives, and several references are stale.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | ADR-011 E9 details say a limit refusal at E9 is "never `inconclusive`" and that an `Input` refusal or `Fault` "neither is `inconclusive`"; ADR-013 O-26 says an `InputRefusal` is "never `inconclusive`". The new T-13, O-16 and C-09 text settles every non-fault replay refusal (including a limit reached) `inconclusive` with cause `replay_refused`. Unqualified, the two statements contradict. Fix: qualify the E9/O-26 sentences as "yields no parity verdict" and point to C-09 for the terminal value. | ADR-011:274-281, ADR-013:917, ADR-011:1401, ADR-013:404, ADR-013:967 |
| FND-002 | medium | The #132 row in ADR-013 §9 still reads "O-23 and SEAM-3: pins checked equal to the lock", the pin-equality duty this PR deletes everywhere else. | ADR-013:1289 |
| FND-003 | low | ADR-016 PI-4 now reads "The gate for a pin-bump PR is `make ci` (ADR-013 O-23)", but O-23 no longer says anything about pin-bump PRs, so the citation supports nothing. Fix: drop the citation, or drop PI-4. | ADR-016:432, ADR-013:801-814 |
| FND-004 | low | FR-121 still says "`spine` is public only for `command`", which contradicts the amended ADR-011 qsl-replay row (public for `command` and the orchestrating driver). Outside the diff, but it restates the rule this PR changed. | spec/functional/FR-121-locate-a-function-call-site-through-the-replay-facade.md:54, ADR-011:891 |
| FND-005 | low | Several ADR-013 sites still describe C-09 as a one-input `KaniOutcomeKind` → `TerminalValue` map and the envelope as carrying only "the typed vacuous-proof cause": O-24 Owner, Implementing ticket and Public type, OQ-H, TK-05. They are not wrong but disagree with the two-input C-09 and the three `inconclusive` causes. | ADR-013:821-823, ADR-013:1207, ADR-013:1228 |
| FND-006 | low | T-13 cites ADR-013 C-10 for CG's adapter parsing the Kani run into IR's `KaniOutcome`, but C-10 is the transcript → `Witness` conversion (O-24 gives "Kani run → `KaniOutcome` (IR)"); and it cites O-24 and O-27 for "the replay request, the replay result and the terminal record", omitting O-26, which owns the request. | ADR-011:1401, ADR-013:825, ADR-013:968 |

## Verdict

IDs and rows are intact: no renumbering; Q209-7 and the O-23 heads paragraph
are deleted with no spec reference left; the #215 row, OBS-031, OBS-036 and
OBS-040 read consistently with §7.1; ADR-012 §5.3 has no remaining heads
drift-check mention; FR-069 now names CG. FND-001 must be fixed before merge.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | low | ADR-012 §8 still says 'Proof results come from IR's KaniOutcomeKind map', a one-input description of the proof-result map. C-09 is CG's and takes the Kani outcome and the E9 replay result. Same class as FND-005, missed in the review pass. Fix: 'Proof results come from CG's C-09 map over IR's KaniOutcomeKind and the E9 replay result (ADR-013 C-09).' | ADR-012:827-828 |

## Dispositions

Round 1, reviewed at a00ace78 (fix commit a00ace78 on top of 6d60826c).

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | a00ace78 |
| FND-002 | fixed | a00ace78 |
| FND-003 | fixed | a00ace78 |
| FND-004 | fixed | a00ace78 |
| FND-005 | fixed | a00ace78 |
| FND-006 | fixed | a00ace78 |
