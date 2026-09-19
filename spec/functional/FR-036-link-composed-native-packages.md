---
id: FR-036
title: "Link composed packages without inventing runtime premises"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-002
    type: implements
  - target: ix://agent-ix/quire-spec-language/FR-035
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-030
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-031
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-034
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-035
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-290
    type: references
---
# FR-036: Link composed packages without inventing runtime premises

## Description

When linking an explicitly inventoried composed package, the compiler SHALL resolve each declaration to its selected definition closure, admitted domain-package model declarations and typed native dependencies before exposing it to a downstream checker.

## Inputs

The complete native source inventory, parsed units, exact supplied edition/profile
definitions, the admitted domain packages from
[FR-056](FR-056-admit-domain-package-model-declarations.md) with their model
declarations and export records, and static binding contracts. Each package traversal has explicit finite caller-lowered
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
owns the static/assessment split and digest domains. Model declarations come
only from domain packages admitted under FR-056; the linker introduces no
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

If a native model reference names a domain package, declaration key or export kind that the admitted domain package does not contain, then the linker SHALL refuse its dependent binding.

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
declared related-instance populations are not those cycles. Model graph meaning
is Quire's, over declarations admitted under FR-056. A failed shared dependency invalidates all dependents;
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
| FR-036-AC-1 | A multi-unit state/temporal/protocol package binds exact definition/model selections and typed cross-family references without runtime observations; unit-local aliases may reuse a spelling without merging owners. All headers agree with the inventory's single language/edition; mixed editions or a header/inventory conflict refuse package namespace admission with the conflicting selections located. | Test (TC-114) |
| FR-036-AC-2 | Omitted inventory units, duplicate native names/authorities, wrong-kind targets, illegal binder/capture scope and self or mutual semantic dependencies produce located typed refusals; bounded protocol repetition is not falsely diagnosed as a dependency cycle. | Test (TC-114) |
| FR-036-AC-3 | Missing or conflicting definition closure, a declaration from a foreign domain package and `sha256-jcs`-versus-raw-byte digest substitutions refuse every dependent declaration while preserving unrelated bound declarations and their identities. | Test (TC-114) |
| FR-036-AC-4 | Binding roles with equal local spelling in two declarations retain different declaration-owned identities; nominally different types and current/activation/invocation anchors remain distinct across families. | Test (TC-114) |
| FR-036-AC-5 | Changing runtime population/window/trace or backend leaves static meaning unchanged; changing a required semantic selection changes it. Resource-only configuration changes remain visible without becoming semantic changes. | Test (TC-115) |
| FR-036-AC-6 | A supported state request and an unsupported independent temporal projection both remain in the request report; complete aggregate success is unavailable, and unsupported family bodies are never represented as checked. | Test (TC-115) |
| FR-036-AC-7 | A dependency chain, diamond and cycle terminate under declared versioned work accounting and lowered traversal budgets. The accounting contract supplies exact-limit expectations, including shared dependencies and revisits; zero never disables a limit. An unaffordable next charge or counter overflow retains unfinished dispositions and yields no complete or executable package. A retry with sufficient budget preserves the original inputs and prior report. | Test (TC-114) |
| FR-036-AC-8 | Historical linked/package artifacts retain their identities and atomic refusals; a composed partial report cannot enter an old reader or runner by changing a profile label. | Test (TC-115) |
| FR-036-AC-9 | Native model references to a field, a Port and a Connection of a domain package admitted under FR-056 bind to their exact declaration keys and kinds; a same-shaped declaration from another domain package or a changed selected digest refuses each dependent declaration while unrelated declarations stay bound. | Test (TC-148) |

## Dependencies

[FR-035](FR-035-parse-composed-native-units.md) supplies located syntax.
[FR-013](FR-013-link-formal-environments.md),
[FR-015](FR-015-project-native-model-semantics.md) and
[FR-019](FR-019-package-checked-native-clauses.md) supply existing compiler seams.
[FR-056](FR-056-admit-domain-package-model-declarations.md) supplies the admitted
domain-package model declarations, and
[IT-012](../integration/IT-012-domain-package-model-intake.md) owns the real
quire-rs and FCD crate integration. L3 predicate evaluation and the family engines may proceed
against the same bound identities; this specification does not invent those engines.

## Status

Under [compiler #35](https://github.com/agent-ix/quire-spec-language/issues/35),
`linking::composed::admit_namespace` implements closed source intake and native
declaration dependency resolution, including forward references, target kinds,
cycles, dependent refusals and versioned work limits. Its constructor-private
syntax namespace retains original units and unfinished dispositions on exhaustion.
`linking::composed::binding::bind` composes exact definition/rule closure,
NativeModel imports and nominal exports, lexical/capture environments, protocol
structural references and dependency-local refusal propagation. Declaration-owned
binders retain original types and anchors. It accepts no runtime observations.

The compiler recognizes a closed registry of the reviewed baseline's exact
definition and rule bytes; callers explicitly supply those artifacts. The
registry is implementation support, not a Markdown reader or an implicit source
of omitted dependencies. Its normative resources preserve the selected standard
bytes and original licensing. The historical native-state-model/1 rule-model
source binds through NativeModel. Domain-package model declarations bind by
declaration key and export kind; a same-shaped record from another domain
package grants no authority. Remaining work: #131 for domain-package intake.

`linking::composed::subject::StaticSubject` retains the package contract's six
declared static components — Sources, Language, Profiles, Models, Native
dependencies and Binding requirements — and compares two subjects component by
component. No canonical digest, structural hash or JSON fingerprint is derived;
canonical identity remains unavailable until its exact domain, version and
algorithm are selected and qualified. `subject::BuildProvenance` retains the full
compile configuration separately, so a resource-only change is visible there and
in no static component.

`linking::composed::requests` records a typed disposition for every requested
clause/capability pair against a caller-declared backend and the supplied
assessment inputs. Unsupported capability, unsupported family, inapplicable
capability, refused subject, unfinished subject and unknown subject are distinct
outcomes; none is dropped or merged, and a required non-admitted request makes
complete aggregate success unavailable. `admitted_bodies` exposes only admitted
family-check subjects, so an unsupported family body is never represented as
checked. Assessment selections and backend support are retained as provenance and
are never written back into the static subject.

The requested claim vocabulary's target design is
[quire-specification `FR-290`](https://github.com/agent-ix/quire-specification/blob/main/spec/objects/protocol/FR-290-protocol-claim-kind.md)'s
six-member closed set — `global-conformance`, `monitorability`,
`local-projection`, `refinement`, `realizability` and `composition` — which
`agent-ix/quire-specification#116` fixes as the shared claim-kind authority
this requirement aligns to. The current `linking::composed::requests::Capability`
implementation instead retains four link-stage request labels (`FamilyCheck`,
`StateOperation`, `FiniteReplay`, `TemporalProjection`) that predate this
alignment; they identify what a downstream backend is asked to admit at the
link boundary, not which of FR-290's six protocol claim kinds a later stage
dispatches. Remaining work: #185 aligns the implementation's vocabulary to
FR-290's six kinds.

`NamesResolved` precedes expression/type/profile checking and complete typed
runtime requirements. TC-114 exercises common nominal types across state,
temporal and protocol families and declaration-owned capture and instance
identities over NativeModel exports; TC-148 and IT-012 bind native model
references to admitted domain-package declarations (planned under #131); TC-115 keeps an
explicit unsupported temporal projection beside an admitted state request. An admitted request is a
handoff record, not a checked or executable clause; no result from this stage is
a checked or executable package. The historical package path remains separate.
