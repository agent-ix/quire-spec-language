---
id: TC-137
title: "Bound checked-term evaluation and retry immutable inputs"
type: TC
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-049, type: verifies }
  - { target: ix://agent-ix/quire-spec-language/NFR-009, type: verifies }
---
# TC-137: Bound checked-term evaluation and retry immutable inputs

## Description

Scope: FR-049-AC-7 to FR-049-AC-8.

Verify caller-owned accounting across the checked-term replacement entries,
including zero/no-work, exact and one-short boundaries, charge-before-work,
no partial result and a fresh-meter retry. Protocol-slot shared-meter
accumulation is exercised in TC-1073.

## Test Procedure

Use public checked-term/family entry points over admitted checked packages and
immutable inputs. Select fixtures that perform input admission, expression
nodes, sequence occurrences, retained outputs, scalar/key comparisons and
model graph navigation. Derive usage from each fixture's successful run under
a sufficient `quire-exact::Meter`; use the accounting counters defined by
NFR-009 and FR-255. For every charged counter, run at zero when the fixture
requires work, at its measured usage, and one below while holding other
counters sufficient. Include a no-work case at zero and raise a caller-set
counter above its default to complete work that exceeds the default. After each
resource stop, retry the same immutable checked package and inputs using a
fresh sufficient meter. On a 512 KiB stack, evaluate a 100,000-record-deep
input, a 10,000-call nested predicate chain and a 10,000-object graph chain.

## Expected Results

No-work at zero and each exact-usage run complete. A one-short run stops before
the denied operation, identifies the counter, and returns no partial completed
value. Fresh-meter retries complete with the same value and leave checked
packages and runtime inputs unchanged. The deep-body and graph cases complete
without native-stack overflow. Raising a caller-set counter above its default
allows the fixture to complete work beyond that default. Each charged counter
and charge point has an independent oracle; omitting one charge makes its
corresponding mutation control fail. TC-1073 separately proves accumulation
across sequential protocol-slot calls on one shared meter.

The protocol-slot boundary trace is
`qsl-eval/tests/it/protocol_slots.rs::protocol_slot_calls_share_meter_usage_and_obey_work_boundaries`.
It measures a successful result, completes at the exact work-unit limit,
refuses one unit short without charging the denied unit, refuses at zero, and
repeats the immutable call with a fresh meter to verify the same result and
usage. Its trace tags include TC-137, TC-1073, FR-049-AC-7/8 and NFR-009-AC-1/2/4.
