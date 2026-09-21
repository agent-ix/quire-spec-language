---
id: TC-177
title: "The proof-result envelope maps every FR-331 outcome to its exact O-16 category"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-069
    type: verifies
---
# TC-177: The proof-result envelope maps every FR-331 outcome to its exact O-16 category

## Description

Verify that the typed proof-result envelope's reader maps each of the seven
ADR-013 O-16 outcome categories the proof column produces from its FR-331
terminal record, with no collapsing; that a vacuous `Proved` (a Kani run
with zero SUCCESS checks) maps to `inconclusive`/`kani_vacuous_proof`
rather than `success`; and that a `tested` backend result maps to category
`success` (the same category as `proved`) while remaining a distinct,
recorded value that is never promoted to `proved`. `undefined` is not
exercised here: ADR-013's O-16 category table marks it "not produced" for
the proof column, so no FR-331 terminal record can construct it — a ninth
attempted record naming `undefined` would not be a valid FR-331 input, not
an omission from this test. A wrong implementation this test would catch:
mapping `declined` and `unsupported` to the same category (collapsing
refusal into unsupported), reading a vacuous `Proved` run as ordinary
success because the reader only inspects the outer `Proved` tag and never
the SUCCESS-check count, or mapping `tested` into the `proved` value instead
of keeping it distinct within the `success` category. Scope: FR-069-AC-1.

## Test Procedure

1. Construct one minimal FR-331 terminal record for each of the eight wire
   values FR-331's `results` vocabulary admits: `proved` (with at least one
   SUCCESS check), `proved` with zero SUCCESS checks (vacuous), `tested`,
   `refuted`, `declined` (with a typed refusal cause), `unsupported` (with a
   typed unavailability cause), `incomplete` (timeout cause), `incomplete`
   (cancellation cause), and `failed`. (Nine records total: two distinct
   `incomplete` causes and two distinct `success`-category values, `proved`
   and `tested`, are each constructed once.)
2. Read each record into the proof-result envelope.
3. Compare the resulting category against the ADR-013 O-16 category table
   row for that FR-331 value, and separately record the resulting value
   (`proved`, `tested`, `refuted`, and so on) alongside its category.
4. Construct one further record identical to the ordinary `proved` record
   from step 1 except that its SUCCESS-check count is zero instead of one;
   read it and compare its category against the ordinary `proved` record's
   category from step 3, to confirm the reader's category assignment
   changes with the SUCCESS-check count and not merely with the outer
   `proved`/`refuted`/... disposition tag.

## Expected Results

- Each of the eight FR-331 wire values maps to its ADR-013 O-16 category
  exactly as the category table states, with no two distinct FR-331 values
  mapping to the same category incorrectly.
- `proved` and `tested` both map to category `success`, but the recorded
  value distinguishes them: `tested` is never rewritten to `proved`.
- The vacuous `proved` record (zero SUCCESS checks) maps to `inconclusive`
  with cause `kani_vacuous_proof`, never to `success`, while the ordinary
  `proved` record (at least one SUCCESS check) maps to `success` — step 4
  shows these two same-tagged records take different categories.
- A timeout and a cancellation both map to `incomplete` but keep their
  distinct causes, never collapsing into one undifferentiated `incomplete`
  value.
