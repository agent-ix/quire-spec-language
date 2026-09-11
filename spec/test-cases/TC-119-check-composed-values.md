---
id: TC-119
title: "Check exact composed types and guarded value obligations"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-040
    type: verifies
---
# TC-119: Check exact composed types and guarded value obligations

## Description

Planned public Rust API controls for
[FR-040](../functional/FR-040-check-composed-values.md), using the real native
parser, exact supplied definition/rule registry and available admitted model
producer through `binding::Report`. The case checks static value judgments and
runtime requirements without supplying observations or evaluating family bodies.
Each numbered group below corresponds to the matching acceptance criterion.

Expected type identities, source spans, proof premises and costs come from fixed
authored source/model contracts and the public charging rules. Missing producer
or proof capability has its own expected refusal control; that control does not
pass the positive semantic case it blocks. Full rational, query and graph cases
remain planned until their real prerequisites and assertions execute.

## Test Procedure

1. Bind a multi-unit package containing a typed Boolean predicate, state clause
   and temporal/protocol consumers of that predicate. Reuse local expression
   indices across units and inspect the actual original owners/spans, native
   types and stage dispositions. Beside an unrelated valid declaration, supply
   a binding refusal and separately unfinished binding work; neither dependent
   may acquire a checked value. Retain family bodies as unadmitted by this stage.
2. Select StateCore, StateQueries and StateGraph over controlled otherwise valid
   source forms. Use direct predicate calls, filter/map/count/sum and graph
   navigation to distinguish their permissions. Put prohibited syntax behind
   `false implies` and in an unused predicate. Import a narrower callee into a
   wider caller and require the callee's original refusal and both call loci.
3. Independently mutate a valid call's arity, argument order, model owner, nominal
   scalar name and unit while preserving source spelling where possible. Check
   ambiguous/context-free literals, mismatched conditional branches, incorrect
   optional payloads, sequence bounds, enum membership and Unicode text bounds.
   Reject Boolean/enum ordering, whole-container equality, integer/rational
   coercion, and a temporal/protocol status used as a Boolean argument. Check
   every predicate body independently of convenient actual call arguments.
4. Exercise integer inclusive bounds and signed-64 extrema with guarded versus
   possibly overflowing arithmetic. For an explicitly authored dimensionless
   rational Q with normalized numerator [-1,1] and denominator [1,2], require
   `rational(2,4)` and `rational(-2,-4)` to normalize to 1/2 and
   `rational(0,-7)` to 0/1; reject 1/0, 0/0, 2/1 and 1/3. Check the quotient
   (1/2)/(1/2) after reducing raw 2/2; reject 1/(1/2) by result bounds and
   possible zero divisors without the exact preceding nonzero guard. Keep raw
   source identities distinct from normalized values. Repeat with missing
   rational producer/proof representation and require the precise prerequisite
   refusal, without claiming those positive cases have executed.
5. Check size/contains/forall/exists/filter/map/count/sum over an authored finite
   sequence, including nested binders and empty/maximal admitted bounds. Inspect
   source occurrence order, result element/max and preserved population roles;
   no checking step deduplicates or flattens the query. Use Amount 1..20/U,
   maximum 5, Total 0..100/U and dimensionless Count 0..5, then independently
   change maximum to 6, Count upper bound to 4, Total unit or representation,
   and Total to exclude zero. Add a fold whose final mathematical sum fits but
   an intermediate prefix can exceed the selected total. Verify these proof
   obligations from domains/lengths rather than executing sample observations.
   Distinguish N=0, 10,000 and 10,001 at the earliest actual admission boundary.
6. Compare a valid optional-value guard and immutable let alias with reversed
   guards, a different receiver, a pre guard for post access and an invalid
   alternative-join union. Check composite `pre(self.version + delta)` and
   `pre(let v = self.version in v + delta)` in one selected invocation; reject
   `pre(delta)`, `pre(result)`, and an outer post capture retagged inside pre.
   Preserve guard/native/proof locations and selected capture identity.
7. Inspect graph dereference/reachability, operation context/frame and immutable
   cross-family capture requirements. Mutate universe, edge target type/shape,
   selected operation or anchor; replace a relationship export with an ordinary
   same-shaped record or UUID. Valid templates retain complete typed population,
   closure, context and capture requirements with no concrete runtime data.
   Two same-named roles in different declarations retain different owners.
8. Supply exact authored FormalSource and declaration/clause/execution-point
   correspondence to the actual IR proof seam for a supported guarded case.
   Independently omit, duplicate or substitute the source bytes, declaration
   owner and execution point. Require typed correspondence refusal and no
   fabricated proof owner. Compare actual discharged goals with a genuinely
   unproved operation, unavailable representation and exhausted proof expansion;
   each preserves its distinct cause and any established typing.
9. Derive exact charges before execution for controlled chain/diamond calls,
   nested queries, type wrappers, guard facts, proof materialization and retained
   runtime-role records. Use fixed input lengths and the public versioned
   traversal/charging contract, never the run's own usage as the expected limit.
   Test zero, exact, one-step-insufficient and hard-clamped capacities in every
   dimension, including byte copies, shared visits, arithmetic work and depth.
   Exercise counter overflow through the public accounting boundary. Require
   the first unaffordable operation to remain unperformed and all unfinished
   dependents to remain unadmitted. Retry with sufficient limits and compare
   the previous report and input bytes for mutation.
10. Run the historical native checker/profile fixtures with their existing
    expected types, identities, proof results and atomic refusals. Attempt to
    pass composed typed-only or partial output into historical checked/package
    execution boundaries; no label change or empty diagnostic list admits it.

## Expected Results

Every completed value judgment is tied to its original declaration, source,
profile and authoritative model types. Only actual supported discharge with
exact formal correspondence satisfies proof obligations. Missing representation,
unproved definedness, known invalidity and resource exhaustion remain distinct;
none becomes Boolean false, absent optional data or aggregate success.

Independent successful values remain inspectable beside a failed dependency.
Runtime requirements preserve exact authority/anchor/scope without manufacturing
observations. Whole-family execution and external producer qualification remain
outside this test case. All groups are planned; catalog validation establishes
document conformance, not implementation or test completion.
