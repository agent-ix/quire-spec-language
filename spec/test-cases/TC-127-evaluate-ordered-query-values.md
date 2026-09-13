---
id: TC-127
title: "Evaluate ordered query values against independent expected results"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-046
    type: verifies
---
# TC-127: Evaluate ordered query values against independent expected results

## Description

Compare compilation and admitted-artifact evaluation of the eight query forms
with independently authored expected values under
[FR-046](../functional/FR-046-execute-predicates-and-ordered-queries.md).

## Test Procedure

Use explicit Amount 1..20/U, N=5, Total 0..100/U and Count 0..5 domains and
validated sequence [2,2,3]. Evaluate size, membership, both quantifiers,
filter(amount=2), map(amount), count(amount=2) and sum(amount). Compare results
with literal expected values, not values computed by the implementation under
test. Repeat on empty inputs, admitted maximum-length inputs and bounded nested
queries. Use observable work accounting to distinguish Boolean short-circuit
from forms that must visit every occurrence; do not replace the evaluator with
a mock to manufacture the count.

Place an admitted unavailable occurrence independently before and after the
decisive member for contains/exists/forall. Choose budgets that exhaust before
and only after that decision under the independent work oracle. Place an
unavailable or one-step-exhausted occurrence in the middle of
map/filter/count/sum and inspect that no partial sequence or aggregate escapes.
Separately corrupt a supplied value before evaluation and verify that complete
input validation refuses even when the bad occurrence would have followed a
decisive Boolean member.

Vary only the result unit, numerical representation, count bound or declared
maximum N=6 while keeping the small observed sample. Include an out-of-range
prefix whose final mathematical sum fits, and rational projections whose
normalized prefixes exercise the declared rational domain. Compare emitted
query/binder/type/capture records separately from the public state evaluator's
actual results.

## Expected Results

Map yields [2,2,3], filter yields [2,2], count is 2, sum is 7 and size is 3.
Empty contains/exists are false, forall is true, map/filter are empty and
size/count/sum are zero in admitted result types. Duplicates and order survive.
Contains/exists/forall stop at the specified decisive occurrence; other forms
visit every required occurrence. Domain and prefix failures cannot be repaired
by the observed sample, reassociation, conversion or floating approximation.
Rational prefixes normalize before bounds. An unavailable occurrence before a
decision propagates, while skipped later data does not erase a decision.
Full-traversal queries retain the first terminal non-complete cause and expose
no partial result. Invalid supplied data refuses before traversal and cannot be
hidden by short-circuit order.

Where proof transfer or execution is unsupported, record that typed outcome as
an explicitly unsupported representation, not as a passing positive case. The
executed case uses both the public state evaluator and the independent emitted
artifact reader; source/type/emission tests alone do not establish its runtime
gate.
