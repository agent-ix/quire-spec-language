# Observation binding and assessment-input contract

Draft F-owned input to OB01–03 and the composed `1-draft` baseline. This defines
the semantic content an observation producer and its temporal/protocol consumers
must exchange. It does not register a wire encoding, canonicalizer, model schema,
result store, or clause authority. A owns source/package/profile identities; D
owns model, relationship, population, and configuration identities; E owns clock,
history, and closure meaning; B owns obligations and result dispositions.

## Contract selections

Every assessment input selects immutable identities for the native linked package,
the observation binding-contract definition and revision, each observation
source/schema, and the applied binding. A compatible field shape, signal name,
trace ID, endpoint, or adapter version does not substitute for any selected
identity. Changing the selected binding-contract definition or revision, including
any change to its binding meaning, requires a newly linked subject. Changing only
the concrete mapping, admitted records, or declared resource limits under the
same selected definition and revision changes assessment-input identity, not
static meaning.

This working draft uses the [current shared specification](definitions/README.md)
and its [range correspondence](definitions/observation-range.md).
Runtime binding/progress identities are supplied by the linked package; this
rule document does not select itself through a copied candidate/digest table.

## Producer interface selection

F consumes the producer-owned **Producer interface 1.2.0** at
[filament-core-data revision 6259d3a](https://github.com/agent-ix/filament-core-data/blob/6259d3a5b99088740df9bcc8e8d60f3720aaa603/docs/semantic-data-system/baseline-1-2.md).
That producer contract defines the exact digest domain; each runtime assessment
retains the actual selected producer revision and digest.
The selected interface supplies relationship, population, snapshot, window,
member, configuration, and closure identities. It is D's authority and is not
an F wire/schema definition.

Each assessment input retains producer interface version `1.2.0`, exact document
identity/digest, `modelIdentity`, `relationshipIdentity` where required,
`populationIdentity`, `snapshotIdentity` or `windowIdentity`,
`configurationIdentity`, and every closure digest selected by the producer.
An unknown version, missing required identity, digest mismatch, or ambient
environment, working-directory, current-time, or network default refuses the
dependent assessment input. A producer's illustrative relationship spelling
does not replace its emitted identity.

## Observation-range correspondence

Every temporal or choreography-aware assessment additionally selects one exact
`quire.observation.range/v1` definition and one of its clock-family coverage
rows. The selected range retains the producer window identity/digest, ordered
record-set identity/digest, clock authority, temporal profile identity/revision/
digest, and all family-specific endpoints. The range is required at assessment,
not for static template checking.

Event-position coverage binds explicit sequence positions without inventing a
timestamp or elapsed time. Fixed-sample coverage binds exact epoch, positive
rational period, unit, and sample indexes without interpolation or binary
floating point. Timestamp coverage remains half-open: a witness at native
inclusive deadline 30 is outside producer `[0,30)` and is admitted only by a
separately selected range that contains 30. A witness at 29 may decide an
obligation from its exact support without manufacturing unrelated coverage or
closure. Member-object identity never substitutes for record identity or the
selected position/sample/timestamp anchor.

## Observation binding

Each declared observation role has the identity of its owning native declaration,
original source region, and role kind; its local name is diagnostic text and never
a global lookup key. An assessment maps every required role exactly once to a
binding admitted under one source/schema contract. One explicit binding may be
referenced by several compatible role mappings, but equal local names never merge
roles. Each binding retains all of the following semantic values:

| Value | Required meaning |
| --- | --- |
| Binding identity | Exact binding-contract definition identity, revision, and byte digest; applied binding identity; and source location of the authored role. |
| Source selection | Observation source identity, schema identity/revision, declared producer, and internal/external visibility. |
| Subject mapping | Exact Producer interface 1.2.0 model, workflow, participant, relationship, population, delivery, attempt, or effect handle selected by the role; no textual lookup fallback. |
| Signal and value mapping | Declared signal identity, typed value declaration, unit, missing-value policy, and required/optional status. |
| Anchor | The selected current, invocation pre/post, temporal origin/activation, protocol occurrence, or compensation registration/activation anchor. |
| Scope premises | Selected Producer interface 1.2.0 snapshot/window, membership rule/digest, configuration/closure identity and digest, exact observation-range definition/coverage row, clock/progress authority, and history premise when the role needs them. |
| Capture dependencies | Immutable capture identities and their authored anchors; a binding cannot retag a capture or import another workflow's values. |

Repeated mappings for one required role, incompatible mappings, selected identities
outside the linked package, and source/schema or unit drift refuse the dependent
request. A valid mapping with no required record remains an assessment input; it
does not manufacture a value, relationship, or future event.

## Admitted records and progress

An admitted record retains its record identity, selected binding, source/schema
identity, selected subject handle, typed value/presence state, event time and
ingestion time. The record does not establish causal order solely from timestamps.
It is classified as internal or externally observed without treating either class
as complete evidence by default.

A progress or closure assertion additionally retains its authority identity, the
same declared scope as the affected population/window, the E-owned clock meaning,
and its asserted boundary. A progress record for O2 or another window cannot
settle O1. EOF, a quiet transport, or a record timestamp is not progress/closure
authority.

The current [progress definition](definitions/observation-progress.md),
`quire.observation.progress/v1` draft revision `1-draft.3`, preserves the assertion, binding, scope,
E-owned temporal, record/revision, and restoration selections, including FCD
`1.2.0` half-open window coverage `[start, end)`: `start` participates once and
`end` belongs only to a later selected window. This is part of the current shared
draft; no further definition catalog publication gates its review. Runtime
assertions retain the exact accepted definition selection independently from
their own assertion identity and revision.

## Completeness and assessment handoff

For every requested obligation, F's handoff supplies the selected binding and
admitted-record identities plus one explicit availability state:

| State | Meaning |
| --- | --- |
| available | All required observations and selected scope/progress premises for this handoff are present and structurally admitted. This is not a truth or execution claim. |
| incomplete | A required observation, relationship, population member, external response, history, progress, or closure premise is unavailable; retain each missing identity/reason. |
| refused | Binding, source/schema, subject, unit, role, scope, or resource admission failed; retain the typed cause and affected identity. |

The handoff never collapses incomplete/refused to `false`, optional-none, an
untriggered obligation, or a Boolean result. It passes B's result adapter only
independent execution, activation, participation, completeness, provenance, and
logical facts; it does not create a second result/evidence family. A replay or
incremental assessment may settle only under E/B semantics and declared authority.

## Consumer obligations and planned controls

The linked-package binder consumes this contract through the exact role selection
in [FR-035](../../spec/functional/FR-035-bind-ecosystem-subjects.md). The
observation family requirements [FR-110](../../spec/functional/FR-110-bind-typed-observations.md)
through [FR-116](../../spec/functional/FR-116-map-observation-results-to-consumers.md)
govern its admission, completeness, replay, result, and output effects.

Planned controls use concurrent O1/O2, split shipments, a payment attempt/effect,
a refund, a duplicate receipt, missing provider evidence, cross-bound progress,
quiet deadline, late record, missing history, and exhausted retention. They are
specified in [TM-007](../../spec/observation/tests.md), not executed evidence.
