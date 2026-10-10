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

Exercise every production evaluator detecting site in FR-090's producer
table. Scope: FR-090-AC-15, FR-090-AC-16, FR-090-AC-17 and FR-096-AC-15.
Fixtures are private malformed checked-state probes, not public unchecked
authority. A test of `InternalFault::new` alone does not exercise a producer.

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
   assert the table's exact identifier at the actual producer.
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

## Expected Results

Every detecting site in step 1 returns an S6a InternalFault with the exact
FR-090 identifier, category internal failure and code `runtime_invariant`,
with no manufactured CheckedInvariantCause, kernel refusal, family refusal or
Evaluation. The inventory exposes missing sites rather than treating a shared
helper test as coverage for its callers. Compile-only seam probes are excluded.

Step 2 returns `boolean-value-expected` directly, completes for Boolean, and
returns `CallFailure::Input` at public admission for Integer. Step 3 retains
the exact ordinary charge prefix with no added fault event, retry or later
retention. Earlier budget/cancellation stops remain Incomplete. Step 4 retains
stage and invariant as `CallFailure::Fault` or a replay Failed terminal with
internal-failure diagnostic and unavailable basis, not Input or Inconclusive.
Step 5 keeps each existing outcome or independently named fault and does not
route it through a default table classification.
