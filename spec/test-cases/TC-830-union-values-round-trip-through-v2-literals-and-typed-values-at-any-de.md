---
id: TC-830
title: "Union values round-trip through v2 union_value nodes at any depth"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-321
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-277
    type: verifies
---
# TC-830: Union values round-trip through v2 union_value nodes at any depth

## Description

Scope: FR-321-AC-3, FR-321-AC-4, FR-321-AC-5, FR-277-AC-4,
FR-277-AC-5.

## Test Procedure

1. Convert `Shape::Empty`, `Shape::Circle(4)` and a depth-3 `Tree` to their
   v2 `union_value` spelling and back.
2. Supply a `Tree` 10,000 levels deep; admit, convert and compare it with
   itself under default run limits.
3. Admit it with the value-occurrence limit one below its occurrence
   count.
4. Retain QSL-503's unchanged full 10,000-level supplied Tree fixture and
   its source declarations, with 29,999 semantic occurrences. Do not
   replace its leaf/member/child shape with a newly invented Tree.
   Use public supplied-value APIs, with no shrunken or private substitute.
   Admit at occurrence bounds 29,998 and 29,999 with other bounds permitting
   progress. Independently enumerate the reviewed QSV event sequence under
   its trusted-admitted-value boundary; call its successful total A. Set
   `supplied.admission_work_units` to A-1 and A, keeping evaluation limits
   identical and sufficient. Do not substitute the research prediction
   69,997 for A.
5. Supply multiple arguments and retain one cumulative admission budget.
   Cancel the original handle before the first helper and during a later
   argument, once with each original cause. Exercise zero helper ceiling,
   checked counter overflow, wrong-kind input and failed storage/capacity
   reservation. Compare the next event and successful spend against an
   independent enumerator; verify a cache-hit lookup still has its owning
   logical event. Preserve QSV's admitted nested-value trust boundary;
   do not revalidate trusted descendants just to increase the event count.

Tag each test `#[trace("TC-830", "<AC id>")]`.

## Expected Results

- Step 1: each round trip gives an equal kernel value.
- Step 2: admitted, converted, equal to itself; no host stack overflow.
- Step 3: the value-occurrence limit, named, with its configured value.
- Step 4: 29,998 denies before evaluation; 29,999 admits. A-1 denies the
  actual next admission event unspent and A admits when other bounds fit.
  Denial names the phase, parameter, typed event, setting, configured bound,
  successful spend and denied amount, with zero evaluation consumption.
- Step 5: no reset/fresh Cancel/free logical lookup/double debit. Cancel
  remains Requested or Deadline, not a limit. Invalid input retains its
  original refusal. Storage and capacity remain distinct from limits,
  reserve before mutation, and yield no partial success. Wrong next-event
  metadata or a FunctionCall charge is a failure of this test.

## Status

Planned. QSL-681 phase controls are proposed, not executed; shared event
and union baseline alignment is pending.
