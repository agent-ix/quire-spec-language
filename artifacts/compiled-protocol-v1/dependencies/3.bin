# Native choreography surface and bindings

Proposed A-owned source integration for B's [protocol contract](protocol-contract.md)
and FR-050–061. The productions live in the one [shared grammar](shared-grammar.md).
This draft defines no second source language, domain schema, workflow executor
or evidence system. Profile definitions and the composed baseline remain subject
to agreement and review before implementation.

## Declarations and authority

A protocol's `over` input is a producer-declared workflow view at an explicitly
bound observation/decision instant. Its activation and initial captures follow
the common rules: an authoritative origin or an explicitly bound semantic
trigger, with immutable captures and no ambient `self`, `result` or pre-state.
Protocol activation identifies an assessed workflow instance; it does not spawn
a service or business workflow. Repeated receipts do not create new instances.

| Native declaration | Linked meaning and required input |
| --- | --- |
| `role Payments on M::PaymentService;` | One authored responsibility and one role-instance binding per assessed workflow. The imported declaration supplies an admissible participant contract. It is not a payload record relabeled as a role. Different role slots may bind the same component when the configuration explicitly permits it. |
| `relationship ShipmentOwner = M::OrderShipment;` | A reference to a first-class relationship declaration, including exact endpoint types, direction, roles and multiplicities. It neither creates that model relationship nor derives one from a field. |
| `channel Requests from Orders to Payments carries M::ChargeRequest ...;` | Typed endpoints and payload, with explicit ordering and delivery premises. A queue address cannot substitute for a role instance. |
| `requires temporal OrderClosureBounded;` | An exact reference to a native temporal declaration in the same linked package, required for the protocol's assessed subject. Its own profile, clock role, trigger and capture contracts remain intact. |
| `compensate Refund for Main::Applied ...` | A compensation template paired with an explicitly named successful-effect node. Every runtime registration also names the concrete forward-effect occurrence. |

Each role slot is static and finite. Reusing a protocol for O1 and O2 creates
distinct assessed role/workflow subjects, even when both use the same external
payment provider. Arbitrary dynamic role creation is not encoded through these
declarations. Related shipment/payment/refund instances are supplied through
declared typed relationships; they need not share a transport trace identifier.

Role/interface, relationship, operation and payload positions resolve different
model declaration kinds. The baseline-1.2 model work in FCD PR #97 is inspected
draft input, not an accepted implementation dependency. D still supplies the
exact standard-facing model/configuration contracts; the compiler refuses a
kind mismatch or unresolved producer capability instead of inventing a model.

Channel `delivery [a,b]` declares inclusive per-send delivery cardinality with
`0 <= a <= b` and finite b. `[0,3]` permits loss and bounded duplicate delivery;
`[1,3]` requires at least one delivery on a complete assessed history; `[1,1]`
states an exactly-one-delivery premise. None promises exactly one business effect.
`unordered` adds no FIFO edge. `fifo by (m: M::Message) { m.key }` declares FIFO
only within that channel and exact typed key. The key expression is total and
read-only; its result must admit stable equality under the selected model type.
No cross-channel FIFO, delivery guarantee or synchronized clock is inferred.

## Events, values and causal control

Every event node declares a local source name and a typed bound record. F supplies
the correspondence from the concrete record to the workflow, node occurrence,
role, operation, message/delivery/attempt/effect and observation identities. The
record payload alone cannot prove that correspondence. The node's expression
is a total Boolean constraint evaluated with that record and the available
immutable values. A false constraint is a violation for an otherwise validly
bound occurrence; unavailable input cannot be converted into a false constraint.

| Node | What it observes |
| --- | --- |
| `send ... via C` | A semantic send on the declared channel, with its own identity and typed payload. |
| `receive ... via C of SendNode` | A distinct receive related to the exact concrete send occurrence; duplicate transport receipts do not create another node occurrence. |
| `attempt ... by R on M::T::op contracts [...]` | One operation attempt under the named role and operation. Listed state pre/postcondition declarations must match this operation and its invocation anchors; missing post-state never becomes successful evaluation. An empty list explicitly requests no extra native state clauses, without bypassing the operation's model/frame contract. |
| `effect ... of AttemptNode` | A separately established successful business effect of that concrete attempt. A successful transport status or an attempt's existence is insufficient. |
| `event ... by R for Compensation` | A typed domain event associated with the exact registered compensation occurrence when `for` is present. Without `for`, it is still an explicitly bound domain event, not an effect-registration shortcut. |
| `check ... using S { p };` | A shared state predicate at the node's bound evaluation instant. It neither waits for p nor invokes business code. |
| `commit ... by R as (...) { p };` | An explicitly observed commit boundary, constrained by p and its causal position. Merely reaching a parser token does not manufacture a commit observation. |

`related by R(left,right)` on an event node requires F's supplied instance of
the imported relationship R with those exact typed endpoints. Both endpoint
expressions use the common type checker. Equality of two scalar IDs does not
establish a relationship, endpoint direction or complete membership. The
annotation does not dereference an opaque UUID or query an ambient object store.
Multiple annotations may require different relationships for the same node;
each is independently bound, so an order relation cannot replace the required
refund-to-payment-effect relation.

Control names and declaration paths identify static nodes. The canonical runtime
occurrence additionally includes the workflow identity and every enclosing
bounded-iteration ordinal. Relative references resolve in their lexical control
scope; fully qualified paths such as `Main::Applied` retain the same resolved
identity. Duplicate or ambiguous declarations refuse. Explicit forward references
in channel/compensation metadata can resolve after declaration collection; value
availability is checked independently.

| Control | Causal and value rule |
| --- | --- |
| `sequence Name { ... }` | Each child completes before the next child begins. This explicit construct gives its authored order meaning; arbitrary table or map order adds none. |
| `parallel Name { branch a ... branch b ... } join all [a,b];` | Branches preserve their own edges and otherwise permit independent interleavings. The join lists every branch exactly once and completes only after all listed branches complete. No implicit cancellation or race winner is introduced. |
| `choice Name by R visible (...) { case x when { p } ... }` | One role owns the decision. Guards are total, mutually exclusive and collectively exhaustive over admitted decision inputs. Each required distinguishing fact needs provenance visible to R at that decision; listing an expression is a visibility obligation, not proof of visibility. |
| `repeat Name by R visible (...) max N while { p } Body exhausted Limit` | Evaluate p at each bound decision instant. False exits normally. True with fewer than N completed iterations enters Body; true after N enters Limit once. The exhausted branch is explicit and never a fabricated false guard. |
| `await Name after Anchor ... within [a,b] match Event then Yes timeout No` | Wait for the bound event relative to the exact anchor occurrence and selected temporal clock. The two paths follow the deadline rules below. |

Every potentially repeating body must make observable causal progress on every
continuing path. A body made solely of checks or empty groups cannot establish
that progress. N is an authored finite maximum, distinct from execution budgets;
N=0 evaluates the guard and selects normal exit or the exhausted branch without
running the body. The logical bounded unfolding is finite and acyclic. An
implementation need not eagerly materialize it, but must preserve every node's
iteration identity and charge expansion before exceeding a resource limit.

All-branch join is the join rule spelled by this candidate. Another join policy
requires its own admitted rule, including what happens to outstanding branches;
it cannot be represented by silently ignoring an unfinished branch. This does
not defer ordinary finite parallelism or related-instance choreography.

Node record binders are immutable. A sequence successor may read a predecessor's
record when that predecessor necessarily occurred on its path. Parallel siblings
cannot read each other's results before their join; after an all-branch join,
records produced on every path in each branch are available. A choice's optional
branch result cannot escape that branch. Iteration-local records cannot escape
as an implicit last result; an explicitly supplied finite result view can be
queried afterward. The matched event of an await is visible only on its `then`
path. Unavailable values refuse linking, rather than becoming runtime nulls.

The protocol's `over` view is different: it denotes a separately bound view at
each evaluation instant, not a cached mutable record. A capture freezes a value
and its provenance at its authored anchor. Each visibility and presence fact is
tied to its actual value/observation; neither can move to another snapshot.

## Deadlines, registration and recovery

An await anchor is an exact preceding event occurrence, or the activation of a
named compensation registration uniquely selected for this workflow/iteration.
A structural group name, timestamp alone or arbitrary latest event cannot serve
as the anchor. Ambiguous or missing occurrence selection is incomplete/refused.
Its interval has the selected temporal profile's tick meaning. Only a `receive`,
`effect` or typed domain `event` is admitted as the awaited event in this profile;
the broader syntactic event-node alternative does not admit waiting on a send
or attempt as a substitute for its business result.

An eligible matching event at the inclusive upper endpoint takes the `then`
path. Timeout takes the other path only after E/F's progress/completeness
premises establish that no eligible event occurred in the whole window. An
event-position clock does not advance during silence. A bound event with a false
constraint, missing valuation or wrong correlation does not select timeout as a
fallback. An input-order tie cannot defeat an event at the deadline. Evidence
that contradicts already-settled completeness follows F's explicit late-data
contract and preserves the old result identity.

A compensation template has two capture stages and one registration per concrete
forward effect and template:

1. Establish the successful forward effect; bind the template's `as` record and
   evaluate its initial captures at that effect's registration anchor.
2. Select the first eligible `activate first` trigger for that registration under
   an admitted causal/sequence order; evaluate the activation captures once.
   The guard can read the candidate trigger and registration captures. The
   forward record is registration-only, unless its values were captured.
3. Assess at most the declared positive number of compensation attempts, counting
   the initial attempt. The retry relation additionally constrains each actual
   predecessor/successor attempt pair; equal counters cannot invent that causal
   relationship. Retries retain one obligation but distinct attempt identities.
4. Assess the typed recovery view against registration/activation captures under
   the inclusive activation-relative deadline and the declared commit boundary.

Two concurrent eligible triggers without a sufficient ordering/binding premise
cannot be arbitrarily ordered by ingestion or timestamp. Repeated delivery of
one trigger does not reactivate the registration. Later distinct eligible
triggers do not register additional compensation for the same forward effect.
Initial and activation captures have separate lexical scopes; activation values
cannot be read before activation, and recovery/attempt inputs cannot be captured
retroactively at registration. All initializers remain total and read-only.

The `commit` member names the exact forbidding commit node, or explicitly says
`never`. The latter means no earlier commit cutoff, not permission to extend a
closed execution indefinitely. Compensation before its forward effect or after
its forbidding commit violates the protocol. Missing commit/effect/trigger input
does not prove the absence of that boundary. Attempt failure, timeout, retry,
duplicate effect and unknown activation remain distinct facts under B's results.

`recover (r: M::RecoveryView) { p }` uses the common predicate language over a
supplied achieved-state/effect view and captured target. A successful refund call
does not imply p. A finite aggregate uses the view's declared sequence bound,
relationship membership, snapshot/window and completeness; it counts distinct
business effects as specified, without counting repeated transport receipts as
effects or silently dropping duplicate business effects.

The final `finish` node requires its exact workflow-closure observation after
the root control completes. Complete global success additionally requires the
selected state/temporal/protocol obligations and communication premises to be
discharged. A matched finish payload, EOF or lack of a violation is insufficient.
No declaration here sends messages, performs attempts or executes compensation.

## Composed source example

This is a declaration fragment; a complete fixture must import exact synthetic
model/profile definitions. M exports distinct order/shipment references, roles,
relationship and operation declarations, and typed observation records. In the
recovery view, each refund amount is integer MoneyItem 1..20/U and the ordered
sequence maximum is five; MoneyTotal is integer 0..100/U. AttemptNumber is 1..3.
All referenced predicate and state/temporal declarations are exact linked
declarations, not built-ins inferred from the names below.

```quire
temporal OrderClosureBounded using T
    over (sample: M::OrderView) clock "order-clock" on origin {
  eventually[0,300] holds(sample.closed)
}

protocol OrderFlow using P
    over (view: M::OrderView) on each (opened: M::OrderStarted) {
  capture order: M::OrderRef = opened.order;
  role Orders on M::OrderService;
  role Payments on M::PaymentService;
  role Fulfillment on M::ShipmentService;
  relationship ShipmentOwner = M::OrderShipment;
  channel Charges from Orders to Payments carries M::ChargeRequest
      ordering fifo by (message: M::ChargeRequest) { message.order }
      delivery [0,3];
  requires temporal OrderClosureBounded;

  compensate Refund for Main::Applied as (forward: M::ChargeEffect)
      by Payments on M::Payment::refund using T clock "refund-clock" {
    capture target: M::MoneyTotal = forward.refundable;
    activate first (failed: M::ShipmentAttempt)
        when { SameOrder(failed, order) and not failed.succeeded } { }
    within [0,30];
    attempts 3 of M::RefundAttempt;
    retry (earlier: M::RefundAttempt, later: M::RefundAttempt) {
      RetryEligible(earlier) and earlier.number < 3 and
      later.number = earlier.number + 1 and
      later.obligation = earlier.obligation
    };
    commit Main::Finalized;
    recover (recovery: M::RecoveryView) {
      sum<M::MoneyTotal>(refund in recovery.refunds: refund.amount) = target
    };
  }

  run sequence Main {
    send Requested via Charges as (sent: M::ChargeRequest) { sent.order = order };
    receive Received via Charges of Main::Requested
        as (received: M::ChargeRequest) { received.order = order };
    attempt Charged by Payments on M::Payment::charge
        contracts [CanCharge,ChargePost] as (charge: M::ChargeAttempt) { true };
    effect Applied of Main::Charged as (paid: M::ChargeEffect) { true };
    parallel Shipments {
      branch east attempt East by Fulfillment on M::Shipment::dispatch
          contracts [] as (eastResult: M::ShipmentAttempt)
          related by ShipmentOwner(order, eastResult.shipment) { true };
      branch west attempt West by Fulfillment on M::Shipment::dispatch
          contracts [] as (westResult: M::ShipmentAttempt)
          related by ShipmentOwner(order, westResult.shipment) { true };
    } join all [east,west];
    choice Outcome by Fulfillment visible (eastResult.succeeded,westResult.succeeded) {
      case shipped when { eastResult.succeeded and westResult.succeeded }
          sequence Shipped { check BothShipped using S { true }; }
      case failed when { not (eastResult.succeeded and westResult.succeeded) }
          sequence Recovered { check RecoveryAccounted using S { RecoveryHolds(view) }; }
    }
    commit Finalized by Orders as (committed: M::OrderCommit) { committed.order = order };
  }
  finish Closed as (closed: M::OrderClosed) { closed.order = order };
}
```

The failed branch's check observes recovery at its bound decision instant; it
does not cause or wait for a refund. A valid recovered trace supplies the actual
refund effects, complete recovery view and causal observations before commit.
Committing prematurely, substituting O2's refund for O1, or presenting an unknown
refund population cannot pass this branch. On the successful shipment branch,
the refund registration remains untriggered and is not demonstrated recovery.

An alternative to an immediate receive explicitly models a response deadline:

```quire
await ResponseWindow after Main::Requested using T clock "reply-clock" within [0,30]
  match receive Reply via Charges of Main::Requested
      as (reply: M::ChargeRequest) { reply.order = order };
  then sequence Responded { check Matched using S { reply.order = order }; }
  timeout sequence TimedOut { check RecordedTimeout using S { TimeoutRecorded(view) }; }
```

This fragment is an alternative control node, not an additional second receive
for the first example. A repeat likewise makes its bound and exhausted path
explicit:

```quire
repeat Poll by Fulfillment visible (view.moreWork) max 3 while { view.moreWork }
  sequence Pass {
    attempt Polled by Fulfillment on M::Shipment::poll contracts []
        as (pollResult: M::PollAttempt) { true };
  }
  exhausted sequence LimitReached { check ExhaustionRecorded using S { view.exhausted }; }
```

The planned cases in TC-170–174 discriminate the source structure and binding
rules. B's protocol, E's temporal and F's observation cases qualify their owned
consumer meanings separately. These fragments do not establish parser support,
typed model availability, global conformance or local realizability today.
