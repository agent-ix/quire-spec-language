---
id: FR-036
title: "Link composed packages without inventing runtime premises"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-002
    type: implements
  - target: ix://agent-ix/quire-spec-language/FR-035
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-030
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-031
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-034
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-035
    type: depends_on
---
# FR-036: Link composed packages without inventing runtime premises

## Description

When linking an explicitly inventoried composed package, the compiler SHALL resolve each declaration to its selected definition closure, model exports and typed native dependencies before exposing it to a downstream checker.

## Inputs

The complete native source inventory, parsed units, exact supplied edition/profile
definitions and model exports, selected producer/native correspondence and static
binding contracts. Each package traversal has explicit finite caller-lowered
limits for supplied bytes, units, definitions, declarations, references and
dependency edges; these are processing controls rather than semantic selections.
No filesystem discovery, network retrieval or installed backend supplies an
omitted dependency.

The public package boundary declares its work-accounting version, supported
dimensions, default and hard capacities, and effective caller-lowered limits
before accepting work. Its versioned charging rules identify the bytes and
inventory entries counted, the resolution attempts and dependency-edge visits
charged, and how shared dependencies, cache hits and revisits are charged.
Counters belong to one invocation; a retry starts a new accounting run without
mutating the prior report or inputs. A zero limit permits no charged work in
that dimension. Check a charge before performing its work; counter overflow
also exhausts the budget. The selected accounting contract makes exact-limit
expectations derivable from a controlled input and traversal, not from elapsed
time or available memory. Historical per-unit limits remain separate; they do
not silently define new whole-package capacities.

The [standard package contract](https://github.com/agent-ix/quire-specification/blob/main/proposals/quire-v1/package-contract.md),
landed in [standard PR #15](https://github.com/agent-ix/quire-specification/pull/15),
owns the static/assessment split and producer/native digest domains. Model binding
also requires the affected producer contract. The compiler consumes producer interfaces; it introduces no
parallel model schema, canonicalizer, predicate semantics or observation store.

## Outputs

A package report retaining every requested declaration's source identity,
resolution disposition and dependency causes. Successfully bound declarations
retain exact model/type ownership, native dependency edges and typed binding
requirements. Binding success is not a checked Boolean expression or an executable
backend package; subsequent type/definedness and family checks remain explicit.

## Behavior

The linker SHALL close the native declaration namespace from the explicit source inventory before resolving cross-unit references.

If source headers do not all agree with the inventory's single selected language and edition, then the linker SHALL refuse package namespace admission with the conflicting selections and source header locations retained.

If a required source unit is unavailable or malformed, then the linker SHALL refuse package namespace admission.

The linker SHALL resolve each profile and model alias within its declaring source unit.

If native declarations share a package name or editable source authority, then the linker SHALL refuse the conflicting declarations and their dependents with all conflicting loci.

The linker SHALL validate each selected definition's identity, revision, bytes and transitive requirements before binding its dependent declarations.

If a definition is missing, conflicting or incompatible, then the linker SHALL refuse every dependent declaration with the failed selection retained.

The linker SHALL resolve native references by required declaration kind, exact owner/type and lexical scope.

If a native semantic dependency graph contains a cycle, then the linker SHALL refuse its participating declarations and dependents with the cycle's source occurrences retained.

The linker SHALL retain each derived runtime role's owning declaration, source region, kind, type or contract, anchor and scope premises.

If a required static producer/native correspondence is absent or inconsistent, then the linker SHALL refuse its dependent binding.

If a package processing budget is exhausted, then the linker SHALL report resource_exhausted for unfinished dependent work without claiming complete package admission.

The compiler SHALL retain each requested clause/capability pair when handing a linked subject to downstream processing.

If a downstream checker or backend cannot admit a requested form, then the compiler SHALL retain its typed unsupported disposition without converting the package to complete success.

Collect declarations before resolving forward references. Alias namespaces stay
unit-local; native declaration names are package-wide. Predicate calls, temporal
requirements, operation-contract references and protocol node/compensation
references retain their distinct target kinds. Use the shared grammar's lexical
and capture scopes: a trigger binder unavailable in a later formula cannot be
rescued by another declaration with the same spelling. An expression handle local
to one syntax unit is never a cross-unit identity.

Only semantic definition/call/declaration dependency cycles are refused by this
rule. Explicit bounded protocol repetition, identity-bearing model graphs and
declared related-instance populations are not those cycles. Model graph policy
remains producer-owned. A failed shared dependency invalidates all dependents;
independent declarations remain inspectable. Any unadmitted declaration prevents
the report from claiming complete package admission.

Concrete instances, observations, populations, windows and progress records are
assessment inputs. Their absence does not prevent a valid template from linking.
Each required role still retains its precise model/scope/anchor and authority
requirements for the later binder. Changing assessment inputs or selected backend
does not change static meaning. A changed static definition, type owner or binding
contract requires an explicit new selection. Preserve full compile configuration
provenance separately from static meaning, including resource-control changes.

The historical single-unit APIs and package encodings keep their existing atomic
refusal and identity behavior. Partial composed reports cannot be serialized as
historical complete packages or enter a historical execution API by retagging.
Future composed interchange is governed separately; linking does not introduce
a new wire format as an incidental implementation choice.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-036-AC-1 | A multi-unit state/temporal/protocol package binds exact definition/model selections and typed cross-family references without runtime observations; unit-local aliases may reuse a spelling without merging owners. All headers agree with the inventory's single language/edition; mixed editions or a header/inventory conflict refuse package namespace admission with the conflicting selections located. | Test (TC-114, IT-009) |
| FR-036-AC-2 | Omitted inventory units, duplicate native names/authorities, wrong-kind targets, illegal binder/capture scope and self or mutual semantic dependencies produce located typed refusals; bounded protocol repetition is not falsely diagnosed as a dependency cycle. | Test (TC-114) |
| FR-036-AC-3 | Missing or conflicting definition closure, foreign model exports and canonical-versus-byte digest substitutions refuse every dependent declaration while preserving unrelated bound declarations and their identities. | Test (TC-114, IT-009) |
| FR-036-AC-4 | Binding roles with equal local spelling in two declarations retain different declaration-owned identities; nominally different types and current/activation/invocation anchors remain distinct across families. | Test (TC-114, IT-009) |
| FR-036-AC-5 | Changing runtime population/window/trace or backend leaves static meaning unchanged; changing a required semantic selection changes it. Resource-only configuration changes remain visible without becoming semantic changes. | Test (TC-115) |
| FR-036-AC-6 | A supported state request and an unsupported independent temporal projection both remain in the request report; complete aggregate success is unavailable, and unsupported family bodies are never represented as checked. | Test (TC-115, IT-009) |
| FR-036-AC-7 | A dependency chain, diamond and cycle terminate under declared versioned work accounting and lowered traversal budgets. The accounting contract supplies exact-limit expectations, including shared dependencies and revisits; zero never disables a limit. An unaffordable next charge or counter overflow retains unfinished dispositions and yields no complete or executable package. A retry with sufficient budget preserves the original inputs and prior report. | Test (TC-114) |
| FR-036-AC-8 | Historical linked/package artifacts retain their identities and atomic refusals; a composed partial report cannot enter an old reader or runner by changing a profile label. | Test (TC-115) |

## Dependencies

[FR-035](FR-035-parse-composed-native-units.md) supplies located syntax.
[FR-013](FR-013-link-formal-environments.md),
[FR-015](FR-015-project-native-model-semantics.md) and
[FR-019](FR-019-package-checked-native-clauses.md) supply existing compiler seams.
[IT-009](../integration/IT-009-composed-package-boundary.md) owns the real producer
integration control. L3 predicate evaluation and the family engines may proceed
against the same bound identities; this specification does not invent those engines.

## Status

Under [compiler #35](https://github.com/agent-ix/quire-spec-language/issues/35),
`linking::composed::admit_namespace` implements closed source intake and native
declaration dependency resolution, including forward references, target kinds,
cycles, dependent refusals and versioned work limits. Its constructor-private
syntax namespace retains original units and unfinished dispositions on exhaustion.
Definition closure, model/type and lexical binding, derived runtime roles and
downstream request handling remain outstanding. This stage produces no fully
linked, checked or executable package; IT-009 remains planned producer integration.
