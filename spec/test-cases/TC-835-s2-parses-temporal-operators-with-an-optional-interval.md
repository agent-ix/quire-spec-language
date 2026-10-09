---
id: TC-835
title: "S2 parses temporal operators with an optional interval, independent of profile"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-325
    type: verifies
---
# TC-835: S2 parses temporal operators with an optional interval, independent of profile

## Description

Verify that S1 and S2 parse every temporal operator with a closed interval,
an open interval or none, keep each interval's span, leave inverted
intervals to S3, build the same forms under every profile selection, and
build each fairness constraint and capture as written.

Scope: FR-325-AC-1 to FR-325-AC-6.

## Test Procedure

1. Parse and build forms for the recovery-stability clause of FR-325-AC-1
   and read each operator's interval and span.
2. Parse `holds(p) until holds(q)`, `holds(p) since[2,4] holds(q)`,
   `eventually[3,*] holds(p)` and `eventually[5,3] holds(p)`.
3. Parse `eventually[0,x] holds(p)`.
4. Build the forms of two units that differ only in the selected profile
   identity (infinite-trace and event-position false-extension), padded so
   their headers have equal length, and compare the clause forms by
   `PartialEq` and by their `Debug` rendering.
5. Build the clause of FR-325-AC-5 and read each fairness form's kind,
   granularity, operation and span.
6. Build the clause of FR-325-AC-6 and read its fairness and capture forms;
   build the same clause with the two captures removed and compare the
   formula forms, up to spans.

Tag the tests `#[trace("TC-835", "FR-325-AC-n")]`.

## Expected Results

- Step 1: `None` on the outer `always` and on `eventually`; `Some{0,
  Finite(10)}` on the inner `always`; three distinct spans.
- Step 2: `None`, `Some{2, Finite(4)}`, `Some{3, Open}`, `Some{5, Finite(3)}`;
  no diagnostic.
- Step 3: an S1 parse diagnostic at `x`'s span and no form.
- Step 4: equal forms under `PartialEq` and equal `Debug` renderings.
- Step 5: (`None`, `None`, `attemptUpdate`), (`Some(Strong)`, `Some(Each)`,
  `reset`) and (`Some(Weak)`, `Some(Whole)`, `tick`), in source order, each
  span covering its own `fair … ;` constraint.
- Step 6: one fairness form, then capture forms `before` (`Int[0, 1000]`,
  `p.value`) and `parent` (`Config::ConfigVersion`, `p.parent`) in source
  order, each span covering its own `capture … ;`; the formula forms are
  equal up to spans.
