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
intervals to S3, and build the same forms under every profile.

Scope: FR-325-AC-1 to FR-325-AC-4.

## Test Procedure

1. Parse and build forms for the recovery-stability clause of FR-325-AC-1
   and read each operator's interval and span.
2. Parse `holds(p) until holds(q)`, `holds(p) since[2,4] holds(q)`,
   `eventually[3,*] holds(p)` and `eventually[5,3] holds(p)`.
3. Parse `eventually[0,x] holds(p)`.
4. Build the forms of one unit under the infinite-trace and the
   event-position false-extension profile selections and compare bytes.

Tag the tests `#[trace("TC-835", "FR-325-AC-n")]`.

## Expected Results

- Step 1: `None` on the outer `always` and on `eventually`; `Some{0,
  Finite(10)}` on the inner `always`; three distinct spans.
- Step 2: `None`, `Some{2, Finite(4)}`, `Some{3, Open}`, `Some{5, Finite(3)}`;
  no diagnostic.
- Step 3: an S1 parse diagnostic at `x`'s span and no form.
- Step 4: byte-equal forms.
