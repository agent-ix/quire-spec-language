---
id: FR-042
title: "Publish and read exact compiled protocol packages"
type: FR
relationships:
  - { target: ix://agent-ix/quire-spec-language/US-004, type: implements }
  - { target: ix://agent-ix/quire-spec-language/FR-036, type: depends_on }
  - { target: ix://agent-ix/quire-spec-language/FR-038, type: depends_on }
  - { target: ix://agent-ix/quire-spec-language/FR-040, type: depends_on }
  - { target: ix://agent-ix/quire-spec-language/FR-041, type: depends_on }
  - { target: ix://agent-ix/quire-specification/FR-004, type: depends_on }
  - { target: ix://agent-ix/quire-specification/FR-015, type: depends_on }
  - { target: ix://agent-ix/quire-specification/FR-017, type: depends_on }
  - { target: ix://agent-ix/quire-specification/FR-031, type: depends_on }
  - { target: ix://agent-ix/quire-specification/FR-039, type: depends_on }
  - { target: ix://agent-ix/quire-specification/FR-044, type: depends_on }
  - { target: ix://agent-ix/quire-specification/FR-047, type: depends_on }
  - { target: ix://agent-ix/quire-specification/FR-050, type: depends_on }
  - { target: ix://agent-ix/quire-specification/FR-051, type: depends_on }
  - { target: ix://agent-ix/quire-specification/FR-052, type: depends_on }
  - { target: ix://agent-ix/quire-specification/FR-053, type: depends_on }
  - { target: ix://agent-ix/quire-specification/FR-054, type: depends_on }
  - { target: ix://agent-ix/quire-specification/FR-055, type: depends_on }
  - { target: ix://agent-ix/quire-specification/FR-056, type: depends_on }
  - { target: ix://agent-ix/quire-specification/FR-057, type: depends_on }
  - { target: ix://agent-ix/quire-specification/FR-058, type: depends_on }
  - { target: ix://agent-ix/quire-specification/FR-059, type: depends_on }
  - { target: ix://agent-ix/quire-specification/FR-130, type: depends_on }
  - { target: ix://agent-ix/quire-protocol/FR-001, type: references }
  - { target: ix://agent-ix/quire-protocol/IT-001, type: references }
---
# FR-042: Publish and read exact compiled protocol packages

## Description

When a complete native package has passed its selected type, definedness and
family admission, the compiler SHALL emit its exact typed protocol package
through the versioned [compiled-protocol wire contract](../../docs/compiled-protocol-v1.md).

## Inputs

The explicit original source inventory, actual source/formal clause owners,
accepted baseline and artifact contract, producer implementation/revision,
exact model/definition/rule/dependency selections, and constructor-private
family-admission evidence derived from that same compilation. Source/profile
acceptance comes from the independently supplied accepted inventory, not a
payload flag, current Git revision, installed default or similar display name.

The reader takes canonical bytes, an independently selected external artifact
reference, that accepted inventory, exact local dependency bytes/admitted
producer views and caller-lowered artifact limits. It consumes no concrete
workflow instances, observations, clocks, assessment requests or backend results.

## Outputs

An immutable `quire.compiled-protocol/1` package and its external exact-byte
reference, or a typed refused, unsupported or resource-incomplete result with
original available loci and partial compilation evidence. The planned Rust
`protocol_artifact` emitter and the `read` entry point expose structured
results; source stdout, shell commands and hand-edited JSON are not the producer
handoff. Successfully verified wire content remains distinct from compiler-owned
family admission and B-owned assessment admission/execution.

## Behavior

The compiler SHALL derive the closed source, type, predicate, state, temporal,
protocol, capture, binding-requirement and recovery records specified by the
wire contract from their actual admitted native owners.

The emitter SHALL require constructor-private evidence covering the complete
explicit source inventory and every dependency before publishing a full-package
artifact. `ComposedUnit`, names-resolved bindings, `TypeReport`, symbolic proof
graphs and manually sealed wire records cannot be retagged as that evidence.

The compiler SHALL retain native Quire as the sole editable formal-clause source.
The value/control graph is a derived representation of the admitted source,
with exact local handles, original loci, immutable binder origins, ordered calls,
short-circuit/conditional behavior and all selected model/profile semantics.
Proof witnesses are not executable expressions. Actual totality, choice
visibility/non-overlap, finite causal control, channel premises and recovery
admission remain prerequisites; absent implementations or authoritative exports
produce explicit unsupported prerequisites rather than omitted graph content.

The encoder SHALL produce the selected compact JSON bytes with the contract's
fixed member order, array/set ordering, exact [FR-038](FR-038-encode-exact-protocol-numbers.md)
numeric objects and independently bounded structural integers. The existing Rust
Serde writer supplies JSON escaping/formatting; `ProtocolNumber` supplies admitted
numeric components; SHA-256 binds the resulting complete bytes. This registers
a wire encoding, not a new source-free semantic canonicalizer or JCS identity.

The emitter SHALL keep its digest in the external artifact reference, outside
the hashed payload. Model raw-byte digests, source digests, producer canonical
object/fingerprint domains, manifest/lock/dependency references and protocol
result JCS identities retain their own exact roles and algorithms. Copying an
external reference does not authorize recanonicalizing its object or substituting
its hash into another role. The outer package canonical identity remains null.

The strict reader SHALL verify the expected byte seal and exact accepted
contract/producer/baseline/source/profile/dependency/feature selections before
admitting their dependent semantic records. It then checks closed shape,
numeric domains, typed reference kinds/owners, scope/control invariants and
canonical byte equality under the finite pass order in the wire contract.
Source text is hashed and indexed for spans without a native parser. Valid
JSON or a recomputed internal seal cannot replace an independently expected
artifact selection. The reader does not claim to prove source compilation from
source text, source digests or a producer label.

The artifact SHALL retain distinct workflow/role-instance, participant,
component, endpoint, message, send, receive, delivery, attempt, effect,
registration, compensation-attempt/effect and commit requirements. Concrete
instances enter B/D/F's later binding interfaces. Shared providers, repeated
deliveries, missing future observations and unactivated recovery do not collapse
those identities or cause fabricated static observations.

The artifact SHALL preserve effect-before-registration, separate registration
and activation captures, bounded retries, commit restrictions and the exact
full/partial recovery predicates with their declared population/relationship
and closure authorities. A compensation operation's success cannot replace its
recovery relation or completeness premises.

If any required declaration or dependency is refused, unsupported or unfinished,
then the emitter SHALL return no full-package artifact while retaining the
independent compilation results. Unknown wire/type/version, missing or substituted
selection, unsupported required feature/domain and resource exhaustion have
typed discriminating causes. Known refusals survive later budget exhaustion;
human-readable diagnostic text does not select their classification. Wire
verification does not imply global replay, projection, realizability or recovery
success. These remain separately requested consumer capabilities.

The artifact stages SHALL enforce the versioned `quire.protocol.artifact-work/1`
limits and charge-before-work rules in the wire contract, with exact effective
limits, successful usage and the source-owned next unaffordable operation.
Zero disables no limit; above-hard requests clamp; overflow is incomplete;
fresh retry neither reuses spent counters nor mutates previous inputs/results.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-042-AC-1 | A complete source-derived package containing a protocol and its predicate/state/temporal dependencies emits the exact registered media/schema/type/encoding/numeric selections, complete original inventory and immutable external byte reference. Missing family admission, a type/proof-only result or an arbitrary wire fixture cannot invoke production emission. | Test (TC-121) |
| FR-042-AC-2 | Fixed compact JSON vectors retain field order, Unicode escaping and authored sequence order; permitted inventory reordering yields identical bytes. Integer safe endpoints, signed-64 extrema and normalized rationals retain exact kind/components in every value/bound field; invalid field-domain values, bare native numbers, floats and noncanonical structural integers refuse without rounding or repair. | Test (TC-121) |
| FR-042-AC-3 | One-axis changes to producer, accepted baseline, contract, source authority/native/formal revision or bytes, profile/rule closure, dependency/export, outer interpretation and required/optional features refuse the affected selection. Source/model/config/manifest/lock/IR/result digests cannot substitute for the expected compiled-artifact or native model byte digest. | Test (TC-121) |
| FR-042-AC-4 | Typed values preserve exact nominal/unit/domain identities, cross-unit callee owners, ordered arguments, all eight query forms, graph-export authority, Boolean roots, source loci, scope and pre/post/activation/capture origins. Wrong type/owner, dangling/cyclic value references, missing totality and proof-witness substitution cannot produce an emitted package. | Test (TC-121) |
| FR-042-AC-5 | Sequence, owned labeled choice, parallel/join, bounded progress, await branches and termination retain their declared causal relations and finite bounds. Missing/foreign joins, unbounded or zero-progress repetition, unobservable/overlapping choices, wrong-kind event targets and cross-channel FIFO assumptions refuse independently. | Test (TC-121) |
| FR-042-AC-6 | Two workflows sharing a provider retain distinct binding subjects; one send/two deliveries/one effect remains 1/2/1. Effect-dependent registration, retries, commit boundaries and full versus partial recovery retain exact typed relations and authorities; identity substitution, missing static authority or an operation-success shortcut cannot satisfy emission/admission. | Test (TC-121) |
| FR-042-AC-7 | The public reader verifies canonical bytes and external selection without a native parser. Unknown/duplicate/missing fields, wrong tags, noncanonical encodings, out-of-range/foreign handles and changed content refuse with typed causes. Resealing a tampered payload under an unchanged expected reference still refuses; digest success alone is never evidence of native-source equivalence. | Test (TC-121) |
| FR-042-AC-8 | Partial, unsupported and resource-incomplete compilation retains independent results but emits no fully linked package. An independently unsupported projection does not erase an admitted global-protocol subject or become complete package/assessment success; historical package/profile identities and entry-point refusals remain unchanged. | Test (TC-121) |
| FR-042-AC-9 | Independently counted source/dependency/table/reference/control and canonical-output vectors distinguish zero, exact and one-short limits in each artifact work dimension, including deep/shared graphs, repeated traversal and bounded iteration. Overflow/clamping and fresh retry retain exact stage/locus/usage without a partially admitted artifact. | Test (TC-121) |
| FR-042-AC-10 | Actual accepted native source passes the real compiler stages and emits the fixture consumed unchanged by quire-protocol's public Rust admission/linking interface. Every producer/source/profile/dependency selector survives; no shell, stdout parser, alternate formal frontend or manually sealed fixture supplies this positive handoff. | Test (TC-121, quire-protocol IT-001) |

## Dependencies

[FR-036](FR-036-link-composed-native-packages.md),
[FR-040](FR-040-check-composed-values.md) and
[FR-041](FR-041-admit-rational-native-model-profile.md) own source/model/value
prerequisites; [FR-038](FR-038-encode-exact-protocol-numbers.md) is the delivered
numeric component. The accepted standard package/protocol contracts and FR-050–059
own static versus runtime identity and protocol meaning; this requirement fixes
their compiler wire representation. [TC-121](../test-cases/TC-121-publish-compiled-protocol-artifacts.md)
and [B's IT-001](ix://agent-ix/quire-protocol/IT-001) retain full producer/consumer
acceptance under compiler #40. The wire reader and numeric/type components
implement part of this contract; strict-reader fixtures alone do not close
native emission, family admission or producer/consumer acceptance.
