---
id: FR-048
title: "Preserve native choreography semantics for assessment"
type: FR
relationships:
  - { target: ix://agent-ix/quire-spec-language/US-002, type: implements }
  - { target: ix://agent-ix/quire-spec-language/US-003, type: implements }
  - { target: ix://agent-ix/quire-spec-language/FR-036, type: depends_on }
  - { target: ix://agent-ix/quire-spec-language/FR-040, type: depends_on }
  - { target: ix://agent-ix/quire-spec-language/FR-042, type: references }
  - { target: ix://agent-ix/quire-spec-language/FR-043, type: depends_on }
  - { target: ix://agent-ix/quire-spec-language/FR-044, type: depends_on }
  - { target: ix://agent-ix/quire-spec-language/FR-045, type: depends_on }
  - { target: ix://agent-ix/quire-spec-language/FR-050, type: depends_on }
  - { target: ix://agent-ix/quire-specification/FR-049, type: depends_on }
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
  - { target: ix://agent-ix/quire-specification/FR-060, type: references }
  - { target: ix://agent-ix/quire-specification/FR-061, type: references }
  - { target: ix://agent-ix/quire-specification/FR-062, type: references }
  - { target: ix://agent-ix/quire-spec-language/FR-056, type: depends_on }
  - { target: ix://agent-ix/filament-core-data/FR-142, type: references }
  - { target: ix://agent-ix/filament-core-data/FR-143, type: references }
  - { target: ix://agent-ix/quire-protocol/FR-001, type: references }
  - { target: ix://agent-ix/quire-protocol/IT-001, type: references }
---
# FR-048: Preserve native choreography semantics for assessment

## Description

When a native protocol family is admitted, the compiler SHALL preserve the
source-owned choreography, binding requirements and causal/recovery structure
needed by downstream protocol and observation assessment.

This requirement retrospectively scopes roadmap L6 under compiler
[#39](https://github.com/agent-ix/quire-spec-language/issues/39). The compiler
produces an immutable assessment subject; it does not send messages, execute
business operations, infer observations or decide protocol conformance.

## Inputs

The exact admitted native source and definition selections; the domain-package
model declarations admitted under
[FR-056](FR-056-admit-domain-package-model-declarations.md), with their object,
record, field, operation, reference, relationship and population exports; the
participant, component, endpoint and configuration selections the protocol
names; the selected L5 temporal declarations; F-owned observation-role
definitions; and finite compiler admission/emission limits.

Static compilation consumes each declaration by its declaration key (domain
package identity, IR node identity, `sha256-jcs`). The domain package digest
stays separate from the native source raw-byte digest and the compiled-artifact
seal. Snapshot, window, member, relationship-instance and closure identities are
assessment selections, not static compiler inputs.

F owns concrete record admission, mapping a record to its exact workflow/node/
role/delivery/attempt/effect occurrence, and the availability, membership,
completeness, progress and closure assertions handed to assessment. F supplies
those facts under the domain package's declaration identities and cannot mint or
reinterpret them.

Future workflow instances, relationship instances, deliveries, attempts,
effects, clock observations, records, progress and closure assertions are
assessment inputs and are not required to compile a static template. A missing
static declaration or export remains an unsupported prerequisite rather than
guessed metadata. A missing or inconsistent F assessment fact remains the
downstream incomplete or refused input selected by F/B; the compiler does not
convert it into a static export or a protocol result.

## Outputs

An admitted compiled-protocol family retaining exact typed role and occurrence
handles, value/control graphs, temporal/recovery requirements and domain-package
model selections, or a typed invalid, unsupported or resource-incomplete family
disposition. An untimed subject remains representable by strict
`quire.compiled-protocol/1`. A timed choreography requires the separately
reviewed `quire.compiled-protocol/2` extension described under Dependencies;
it cannot be emitted as `/1` with unauthenticated clock premises. Both versions
are consumed only through the version-exact parser-free Rust readers owned by
[FR-042](FR-042-publish-compiled-protocol-artifacts.md) and
[FR-050](FR-050-publish-authenticated-temporal-artifacts.md).

## Behavior

The compiler SHALL retain the static protocol declaration, authored role slots,
participant/component/endpoint selections, relationship declarations and every
authored node handle. Its admitted-package occurrence-schema projection SHALL
return the protocol declaration, workflow-instance binding requirement, each
authored node, the node's role slot when that node is role-owned, and every
outer-to-inner enclosing repeat ordinal domain. It SHALL NOT emit future O1/O2
workflow instances or concrete ordinals. Those concrete identities and ordinal
values are supplied later, and the retained schema prevents two instances from
collapsing when they share a provider, model, local name or transport value.

The compiler SHALL retain channel sender/receiver roles, payload type,
delivery bounds, ordering rule and each authored send/receive occurrence without
inventing concrete delivery correspondence.

The compiler SHALL retain sequence order, choice branches, parallel branches
and joins, bounded repeat bodies/exits, awaits, checks, events and commits as
distinct authored control nodes.

When a choice or repeat guard consumes observations, the compiler SHALL derive
each visible atom from its exact eligible receive, same-owner attempt or
same-owner domain-event Boolean binding and original anchor.

The compiler SHALL establish choice coverage/non-overlap and repeat feasibility
over the conservative independent-atom abstraction without replacing authored
guard operands with proof witnesses.

If repeat exit is feasible, then the compiler SHALL treat the repeat as
contributing no guaranteed progress to an enclosing progress obligation.

If repeat exit is infeasible, then the compiler SHALL derive the repeat's
enclosing-progress contribution from the entered body's independently admitted
progress.

The compiler SHALL require observable progress in every feasible continuing
repeat body and preserve the authored maximum and exhaustion branch.

The compiler SHALL retain delivery, attempt, operation effect, domain event,
compensation registration, compensation activation and commit as different
requirements; evidence for one cannot satisfy another.

The compiler SHALL bind each compensation to its exact forward-effect node,
trigger, target capture, registration capture, activation capture, positive
attempt maximum, retry relationship, recovery predicate and forbidding commit
node or explicit `never`.

The compiler SHALL preserve one compensation obligation across distinct retry
attempt identities without treating operation success as recovery success.

For `activate first`, the compiler SHALL preserve the exact trigger identity,
registration identity and requirement for an admitted causal/sequence order
between distinct eligible triggers. It SHALL NOT encode ingestion order or
timestamp as the selection rule. It SHALL retain the downstream requirements
that repeated delivery of one semantic trigger keeps separate receipt provenance
without another activation/registration and that a later distinct trigger does
not reactivate the same registered forward effect. The compiler does not execute
those rules; missing order/correlation and inconsistent runtime bindings remain
typed F/B assessment outcomes.

The compiler SHALL derive recovery population, relationship, snapshot,
progress, closure, clock and captured-origin requirements from original recovery
operands and anchors rather than source-span proximity or simplified proofs.

The compiler SHALL retain the selected temporal declaration/clock/activation
interface without evaluating temporal settlement inside choreography admission.

For a timed choreography, the `/2` artifact SHALL carry each selected temporal
definition's identity, revision, raw-byte digest and immutable artifact reference,
plus exactly one tagged clock configuration for that selection. Missing,
duplicate, untagged or profile-incompatible clock configuration refuses the
timed family. The strict `/1` reader SHALL continue to reject `/2`, and the
strict `/2` reader SHALL NOT reinterpret or silently upgrade `/1`. This extension
authenticates the inputs required by L5; it defines no clock progression,
deadline settlement or late-evidence meaning.

The compiler SHALL retain every required observation binding's owning native
declaration, source locus, role kind, anchor, scope premises and capture
dependencies without appointing a concrete observation adapter.

If a domain-package declaration, relationship, participant, component, endpoint,
configuration or typed effect selector required by the selected profile is
unavailable, then the
compiler SHALL return an explicit unsupported prerequisite without omitting or
fabricating the dependent record.

The compiler SHALL preserve independent source, domain-package, dependency,
compiled-artifact and protocol-result identity/digest domains across the
compiler/consumer handoff.

The compiler SHALL produce the same static subject when only valid concrete
runtime values, relationship instances, observation records or assessment-time
limits change under the same selected contracts. Compiler admission/emission
limits are excluded from this invariance: an insufficient compiler limit returns
resource incompleteness and emits no static subject.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-048-AC-1 | One reusable template retains its static role/relationship requirements and exposes the full runtime occurrence-key schema from an admitted `/1` or `/2` package without changing wire bytes. Later O1/O2 bindings sharing one external payment provider therefore require distinct workflow and node-occurrence identities plus one concrete ordinal for every returned repeat domain, without the compiler fabricating any instance or ordinal. | Test (TC-132) |
| FR-048-AC-2 | Channels preserve exact endpoint roles, payloads, order/delivery bounds and separate send, receive, delivery, attempt and effect identities; concrete fan-out remains a consumer binding. | Test (TC-132) |
| FR-048-AC-3 | Sequence, choice, parallel, join, await, repeat, exhaustion, check, event and commit nodes retain their authored topology and exact typed control edges. | Test (TC-133) |
| FR-048-AC-4 | Observed choice/repeat guards admit only causally visible eligible atoms; same spelling, wrong owner, branch-local or success-only values refuse without disappearing in unused operands. | Test (TC-133) |
| FR-048-AC-5 | A repeat with a feasible false guard contributes no guaranteed enclosing progress; an infeasible false valuation permits the entered body's proven progress; zero/exact/one-short proof budgets retain charge order and typed outcomes. | Test (TC-133) |
| FR-048-AC-6 | Split shipments, one authored payment attempt under distinct supplied repeat ordinals, and refund paths preserve separate static subjects and required relationships while one compensation registers only for its exact successful forward effect. Attempt bounds admit one and the largest representable positive value and refuse zero or overflow without weakening the retained obligation or minting another static attempt. | Test (TC-134) |
| FR-048-AC-7 | Registration, activation policy, retry attempt, effect, commit and recovery remain distinct; swapped anchors/operations/subjects or a commit-before-recovery mutation refuse with typed causes. The artifact retains the exact causal-order, semantic-trigger, receipt-provenance and no-reactivation requirements for downstream F/B assessment without claiming that any runtime trigger set satisfies them. | Test (TC-134) |
| FR-048-AC-8 | Recovery retains exact target/captures, relationship/population closure, temporal activation/deadline and progress requirements. A timed subject uses `/2` with every selected temporal definition identity/revision/raw-byte digest/artifact and exactly one tagged clock configuration; `/1` remains strict and cannot carry or infer them. A missing static domain-package declaration is unsupported; missing concrete assessment authority remains an external incomplete/refused outcome and is never represented by A as recovery success. | Test (TC-134) |
| FR-048-AC-9 | The release compiler emits canonical bytes and selections that B independently rederives and admits through its public Rust intake; source/domain-package/artifact/result digest substitution or recanonicalization refuses. | Test (TC-135) |
| FR-048-AC-10 | A's release compiler contributes the exact source-derived artifact, selection and static binding requirements to the separately owned pinned campaign run. The contribution is byte-identical at B's intake and retains every required domain-package, L5 and F role selection; A's local evidence does not claim PT02 truth, observation adequacy, batch/incremental agreement or campaign completion. | Test (TC-135) |

## Dependencies

[FR-042](FR-042-publish-compiled-protocol-artifacts.md) owns the artifact and
compiler handoff. [FR-056](FR-056-admit-domain-package-model-declarations.md)
owns the model, relationship and population declarations A consumes, admitted
from spec artifacts through quire-rs and the FCD semantic IR crates. F's
observation contract owns concrete record/correlation, availability, membership,
completeness, progress and closure assertions.

The pinned external L5 requirement and Rust-interface baseline is merged PR #70,
compiler revision `72507f8`, comprising FR-043/044/045 and
`temporal::evaluate`, `temporal::evaluate_with_progress` and
`temporal::mapping_support`. That baseline records its narrow A-owned prerequisite:
`quire.compiled-protocol/2` must extend `/1` with each selected temporal
definition identity/revision/raw-byte digest/artifact and exactly one tagged
clock configuration, while strict `/1` readers remain unchanged. This
FR consumes that requirement but does not invent its schema or treat the L5
evaluator alone as an authenticated compiler seam;
[FR-050](FR-050-publish-authenticated-temporal-artifacts.md) owns the versioned
wire/reader change. E remains the semantic authority for clock progression,
deadline settlement and late evidence. The `/2` extension and L5 acceptance are
prerequisites only for the timed campaign case, not for untimed static
choreography emission.

`quire-protocol` [#11](https://github.com/agent-ix/quire-protocol/issues/11)
owns artifact intake, [#6](https://github.com/agent-ix/quire-protocol/issues/6)
owns PT02 conformance, and
[#12](https://github.com/agent-ix/quire-protocol/issues/12) owns the temporal/
observation integration. D owns the version-lock manifest, Rust integration
driver and aggregate campaign record under
[quire-research#39](https://github.com/agent-ix/quire-research/issues/39) and
[IN01 #49](https://github.com/agent-ix/quire-research/issues/49) until an
integration-repository transfer is approved. Standard FR-049 supplies choreography syntax;
FR-050–059 supply protocol meaning; FR-060–062 are downstream claim, result and
portable-projection boundaries referenced but not implemented here. The compiler
owns only source-to-artifact preservation and the exact static roles and runtime
key requirements those consumers must bind. This retrospective cycle under
[#66](https://github.com/agent-ix/quire-spec-language/issues/66) records that
earlier choreography implementation and tests preceded this requirement.
