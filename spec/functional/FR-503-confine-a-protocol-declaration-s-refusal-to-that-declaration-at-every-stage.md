---
id: FR-503
title: "Confine a protocol declaration's refusal to that declaration at every stage, and carry the per-protocol results in the stage failure"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-024
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-002
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-067
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-105
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-278
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-285
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-383
    type: depends_on
---
# FR-503: Confine a protocol declaration's refusal to that declaration at every stage, and carry the per-protocol results in the stage failure

## Description

Each of S1 (parse), S2 (forms), S3 (check) and S4 (emit) SHALL attribute a
refusal to the one protocol declaration that caused it whenever the refusal lies
inside that declaration, and the lifecycle operations SHALL carry one
`ProtocolResult` for every protocol declaration of the unit in the failure they
return. This is the stage-level obligation behind FR-383's per-protocol results
(FR-383-AC-18, AC-28 to AC-31) and the retained behavior of FR-042-AC-8: one
protocol's refusal at any stage leaves the other protocols' results intact.

## Use case

An author edits a unit of three protocols and mistypes a brace in one. The
compile reports that protocol refused at its own line, and the other two still
compile; the unit is not admitted, and the report holds all three results.

## Inputs

A source unit and the lifecycle operations `parse`, `check` and `compile`
(FR-278, FR-275).

## Outputs

- From `qsl_cst::parse`: a `ParsedSource` whose recovered CST gives each protocol
  declaration its own **delimited span** (from its `protocol` keyword to its
  closing delimiter) and its own diagnostics.
- From each operation, on failure: `StageFailure::Refused` or `StageFailure::Limit`
  whose payload holds `protocol_results`, one `ProtocolResult` per protocol
  declaration the parser delimited, in declaration order: `Emitted`, `Refused`,
  `Incomplete` (FR-383), or `Cleared { stage }` for a declaration that passed
  every stage the operation ran (`parse` runs S1 and S2, `check` adds S3, and only
  `compile` reaches S4 and `Emitted`).

## Behavior

- **S1.** When a syntax error lies inside the delimited span of one protocol
  declaration, `qsl_cst::parse` SHALL recover at that declaration's closing
  delimiter and attach the diagnostic to that declaration.
- Every other declaration SHALL parse as it would have without the error.
- When the parser cannot delimit the declaration (an unbalanced delimiter that runs to the end
  of the source, or an error before any declaration's keyword, in the unit
  header, a profile selection or a model import), the error is **unattributable**:
  the operation SHALL return the unit refusal with no `protocol_results`.
- **S2.** S2 SHALL build the forms of each protocol declaration independently,
  and a refusal of one declaration's forms SHALL carry that declaration's source
  index and span as its locus.
- **S3.** S3 SHALL attribute each refusal by its locus to the declaration that
  owns it (FR-042-AC-8: a source index and span from the same owning declaration,
  never a preceding source), and SHALL check each protocol against the unit's
  model and profile selections only, so a refused sibling changes no result
  except through a reference (FR-383 "Per-protocol results").
- **Continuation.** When a failure is attributed to protocol declarations, the
  `check` and `compile` operations SHALL continue S2 to S4 (as far as the
  operation runs) for every declaration whose own earlier stages are clean, so a
  clean protocol reaches `Emitted` in `compile` although its unit is refused. The
  continuation SHALL NOT admit the unit: the aggregate is `Refused` (FR-383).
- **Payload and category.** The payload SHALL NOT change the failure's category.
  A `StageFailure::Refused` takes the category of the first `Refused` result's
  cause in declaration order, and `StageFailure::Limit` is incomplete (FR-285).
  The `package` stage stays all or nothing for the unit (FR-105): no package is
  produced, though the retained results hold the nodes of the protocols that
  emitted.
- A failure that is not about any protocol declaration (a non-protocol
  declaration's refusal, an unattributable error) SHALL carry the
  `protocol_results` of the declarations that cleared the stages reached, each
  `Cleared`, and SHALL NOT turn them into `Refused`.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-503-AC-1 | `qsl_cst::parse` over a unit of protocols **W**, **X**, **Z** in which **X** holds a missing closing brace inside its own span before the next `protocol` keyword returns a recovered `ParsedSource` with an `invalid_syntax` diagnostic attached to **X**'s declaration, and **W** and **Z** parsed with the CST and diagnostics they have in a unit without **X**. | Test (TC-1298) |
| FR-503-AC-2 | The `parse` operation over AC-1's unit refuses at S1 with `protocol_results` [W `Cleared{S2}`, X `Refused` `invalid_syntax` at **X**'s span, Z `Cleared{S2}`]; over a unit whose header holds a syntax error, and over a unit whose last brace is missing so that it runs to the end of the source, it refuses with no `protocol_results`. | Test (TC-1298) |
| FR-503-AC-3 | The `compile` operation over AC-1's unit returns a failure whose `protocol_results` are [W `Emitted`, X `Refused`, Z `Emitted`] with **W**'s and **Z**'s nodes byte-identical to compiling each alone, the category `refusal` (exit 20), and no package. | Test (TC-1298) |
| FR-503-AC-4 | An S2 refusal of **X**'s forms and an S3 refusal in **X** each give **X** a `Refused` result whose locus is **X**'s source index and span (in a two-source unit, the source that holds **X**, not the preceding one), leave **W** `Emitted`, and give the failure the code of the first `Refused` result. | Test (TC-1298) |
| FR-503-AC-5 | The `check` operation over a unit with **X** refused at S3 returns `protocol_results` [W `Cleared{S3}`, X `Refused`], and its category and exit equal those of **X**'s cause alone; the `Limit` failure of an exhausted `s3.work_units` budget carries the FR-383-AC-30 results and exits 22. | Test (TC-1298) |
| FR-503-AC-6 | A refusal of a non-protocol declaration (a state clause) in a unit holding **W** gives `protocol_results` [W `Cleared{S3}`] or [W `Emitted`], never `Refused` for **W**. | Test (TC-1298) |

## Dependencies

- FR-383 (the per-protocol result vocabulary and aggregate), FR-002 (S1), FR-067
  (S2), FR-105 (the package's all-or-nothing rule), FR-278 (the operations'
  outputs), FR-285 (the failure's category and exit).
- FR-042-AC-8 (retained: independent results; the owning declaration's locus).
