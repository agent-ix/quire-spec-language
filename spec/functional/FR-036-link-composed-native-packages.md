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
  - target: ix://agent-ix/quire-spec-language/FR-057
    type: depends_on
---
# FR-036: Link composed packages without inventing runtime premises

## Description

When linking an explicitly inventoried composed package, the compiler SHALL resolve each declaration to its selected definition closure, admitted domain-package model declarations and typed native dependencies before exposing it to a downstream checker.

`qsl_replay::spine::StaticSubject` (FR-510) retains the package contract's six
declared static components — Sources, Language, Profiles, Models, Native
dependencies and Binding requirements — and compares two subjects component by
component. Nothing is derived from a subject beyond that comparison;
canonical identity remains unavailable until its exact form and version
are selected and qualified. `BuildProvenance` retains the full
compile configuration separately, so a resource-only change is visible there and
in no static component.

Profile and definition selections resolve against the closed definition
catalog under FR-110; callers explicitly supply the artifacts they select, and
the catalog is not a Markdown reader or an implicit source of omitted
dependencies. Domain-package model declarations bind by declaration key and
export kind; a same-shaped record from another domain package grants no
authority. Domain-package intake belongs to #131.

## Inputs

The complete native source inventory, parsed units, exact supplied edition/profile
definitions, the admitted domain packages from
[FR-056](FR-056-admit-domain-package-model-declarations.md) with their model
declarations and export records, and static binding contracts. Each package traversal has explicit caller-configured
limits, each with a published default and used as given with no ceiling, for supplied bytes, units, definitions, declarations, references and
dependency edges; these are processing controls rather than semantic selections.
No filesystem discovery, network retrieval or installed backend supplies an
omitted dependency.

Package-wide capacities are separate from the historical per-unit limits and
are named at each entry point under FR-255. A zero limit permits no charged
work in its dimension, a charge is checked before its work is performed, and a
retry starts a new accounting run without mutating the prior report or inputs.

The [standard package contract](https://github.com/agent-ix/quire-specification/blob/main/proposals/quire-v1/package-contract.md),
landed in [standard PR #15](https://github.com/agent-ix/quire-specification/pull/15),
owns the static/assessment split. Model declarations come
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

For spine compilation, `qsl_replay::spine::check` SHALL retain a typed
per-use profile outcome in its checked result. The outcome SHALL distinguish
the selected profile identity, required use-site kind, definition-closure
status and any typed closure cause; closure completion SHALL remain distinct
from use-site admission.

When a protocol use selects `quire.observation.binding/v1`, the spine checker SHALL retain its exact definition closure, including the registered
`quire.protocol.finite-global/v1` dependency and that definition's transitive
requirements. The spine checker SHALL separately report `WrongProfileKind`
for this protocol use because observation binding is not a protocol profile.

When the required protocol definition is omitted, conflicting or incompatible,
the spine checker SHALL retain the corresponding typed closure cause in the
per-use outcome and mark the protocol declaration refused or incomplete. The
spine checker SHALL NOT report a complete checked package while this closure
failure remains.

The linker SHALL resolve native references by required declaration kind, exact owner/type and lexical scope.

If a native semantic dependency graph contains a cycle, then the linker SHALL refuse its participating declarations and dependents with the cycle's source occurrences retained.

The linker SHALL retain each derived runtime role's owning declaration, source region, kind, type or contract, anchor and scope premises.

If a native model reference names a domain package, declaration key or export kind that the admitted domain package does not contain, then the linker SHALL refuse its dependent binding.

If a package processing budget is exhausted, then the linker SHALL report resource_exhausted for unfinished dependent work without claiming complete package admission.

Requested clause/capability pairs, their admitted kinds and their settled dispositions are owned by [FR-057](FR-057-admit-shared-capability-kinds.md); a downstream family checker's refusal of a requested form is retained at the pair's request index without converting the package to complete success, and the linker reads no backend state itself.

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

The linker SHALL bind a native reference to a domain declaration by its FR-154 key and kind.

The linker SHALL bind a type site to an object, interface, record value or value type.

The linker SHALL bind a protocol role to a Part or Port in addition to the type-site kinds.

The linker SHALL bind a protocol relationship to a Connection or navigation relationship.

The linker SHALL refuse a native reference to a declaration of any other kind.

The type checker SHALL type a domain object type, Interface or record value type, and a field read by the field's value type (`Boolean`, or another such domain type, under multiplicity `1`, `0..1` or `0..n`).

The type checker SHALL refuse any other field type, and a domain value type, as an unsupported prerequisite.

**QSL-499 disposition:** the composed linker (`linking::composed` namespace, binding, definition-closure, model-binding and request-report stages) is deleted under ADR-011, ADR-033 and FR-499; the spine check (`qsl_replay::spine::check`) is the single path. The retired criteria below are retained as migration history and are not active acceptance obligations. Their behavior is owned as follows: header and definition selection by FR-110 and FR-056 (AC-1, AC-3); scoped names, anchors, duplicate and cyclic declarations by FR-113 and FR-218 (AC-2, AC-4); requested clause/capability pairs by FR-057 (AC-6); work accounting by FR-218-AC-36 and FR-255 (AC-7). FR-036-AC-5, AC-8 and AC-9 stay active.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-036-AC-1 | **RETIRED in QSL-499; historical only.** A multi-unit state/temporal/protocol package binds exact definition/model selections and typed cross-family references without runtime observations; unit-local aliases may reuse a spelling without merging owners. All headers agree with the inventory's single language/edition; mixed editions or a header/inventory conflict refuse package namespace admission with the conflicting selections located. | Test (TC-114) |
| FR-036-AC-2 | **RETIRED in QSL-499; historical only.** Omitted inventory units, duplicate native names/authorities, wrong-kind targets, illegal binder/capture scope and self or mutual semantic dependencies produce located typed refusals; bounded protocol repetition is not falsely diagnosed as a dependency cycle. | Test (TC-114) |
| FR-036-AC-3 | **RETIRED in QSL-499; historical only.** Missing or conflicting definition closure, a declaration from a foreign domain package and an artifact-identity substitution refuse every dependent declaration while preserving unrelated bound declarations and their identities. | Test (TC-114) |
| FR-036-AC-4 | **RETIRED in QSL-499; historical only.** Binding roles with equal local spelling in two declarations retain different declaration-owned identities; nominally different types and current/activation/invocation anchors remain distinct across families. | Test (TC-114) |
| FR-036-AC-5 | Changing runtime population/window/trace or backend leaves static meaning unchanged; changing a required semantic selection changes it. Resource-only configuration changes remain visible without becoming semantic changes. | Test (TC-115) |
| FR-036-AC-6 | **RETIRED in QSL-499; historical only.** Two required requested pairs over different FR-057 capability kinds both remain in the request report in caller order; when either is not settled `supported`, complete aggregate success joined from the FR-331 accounting records on request index is unavailable, and a family body refused by its checker is never represented as checked. | Test (TC-115) |
| FR-036-AC-7 | **RETIRED in QSL-499; historical only.** A dependency chain, diamond and cycle terminate under declared versioned work accounting and lowered traversal budgets. The accounting contract supplies exact-limit expectations, including shared dependencies and revisits; zero never disables a limit. An unaffordable next charge or counter overflow retains unfinished dispositions and yields no complete or executable package. A retry with sufficient budget preserves the original inputs and prior report. | Test (TC-114) |
| FR-036-AC-8 | Through `qsl_replay::spine::check`, the typed per-use profile outcome for `protocol P using ob`, where `ob` selects `quire.observation.binding/v1`, retains the selected identity, completed closure status, exact registered `quire.protocol.finite-global/v1` dependency and its transitive requirements; use-site admission separately refuses `WrongProfileKind`. If the required protocol definition is omitted, conflicting or incompatible, that outcome retains the corresponding typed closure cause, the declaration remains refused or incomplete, and no complete checked package is reported. | Test (TC-114) |
| FR-036-AC-9 | Native model references to a field, a Port and a Connection of a domain package admitted under FR-056 bind to their exact declaration keys and kinds, and a declaration of a kind the site does not bind refuses at the linker; a same-shaped declaration from another domain package or a changed selected identity or version refuses each dependent declaration while unrelated declarations stay bound. | Test (TC-148) |

## Dependencies

[FR-035](FR-035-parse-composed-native-units.md) supplies located syntax.
[FR-013](FR-013-link-formal-environments.md) and
[FR-015](FR-015-project-native-model-semantics.md) supply existing compiler seams.
[FR-056](FR-056-admit-domain-package-model-declarations.md) supplies the admitted
domain-package model declarations, and
[IT-012](../integration/IT-012-domain-package-model-intake.md) owns the real
quire-rs and FCD crate integration. L3 predicate evaluation and the family engines may proceed
against the same bound identities; this specification does not invent those engines.
