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
The caller explicitly supplies revision namespaces for authored formal sources,
requirements and registered semantic definitions; source-artifact revision labels
remain separate from the semantic revisions derived from those owners.

The reader takes canonical bytes, an independently selected external artifact
reference, that accepted inventory, exact local dependency bytes/admitted
producer views and caller-lowered artifact limits. It consumes no concrete
workflow instances, observations, clocks, assessment requests or backend results.

For the cross-repository positive handoff, the quire-protocol integration caller
SHALL construct the accepted inventory as Rust values from independently selected
original sources, admitted models, registered definitions, artifact contract and
producer bytes. The compiler supplies those original inputs with its emitted
bytes and reference; the offered payload cannot appoint its own accepted inventory.
The producer example's `expected.json` is an inspection aid for the fixture, not
an accepted-inventory interchange format or an authority B may accept by default.

## Outputs

An immutable `quire.compiled-protocol/1` package and its external exact-byte
reference, or a typed refused, unsupported or resource-incomplete result with
original available loci and partial compilation evidence. The public Rust
admission, emission and reader interfaces expose structured results through the
wire contract; source stdout, shell commands and hand-edited JSON are not the producer
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

When admitting an observed-Boolean choice, the compiler SHALL derive each atom
from the exact receive, own-attempt or domain-event binder, admitted Boolean
field/export and original anchor. The actual channel receiver, attempt role or
domain-event role must equal the choice owner, and existing source scopes must
establish guaranteed causal availability at that decision, including all-branch
joins without exporting branch-local or await-success-only records.
Immutable aliases and captures retain their actual initializer/provenance;
source order, field spelling or a shared provider cannot replace this authority.
Attempt ownership uses the resolved role identity, not equality of role model
types. An attempt observation does not prove operation success or a business
effect; operation and contract admission remain independent prerequisites.
The compiler SHALL retain one attempt identity per bounded occurrence of a
forward attempt authored inside a bounded repeat, carrying its enclosing
iteration ordinal.
The compiler SHALL keep that attempt identity distinct from the transport
delivery that carried it and from the declared business effect.
The compiler SHALL NOT mint a further successful effect when an attempt repeats
under an unchanged effect identity.
The compiler SHALL NOT let a retried attempt inherit an earlier occurrence's
effect. Attempt count, delivery count and effect count are independent
cardinalities.
The compiler SHALL preserve a compensation-qualified domain event's exact
registration prerequisite independently of its Boolean atom. Reading an event
field establishes no registration, activation, operation success, effect, send
or commit observation. Existing family and binding checks remain prerequisites.

The compiler SHALL require each `visible(...)` entry to be an eligible received
or same-owner attempt/domain-event Boolean field, transparent grouping/immutable
alias to that atom, or a closed Boolean constant, with every guard atom in the
validated atom set.
Composite visible-expression determinacy remains `Unsupported::FamilyProof`;
listing `a and b` cannot grant individual visibility to `a` and `b`.
The selected guard fragment admits Boolean literals/fields, grouping, `not`,
`and`, `or`, `implies`, Boolean `=`/`!=`, Boolean conditionals and immutable `let`
aliases. Arbitrary inputs, numeric comparisons, queries and callee-body expansion
remain unsupported family prerequisites, including when present in an unused
operand.

A bounded repeat's guard MAY be an observed Boolean over that same selected
fragment. The compiler SHALL apply the observed-Boolean atom, ownership and
`visible(...)` obligations above to a repeat exactly as to a choice: each atom
derives from the exact receive, own-attempt or domain-event binder, admitted
Boolean field/export and original anchor; the actual channel receiver, attempt
role or domain-event role must equal the repeat owner declared by `by`; and every
guard atom must appear in that repeat's validated atom set. A foreign-role or
unlisted atom returns `Unsupported::FamilyProof`. Equal role model types do not
grant repeat ownership. The authored finite maximum and the explicit `exhausted`
branch remain prerequisites of an observed guard, not substitutes for it.

A repeat's guard is evaluated at each bound decision instant under the repeat's
own anchor. The compiler SHALL NOT admit a guard atom that is established only
inside the repeat body: the body's bindings do not flow back to the decision, and
a guard depending on them has no causal availability at that instant. Such a
guard returns `Unsupported::FamilyProof` rather than an assumed value.

The compiler SHALL establish exactly one case for every valuation of the
conservative independent-atom abstraction before admitting a symbolic choice.
If that proof fails, then the compiler SHALL return `Unsupported::FamilyProof`
without claiming a concrete overlap or hole from an abstract valuation.
Fully closed decisions retain `Invalid::Control` for an evaluated overlap/hole.
Closed classification requires every original operand and immutable initializer
to be closed; short-circuit truth or an unselected conditional branch cannot
erase a visibility or unsupported-fragment prerequisite.
Definedness discharge remains a separate prerequisite: IR `check_expression`
does not establish Boolean validity. Original operands, guards, handles and
anchors remain in emitted code rather than proof witnesses or simplified output.
When a symbolic choice contributes continuing-loop progress, the compiler SHALL
require observable progress in every abstractly feasible branch.
An unproved branch leaves the choice's progress unestablished; if the continuing
body depends on that proof, admission returns FamilyProof. Other guaranteed
progress in the body retains its ordinary control meaning.

An observed repeat guard partitions each valuation into exactly the continuing
and exhausting branches.
The compiler SHALL establish that partition before admitting the repeat.
If that proof fails, then the compiler SHALL return `Unsupported::FamilyProof`
without claiming a concrete overlap or hole from an abstract valuation. Because a true
guard is feasible under that abstraction whenever any valuation selects it, the
continuing body SHALL establish observable progress exactly as under a true
constant guard. A body proving no progress remains `Invalid::Control`, and a body
whose progress is unproved remains `Unsupported::FamilyProof`. An observed guard
neither supplies progress itself nor weakens the authored finite maximum; guard
falsification is a normal exit and is never evidence of body progress.

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

When a native reference requires a population, the compiler SHALL derive its
population export and runtime input requirements from the exact admitted object
role, selected model, universe and original evaluation anchor. The reader SHALL
refuse a reference/object/population triple assembled from different roles or
models as `Invalid::Type`, even when carrier types or universe labels agree. Pre-state and capture
origins retain their original population/closure requirements; future membership
and completeness remain consumer inputs rather than compilation prerequisites.
The population/closure set SHALL equal the set required by the declaration's
original input binders and anchored values: missing or surplus pairs refuse as
`Invalid::Binding`. Derived binders retain their initializer's observation;
selected values retain their contributing origins without adding a population
at the selection site. FR-042-AC-4 and FR-042-AC-7 cover these identity and
reader-refusal obligations under the wire contract's typed cause catalog.

The artifact SHALL preserve effect-before-registration, separate registration
and activation captures, bounded retries, commit restrictions and the exact
full/partial recovery predicates with their declared population/relationship
and closure authorities. A compensation operation's success cannot replace its
recovery relation or completeness premises.

When the source declares no separate compensation-effect payload, the compiler SHALL
emit an effect-instance requirement with null value type, the exact
compensation operation export, and its owning obligation, retry anchor and
attempt-binding prerequisite. This narrow identity role does not select an
attempt, operation-result or recovery-view payload as effect evidence. The null
type does not authorize an untyped observation or waive F's later admission of
the actual effect subject and its typed signal/value mapping under the unchanged
selected observation-binding contract.

The reader SHALL refuse any compensation effect without its exact operation,
subject, anchor and registration/activation/attempt prerequisite chain under the
selected observation-binding contract. If an otherwise valid compensation effect
carries a non-null value type, then the current reader SHALL return
`Unsupported::Export`, because the selected native profile supplies no
authoritative effect-payload selector or correspondence. An inserted type cannot
waive an identity mismatch, which remains `Invalid::Binding`. Ordinary effects
retain non-null value type and model.

The reader SHALL require the compensation clock to retain its exact subject,
activation anchor, activation prerequisite and selected temporal contract and
authority. The reader SHALL require its recovery snapshot to retain the recovery
binder's exact type/model, owning recovery anchor and activation prerequisite
under the observation-binding contract. The reader SHALL require both recovery
progress and closure at that anchor under the progress contract, each depending
on exactly the clock, effect instance and snapshot. The reader SHALL require
`recovery_bindings` to contain exactly those three records and the declaration's
population/closure pairs for recovery and contributing captured origins.

The compiler and reader SHALL use one shared dependency rule to derive contributing recovery origins.
Starting with the owning recovery anchor, the rule follows all original
value-operand, read-binder initializer and non-self selected-origin edges
reachable from `recover`, retaining reachable original anchored origins.
Self-selected markers add no edge; source-span containment and proof folding
cannot add or remove dependencies. Calls retain their argument dependencies
without importing callee-owned graphs or guessed captures. The resulting anchors
select exactly the declaration-owned population/closure pairs; another
compensation's phase anchor refuses. Existing model
validation retains nominal identity and pair-integrity ownership. No future
observations or established closure are supplied by this derivation.

The compiler and reader SHALL retain compensation attempt maxima in
`1..=9223372036854775807` without expanding the declared attempts. Actual consumer
work budgets remain separate from this finite authored bound.

If any required declaration or dependency is refused, unsupported or unfinished,
then the emitter SHALL return no full-package artifact while retaining the
independent compilation results. Unknown wire/type/version, missing or substituted
selection, unsupported required feature/domain and resource exhaustion have
typed discriminating causes. Known refusals survive later budget exhaustion;
human-readable diagnostic text does not select their classification. Wire
refusals use the contract's published typed vocabulary and pass precedence;
package-wide output work carries no unrelated last-visited source locus. Wire
verification does not imply global replay, projection, realizability or recovery
success. These remain separately requested consumer capabilities.

The artifact stages SHALL enforce the versioned `quire.protocol.artifact-work/1`
limits and charge-before-work rules in the wire contract, with exact effective
limits, successful usage and the source-owned next unaffordable operation.
Zero disables no limit; above-hard requests clamp; overflow is incomplete;
fresh retry neither reuses spent counters nor mutates previous inputs/results.
Private Boolean proof records, provenance/formula/valuation visits and traversal
depth consume these existing counters before allocation or work; resource
exhaustion remains incomplete rather than a failed or successful truth proof.
Each dynamic choice rebuilds its private read index and lowering records;
`Entries` charges their creation per choice, including transient records, rather
than measuring peak live memory. Multiple individually admissible decisions may
therefore exhaust one invocation's shared limits; this returns incomplete.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-042-AC-1 | A complete source-derived package containing a protocol and its predicate/state/temporal dependencies emits the exact registered media/schema/type/encoding/numeric selections, complete original inventory and immutable external byte reference. Missing family admission, a type/proof-only result or an arbitrary wire fixture cannot invoke production emission. | Test (TC-121) |
| FR-042-AC-2 | Fixed compact JSON vectors retain field order, Unicode escaping and authored sequence order; permitted inventory reordering yields identical bytes. Integer safe endpoints, signed-64 extrema and normalized rationals retain exact kind/components in every value/bound field; invalid field-domain values, bare native numbers, floats and noncanonical structural integers refuse without rounding or repair. | Test (TC-121) |
| FR-042-AC-3 | One-axis changes to producer, accepted baseline, contract, source authority/native/formal revision or bytes, profile/rule closure, dependency/export, outer interpretation and required/optional features refuse the affected selection. Source/model/config/manifest/lock/IR/result digests cannot substitute for the expected compiled-artifact or native model byte digest. | Test (TC-121) |
| FR-042-AC-4 | Typed values preserve exact nominal/unit/domain identities, cross-unit callee owners, ordered arguments, all eight query forms, graph-export authority, Boolean roots, source loci, scope and pre/post/activation/capture origins. Wrong type/owner, dangling/cyclic value references, missing totality and proof-witness substitution cannot produce an emitted package. | Test (TC-121) |
| FR-042-AC-5 | Sequence, owned labeled choice, parallel/join, bounded progress, await branches and termination retain their declared causal relations and finite bounds. Received, own-attempt and same-owner domain-event Boolean choices preserve exact atom/owner/provenance identity and prove one case for every abstract valuation; unsupported visibility or an unproved partition returns FamilyProof, while evaluated closed overlap/hole remains Invalid(Control). Equal role model types do not grant ownership. Compensation-qualified events retain their registration prerequisite; Boolean observations grant no activation or effect-success evidence. A dynamic choice establishes continuing-loop progress only when every feasible branch progresses. An observed repeat guard preserves the same atom/owner/provenance identity and partitions the continuing and exhausting branches; a foreign-role, unlisted, composite or body-established guard atom returns FamilyProof, a non-progressing continuing body remains Invalid(Control), and an unproved one returns FamilyProof. Missing/foreign joins, unbounded or zero-progress repetition, wrong-kind event targets and cross-channel FIFO assumptions refuse independently. | Test (TC-121) |
| FR-042-AC-6 | Two workflows sharing a provider retain distinct binding subjects; one send/two deliveries/one effect remains 1/2/1. A forward attempt repeated across bounded iterations retains one attempt identity per occurrence with its iteration ordinal, distinct from the carrying delivery and from the single business effect; a retried attempt inherits no earlier effect. Effect-dependent registration, bounded retries through the signed-64 maximum, commit boundaries and full versus partial recovery retain exact relations and authorities. Producer and reader derive identical recovery population membership through original operand, binder-initializer and selected-origin reachability, preserving captured anchors without span-based or proof-folded substitutions. Compensation effects retain exact operation/subject/retry/attempt identity even when a type is inserted; an otherwise valid typed compensation effect remains explicitly unsupported. Missing/cross-wired registration, activation, clock or recovery prerequisites, unrelated recovery/population records, null model, an ordinary effect with null type, or an operation-success shortcut refuse. | Test (TC-121) |
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
implement part of this contract. The native path emits supported complete
families from real source/type/proof owners, including static compensation and
full/partial recovery requirements; strict-reader fixtures alone do not establish
that authority. The [Rust producer recipe](../../examples/protocol-handoff/README.md)
retains four original source units, cross-unit dependencies, queries,
population/reference roles, admitted model operations and actual executable bytes.
Its joined receive/Boolean-choice scenario retains both received facts and
authored branches alongside those dependencies and recovery obligations.
General dynamic choice/progress proofs, first-class relationship exports,
runtime recovery and the actual B public consumer handoff remain open until
implemented and exercised through the corresponding public interfaces. A's
emission and local reader check do not satisfy FR-042-AC-10 by themselves.
