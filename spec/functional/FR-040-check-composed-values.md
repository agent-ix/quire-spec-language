---
id: FR-040
title: "Check composed value types and guarded definedness"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-002
    type: implements
  - target: ix://agent-ix/quire-spec-language/FR-036
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-014
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-041
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-016
    type: traces_to
  - target: ix://agent-ix/quire-specification/FR-033
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-034
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-039
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-040
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-041
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-042
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-043
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-044
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-045
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-046
    type: depends_on
---
# FR-040: Check composed value types and guarded definedness

## Description

When checking a composed binding report, the compiler SHALL establish each dependent value expression's selected profile permissions, exact native type and guarded definedness before exposing that expression as checked.

## Inputs

The immutable `linking::composed::binding::Report`, its original source units,
definition closures, admitted model exports and lexical/anchor bindings; exact
authored formal correspondence needed by the selected proof interface; and finite
caller-lowered checking limits. A declaration must have completed name binding
before dependent type checking. Concrete observations, populations, clocks,
invocation values and backend instances are absent from static checking inputs.

Formal correspondence follows the existing `CheckBindings` boundary: exact
`FormalSource` content/source identity and each authored declaration's
requirement, clause and execution-point selection. A `UnitId`, native source
label or model owner's `RequirementRef` cannot substitute for this correspondence.
Missing, duplicate, foreign or inconsistent bindings prevent dependent discharge;
already established native typing remains inspectable.

The selected StateCore, StateQueries and StateGraph definitions and their
[state contract](../../resources/native-v1/proposals/quire-v1/state-contract.md)
own the value semantics. Shared values and explicitly bound predicate calls in
temporal/protocol guards, captures and obligations use the same rules; their
family-specific admission and execution remain separate stages.

## Outputs

An immutable report retaining original unit/declaration/expression identities,
source byte spans, exact profile and model/type selections, inferred native
types, typed checking causes, available proof/source correspondence and derived
runtime requirements. Profile/type admission, definedness discharge and completed
value checking have distinct inspectable dispositions. Refused and unfinished
declarations retain their dependencies without erasing unrelated checked values.

Typed-only output is not a checked expression. Value checking alone is not
complete temporal/protocol admission, a validated runtime input, a Boolean
assessment, an executable package or a new serialized wire artifact.

## Behavior

The compiler SHALL check profile permissions and type constraints for every authored value expression, including unused predicates and unselected branches.

The compiler SHALL retain each callee's own definition, parameter order, nominal types and immutable anchor environment at every typed predicate call.

If a named predicate body is not total and Boolean over its declared parameter domains, then the compiler SHALL refuse that predicate and its dependent calls.

The compiler SHALL solve literal and expression constraints against exact supplied native types without implicit conversion, inferred producer bounds or a first-fitting imported domain.

When checking a potentially evaluated partial operation, the compiler SHALL establish its range, nonzero or presence obligation under sound preceding native control facts.

If an obligation cannot be proved, then the compiler SHALL retain an unproved-definedness cause without manufacturing a logical counterexample.

If the selected producer or proof interface cannot represent a required meaning, then the compiler SHALL retain a typed unsupported-prerequisite cause identifying that meaning and its original selection.

The compiler SHALL derive declaration-owned typed runtime requirements from checked values and authoritative contracts without supplying concrete assessment data.

If an upstream or checking dependency refuses, then the compiler SHALL prevent every dependent value from being exposed as checked while preserving unrelated completed results.

The following obligations apply together; an implementation subset does not
redefine the selected profile's complete admitted forms.

| Concern | Required checking meaning |
| --- | --- |
| Native types and operators | Reuse `checking::NativeType` and admitted producer declarations where compatible. Preserve model/type/scalar identity, exact unit, finite bounds, enum and Unicode text rules, optional payload type, ordered sequence element/max and object/reference universe. Equal spellings or structures do not merge nominal owners. Only admitted equality/order and numeric operators pass; whole-record/sequence/option equality, Boolean/enum order, integer division/remainder and implicit conversions refuse even in unreachable syntax. |
| Exact numbers | Integer proofs preserve authored inclusive signed-64 domains without wrap, saturation or silent widening. `rational(n,d)` rejects zero denominator, normalizes sign/gcd and zero to 0/1 before authored bounds, with unique expected rational type. Rational operations normalize exact results before bounds; rational division proves its exact divisor nonzero in the same named dimensionless type. Ordinary operands/results share their named type/unit; multiplication requires dimensionless operands. Representation gaps never authorize float approximation or removal of rational obligations. |
| Predicates and ordered queries | Preserve exact argument arity/order/type and Boolean roots without ambient self, I/O or monitor-status truthiness. Check `size`, `contains`, `forall`, `exists`, `filter`, `map`, `count` and `sum` with their original binders and ordered duplicate-preserving sequence shape, including nested queries and maxima through 10,000. Size/count require a dimensionless result domain containing 0..N. Sum's explicit result domain has the same representation/unit, admits zero and all projections, and proves every accumulated prefix; a final in-range sum or a small observed sample cannot repair a failed obligation. |
| Control and presence | Preserve left-to-right short-circuit and conditional facts, single immutable let initialization and only facts valid at all joined alternatives. A guard applies to the same value/field/receiver/observation identity; later guards, another object or a repeated expression spelling cannot prove it. Optional absence, producer missingness and missing observations remain distinct. |
| Anchors and captures | Preserve declaration-owned current/activation/protocol/compensation and invocation pre/post anchors. Apply the selected `pre(expr)` rules without replaying or retagging an outer capture; invocation parameters stay immutable and result stays post-only. A pre-qualified reference retains its observation for navigation even when identity comparison spans pre/post. |
| Graphs and runtime requirements | Admit only the selected graph interface and exact object/reference/universe and eligible edge shape. Retain required finite population, target closure/completeness, context, operation/frame, observation, membership and capture contracts with owning declaration/source region, kind, type/authority, anchor and scope. Reference cycles are not predicate recursion. No ambient lookup, UUID cast or fabricated relationship supplies missing authority. Current, pre/post and activation roles with equal local names remain distinct. |
| Proof correspondence | Reuse `DeclarationEnvironment::check_expression` and existing native proof abstractions only where they preserve exact native types, guards, source and selected execution point. Every discharged goal retains its actual upstream result and native correspondence. A proof for another source, lexical owner, observation or declaration cannot discharge this one. Unsupported representations and incomplete correspondence stay explicit; no synthetic semantic owner, reparsed source or successful type check supplies proof. |

## Work accounting

The compiler SHALL expose a versioned checking-accounting contract, finite default and hard capacities, effective caller-lowered limits and consumed work in every report.

The public contract defines deterministic stage/traversal order and the following
charges before their work. Retained inputs may be borrowed; copying them cannot
evade the byte or structure charges. Parser and binding budgets remain separate.

| Work | Charged unit |
| --- | --- |
| Bytes | Each offered formal-correspondence metadata/content byte once on intake, plus each subsequent byte copied, hashed or scanned by this stage. |
| Declarations and expressions | Each visited declaration and each visited native value/control occurrence; repeated visits and callee instantiations count again. |
| Type and dependency work | Each constraint creation/inspection, type-wrapper traversal/copy, resolution attempt and native call/dependency-edge visit, including shared dependencies and cache hits. |
| Proof and guard work | Each inspected/propagated fact, generated symbolic value/graph node, proof goal and materialized IR node, with both per-goal and total expansion limits. Rational normalization/arithmetic steps have explicit charged work before computation. |
| Retained output | Each created type/result/cause/runtime-role record and retained dependency or provenance entry, before allocation/copy. |
| Depth | The active native/type/predicate/proof depth is checked before descent or expansion; bounded input size alone does not waive depth limits. |

If the next charge exceeds an effective limit or overflows its counter, then the compiler SHALL stop unfinished checking work with typed resource exhaustion retaining the dimension, limit, prior usage and source-owned next operation.

Zero permits no charged work in its dimension. An exhausted stage cannot expose
unfinished values as checked or erase a known refusal. A retry starts fresh
counters and preserves prior inputs/results. Resource settings remain visible
configuration provenance, separate from selected semantic identities.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-040-AC-1 | Exact multi-unit predicate/state inputs and shared values in temporal/protocol bodies retain unit-qualified expression types and source spans; names-resolved, typed, proof-discharged and family/execution dispositions cannot be interchanged. Refused or unfinished input produces no dependent checked value. | Test (TC-119) |
| FR-040-AC-2 | StateCore refuses predicate/query-extension/graph forms where absent from its catalog; StateQueries admits its typed extension while refusing graph forms; StateGraph admits its eligible graph forms. Unused or unreachable prohibited forms still refuse and a wider caller cannot upgrade a callee. | Test (TC-119) |
| FR-040-AC-3 | Exact parameter arity/order, nominal model/scalar/unit identity, enum/text/option/sequence constraints and Boolean roots distinguish valid inputs from same-spelled foreign types, ambiguous literals, wrong arguments and forbidden coercions or status-as-Boolean. | Test (TC-119) |
| FR-040-AC-4 | Inclusive integer extrema and guarded arithmetic preserve exact bounds. Rational normalization-before-bounds, zero denominator, exact division/nonzero guards and result-domain failures follow the selected contract. Missing producer/proof representation yields an explicit unsupported prerequisite and leaves the corresponding full-feature acceptance outstanding. | Test (TC-119) |
| FR-040-AC-5 | All eight ordered query forms retain their exact binder/result shape. N=0 and N=10,000 are checked against authored maxima; N=10,001 refuses. The declared Amount 1..20/U, N=5, Total 0..100/U, Count 0..5 case checks; N=6, wrong units/representation or an unproved intermediate prefix refuse despite a convenient sample/final sum. | Test (TC-119) |
| FR-040-AC-6 | Guard-before-use and immutable aliases discharge supported presence/range obligations; reversed guards, invalid alternative joins, foreign receiver/observation and pre-retagged captures refuse at their original loci. Valid composite pre reads and same-invocation immutable parameters retain distinct provenance. | Test (TC-119) |
| FR-040-AC-7 | Graph/context/operation/capture uses emit typed declaration-owned runtime requirements with exact universe, anchor, scope/completeness and frame selections without observations. Same-named roles, foreign universes and opaque scalar/relationship substitutions cannot satisfy them. | Test (TC-119) |
| FR-040-AC-8 | Exact authored formal correspondence permits actual supported IR discharge. Omitted, duplicate, changed-source, foreign-declaration or wrong-execution-point correspondence prevents dependent proof without synthetic ownership. Unproved definedness, unsupported prerequisite and resource exhaustion retain distinct typed causes; typing remains inspectable. | Test (TC-119) |
| FR-040-AC-9 | Independently calculated small chain/diamond, nested-query and proof-expansion vectors distinguish zero, exact and one-step-insufficient limits in every public dimension, including output retention and overflow. Shared work/retries obey the published charges; an exhausted or refused dependent remains unadmitted beside unrelated completed work. | Test (TC-119) |
| FR-040-AC-10 | Historical checker/profile/package fixtures retain their original identities, successes and atomic refusals. Composed typed/partial reports cannot enter historical checked/execution interfaces by retagging or introduce an implicit wire contract. | Test (TC-119) |

## Dependencies

[FR-036](FR-036-link-composed-native-packages.md) supplies the binding report;
[FR-014](FR-014-bind-native-formal-source.md) and
[FR-016](FR-016-check-native-clauses.md) provide existing formal-source, native
type and IR proof seams without changing their historical contract.
[TC-119](../test-cases/TC-119-check-composed-values.md) is the planned compiler
test case; [IT-009](../integration/IT-009-composed-package-boundary.md) retains
real external-producer integration ownership.

Compiler [#35](https://github.com/agent-ix/quire-spec-language/issues/35),
[#36](https://github.com/agent-ix/quire-spec-language/issues/36) and
[#39](https://github.com/agent-ix/quire-spec-language/issues/39) retain full
delivery obligations. The pinned IR already supplies rational types, literals,
arithmetic and definedness. [FR-041](FR-041-admit-rational-native-model-profile.md)
specifies the explicit `/2` native model producer extension while historical
`/1` admission retains its rational refusal. Composed definedness consumes those
exact representations with the original authored correspondence. Query-body
safety covers the admitted element domain; sum safety requires every bounded
prefix. Broader rational sum-domain transfer remains an explicit unsupported
prerequisite where the selected proof interface cannot establish it. This does
not remove rational sums from the requirement. The existing native scalar type
retains the underlying IR value type; a second numeric authority is unnecessary.
D's exact producer/native correspondence and authoritative relationship exports
remain producer-owned prerequisites. Unsupported prerequisites are implementation
gaps, not a narrower definition of this requirement; reference evaluation,
family checks, runtime validation and full artifact delivery remain downstream.
