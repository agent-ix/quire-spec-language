# Linked packages and assessment bindings

FS05 accepts the package-reference and evolution portion of this consumer
contract for [FR-030](../../spec/functional/foundation/FR-030-bind-composed-definitions.md)
and [FR-035](../../spec/functional/foundation/FR-035-bind-ecosystem-subjects.md),
for the composed `1-draft` definition. Runtime/assessment portions retain their
own tickets. This acceptance does not register a public wire format or
canonicalizer, qualify an executable linked-package reader, adopt a clause
profile, or replace the model intake of
[AD-006](../../spec/assurance/AD-006-model-graph-state-contracts.md), F's
observation or B's request/result contracts.

## Static package subject

An explicit compiler input inventory supplies the complete native source-unit
set and exact locally supplied dependencies. It identifies the language/edition
definition, which must agree with every source header. Directory contents,
installed tools and retrieval locations cannot add units or select dependencies.
All units use one edition. Aliases are unit-local; native declaration names share
the closed package namespace. A missing source unit prevents establishing that
namespace. A missing model/profile refuses its dependents without erasing
independent declarations whose names and dependencies are established.

| Static component | Information retained and checked |
| --- | --- |
| Sources | Exact authority, identity, revision and byte digest per unit; original source regions and clause/declaration identities. Two editable authorities for one clause refuse. |
| Language | Exact language/edition and supplied definition artifact; no implementation-version default. |
| Profiles | Each declaration's selected root definition name/revision/byte digest, exact required definition closure and the meaning owned by each dependency. |
| Models | Domain package imported by identity, version and `sha256-jcs` digest; its effective declaration keys and exact manifest/lock/dependency closure. |
| Native dependencies | Resolved predicate, operation-contract and temporal-obligation references with their defining source, model and profile selections; typed anchors and capture requirements. |
| Binding requirements | Typed roles, selected model/observation contracts and clock/scope premises described below. Concrete runtime instances are not inserted here. |

The native `model` import names a domain package by identity, version and
`sha256-jcs` digest. The domain package is the semantic IR document lifted from the imported
spec's artifacts. Manifest/lock digests, package fingerprints and source digests
remain separate. The domain package identity and its declarations must match the
supplied document and its validated closure. A transitively reachable type is
not thereby a directly imported type. The spec's artifacts declare model closure,
presence, units, relationships and configuration; QSL model intake checks them
and refuses unsupported meaning. The minimum-only ConfigVersion
domain and parent relationship remain admission controls, not implicit finite
bounds or fields.

Each profile definition names its exact required definitions and the forms,
value rules and interpretation it supplies. Transitive selections are validated
before dependent declarations are published as checked. Definition cycles,
conflicting content for one immutable definition identity, missing requirements
and incompatible requirements within one selected closure refuse. Installing a
profile cannot grant operators. Declarations can select different compatible
interfaces; a referenced predicate retains its own checked definition rather
than acquiring caller permissions. This definition-dependency rule does not
prohibit identity-bearing model graphs or redefine the domain package dependency policy.

Partial compilation retains every requested declaration's disposition and
dependency cause. Only dependency-closed checked declarations reach a consumer;
partial output cannot claim whole-package admission. An unrelated backend's lack
of support is a request disposition, not a static source failure.

The complete value root `quire.value.complete/v1` uses the closed semantic
selections in [AD-005](../../spec/assurance/AD-005-complete-value-expression-system.md).
Every package selecting that root retains `quire.value.accounting/v1` and its
exact supplied definition bytes; no evaluator or backend may substitute a local
cost model. It retains `quire.value.compound-unit/v1` with its exact schema and
vector artifact digests. It also retains `quire.value.complete.rules/v1` plus every exact
rule artifact named by that manifest as separate domain-tagged DefinitionRefs;
the root digest alone does not freeze transitive meaning.
Text-bearing packages retain `quire.value.text.unicode-17.0.0/v1`; integer
`div`/`rem` retains exactly one truncating, floor or Euclidean definition; IEEE
operations retain `quire.value.ieee754-2019-default/v1`. Those supplied
definition bytes, not a runtime library version, select the meaning. `mod`
always denotes Euclidean remainder and cannot be retagged by the package's
`div`/`rem` selection.

## Typed binding requirements

A required role is identified by its owning native declaration, original source
region and role kind. Its local name is retained for diagnostics, not used as a
global lookup key. Two `"refund-clock"` roles in different clauses remain distinct
even when an explicitly supplied clock satisfies both.

| Role information | Required interpretation |
| --- | --- |
| Type and authority | Exact effective declaration key or selected binding contract; kind distinguishes value input, invocation, workflow/participant, relationship, population and clock/progress. Equal field shapes do not substitute for identity. |
| Anchor | Current observation, exact invocation pre/post, temporal origin/activation, protocol occurrence, compensation registration or activation, as authored. |
| Scope premises | Workflow/role/relationship endpoints, snapshot or window/membership contract, clock kind/unit/history and completeness authority where applicable. |
| Dependencies | Prerequisite roles, supplied record correspondence and immutable captures. A clock, population and its progress record must describe the same declared scope. |

Roles derive from checked native declarations, imported domain package
declarations and selected binding contracts. They introduce no field grammar, event schema or model store. The
linker retains their contract selections; it does not infer an adapter from a
binder's spelling. A semantic change to an imported binding contract requires a
new static selection. Switching between admitted concrete bindings satisfying
the same contract is an assessment input change.

The runtime request maps each required role to one binding under F/B's selected
contracts. Repeated entries for the same role refuse, even when equal; one
explicit concrete binding may serve several compatible roles. Missing or
incompatible mappings refuse the affected request. Missing observations under a
valid mapping remain incomplete when the obligation requires them. An absent
observation is not a model's optional-none value.

Binding a template does not require inventing future events. A recovery role can
be mapped before compensation registration or activation. Absence of a recovery
record cannot fail a prerequisite of an unactivated recovery obligation.
Activation, valid open-prefix pending, missing required evidence and inactivity
at closure remain distinct under B/E/F's rules. Captures bind once at the
authored anchor; mapping another role cannot retag them or supply another
workflow's values.

## Configuration and assessment identity

### Prerequisites by processing stage

The linker SHALL derive concrete-input requirements without requiring their
future observations to exist. The assessment binder SHALL check the inputs
required by the selected claim when that claim consumes them. These obligations
apply equally to state, temporal and choreography declarations.

| Stage | Required selections and checks | Inputs not required at this stage |
| --- | --- | --- |
| Source recognition | Explicit source inventory, original bytes, edition and lexical/parser limits | Model instances, runtime populations, snapshots, windows, observations and progress |
| Resolution and type/profile admission | Exact domain package declarations and definition closure; selected binding contracts; typed anchor, scope and clock roles; static configuration selections consumed here | Concrete workflow instances, runtime membership, a future snapshot/window, progress/closure records or compensation observations |
| Assessment binding | An admitted subject; exact consumed runtime configuration; matching concrete role bindings; the finite population/snapshot/window and authority required by the selected assessment | Unrelated scope inputs and observations needed only by an unactivated obligation |
| Evaluation and result publication | The admitted inputs and exact decision support required by the selected B/E result rule, with relevant availability, progress and completeness retained separately | Complete surrounding execution or unrelated observations when the selected rule admits decisive truth without them |

Absence of a required static definition or domain declaration refuses the dependent linking.
An unknown, conflicting or incompatible runtime selection refuses the dependent
binding; unavailable observations under a valid selected binding remain explicit
incomplete inputs. Static success does not waive those later prerequisites.
Missing support cannot become a value. Conversely, an incomplete-input
classification cannot erase a truth already justified by the evaluator's complete
exact support; it remains visible on the affected coverage/completeness dimension.

Which runtime scope is required follows the exact profile and role. A timestamped
window and its authority do not substitute for event-position or fixed-sample
inputs. The F/E observation-range contract supplies any permitted range mapping;
this stage table does not invent one, require UTC for every clock, or infer a
window from current time. Evaluators still require their own closure premises
before publishing complete global success.

### Retained configuration

Retain the complete, exact configuration artifact consumed at each stage. Its
semantic selections must agree with source and linked requirements. A runtime
configuration cannot change a linked model, profile or binding-contract meaning:
such a request refuses and requires an explicitly newly linked subject. Concrete
adapters, populations, traces, clocks, progress, implementations, requested claims
and resource controls remain identified assessment inputs.

The static subject is compared through the components above. Build provenance
retains the full compile configuration and implementation; assessment provenance
retains the full runtime configuration and inputs. Changing only a resource limit
changes configuration and assessment input identity while preserving static
meaning. Changing a
selected domain package, profile or required binding meaning changes the static subject
as well. A projected configuration cannot stand in for the original input.

Exact serialized bytes, static language subject and assessment inputs have
distinct identities. Reordering a set in a permitted encoding may preserve its
subject while changing raw bytes. Source-byte changes alter source correspondence
even if a typed-expression canonicalizer establishes equal expression meaning.
Canonical identity is optional until its exact domain/version/algorithm is
selected and qualified; structural comparison or ordinary JSON hashing cannot
claim it. Existing ArtifactRef/SemanticRef, domain package `sha256-jcs` and IR domains
retain their selected contracts. Definition digests cover exact supplied bytes;
an artifact need not contain its own byte digest.

## Admission examples

Use a three-family order package with two typed per-observation inputs and a
refund-clock role. Bind O1 and O2 through distinct workflow/relationship instances
even when they share a provider and clock authority. Swapping O2's progress into
O1's population must fail the scope check. Raising the run budget preserves the
static subject; changing the temporal boundary profile requires a new linked
subject. Unactivated compensation needs no fabricated refund record.
[IT-010](../../spec/integration/IT-010-bind-linked-package-assessments.md) specifies
these controls; none has executed yet.

The spec's artifacts declare presence, relationships, populations and
configuration, and the domain package carries them. The
[observation contract](observation-contract.md) keys runtime subjects on the
resulting declaration keys. The integrated observation
range correspondence covers event-position, fixed-sample and timestamp inputs,
distinct record/member identities and half-open coverage versus inclusive
deadlines. Decisive truth retains its exact support independently of unrelated
incompleteness.

The FS05 reference/evolution rules are accepted; the remaining runtime and
assessment rules stay in the [current shared draft](definitions/README.md).
Further draft successor definitions or
checksum publication are not prerequisites. Historical accepted selections
retain their original meaning.
