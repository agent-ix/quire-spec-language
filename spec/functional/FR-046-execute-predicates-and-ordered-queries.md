---
id: FR-046
title: "Execute exact reusable predicates and ordered bounded queries"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-002
    type: implements
  - target: ix://agent-ix/quire-spec-language/US-003
    type: implements
  - target: ix://agent-ix/quire-spec-language/FR-036
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-040
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-042
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-049
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-033
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-034
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-041
    type: depends_on
---
# FR-046: Execute exact reusable predicates and ordered bounded queries

## Description

When an explicitly selected native query definition admits a predicate or sequence query, the language pipeline SHALL preserve its exact checked meaning through compilation and evaluation over validated supplied inputs.

This scopes roadmap L3 under compiler
[#36](https://github.com/agent-ix/quire-spec-language/issues/36). It restates the
accepted `quire.state.queries/v1` query definition and state contract from the
standard's [PR #15](https://github.com/agent-ix/quire-specification/pull/15)
as a compiler-owned delivery obligation. It does not change those meanings or
promote the historical no-call profile in place.

## Inputs

Original native declarations, exact domain-package model declarations
([FR-056](FR-056-admit-domain-package-model-declarations.md)) and definition selections, explicit
predicate arguments and immutable capture/anchor environments; checked values
and their original source correspondence; validated finite sequences or declared
population views with independently supplied membership/completeness authority;
and caller-lowered finite checking and execution budgets.

A typed expression, emitted graph or successful independent wire read is not a
validated runtime input. Population declarations come from the admitted domain
package; concrete population views are supplied through the observation
interface, not an additional observation store.

## Outputs

Located checked predicates/calls and compiled query records preserving the
original identities, followed by exact query values or Boolean predicate values
when execution completes. Refused, unsupported and resource-incomplete stages
retain their distinct dispositions without exposing an unfinished value as a
completed result or treating inability to evaluate as a logical violation.

## Behavior

The compiler SHALL retain each acyclic Boolean predicate's exact declaration, parameter order, nominal type/unit, checked definition and immutable anchor environment at every call.

If a predicate has a direct or indirect call cycle, an unresolved or ambiguous call, an incompatible argument, a non-Boolean root or an unproved partial operation, then the compiler SHALL refuse the affected predicate and its dependent calls at their original locations.

The compiler SHALL apply the selected profile to unused declarations and unselected syntax without upgrading a callee through a wider caller profile.

The evaluator SHALL bind each query occurrence in supplied sequence order while retaining duplicate occurrences and the original binder environment.

The evaluator SHALL validate the selected package, declaration, binder and
supplied input authority before entering any query body.

A request, artifact or supplied-value refusal found by that complete validation SHALL
terminate before query evaluation regardless of occurrence order. Query short-
circuiting may skip an already admitted `Unavailable` placeholder and work that
was never entered; it cannot hide invalid input.

For `contains`, `forall` and `exists`, the evaluator SHALL return a decisive
completed result only when it encounters that decisive occurrence before any
incomplete, refused or exhausted occurrence in source order.

For `contains`, `forall` and `exists`, the evaluator SHALL skip every occurrence
after a decisive completed result without allowing unavailable skipped data to
replace that result.

For `filter`, `map`, `count` and `sum`, the evaluator SHALL stop at the first
incomplete or exhausted occurrence in source order and discard every partially
materialized output and aggregate.

For every query, the evaluator SHALL retain the first runtime incomplete or
exhausted outcome actually encountered after input validation without evaluating
later occurrences to select a different cause. A defensive evaluator refusal
terminates immediately and identifies the violated admitted invariant.

The evaluator SHALL use the following results and stopping rules for the eight query forms.

| Form | Required evaluation |
| --- | --- |
| size | Exact length in the uniquely selected dimensionless integer domain admitting 0..N. |
| contains | Same admitted equality type; stop at the first equal occurrence; empty is false. |
| forall | Stop at the first false body result; empty is true. |
| exists | Stop at the first true body result; empty is false. |
| filter | Evaluate each occurrence and retain matching original occurrences in order; empty remains empty. |
| map | Evaluate each occurrence once and retain every result in order, including duplicates; no flattening. |
| count | Evaluate each occurrence and count true results in the explicitly selected dimensionless integer domain admitting 0..N; empty is zero. |
| sum | Exact left fold from zero in the explicitly selected result domain, evaluating each projection once in order; empty is zero only in an admitted result domain. |

The compiler SHALL establish sum projection representability and every accumulated prefix bound in the explicit same-representation, same-unit result domain before admitting the query as checked.

The evaluator SHALL normalize each exact rational sum prefix before checking its authored numerator and denominator bounds.

An in-range final sum, reordered summands or convenient observed sample cannot
repair a failed declared-domain obligation. No implicit integer/rational or
unit conversion, floating approximation, saturation, deduplication or domain
narrowing is permitted. General rational sum-domain transfer remains required;
an unsupported proof prerequisite records incomplete implementation of this
requirement, not removal of that case from the selected language.

If a required sequence, population member or completeness premise is unavailable, then the evaluator SHALL return an unavailable/incomplete input disposition without substituting an empty domain or a successful aggregate.

The language pipeline SHALL charge checking, predicate-call expansion, actual traversal and retained output against each stage's finite caller-lowered budgets before performing the corresponding work.

If the next charged operation exceeds the run's admitted budget, then the language pipeline SHALL retain resource incompleteness without returning an unfinished query value as complete.

Checking accounting follows [FR-040](FR-040-check-composed-values.md); runtime
accounting must expose its own caller-lowered limits and consumed work. A sequence's
declared maximum is 1..10,000 per admitted wrapper; runtime sequences and filter
results may be empty. Nested finite bounds do not waive total-work bounds.
The stage does not select a new numerical hard limit by observing current code.

Predicate reuse supplies neither temporal/protocol family admission nor
monitor-status truthiness. Graph navigation requires the separately selected
graph definition. General functions, hidden I/O, ambient receivers, mutable
captures, dynamic dispatch and unbounded recursion remain outside this scope.
The derived artifact remains [FR-042](FR-042-publish-compiled-protocol-artifacts.md)'s
wire contract; evaluation does not reparse native source or appoint a second
editable formal representation.

Every value a predicate call or ordered query returns is a member of the one
shared typed value domain that [FR-040](FR-040-check-composed-values.md)
checks over admitted domain-package model declarations
([FR-056](FR-056-admit-domain-package-model-declarations.md)). This
requirement evaluates within that domain and does not define a second value
system. A sum-type value, once some future requirement admits it into that
domain, evaluates through the same predicate-call and ordered-query boundary
this requirement already defines; it does not require a separate query
semantics.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-046-AC-1 | An acyclic predicate shared across state, temporal and protocol call sites retains one exact callee and each site's ordered arguments, source and immutable anchor/capture environment; same-spelled foreign declarations cannot substitute. | Test (TC-126) |
| FR-046-AC-2 | Direct and two-predicate cycles, wrong arity/type/unit, ambiguous calls, non-Boolean roots and unguarded partial operations refuse; a corresponding supported guarded total body admits. Historical no-call and narrower-callee prohibitions remain effective in unused syntax. | Test (TC-126) |
| FR-046-AC-3 | In independently authored Amount 1..20/U, N=5, Total 0..100/U and Count 0..5 domains, evaluating [2,2,3] preserves duplicate ordered map/filter results and returns size=3, count(amount=2)=2 and sum=7. Compilation and admitted-artifact evaluation agree with those independently expected values. | Test (TC-127) |
| FR-046-AC-4 | Invalid requests or supplied values refuse before any query result. Empty inputs produce all eight declared identities; contains/exists/forall stop at their decisive occurrence and ignore later unavailable data, but propagate unavailable data encountered before a decision. Filter/map/count/sum visit each required occurrence and discard partial results on a runtime non-complete outcome. Maximum-admitted and bounded nested inputs retain occurrence order and duplicates. | Test (TC-127) |
| FR-046-AC-5 | Wrong result unit/representation, a count domain omitting an admitted length, N=6 against the example Total, or an out-of-range sum prefix refuses despite an in-range observed final sum. Exact rational prefixes normalize before bounds without approximation; unsupported proof representations remain explicitly outstanding. | Test (TC-127) |
| FR-046-AC-6 | Missing input, unknown required membership and missing completeness produce no successful total; an independently supplied complete counterpart evaluates without reconstructing missing members or consulting an ambient store. | Test (TC-128) |
| FR-046-AC-7 | Zero, exact and one-step-insufficient caller budgets distinguish completed work from resource incompleteness for predicate expansion, nested traversal and retained output; retrying starts fresh accounting without changing the selected semantics or earlier immutable inputs. | Test (TC-128) |
| FR-046-AC-8 | Emitted calls/query records preserve original handles, binder scopes, type/unit selections and captures. Static compiler and admitted-artifact evaluation evidence remain separately identifiable; a passed earlier stage cannot stand in for an unexecuted later stage. | Test (TC-126, TC-127) |

## Dependencies

[FR-036](FR-036-link-composed-native-packages.md) owns binding;
[FR-040](FR-040-check-composed-values.md) owns shared checking/definedness;
[FR-042](FR-042-publish-compiled-protocol-artifacts.md) owns native emission;
[FR-049](FR-049-admit-composed-evaluation-inputs.md) owns runtime input
validation and the result interface; [NFR-009](../non-functional/NFR-009-bound-composed-evaluation.md)
owns runtime accounting.
The admitted domain package declares populations; the observation contract
supplies concrete members, membership and completeness. A missing declaration or
observation fact is an explicit prerequisite, not permission to fabricate it
locally.

This specification cycle is retrospective under
[#66](https://github.com/agent-ix/quire-spec-language/issues/66). Existing
compiler/query-emission work predates this scoped artifact. The subsequently
implemented public evaluator, rational prefix proofs, explicit population
completeness inputs and matrix bindings supply this scoped requirement's local
acceptance; ecosystem acceptance remains separately owned by TC-135.
