---
id: SR-678
title: "QSL-271 PR 469 integrity review of FR-100's refusal-record mapping"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@ceb5d905; spec/functional/FR-100-run-a-named-function-through-the-spine.md; spec/test-cases/TC-452-spine-run-entry-lives-in-qsl-replay.md; consistency against spec/functional/FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md, spec/functional/FR-109-run-a-state-clause-through-the-spine.md, spec/test-cases/TC-468-spine-clause-run-reports-typed-dispositions.md, spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md O-16/O-17/T-4, QSpec spec/functional/tooling/FR-301-expose-complete-cli.md (origin/main), Linear STD-110, qsl-foundation/src/diagnostic.rs, src/command.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-100
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-109
    type: reviews
---

## Summary

Ticket: QSL-271 (PR agent-ix/quire-spec-language#469). This review checks
consistency with FR-096, ADR-013, QSpec FR-301 and the specs that consume
FR-100's mapping.

Every claim about FR-096 and the #465 code is correct on main 9425dd82:

- `Evaluation::refusal_record` dispatches to the family cause's
  `refusal_record` or to `kernel_refusal_record` (`qsl-eval/src/value/expression/evaluate.rs:86-102`).
- `kernel_refusal_record` builds a record only for `CardinalityOutOfBound`,
  with code `cardinality_out_of_bound`, cause `violation.as_str()`, and fields
  `collection`, `bound` and `count` (`qsl-foundation/src/diagnostic.rs:916-962`).
- `RefusalRecord` carries the code, locus and fields, and its category is
  always refusal.
- `catalog_fields` returns `None` for a cause with no key-table row, and
  `ModelQueryRefusal` `absent-key` gives `binding` and `key`.

The PR also meets the two checks the team leader added:

- STD-110 is cited for all ten causes that have no catalog code (lines 133 and
  284). `ForeignReference` correctly cites the `quire-exact` variant instead:
  `foreign_reference` is already in `1-draft.7`, so STD-110 is not its blocker.
- FR-096 is cited as the rule behind `spine::run`'s `CheckedInvariant` to
  `InternalFault` conversion (lines 122, 135, 168, 236-238, 274-276 and
  286-288).

Exit codes 20, 22 and 30 match QSpec FR-301 (`20` invalid/refused, `22`
incomplete, `30` tool failure).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-109, on main, maps every non-`Completed` S6a outcome "by FR-100's outcome mapping, to FR-100's `outcome` member and FR-100's exit status, category `refusal`, `undefined` or `incomplete`". `ClauseRunReport::exit_code()` is a total match over those categories with no internal-failure arm. FR-109-AC-5 checks equality "over the outcomes FR-100-AC-9 constructs". After this PR, FR-100 gives `CheckedInvariant` no `outcome` member: it is a command error, exit 30, category internal failure. FR-109's disposition has no stage, category or record for it. Failure scenario: TC-468 constructs `CheckedInvariant` per FR-109-AC-5. The report must carry an `outcome` member FR-100 no longer defines and a category FR-109 does not have, so FR-109 is unimplementable as written. The PR did not touch FR-109. Fix: in this PR, amend FR-109 to add an internal-failure disposition (stage `evaluate`, `runtime_invariant`, exit 30), or exclude `CheckedInvariant` from FR-109-AC-5's set. | spec/functional/FR-109-run-a-state-clause-through-the-spine.md:72-77, 116-128, 144; spec/functional/FR-100-run-a-named-function-through-the-spine.md:122, 135, 168 |
| FND-002 | medium | The `CheckedInvariant` row went into the "Refusals before S6a" table. Its heading and preamble say every row is a refusal before S6a, with "a `details` object", and "exits with that code's exit status". The row is an S6a outcome, not a refusal, with `details` `null` and exit 30. `Code::RuntimeInvariant.exit_code()` is 20 (`qsl-foundation/src/diagnostic.rs:310-318`). The CLI's `RunCause::exit_code` sends every code except `Output` through it (`src/command.rs:265-269`), and native run's `runtime_invariant` exits 20 today. Failure scenario: an implementer adds the spine fault as a `RunCause` carrying `Code::RuntimeInvariant`, following the preamble. It exits 20, and TC-452 expects 30. Fix: move the row into its own "Internal failure at S6a" subsection that states exit 30 as an explicit exception to `Code::exit_code`, and give `details` as the object FR-026's envelope requires, or say `null` is allowed. | spec/functional/FR-100-run-a-named-function-through-the-spine.md:150-168 |
| FND-003 | low | FR-100:111-112 says the mapping runs "from its ADR-013 O-16 category". O-16's evaluation column maps every `Refused(Refusal)` to refusal, including `CheckedInvariant`. The internal-failure category for `CheckedInvariant` comes from FR-096's ruling (and T-4's `InternalFault`), not from O-16's table. Failure scenario: a reader who checks against O-16 sees a refusal mapped to internal failure. Fix: say the category is O-16's, except that `CheckedInvariant` is internal failure by FR-096. | spec/functional/FR-100-run-a-named-function-through-the-spine.md:111-112, 122; spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md:382-405 |
| FND-004 | low | Line 133 says STD-110 "gives them codes" for all ten causes. STD-110's title and deliverable name eight. Its body (untrusted Linear text, read as data) notes that `ieee_nan_payload_not_representable` and `ieee_rational_out_of_domain` are named by QSpec FR-148 but omitted from the catalog. It does not clearly commit to adding them. FR-096's Status has the same "so do" reading. Failure scenario: `1-draft.8` ships eight codes, and the two IEEE rows keep rendering bare with no ticket tracking them. Fix: get STD-110's scope to state the two IEEE causes, or cite the ticket that will cover them. | spec/functional/FR-100-run-a-named-function-through-the-spine.md:133, 284 |
| FND-005 | low | No ticket is cited for the `ForeignReference` blocker ("until the `quire-exact` variant carries the universes"), and a Linear search found none. FR-096 cites none either. Failure scenario: the kernel variant never gains its universes, and the bare rendering is permanent. Fix: file a ticket and cite it beside STD-110. | spec/functional/FR-100-run-a-named-function-through-the-spine.md:134, 285 |
| FND-006 | low | The record row's exit is "the record's code's FR-301 exit status (`Code::exit_code`)". A `RefusalRecord` holds a `CatalogCode` (a string pair), not a `Code`. No `CatalogCode` to `Code` conversion is named, and some catalog codes (for example `extraction-requires-run`) have no `Code` variant. Failure scenario: an implementer defaults an unmapped code to 20, or panics. Fix: name the conversion (match on `Code::all()` by `as_str`) and say what an unmapped code exits with. | spec/functional/FR-100-run-a-named-function-through-the-spine.md:120 |

## Verdict

Changes requested: FND-001 breaks FR-109's reuse of FR-100's mapping, and
FND-002 contradicts itself on the exit status.

## Dispositions

Disposition pass at `agent-ix/quire-spec-language@bec5791c` (fix commits `e71e60cc` and `bec5791c`, on main 9425dd82). I checked each outcome against the spec at that head and the code on main 9425dd82, not against the commit message. `quire validate` over FR-100, FR-109, TC-452, TC-468 and these reviews exits 0.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed e71e60cc | FR-109 adds category `internal-failure` (stage `evaluate`, the fault's stage and invariant, no `outcome` member), an If statement, and an `exit_code()` arm, and FR-109-AC-5 covers it. TC-468 step 5 constructs `CheckedInvariant` and `CallFailure::Fault` with that oracle. |
| FND-002 | fixed bec5791c | The row moved to a new "Internal failure at S6a" section (e71e60cc) with `details` `{"stage", "invariant"}` and exit 30. bec5791c states that the path exits 30 directly and not through `Code::exit_code`. That `Code::RuntimeInvariant.exit_code()` is 20 elsewhere is ruled out of scope by the team leader and routed to the core lane. |
| FND-003 | fixed e71e60cc | FR-100 now says each category is O-16's except `CheckedInvariant`, which is internal failure by FR-096 (T-4 `InternalFault`). |
| FND-004 | fixed e71e60cc | Status separates the eight causes STD-110 names from the two IEEE causes, which it cites "pending confirmation that it covers them". |
| FND-005 | fixed e71e60cc | Status cites QSL-281 (verified in Linear, Backlog: "Refusal::ForeignReference carries its universes so FR-096 builds a foreign_reference record"). |
| FND-006 | fixed e71e60cc | The exit is `Code::exit_code` of `Code::from_code(code)`, which exists on main (`qsl-foundation/src/diagnostic.rs:277`, `Code::all()` matched by `as_str`). An unmapped code exits 20. |

New finding (low, non-blocking): FR-109:118-121 still maps "every other S6a outcome" to "category `refusal`, `undefined` or `incomplete` by its ADR-013 O-16 category". `CheckedInvariant` is an O-16 refusal, so this overlaps the new internal-failure If statement at :124. Add "other than FR-100's internal failures".

Fixed in 720308d3: FR-109 now maps "every other S6a outcome other than FR-100's internal failures" by FR-100's mapping, so it no longer overlaps the internal-failure If statement.
