# Native protocol, compensation and result contract

Draft PT01/PT02 contribution to the composed Quire v1 baseline. It defines
semantic representation and consumer contracts. Agent A owns the single native
grammar and integrates the eventual protocol surface; this document does not
create another authored language, parser or evidence system.

## Selected v1 boundary

The v1 protocol profile represents a finite global protocol over explicitly
identified workflow instances. Its admitted structure consists of finite named
roles and role instances, typed messages, explicit channels, distinct send and
receive events, authored causal order, role-owned labeled choice, bounded
parallel branches and join, bounded iteration and declared termination. A
protocol package also states delivery and duplication premises and the facts
visible to the role that owns each choice.

The evaluator checks supplied observations. It never sends a message, invokes a
business operation, registers a real compensator or performs recovery. Global
finite replay and conformance are selected v1 capabilities. Projection,
realizability, refinement and assume-guarantee composition remain independent
requested capabilities with their own premises and may be unavailable while
global conformance remains useful.

All choice guards, obligation triggers and recovery relations are total,
read-only typed predicates over immutable supplied values and captures. They
perform no I/O, callbacks, mutation, reflection or user-code execution. An
evaluation failure is a typed failed or unavailable result; it never selects a
branch or falls back to truthiness.

The representation consumes A-owned source/profile/predicate identities,
D-owned component, relationship and population identities, E-owned clock and
closure semantics, and F-owned observation/correlation/completeness bindings.
It preserves those authorities rather than copying their models.

## Identity model

These identities remain distinct and typed:

| Subject | Meaning |
| --- | --- |
| Protocol definition | Static authored global behavior under an exact language edition and profile definition. |
| Workflow instance | One assessed occurrence of a protocol; related instances remain separate occurrences. |
| Role / role instance | An authored responsibility / its configuration-bound participant in one workflow instance. |
| Message | The authored typed communication obligation. |
| Send / receive | Distinct protocol events; neither one implies the other. |
| Delivery | A transport observation carrying a message attempt; repeated delivery need not be a new effect. |
| Attempt | One invocation try under an authored operation and retry relation. |
| Effect | A declared successful business-state transition, identified separately from attempts and receipts. |
| Compensation registration | An obligation created only after its paired effect succeeds. |
| Compensation attempt / effect | A recovery try / its successful business effect. Success does not imply restoration unless the recovery relation establishes it. |
| Commit | An authored boundary after which specified compensations are no longer permitted. |
| Result | Facts about one exact request and subject, with assessment execution, decision-scope progress/closure, surrounding-execution progress/closure, complete-global-conformance closure, truth, settlement basis, activation, participation, assumptions, completeness, observation adequacy and protocol adequacy separated. |

Identity substitution refuses. A telemetry trace identifier, endpoint, queue,
timestamp or equal display name cannot stand in for a workflow, role, message,
attempt or effect identity. Related order, shipment, payment-attempt, payment-
effect and refund instances use explicit typed relationships. Population
obligations retain their declared snapshot or event-time window plus membership,
progress and completeness dependencies; unknown required membership cannot
establish success.

## Control and observability

Sequence contributes explicit causal edges. Parallel branches admit every
interleaving consistent with their branch edges and join condition; input order
or wall-clock sorting does not add causality. An iteration has an authored finite
maximum and a progress edge. Termination names the required branch/workflow
closure condition.

Every choice has one owning role, labels, branch guards and a visibility set.
The owner must be able to distinguish the branch from information available at
the decision point. A choice whose branches are distinguishable only by a hidden
fact is valid source syntax but refused by the selected protocol profile. This
is an admission/conformance premise; it does not claim a general projection
theorem.

Channel premises state participating roles, message types, ordering policy,
delivery cardinality and whether duplication can occur. V1 does not silently
assume exactly-once delivery, FIFO across channels, reliable delivery or globally
synchronized clocks. A trace is conformant only relative to the exact premises
selected by the request.

## Compensation semantics

A compensation definition binds a forward effect, a compensating operation,
the role that owes it, eligible failure/abort conditions, a retry relation, a
commit boundary and an authored recovery relation. Registration occurs after the
forward effect is established. An untriggered registered compensation is
inactive; an eligible but unobserved compensation is incomplete or pending under
the supplied progress/closure authority; an attempted compensation can succeed,
violate, fail or time out.

The recovery relation compares the declared post-recovery state with its
authored target. A partial refund can satisfy a partial recovery obligation while
failing full restoration. Two refund attempts, two transport receipts and two
refund effects are three different cardinalities. Retrying under the same effect
identity cannot manufacture another successful business effect. Commit before a
required registration, compensation before its forward effect, and compensation
after a forbidding commit are protocol violations.

## Shared assessment result

Each requested subject/capability pair produces one retained result even when it
cannot execute. The minimum orthogonal dimensions are:

| Dimension | Closed meanings |
| --- | --- |
| Assessment execution | completed, refused, unsupported, resource-incomplete, failed |
| Decision-scope progress | exact progress for the claim's decision scope with its authority and boundary identities |
| Decision-scope closure | open, closed, incomplete or contradicted for the exact decision scope and closure authority |
| Surrounding-execution progress | exact progress for the whole surrounding execution with its distinct authority and boundary identities |
| Surrounding-execution closure | open, closed, incomplete or contradicted for the exact surrounding execution and closure authority |
| Complete-global-conformance closure | closed, open, incomplete or contradicted plus exact required execution, branch and workflow closure-premise identities for a global-conformance claim; `not-required` with no premise set for another claim |
| Truth | satisfied, violated, pending, unavailable |
| Settlement basis | closed-scope, decisive-witness, decisive-counterexample, unsettled, unavailable, with the exact decision-support set |
| Activation | triggered, inactive, activation-unknown |
| Participation | exact participating and required role-instance sets, or unknown with reasons |
| Assumptions | exact assumption identities, each met, not-met, unknown or not-assessed |
| Completeness | complete, open, incomplete, with the exact facts required by each returned claim, missing inputs and authority/progress identities; it does not encode any closure axis |
| Observation adequacy | adequate, inadequate, unknown or not-assessed for the exact observations required to interpret the result |
| Protocol adequacy | demonstrated, not-demonstrated, inapplicable, unknown, with the exact obligations and cases considered |
| Claim | global-conformance, monitorability, local-projection, realizability, refinement or composition |
| Provenance | source/profile/model/protocol/request/binding/observation/progress/backend identities and derived artifact digests |

The combination is constrained by the normative table in
[FR-061](../../spec/functional/FR-061-report-orthogonal-results.md). Missing
observations, required roles, population members or adverse cases cannot change
truth, adequacy or package-level status from unsuccessful or unknown to
successful. Independent supported results remain available when another request
is unsupported.

Complete global conformance is a separate claim whose closure record retains
every required execution, branch and workflow premise. Neither a settled
per-obligation truth, a closed decision scope, a closed surrounding execution,
completed assessment execution nor completeness can substitute for that exact
record or one of its premises. Mutating the global closure record cannot erase
or change an independently settled per-obligation truth.

A completed assessment may settle satisfied from a decisive witness or violated
from a decisive counterexample while the assessed subject remains open. The
result retains that open subject progress, the exact settlement basis and every
supporting fact; it never relabels the subject as closed. The selected profile
must prove the result is preserved by all admitted continuations. Otherwise an
open subject remains pending, or unavailable when a required current fact is
missing.

A later contradiction to retained support, progress or closure creates a new
source-bound superseding or invalidating result. The prior bytes and identity
remain unchanged, and the contradiction is not treated as an admitted
continuation under the old input identity.

The result is a portable semantic contract for A/E/F and future Engineering
Assurance consumers. Quoin retains evidence ownership. The protocol evaluator
does not introduce a second evidence store, execution runner, report registry or
verification-method catalog.

The projection into `quire-verification/e02-draft-2` is a derived compatibility
result, not the canonical protocol result. FR-062 owns the total mapping, source
artifact retention, distinct derived identity, adequacy separation and refusal
rules. Consumers that need subject closure, settlement support, protocol
adequacy or claim meaning read the canonical result; they do not recover those
facts from the narrower E02 assessment view.

## Distinguishing corpus

The planned corpus uses orders O1 and O2, shared external payment provider P,
inventory, split shipments S1/S2, payment attempts A1/A2, payment effect E1 and
refunds R1/R2.

Accepted cases:

1. Payment success registers one refund obligation; concurrent S1/S2 branches
   join before order completion.
2. A failed shipment activates a refund; one retry shares the refund obligation
   but retains a distinct attempt; a single full refund effect satisfies the
   recovery relation.
3. Two related partial refunds sum exactly to the authored amount under a closed
   declared population and satisfy partial recovery.
4. O1 and O2 share provider P while retaining distinct workflow/effect subjects.
5. Global replay completes while local projection is explicitly unavailable.
6. `eventually[0,30] paid` settles satisfied at its admitted payment witness
   while the workflow remains open, and `always[0,30] no_duplicate_effect`
   settles violated at its first admitted counterexample while the workflow
   remains open; both retain exact decision support and open subject progress.

Refused, violating or incomplete controls:

1. A choice owned by fulfillment depends only on a payment-provider fact hidden
   from fulfillment: refused as unobservable.
2. The same transport delivery is relabeled as a second payment effect: refused
   identity binding or duplicate-effect violation.
3. Compensation appears before its forward effect, registration occurs after a
   forbidding commit, or refund occurs after that commit: violation.
4. A triggered refund fails, times out, or lacks closure/progress needed to
   decide: failed, pending or incomplete remain distinct.
5. A refund belongs to O2 while its compensation obligation belongs to O1:
   correlation refusal; equal trace/provider identifiers do not repair it.
6. One expected partial-refund population member is missing or membership is
   unknown: no aggregate success.
7. Mutual local assumptions admit a circular story without a causal global
   strategy: no realizability or composition success.
8. A satisfiable but unprojectable global trace remains eligible for global
   replay while the projection request is unavailable or refused.
9. An open prefix without a decisive basis is relabelled satisfied or violated,
   a decisive result omits one supporting fact, or open subject progress is
   restamped as closed: refused as an impossible result combination.
10. A late contradiction silently rewrites or reseals the earlier settled
    result instead of emitting a linked superseding or invalidating result:
    refused as result-history corruption.

Every case pins finite branch, iteration, message, active-instance and observation
bounds in its fixture. Example cardinalities do not become semantic maxima.

## Reuse and claim limits

No external protocol, solver or monitor library is adopted by this specification
slice. The mapping at this revision is explicit:

| Capability | Selected ownership | External candidate disposition |
| --- | --- | --- |
| Protocol representation, compensation and shared result meaning | Owned Quire semantics in this specification | No external semantic authority. |
| Finite global conformance | Required future first-party Rust implementation or qualified Rust library | Candidate selection open under E03; no version or license accepted yet. |
| Projection, realizability, refinement and composition | Separately requested later analysis capabilities | Research only; no theorem, algorithm, version or dependency adopted. |
| Temporal monitoring used by protocol predicates | E-owned native/TL bridge and qualified Rust consumers | R2U2/C2PO and other foreign-runtime candidates are not production or qualification dependencies. |
| Execution and evidence retention | Engineering Assurance and Quoin contracts | Consumed interfaces; the protocol implementation owns neither runner nor evidence store. |

PT01/PT02 may adopt a Rust library only after E03 records exact version, source,
license, runtime boundary, admitted algorithm/profile and discriminating adverse
cases. Similar terminology does not transfer a theorem. General projection,
realizability and composition are not prerequisites for the selected global
replay/conformance slice.

The maintained architecture and its library list are research inputs. Earlier
FRETish-source and foreign-runtime recommendations are superseded by the owner
ruling: native Quire is the sole editable formal-clause language, external forms
are export-only, and production or possible qualification integrations are Rust.
Until a required producer interface is reviewed and available, the affected
request is refused or reported unsupported with the missing dependency. No
hardcoded fixture, inferred default, source-text adapter or local stub substitutes
for A/D/E/F/Engineering Assurance producer contracts.
