---
id: FR-218
title: "Check every protocol control construct the protocol system runs at S3"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-024
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-027
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-017
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-104
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-112
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-113
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-114
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-052
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-053
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-170
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-171
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-228
    type: depends_on
---
# FR-218: Check every protocol control construct the protocol system runs at S3

## Description

When S3 checks a protocol declaration, the `ProtocolClause` family SHALL
check `parallel` with its branches and its join policy and `outstanding`
rule, `choice`, `repeat`, `await`, `check`, `send`, `receive`, `effect`,
`event` and `commit` nodes, channels, captures, compensation templates,
replicated roles, `activation on each`, and node bodies, and SHALL produce
an in-process `CheckedProtocolClause` holding the control tree, the
compiled causal edge relation and the scopes the protocol system reads
(ADR-027 §9 DS-1, PS-5, RR-4). The `ProtocolClause` family's identities
stay as they are (ADR-017 PF-7, scenario 5).

## Use case

A verification operator writes a protocol with two branches, a channel
between them, a choice on a received value and a refund template. They need
it checked as a whole, with a mistake such as a join that names a missing
branch or a guard that reads model state refused at its span, so that the
protocol system only ever runs a protocol whose meaning is fixed.

## Inputs

- The S2 forms of a protocol declaration, with its scoped anchors resolved
  (FR-113) and its attempts bound (FR-114).

## Outputs

- `CheckedProtocolClause` (`qsl_semantics::check::protocol_clause`): the control tree
  with each node's checked kind, binder type and body; the compiled edge
  relation (structural, `branch`, `join`, send.out → receive.in,
  attempt.out → effect.in and await anchor edges); each binder's reader set
  by S3's scope; each replicated role's `scope` region; channels with
  capacity, overflow, delivery and ordering; compensation templates; roles;
  the activation; and the protocol's `terminal` and `scheduling` members
  (FR-211, FR-212).
- A typed `CheckRefusal` with a span on refusal, and no checked clause.

## Behavior

### Constructs

- S3 SHALL check each node kind, control and declaration listed in the
  Description against QSpec's grammar and contracts for it (QSpec FR-052,
  FR-053, FR-170, FR-171, FR-228), and SHALL record each in the checked
  clause.
- S3 SHALL check each body (event, send, receive, finish and capture
  constraints, guards, activation guards, retry relations and recovery
  predicates) through the one clause checker (FR-104) over its declared
  inputs.
- A choice guard, repeat guard or `check` SHALL read binders and constants
  only. S3 SHALL refuse one that reads model state with `ill_typed`/
  `operator-ineligible` at the read, naming the guard.
- S3 SHALL refuse a choice whose guards are not proved exhaustive and
  exclusive (QSpec FR-053) with `undefined_expression`/
  `unproved-exhaustiveness` at the choice.
- S3 SHALL refuse a join that names a label that is not a branch of its
  `parallel` with `missing_declaration`/`missing-name` at the label.
- S3 SHALL refuse a causal edge relation with a cycle with
  `invalid_package`/`definition-cycle`, naming the nodes on the cycle.
- A `repeat` SHALL have progress: every cycle in its body graph passes
  through a step node, with the body graph and step nodes as ADR-027 PS-6
  defines them: an event node, an `await`, a `parallel` (its `fork` and
  `join`) and `finish` are step nodes, and a nested `repeat` contributes
  both its back edge and its skip edge. S3 SHALL refuse a
  `repeat` with a cycle through no step node, such as a body `choice` with
  a step in one case and none in another, with
  `unsupported_construct`/`declaration-form` at the `repeat`, naming it
  (QSpec FR-052).
- S3 SHALL check a `repeat` with or without an iteration maximum and with or
  without an invariant and variant, and SHALL record the maximum, invariant
  and variant it has. A termination claim is the only reader of the
  variant.
- `parallel`, `repeat`, `choice` and `sequence` SHALL nest with no ceiling
  of their own; only the caller's resource limits bound a declaration's
  size.

### Scopes

- For each binder, S3 SHALL record the set of nodes that read it, by the
  scope rules of the compiled form: a branch binder stays readable after an
  all-branch join; a binder read outside its scope refuses as FR-113
  refuses an unresolved name.
- For each replicated role with lifetime `scope`, S3 SHALL record the
  smallest control holding every node `by` that role.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-218-AC-1 | ADR-027 §7's `Fill`, §7.1's `Pay`, FR-212's `Chatter` and FR-208-AC-1's replicated-role protocol each check to a `CheckedProtocolClause`; none refuses `unsupported_construct`/`not-yet-implemented`. `Fill`'s edge relation holds in → `A`, in → `B`, `A` → join and `B` → join; `Chatter`'s holds `Tx`.out → `Rx`.in. | Test (TC-663) |
| FR-218-AC-2 | A `choice` guard that reads `k.v` refuses `ill_typed`/`operator-ineligible` at the read. A two-case `choice` whose guards are `m.n > 0` and `m.n > 1` refuses `undefined_expression`/`unproved-exhaustiveness`. `join all [left, middle]` in `Fill` refuses `missing_declaration`/`missing-name` at `middle`. | Test (TC-663) |
| FR-218-AC-3 | In a variant of `Fill` where a `check` after the join reads binder `a`, the checked clause records that `check` as `a`'s only reader; a node in branch `right` that reads `a` refuses as FR-113 refuses an unresolved name. In FR-208-AC-1's protocol with lifetime `scope`, the recorded region is the smallest control holding `Work`. | Test (TC-663) |
| FR-218-AC-4 | A `parallel` nested inside a `repeat` inside a branch of another `parallel`, three levels deep, checks; its checked tree holds all three levels with their paths. | Test (TC-663) |
| FR-218-AC-5 | A `repeat` whose body is one attempt checks with a maximum, with an invariant and variant and no maximum, with both, and with neither, and the checked clause records the maximum, invariant and variant each has. A `repeat` whose body is only a `check` refuses `unsupported_construct`/`declaration-form` at the `repeat`, naming it, and so does a `repeat` whose body is a `choice` with one attempt in one case and an empty `sequence` in the other. | Test (TC-663) |
| FR-218-AC-6 | A protocol whose `send Tx` on channel `ch` follows `receive Rx` on the same thread, while `Rx` receives only from `Tx`, refuses `invalid_package`/`definition-cycle` naming `Tx` and `Rx`; with `Tx` on one branch and `Rx` on another, the protocol checks. | Test (TC-663) |
| FR-218-AC-7 | A `repeat` whose body is a `choice` with an attempt in one case and a nested `repeat` around an attempt in the other refuses `unsupported_construct`/`declaration-form` at the outer `repeat`, because the nested `repeat`'s skip edge closes a cycle with no step; a `repeat` whose body is one `await` with a `timeout` control, or one `parallel` with a `join`, checks. | Test (TC-663) |

## Dependencies

- ADR-027 §9 DS-1, §1 PS-5 and PS-6, §2.2a RR-4, §2.1 FO-2; ADR-017 PF-7
  (scenario 5).
- [FR-112](FR-112-build-protocol-scoped-anchor-forms.md),
  [FR-113](FR-113-resolve-scoped-anchors-in-nested-control-scopes.md),
  [FR-114](FR-114-bind-a-protocol-attempt-to-its-operation-frame.md),
  [FR-104](FR-104-check-state-clauses.md).
- QSpec owns the grammar and contracts of every construct, and the catalog
  spellings of its refusals: QSpec FR-052, FR-053, FR-170, FR-171, FR-228.

## References

- Owning ticket: Linear QSL-396. QSpec half: QSpec FR-052, including its loop rule
  (Linear STD-140).
