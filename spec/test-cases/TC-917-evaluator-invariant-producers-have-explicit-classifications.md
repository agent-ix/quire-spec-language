---
id: TC-917
org: agent-ix
title: "Evaluator invariant producers have explicit QSL classifications"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-090
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-096
    type: verifies
---
# TC-917: Evaluator invariant producers have explicit QSL classifications

## Description

Verify every production evaluator detecting site in FR-090's producer
table. Scope: FR-090-AC-15, FR-090-AC-16, FR-090-AC-17, FR-090-AC-18,
FR-090-AC-19, FR-090-AC-20 and FR-096-AC-15.
Fixtures are private malformed checked-state probes, not public unchecked
authority. A test of `InternalFault::new` alone does not exercise a producer.
`record-present-value-missing` uses its existing local source-inspection
proof. The fixed Rational caller's `ieee-conversion-rejected` uses the bounded
temporary Inspection proof below until the reviewed, merged IR-728 API is
adopted by QSL. All other rows require real detecting sites and malformed-state
controls; neither exception grants unchecked public authority.

## Test Procedure

1. Maintain a fixture inventory keyed by FR-090's producer condition and
   detecting site. Cover stack finalization and extraction; frame/local/task
   scheduling; reaches and conversions; IEEE and quantity construction;
   field/attribute/Option projection; local/imported/dispatch resolution;
   tuple/record construction; flatten and population-query checked shapes;
   edge traversal; ordered comparison; iteration, fold, sum and stop reporting.
   Where a row names several detecting sites, exercise each site (including
   both return-frame consumers, all callable-resolution consumers, both sum
   accumulator checks, and iteration start/resume/finish). For every fixture,
   mutate only the prerequisite needed to reach the stated condition and
   assert the table's exact identifier at the actual producer, except for
   `record-present-value-missing` and the fixed Rational caller's temporary
   `ieee-conversion-rejected` exception. For the Record row, inspect the Record path
   and `pop_many` in `qsl-eval/src/value/expression/evaluate.rs`: `present`
   counts all Present slots; `pop_many(present)` either stops with stack
   underflow or returns exactly `present` values; the zip with declared
   fields visits a prefix of slots and consumes one value only for each
   Present slot. Thus `values.next()` cannot return None at a Present slot,
   even with a shorter declaration or a malformed value stack. Record this
   proof against FR-090-AC-18 and inspect the defensive branch's table
   classification. Do not replace producer logic or inject a preconstructed
   fault to simulate execution of that branch.
   For `ieee-conversion-rejected` only, inspect the real
   `NodeKind::IeeeToRational` call in
   `qsl-eval/src/value/expression/evaluate.rs` and the immutable kernel source
   linked in FR-090. Establish that the call selects Rational unconditionally
   and that the kernel's outer Err discriminant selects only Decimal, Integer
   or BoundedInteger; all Rational results are wrapped in Ok. Record the
   API/caller proof against FR-090-AC-19 and inspect the retained defensive
   S6a fault mapping. Do not replace the target with a non-Rational target,
   mock the kernel, inject a fault, throw or panic to claim producer execution.
2. Include direct S6a `not x`, `x: Boolean`, with an Integer argument bypassing
   admission. Pair it with a valid Boolean argument and with the same Integer
   supplied through each public package API's normal admission.
3. Record each malformed run's meter events. Pair it with a valid run sharing
   the executed prefix; include producers reached before any charge and after
   positive spend. Deny or cancel each preceding charge, without forcing the
   later producer, and observe the earlier stop.
4. Drive an actual QSL producer fault through each public call/evaluate fault
   propagation seam and replay fault mapping. Retain its original identifier.
5. Exercise negative controls: absent Option payload remains NoneValue;
   absent/null optional edge gives no targets; false dispatch precondition
   remains family undefined; absent-key and cardinality remain family
   outcomes; sum out-of-domain remains undefined; unresolved replay reference
   retains ReferenceUnresolved; witnessed reads keep their existing behavior.
   Include independently named quantity-unit, population-resolution and
   occurrence-provenance faults, and ordinary kernel operation outcomes.
   Execute actual IEEE-to-rational finite in-domain, negative-zero loss,
   NaN/infinity undefined, out-of-domain refusal and denied-charge controls;
   retain their ordinary Outcome discriminants and precise charge prefixes.
   Exercise request cancellation through QSL's own polling/stop boundary,
   not an invented kernel Outcome cancellation variant.
6. On QSL adoption of the reviewed, merged IR-728 Rational-only API, inspect
   that the obsolete outer-Err arm, its producer-table row and the temporary
   exception are removed together under FR-090-AC-20. Reconcile the inventory
   with actual remaining producers; retain the Record proof and every other
   detecting-site fixture. If caller/API discriminants change earlier, the
   old proof is invalid and must not exempt a now-reachable producer.

## Expected Results

Every execution fixture in step 1 returns an S6a InternalFault with the exact
FR-090 identifier, category internal failure and code `runtime_invariant`,
with no manufactured CheckedInvariantCause, kernel refusal, family refusal or
Evaluation. The inventory exposes missing sites rather than treating a shared
helper test as coverage for its callers. Compile-only seam probes are excluded.
The Record inspection-only row establishes the count/iterator proof and retains
the specified `record-present-value-missing` classification without claiming
an executed fault. The temporary Rational inspection establishes only the
outer-Err API/caller proof and preserves its safe fault classification while
the arm exists. Neither proof grants an exception to any other row, especially
the publicly admission-reachable empty object identity. Step 6 enforces the
adoption-bound expiry, not an indefinite waiver.

Step 2 returns `boolean-value-expected` directly, completes for Boolean, and
returns `CallFailure::Input` at public admission for Integer. Step 3 retains
the exact ordinary charge prefix with no added fault event, retry or later
retention. Earlier budget/cancellation stops remain Incomplete. Step 4 retains
stage and invariant as `CallFailure::Fault` or a replay Failed terminal with
internal-failure diagnostic and unavailable basis, not Input or Inconclusive.
Step 5 keeps each existing outcome or independently named fault and does not
route it through a default table classification. Ordinary IEEE outcomes and
QSL-owned cancellation retain their actual channels, meter prefixes and loss
provenance; this inspection exception does not weaken their execution tests.
