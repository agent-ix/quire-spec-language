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
  - target: ix://agent-ix/quire-specification/FR-146
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-065
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-051
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-054
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-466
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-086
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-088
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-308
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-381
    type: traces_to
---
# FR-218: Check every protocol control construct the protocol system runs at S3

## Description

When S3 checks a protocol declaration, the `ProtocolClause` family SHALL
check `parallel` with its branches and its join policy and `outstanding`
rule, `choice`, `repeat`, `await`, `check`, `send`, `receive`, `effect`,
`event` and `commit` nodes, channels with their premises, relationship
declarations and `related by` clauses, `requires temporal` requirements,
replicated roles, the owner and visible facts of each `choice` and `repeat`,
and node bodies, and SHALL produce
an in-process `CheckedProtocolClause` holding the control tree, the
compiled causal edge relation and the scopes the protocol system reads
(ADR-027 §9 DS-1, PS-5, RR-4). S4 emits the checked clause as the protocol
nodes of FR-383 and reads them back equal to it. The `ProtocolClause` family's identities
stay as they are (ADR-017 PF-7, scenario 5). The obligation members of a
protocol (its captures, `activation on each` and compensation templates) are
checked by FR-381 and held by the same checked clause.

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
  capacity, overflow, delivery and ordering; roles; the declared
  relationships, each with the key of its model relationship and the types of
  its two ends; each event node's `related by` clauses, in source order; the
  `requires temporal` requirements, each bound to the temporal clause it names
  by that clause's position among the unit's `temporal` clauses in declaration
  order; the owner and visible facts of each `choice` and `repeat`; the owner of each `send` (its channel's `from` role) and each `receive` (its channel's `to` role);
  the compensation templates and the activation (FR-381); and the protocol's
  `terminal` and `scheduling` members (FR-211, FR-212).
- A typed `CheckRefusal` with a span on refusal, and no checked clause.
  Every refusal in this requirement holds the S3 stage, the `ProtocolClause`
  family, the code and cause named at it, the located span named at it and the
  members named at it (for example `channel`, `expected`, `actual`), so a
  consumer reads each as a field and never from a message.

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
- A choice guard, repeat guard or `check` SHALL read binders, protocol captures and constants
  only. S3 SHALL refuse one that reads model state with `ill_typed`/
  `operator-ineligible` at the read, naming the guard.
- Model state is what a read that lowers to a model-state operation reads (QSpec
  FR-464, "Slots that bar model state"): a field read of a binder whose type is
  an object type, a field read through a reference-typed value, an explicit
  dereference, a navigation or reachability over a relationship end, a lookup
  or population read, a dispatch call and a pre-state read. A choice guard, a
  repeat guard, a `check`, a `visible` entry and a FIFO key bar it, as the
  activation guard, trigger guard, retry relation and recovery predicate do
  (FR-381). A field read of a record binder or an event binder is not a
  model-state read.
- S3 SHALL refuse a choice whose guards are not proved exhaustive and
  exclusive (QSpec FR-053) with `undefined_expression`/
  `unproved-exhaustiveness` at the choice (Choice and repeat decisions).
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
  (QSpec FR-052). For this progress proof, a nested guarded repeat contributes
  its back edge and contributes its skip edge only when its guard can be false
  under the independent-atom proof below. A nested repeat whose guard cannot
  be false contributes the progress established by its entered body instead
  of a skip edge.
- When an enclosing repeat's progress depends on a nested guarded repeat,
  S3 SHALL decide whether the nested repeat can exit before entering its
  body by evaluating its guard over the same conservative independent-atom
  Boolean abstraction used for choice partitioning. It SHALL use the exact
  checked-term identity rule for atoms stated below. If at least one
  valuation makes the nested guard false, that nested repeat contributes no
  guaranteed progress to the enclosing repeat. If every valuation makes the
  guard true, the nested repeat contributes the progress established by its
  entered body. This proof concerns only whether the nested repeat may skip
  its body; it does not claim termination or replace its authored maximum,
  invariant, variant or exhaustion behavior.
- S3 SHALL check a `repeat` with or without an iteration maximum and with or
  without an invariant and variant, and SHALL record the maximum, invariant
  and variant it has. A termination claim is the only reader of the
  variant.
- `parallel`, `repeat`, `choice` and `sequence` SHALL nest with no ceiling
  of their own; only the caller's resource limits bound a declaration's
  size, and the partition proof of a `choice` charges `s3.work_units`
  (Choice and repeat decisions).

### Admission for emission

The QSpec protocol node contract (QSpec FR-464, FR-465, FR-466) refuses the
nodes of each rule below, so each rule is refused before S4, by S3 at its
construct or, where a bullet says so, by the parser, and the construct never
reaches S4 (FR-383).

- S3 SHALL refuse two cases of one `choice`, or two branches of one
  `parallel`, with equal labels with `ambiguous_declaration`/`ambiguous-name`
  at both labels, naming the label, in source order.
- A `choice` with fewer than two cases and a `parallel` with fewer than two
  branches are not in the grammar and fail at the parser with its syntax
  diagnostic, before any form is built (FR-308).
- S3 SHALL refuse `join all` whose list omits a branch, a `quorum(n)` whose
  `n` is below one or above the number of distinct labels joined, and a
  `join any` or `join predicate` that lists no label, with
  `invalid_package`/`invalid-value` at the list (the count for `quorum`). A
  label listed twice is one joined label.
- S3 SHALL refuse a role whose model is neither a model alias (Role on a
  model alias) nor an object type, a process, a state machine, a systems
  interface or a persistence interface with
  `invalid_model_binding`/`malformed-declaration` at the model name, and a
  channel whose `carries` type is none of a scalar type, a composite type, a
  record, an event type or a variant type with the same refusal at the type
  name. S3 SHALL NOT compare a `send` or `receive` binder, or a FIFO binder,
  with a refused `carries` type.

### Predicate joins

`join predicate(f) [l1, ..., ln]` settles when the function `f`, applied to
the dispositions of the joined branches, is true (QSpec FR-171, FR-228). The
function sees the joined labels only, which are the branch dispositions the
join captures explicitly (QSpec shared grammar), and a branch the join does
not list is not an argument.

- S3 SHALL resolve `f` once, in the unit's own declaration scope, as a call's
  name resolves (FR-065). `f` is an unqualified name, so it names a function
  the unit declares. A name that resolves to no declaration of the unit SHALL
  refuse with `missing_declaration`/`missing-name` at the name, retaining
  `node` (the `parallel`) and `name`. A name that resolves to a declaration
  that is not a `function`, such as a `predicate`, a clause, a type or a
  protocol, SHALL refuse with `ill_typed`/`operator-ineligible` at the name,
  naming the resolved kind, as QSpec FR-146 refuses a call to a declaration
  that is not a checked pure function.
- The joined labels, each counted once, are taken in ascending order of their
  UTF-8 bytes, which is the order S4 writes the `joined` labels (FR-383).
  The function SHALL have one parameter for each of those labels, the `k`-th
  parameter receiving the disposition of the `k`-th label, and SHALL have the
  result type `Boolean`. Each parameter SHALL have the type `Boolean`: the
  argument is `true` when that branch thread is `completed` and `false` when
  it is `running`. No third value is needed, because a branch thread is
  `cancelled` only together with the thread that owns the join, and a
  `cancelled` thread takes no step (FR-206), so no join step ever reads a
  `cancelled` branch. S3 SHALL refuse a function whose parameter count
  differs from the number of distinct joined labels, a parameter whose type
  is not `Boolean`, or a result type that is not `Boolean`, with
  `ill_typed`/`type-mismatch` at the name, retaining `node`, `function`,
  `expected` (the required parameter types and result type) and `actual` (the
  function's declared ones).
- A function that reaches this check is an admitted function: QSpec FR-146
  establishes its purity, its total definedness and the termination of every
  call cycle it reaches when its own declaration is checked, and its body
  reads its parameters and constants only, never model state, a binder or a
  capture. The join therefore adds no purity or totality check of its own and
  no refusal for one. A unit whose function is impure, partial or has an
  unproved decrease refuses at that function's declaration by FR-146's
  rules, whatever join names it.
- S3 SHALL record the checked policy as `Predicate{function}`, `function`
  the typed node key (`quire.checked-semantic-node/v1`) of the resolved
  function's `function` node (FR-092), bound once and never looked up again
  by name (FR-088). Two joins that name one function hold one key. The
  joined labels stay recorded as for every other policy.

### Role on a model alias

S3 SHALL admit `role w on M`, `M` a model alias of the unit, as a static
role on the whole model `M` selects, recording the identity of the package `M`
selects. It admits the role whatever the clause's `over` type
is (a model type, a reference, a scalar, a record or a collection), and
beside roles of the same clause on other aliases: `role w on M` and
`role u on N` in one clause both check. S4 emits it as QSpec FR-465's role on
a whole model, whose `model` argument is that identity text (QSpec
FR-465-AC-9), and the subject builder binds it by construction (FR-205). A
role on a whole model attempts, owns choices and repeats, and sends and
receives as any static role does.

S3 admits a protocol whatever its `over` type, but S4 writes a clause node only
when `over` is a reference to an object type of a selected model, because QSpec
FR-464 requires the clause's `owner_model` edge (FR-383). Any other `over`
(a scalar, an `Option`, a collection) is refused at S4 as
`unsupported_construct`/`not-yet-implemented` naming `NoOwnerModel` (planner
ruling STD-498, QSL-1147); QSL-1165 tracks the follow-up.

A role on a whole model is static only (QSpec FR-465: a replicated role draws
its instances from a population of a model type, so its `model` is a
reference). S3 SHALL refuse a replicated role over a model alias,
`role r each M from pop` with `M` a model alias, with
`invalid_model_binding`/`malformed-declaration` at `M`, before it resolves
`pop`: a replicated role needs an object type, and QSL's I2 reader refuses
the same form under QSpec FR-465 step 1, so S3 admits nothing S4 cannot
emit (Admission for emission).

### Channels

A channel declares its two roles, its payload type, its ordering, its delivery
policy, its capacity and its overflow policy (QSpec FR-054, FR-170). Each
premise is a separate fact; S3 SHALL NOT derive one from another.

- S3 SHALL resolve a channel's `from` and `to` to roles the protocol declares.
  A name that is no declared role SHALL refuse with `missing_declaration`/
  `missing-name` at that role name, retaining `channel`, `end` (`from` or
  `to`) and `name`. Both ends refuse independently, `from` first.
- S3 SHALL resolve a channel's `carries` type as FR-104 resolves a declared
  type, and SHALL refuse an unresolved name with `missing_declaration`/
  `missing-name` at the type name, retaining `channel` and `name`.
- If a scope declares two channels of one name, then S3 SHALL refuse
  `ambiguous_declaration`/`ambiguous-name` at each declaration, naming both in
  source order, as FR-113 refuses two nodes of one name. Channels and
  relationships have separate namespaces.
- S3 SHALL record each checked channel under its qualified name, the
  qualified name of the protocol definition that declares it (QSpec
  FR-050), then `::`, then the channel's declaration-local handle (ADR-027
  PS-7). Two protocols
  that each declare a channel `Pay` declare two channels.
- A `send` or `receive` names a channel with `via`. S3 SHALL refuse a name
  that is no declared channel with `missing_declaration`/`missing-name` at the
  name, retaining `node` and `name`. It SHALL refuse a record binder whose
  declared type is not the channel's `carries` type, by type identity
  (FR-088), with `ill_typed`/`type-mismatch` at the binder's type, retaining
  `channel`, `expected` (the `carries` type) and `actual`. The rule that a
  `receive` names the channel of the `send` its `of` anchor names is FR-113's.
- For `ordering fifo by (m: T) {key}` S3 SHALL refuse a binder type `T` that
  is not the channel's `carries` type with `ill_typed`/`type-mismatch` at `T`,
  retaining `channel`, `expected` and `actual`. It SHALL check `key` through
  the clause checker over `m` and constants only, and SHALL refuse a read of
  model state with `ill_typed`/`operator-ineligible` at the read, naming the
  key. The key's result type SHALL be one over which the clause checker
  admits `=` (QSpec FR-054: stable equality); another type refuses
  `ill_typed`/`operator-ineligible` at the key, retaining `operator` (`=`) and
  the type. `ordering unordered` holds no binder and no key.
- S3 SHALL record on the checked channel the ordering (`Unordered` or
  `Fifo{binder, key}`), the delivery policy, the capacity (`Literal(u64)` or
  `Symbolic(name)`, the name unresolved) and the overflow policy (`Reject`,
  `Block` or `Loss(selector)`, the selector `Head` for the oldest queued
  message, `Tail` for the newest queued message and `New` for the arriving
  message, QSpec FR-170) exactly as written. A loss selector is a premise
  like the others and S3 derives none from the delivery policy. Two channels
  that differ in exactly one premise, the selector included, are different
  checked channels.

### Relationships

A relationship member `relationship r = M::R;` declares the protocol-local name
`r` for a relationship of the selected domain package (FR-056, FR-086). A
`related by r (a, b)` clause on an event node requires the supplied instance of
that relationship with those two typed ends (QSpec FR-051).

- S3 SHALL resolve `M::R` in the unit's own declaration scope. A name that
  resolves to no export of the package SHALL refuse with `missing_declaration`/
  `missing-name` at the name. A name that resolves to an export that is not a
  relationship SHALL refuse with `invalid_model_binding`/`wrong-export` at the
  name, retaining the required kind `relationship`, the actual kind and the
  export's declaration key, as FR-086 refuses a wrong kind. The relationship
  member being checked has no declaration key at S3, so the export's key is
  the only key the refusal holds.
- If a scope declares two relationships of one name, then S3 SHALL refuse
  `ambiguous_declaration`/`ambiguous-name` at each declaration, naming both in
  source order.
- S3 SHALL bind each checked relationship to its model relationship's
  declaration key and the types of its two ends, in the order the model
  declares them, never to the written name.
- S3 SHALL resolve a `related by` clause's relationship name among the
  protocol's relationship declarations. A name that is no declared relationship
  SHALL refuse with `missing_declaration`/`missing-name` at the name, retaining
  `node` and `name`.
- S3 SHALL check each of the clause's two ends through the clause checker over
  the binders readable at the node and the protocol's `over` input (an end is a value, not a guard), and SHALL refuse an end whose type is not
  its relationship end's type with `ill_typed`/`type-mismatch` at the end
  expression, retaining `relationship`, `end` (`first` or `second`),
  `expected` and `actual`. Two equal scalar ids never satisfy a relationship
  end.
- S3 SHALL check every `related by` clause of a node independently, so a node
  holds one checked clause per written clause, in source order, and an order
  relation never stands in for another clause.

### Requirements

- A `requires temporal T;` member names one temporal clause declared in the
  same unit. S3 SHALL resolve the written name once, in the unit's own
  declaration scope, and SHALL bind the checked requirement to that clause's
  position among the unit's `temporal` clauses in declaration order, never to
  the name; S4 SHALL mint the requirement's target node key
  (`quire.checked-semantic-node/v1`) from that position when it emits the
  clause, as a protocol reference binds its target (FR-092). No later stage
  looks the name up again (FR-088).
- S3 SHALL bind a requirement by position for every temporal clause,
  whatever its activation or captures, and SHALL decide the refusals below at
  S3 before S4 emits any node, so a unit with a requirement refusal reports it
  whatever its other clauses hold. FR-337 specifies the emission of every
  checked clause, from which S4 mints the target key.
- A name that resolves to no declaration of the unit SHALL refuse with
  `missing_declaration`/`missing-name` at the name. A name that resolves only
  through an import SHALL refuse the same way: a requirement binds a local
  clause only. A name that resolves to a declaration that is not a temporal
  clause SHALL refuse with `ill_typed`/`operator-ineligible` at the name,
  naming the resolved kind.
- A target named by two requirements SHALL be recorded once, in the position of
  its first requirement.
- A requirement whose temporal clause reaches the requiring protocol through a
  protocol reference, directly or through other clauses, is a definition cycle.
  S3 SHALL refuse it with `invalid_package`/`definition-cycle` at the
  requirement, naming the protocol and the temporal clause, as it refuses a
  causal cycle.

### Choice and repeat decisions

A `choice` and a `repeat` have one owning role, written `by R`, and a
`visible(...)` list of the facts the owner observes at the decision (QSpec
FR-053, FR-172). A decision fact is a field read of an eligible record
binder, of any type the clause checker admits (a Boolean flag, or a number or
other scalar a guard compares). A `visible` entry is either exactly one
decision fact, which a guard may read, or a composite expression over decision
facts, protocol captures and constants, which makes its whole value visible
and none of its operands. If a `visible` entry is a bare literal or bare
capture, then S3 SHALL refuse it with `ill_typed`/`operator-ineligible` at that
visible entry, the typed cause used for a bare binder read. A literal or
capture enclosed within a composite expression remains permitted.

- S3 SHALL refuse an owner that is no declared role with
  `missing_declaration`/`missing-name` at the owner name, retaining `node` and
  `name`.
- A record binder is eligible for owner `R` only as this table gives, and a
  binder of any other kind is never eligible:

  | Binder | Eligible for `R` when |
  | --- | --- |
  | `receive` record | `R` is the `to` role of the receive's channel |
  | `attempt` record | the attempt is `by R` |
  | `event` record | the event is `by R`, with or without `for` |
  | `send` payloads, `effect` and `commit` records, the protocol's activation binder | never |

  Where the control is a `repeat`, S3 SHALL make a record binder declared in its body readable
  at that repeat's `visible(...)` entries and `while` guard when the binder is
  readable at the body's exit. The value read at the repeat decision SHALL be
  the value from the most recently completed body iteration. This rule SHALL
  compose through the body's `sequence`, `branch`, `parallel` and other
  controls using their normal scope and join rules; it SHALL NOT make a
  sibling-branch value readable before an all-branch join or make a binder
  eligible for a role other than the role that produced it.

  Where the `while` guard reads a binder declared in that repeat's body, the
  repeat SHALL use post-test semantics: it SHALL execute the body once before
  evaluating the guard, then SHALL evaluate the guard after each completed
  body iteration using that iteration's binder values. Such a repeat SHALL
  complete at least one body iteration. Where the guard reads no binder
  declared in that repeat's body, the repeat SHALL retain pre-test semantics:
  it SHALL evaluate the guard on entry and after each completed body iteration,
  and a false entry guard SHALL skip the body.

  Where a post-test repeat has `max 0`, S3 SHALL refuse the repeat with
  `invalid_package`/`invalid-value` at the repeat declaration, because no
  execution can both complete its required first body iteration and respect
  the authored maximum.

  A `finish` record and a compensation's binders are bound where no node of
  the `run` tree reads them, so a decision that names one refuses as an
  out-of-scope read (Scopes) and never as `ineligible-kind`. The refusals of
  one read are ordered as QSpec FR-464 orders them: the `over`/model-state
  refusal first, then the out-of-scope refusal, then the decision-fact
  refusal, so each reason below is reachable only by a read the earlier
  refusals admit.

  The protocol's `over` input is model state, not a record binder. A guard,
  a `visible` entry or a FIFO key that reads it refuses `ill_typed`/
  `operator-ineligible` at the read, naming the guard; this rule has
  precedence over the table, which classifies record binders only. A protocol
  capture is an immutable value that every role holds: a guard or a `visible`
  entry may read one, and it needs no listing.

- Each `visible` entry and each guard (a `case when`, a `repeat while`) SHALL
  read decision facts of eligible binders, protocol captures and constants
  only. A guard SHALL read a fact only when the fact is an individual `visible`
  entry. S3 SHALL count
  a read inside a discarded `let` initializer as a read.
- S3 SHALL refuse a read of a fact that is not an eligible, individually listed
  fact of the owner with `wrong_snapshot`/`wrong-observation` at the read, retaining `node`,
  `owner`, `read` (the binder and field) and `reason`: `other-role` when the
  binder is eligible only for a different role, `ineligible-kind` when its kind
  is in the table's last row, and `unlisted` when it is eligible but no
  individual `visible` entry lists the fact. An eligible record that is not
  readable at the decision, such as a parallel sibling's record before the join,
  an optional branch's record or an `await` match read outside its `then` path,
  refuses as an out-of-scope read
  (Scopes).
- The partition of a `choice` SHALL be proved over the atoms of its guards as
  independent Booleans (QSpec FR-053). An atom is a Boolean fact, or a
  comparison or other Boolean-typed term over non-Boolean facts; two atoms are
  equal exactly when their checked terms are syntactically identical, and `not`
  is the only negation recognised. So `got.count > 0` and `not (got.count > 0)`
  partition, while `m.n > 0` and `m.n > 1`, or `m.n > 0` and `m.n <= 0`, are
  two independent atoms that leave a valuation uncovered and an overlap, and
  the check does not relate atoms by arithmetic. S3 SHALL refuse a gap or an overlap,
  including a gap or overlap between constant guards, with `undefined_expression`/
  `unproved-exhaustiveness` at the choice, retaining the choice's node identity
  in place of a `union-decl`, the names of the cases involved and `failed`, the
  non-empty ordered subset of `missing-arm` (a valuation no guard covers) and
  `duplicate-arm` (a valuation two guards cover) that the valuations show.
- The partition proof SHALL charge one unit of the S3 work meter
  (`s3.work_units`, FR-255) for each valuation of the choice's `n` distinct
  atoms it evaluates, `2^n` valuations in all, each charge made before that
  valuation is evaluated. The proof SHALL run during the protocol family's
  `check` of the declaration, after the declaration's own precheck charge
  (FR-062-AC-12) and before any lowering charge of that declaration
  (FR-096). Atoms SHALL be collected and compared once per guard term node,
  so collecting and comparing them adds no charge beyond the guard terms'
  node charges.
- When a valuation's charge would exceed the work budget, S3 SHALL stop with
  a `CheckRefusal`/`stage_limit_exceeded`, kind work budget and cause
  `work-budget-exceeded`, at the choice's span, naming
  the bound, the count the denied charge would reach and the setting
  `s3.work_units`, and SHALL NOT report `unproved-exhaustiveness` for that
  choice. The repeat-feasibility proof SHALL charge one unit of the same
  `s3.work_units` meter before evaluating each valuation of its distinct
  guard atoms. It MAY stop once it finds a valuation that makes the guard
  false; proving that no such valuation exists requires evaluating all
  `2^n` valuations. When a valuation's charge would exceed the budget, S3
  SHALL stop with a `CheckRefusal`/`stage_limit_exceeded`, kind work budget
  and cause `work-budget-exceeded`, at the nested repeat's span, naming the
  bound, the count the denied charge would reach and the setting
  `s3.work_units`; it SHALL NOT report a progress refusal for the enclosing
  repeat. Neither proof has an atom or valuation ceiling of its own
  (ADR-030): the caller's `s3.work_units` is its only bound, raised by
  FR-255's setting at every entry point. Atom collection and comparison add
  no charge beyond the checked guard-term node charges.

### Scopes

- For each binder, S3 SHALL record the set of nodes that read it, by the
  scope rules of the compiled form: a branch binder stays readable after an
  all-branch join; a binder read outside its scope refuses as FR-113
  refuses an unresolved name.
- For each replicated role with lifetime `scope`, S3 SHALL record the
  smallest control holding every node `by` that role, counting a `send` as a
  node `by` its channel's `from` role and a `receive` as a node `by` its
  channel's `to` role.
- S3 SHALL resolve the `pop` of `role r each T from pop` to a population
  declared by the domain package that declares `T`. It SHALL refuse a name
  that is no population of that package with `missing_declaration`/
  `missing-name` at the name, and a population whose element type is not
  `T` with `ill_typed`/`type-mismatch` at the name, each retaining `role`
  and `population`. The checked role holds the resolved population.

### Protocol references

A protocol reference expression (`protocol <qualified-name>`, FR-308) names
one protocol declared in the same unit. It carries no package selector.

- S3 SHALL resolve the written name once, in the unit's own declaration
  scope, to the target protocol's declaration, and SHALL bind the checked
  reference to that declaration's typed node key
  (`quire.checked-semantic-node/v1`), the key of the `protocol`/`protocol_clause`
  node FR-092 defines for it, never to the name. The checked
  reference holds the target key and the target's semantic type (today the
  Boolean type node the protocol clause carries) as its own type, and its
  value is the value of the target protocol clause (FR-308). No later stage looks the name up again (FR-088).
- A name that resolves to no declaration of the unit SHALL refuse
  with `missing_declaration`/`missing-name` at the reference, naming the
  written segments, as FR-113 refuses an unresolved name. A name that
  resolves only to a declaration reached through an import SHALL refuse the
  same way: a reference binds a local protocol only.
- A name that resolves to a declaration that is not a protocol SHALL refuse
  with `ill_typed`/`operator-ineligible` at the reference, naming the
  resolved kind. This is the cause QSpec FR-322-AC-66 gives a present target
  of another kind.
- A bound target key that no node of the checked graph carries when the
  reference is emitted is stale. S4 SHALL refuse it with
  `missing_declaration`/`missing-name` at the reference, naming the bound
  key, and produce no package (FR-092), as the IR reader and QSpec
  FR-322-AC-66 refuse a target key absent from the graph. A stale key is
  never repaired by looking the name up again.
- A package holds one reference node per target protocol. The carrier key
  (`quire.structural-node/v1`) is derived from the tag, form, semantic type
  and body and not from occurrences, and the body is exactly
  `{term: "reference", target}`, so two references to one target share one
  key and one node. A repeated reference to the same target in the same unit
  is admitted: it adds one `expression` occurrence, in source order, to the
  existing node (FR-092), and leaves the node's key, body and
  `dependencies` unchanged; the occurrences enter neither the node key nor the
  package identity.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-218-AC-1 | ADR-027 §7's `Fill`, §7.1's `Pay`, FR-212's `Chatter` and FR-208-AC-1's replicated-role protocol each check to a `CheckedProtocolClause`; none refuses `unsupported_construct`/`not-yet-implemented`. `Fill`'s edge relation holds in → `A`, in → `B`, `A` → join and `B` → join; `Chatter`'s holds `Tx`.out → `Rx`.in. | Test (TC-663) |
| FR-218-AC-2 | A `choice` guard that reads `k.v` refuses `ill_typed`/`operator-ineligible` at the read. A two-case `choice` whose `visible` entry `m.n` is an individual non-Boolean fact, admitted as a decision fact, and whose guards are `m.n > 0` and `m.n > 1`, refuses `undefined_expression`/`unproved-exhaustiveness`. `join all [left, middle]` in `Fill` refuses `missing_declaration`/`missing-name` at `middle`. | Test (TC-663, TC-1056) |
| FR-218-AC-3 | In a variant of `Fill` where a `check` after the join reads binder `a`, the checked clause records that `check` as `a`'s only reader; a node in branch `right` that reads `a` refuses as FR-113 refuses an unresolved name. In FR-208-AC-1's protocol with lifetime `scope`, the recorded region is the smallest control holding `Work`. | Test (TC-663) |
| FR-218-AC-4 | A `parallel` nested inside a `repeat` inside a branch of another `parallel`, three levels deep, checks; its checked tree holds all three levels with their paths. | Test (TC-663) |
| FR-218-AC-5 | A `repeat` whose body is one attempt checks with a maximum, with an invariant and variant and no maximum, with both, and with neither, and the checked clause records the maximum, invariant and variant each has. A `repeat` whose body is only a `check` refuses `unsupported_construct`/`declaration-form` at the `repeat`, naming it, and so does a `repeat` whose body is a `choice` with one attempt in one case and an empty `sequence` in the other. | Test (TC-663) |
| FR-218-AC-6 | A protocol whose `send Tx` on channel `ch` follows `receive Rx` on the same thread, while `Rx` receives only from `Tx`, refuses `invalid_package`/`definition-cycle` naming `Tx` and `Rx`; with `Tx` on one branch and `Rx` on another, the protocol checks. | Test (TC-663) |
| FR-218-AC-7 | A `repeat` whose body is a `choice` with an attempt in one case and a nested `repeat` around an attempt in the other refuses `unsupported_construct`/`declaration-form` at the outer `repeat`, because the nested `repeat`'s skip edge closes a cycle with no step; a `repeat` whose body is one `await` with a `timeout` control, or one `parallel` with a `join`, checks. | Test (TC-663) |
| FR-218-AC-8 | In a unit that declares `protocol Fill`, `protocol Fill` as an expression checks to a reference bound to `Fill`'s node key, typed with `Fill`'s semantic type. `protocol Missing` refuses `missing_declaration`/`missing-name` at the reference naming `Missing`; the same name offered only through an import alias refuses the same way; a name that resolves to a function refuses `ill_typed`/`operator-ineligible`. A second `protocol Fill` checks to a second reference bound to the same node key. A reference to a protocol requires a selected protocol profile (FR-092-AC-18): with none it refuses `missing_declaration`/`missing-selection`. Each refusal produces no package. | Test (TC-1021) |
| FR-218-AC-9 | A checked graph whose reference is bound to a key that no node of the graph carries, because the target protocol was re-keyed after binding, refuses `missing_declaration`/`missing-name` at the reference, naming the bound key, with no package and no name lookup. | Test (TC-1021) |
| FR-218-AC-10 | A channel `from Seller to Buyer` with no role `Buyer` refuses `missing_declaration`/`missing-name` at `Buyer`, retaining channel, `end` `to` and the name; with neither role declared both ends refuse, `from` first; `carries M::Absent` refuses `missing_declaration`/`missing-name` at `Absent`. Two channels `Pay` in one protocol refuse `ambiguous_declaration`/`ambiguous-name` at both declarations, and a channel and a relationship of one name check. | Test |
| FR-218-AC-11 | A `send` via a name that is no channel refuses `missing_declaration`/`missing-name` at the name. A `send` or `receive` whose binder type differs from the channel's `carries` type refuses `ill_typed`/`type-mismatch` at the binder type, retaining channel, expected and actual; the same node with the channel's type checks, and a `receive` via a channel other than its `send`'s still refuses as FR-113-AC-7 states. The binder of a `send` holds the payload, not the send record. | Test |
| FR-218-AC-12 | `ordering fifo by (m: M::Plain) {m.ready}` on a channel carrying `M::Plain` checks and the checked channel holds `Fifo` with that binder and key; a binder of another type refuses `ill_typed`/`type-mismatch` at the type; a key that reads model state refuses `ill_typed`/`operator-ineligible` at the read; `ordering unordered` holds no key. | Test |
| FR-218-AC-13 | A channel written with delivery `exactly-once-premise`, capacity `symbolic(Bound)`, overflow `loss head` and `ordering unordered` records exactly those four premises, `Bound` unresolved and the selector `Head`; the same channel with delivery `at-most-once` is a different checked channel, so is the same channel with `loss tail` and with `loss new`, and no premise changes another. | Test |
| FR-218-AC-14 | `relationship Pipe = M::pipe;` over an exported relationship binds the checked relationship to the export's key and the types of its two ends; `M::nothing` refuses `missing_declaration`/`missing-name` at the name; `M::out`, a port, and `M::Node`, an object type, each refuse `invalid_model_binding`/`wrong-export` at the name retaining required kind `relationship`, the actual kind and the export's declaration key; two relationships `Pipe` refuse `ambiguous_declaration`/`ambiguous-name` at both. | Test |
| FR-218-AC-15 | `related by Pipe (a, b)` checks when `a` and `b` have the types of the relationship's first and second ends, and the node holds the clause; `related by Absent (a, b)` refuses `missing_declaration`/`missing-name` at `Absent`; swapped ends, or an end of another type, refuse `ill_typed`/`type-mismatch` at that end, retaining relationship, `end`, expected and actual. A node with `related by Pipe (a, b)` and `related by Owns (c, d)` holds two checked clauses in source order. | Test |
| FR-218-AC-16 | `requires temporal Due;` over a temporal clause `Due` binds the requirement to `Due`'s position among the unit's temporal clauses, from which S4 mints its node key, the same for a clause with an `on each` activation and a capture (S4 emits such a clause, FR-337-AC-8 to FR-337-AC-10, and the requirement is carried by the emitted clause node of `Due`'s position, so its key is that clause node's key, captures included); `requires temporal Nothing;` and a clause offered only through an import alias each refuse `missing_declaration`/`missing-name` at the name; `requires temporal Accept;` naming a predicate refuses `ill_typed`/`operator-ineligible` at the name, naming the kind; `Due` required twice is recorded once. Each refusal produces no package. | Test |
| FR-218-AC-17 | A `choice` or `repeat` owned by an undeclared role refuses `missing_declaration`/`missing-name` at the owner. An owner that is the channel's `to` role decides on its `receive` record's Boolean field listed individually in `visible`; the other role, a `send` binder, an `effect` record, a `commit` record, the protocol's activation binder, an unlisted fact and a fact reachable only inside a composite entry each refuse `wrong_snapshot`/`wrong-observation` at the read, retaining node, owner, read and `reason` `other-role`, `ineligible-kind` (for the four binder kinds), `unlisted`; a `finish` record read by a decision refuses `missing_declaration`/`missing-name` as an out-of-scope read, because no decision reaches it, and a read that is out of scope and also unlisted reports the scope refusal only. A discarded `let` that reads a fact still needs the fact listed. A read of the `over` input refuses `ill_typed`/`operator-ineligible` at the read, naming the guard; a read of a protocol capture checks without listing. | Test |
| FR-218-AC-18 | A sibling branch's record before its join, an optional choice branch's record, and an `await` match read after the `await` each refuse `missing_declaration`/`missing-name` at every original use, retaining `name`. A repeat's own visible entry and guard may read its body attempt binders from the most recently completed iteration, subject to owner-role eligibility. | Test (TC-1056, TC-1268) |
| FR-218-AC-19 | Over two visible facts, guards that leave both facts true uncovered, that overlap on both true, that leave one fact out, and constant guards `true`/`not false` or `false`/`not true` each refuse `undefined_expression`/`unproved-exhaustiveness` at the choice, retaining `failed` (`[missing-arm]`, `[duplicate-arm]` or `[missing-arm, duplicate-arm]`) and the case names; complementary guards over the individually listed facts check. | Test |
| FR-218-AC-20 | A FIFO key of type `Boolean`, and a key of a record type, check because the clause checker admits `=` over them; a key of a type over which it admits no `=` refuses `ill_typed`/`operator-ineligible` at the key, retaining `operator` `=` and the type. | Test |
| FR-218-AC-21 | Channels written `overflow reject`, `overflow block`, `overflow loss head`, `overflow loss tail` and `overflow loss new` record `Reject`, `Block`, `Loss(Head)`, `Loss(Tail)` and `Loss(New)`; no other overflow policy records a selector, and a delivery policy never changes the recorded overflow policy. | Test (TC-1065) |
| FR-218-AC-22 | A `choice` with cases `shipped` and `shipped` and a `parallel` with branches `east` and `east` each refuse `ambiguous_declaration`/`ambiguous-name` at both labels, in source order, naming the label; a `choice` with one case and a `parallel` with one branch each fail at the parser with a syntax diagnostic and build no form; two cases and two branches check. | Test (TC-1065) |
| FR-218-AC-23 | Over branches `east` and `west`, `join all [east]` and `join quorum(0) [east, west]` and `join quorum(3) [east, west]` and `join any []` each refuse `invalid_package`/`invalid-value` at the list (the count for `quorum`); `join all [east, west]`, `join quorum(2) [east, west]`, `join any [east]` and `join all [east, west, east]` check, the last joining each label once. | Test (TC-1065) |
| FR-218-AC-24 | A role `on M::Pt` where `Pt` is a record type refuses `invalid_model_binding`/`malformed-declaration` at `M::Pt`; a role on an object type, a process and a state machine checks; in ADR-027 §7's `Fill` unit (alias `M`, `over (k: M::Cell)`), `role w on M` checks as a role on the whole model recording the identity of the package `M` selects, and so do `role w on M` beside `role u on N` in one clause over two aliases and `role w on M` in a clause whose `over` type is a scalar, a record and a `Set<M::Cell>` (Role on a model alias), while the replicated `role r each M from cells` refuses `invalid_model_binding`/`malformed-declaration` at `M` with no `pop` refusal and no package; a channel `carries M::Node` where `Node` is an object type refuses the same at `M::Node`, and a channel carrying a record, an event type, a variant type and a scalar type checks. | Test (TC-1065) |
| FR-218-AC-25 | `requires temporal Due;` where `Due`'s formula holds `protocol Fill` and the requiring protocol is `Fill`, and the same through a second temporal clause that names `Fill`, each refuse `invalid_package`/`definition-cycle` at the requirement, naming `Fill` and the clause; the same requirement from a protocol `Drain` that no clause names checks. Each refusal produces no package. | Test (TC-1065) |
| FR-218-AC-26 | A choice guard and a `visible` entry that read a field of an object-typed attempt binder, and a FIFO key that reads a field through a reference-typed field of its record binder, each refuse `ill_typed`/`operator-ineligible` at the read, naming the guard, the `visible` entry or the key; the same reads over a record binder's own fields check. A channel that carries an object type refuses once, at the type, and gives no refusal at its `send`, `receive` or FIFO binders. | Test (TC-1056, TC-1065) |
| FR-218-AC-27 | The unit of TC-1056 with a second protocol `Billing` that also declares `channel Pay` with the same premises checks, and the checked clauses hold two channels: `Pipeline`'s `Pay` and `Billing`'s `Pay`. Each is recorded under its protocol's qualified identity and the handle `Pay`; the two have different qualified names and equal handles. | Test |
| FR-218-AC-28 | `role w each M::Worker from workers max 2 lifetime workflow` checks when `workers` is a population of `M::Worker` and the checked role holds that population; with `from absent` it refuses `missing_declaration`/`missing-name` at `absent`, and with `from jobs`, a population of `M::Job`, it refuses `ill_typed`/`type-mismatch` at `jobs`, each retaining `role` and `population`. | Test (TC-663) |
| FR-218-AC-29 | In a protocol with channels `from w to c` and `from c to w`, the checked clause holds the owner `w` for the `send` on the first and `c` for its `receive`, and `c` for the `send` and `w` for the `receive` on the second; a lifetime `scope` role `w` with its `Work` nodes inside `sequence S` and a `send` from `w` after `S` has the region that holds `S` and the `send`, which is `S`'s parent control and not `S`. | Test (TC-663) |
| FR-218-AC-30 | Over three visible Boolean facts `a`, `b`, `c`: a two-case `choice` with guards `a and b and c` and `not (a and b and c)` checks, and its unit's measured `s3.work_units` spend exceeds that of the same unit with guards `a and a and a` and `not (a and a and a)` (one distinct atom, two valuations, the same guard nodes) by exactly 6, so the proof charges one unit per valuation and the atom comparison charges nothing beyond the node charges. A `choice` of the same shape over 24 distinct atoms (`2^24` valuations) stops under the default `s3.work_units` (16777216) with a `CheckRefusal`/`stage_limit_exceeded`, cause `work-budget-exceeded`, at the choice's span, naming bound 16777216 and setting `s3.work_units`, and no `unproved-exhaustiveness`; it checks with `s3.work_units` set to 33554432 through the library builder, the replay request's `stage_limits` entry and FR-255's settings operation. Guards `a and b and c` and `not a`, over the three atoms `a`, `b`, `c`, leave a valuation uncovered and refuse `unproved-exhaustiveness` with `failed` `[missing-arm]`. | Test (TC-1172) |
| FR-218-AC-31 | Over branches `a`, `b` and `c`, with `function f using v(x: Boolean, y: Boolean): Boolean pure { x and y }` declared in the unit, `join predicate(f) [a, c]` and `join predicate(f) [c, a, c]` each check to `Predicate{function}` holding `f`'s node key and the joined labels `a` and `c`, and a second join naming `f` holds the same key. `join predicate(g) [a, c]` with no `g` declared refuses `missing_declaration`/`missing-name` at `g`, retaining node and name; `g` declared as a `predicate` refuses `ill_typed`/`operator-ineligible` at `g`, naming the kind. `join predicate(f) [a, b, c]` (three labels, two parameters), a function `h(x: Boolean, y: Int[0, 1]): Boolean` joined on `[a, c]`, and a function `k(x: Boolean, y: Boolean): Int[0, 1]` joined on `[a, c]` each refuse `ill_typed`/`type-mismatch` at the name, retaining node, function, expected and actual. A unit whose joined function is `function r using v(x: Boolean, y: Boolean): Boolean pure { r(x, y) }`, which calls itself with no `decreases` measure, refuses `undefined_expression`/`unproved-decrease` (failed obligation `missing-measure`) at `r`'s declaration and gives no refusal at the join. Each refusal produces no package. | Test (TC-1065) |
| FR-218-AC-32 | In an outer repeat whose only body is a nested repeat with an event in its body, `repeat max 1 while a` has a feasible false valuation, so its skip edge makes the outer repeat refuse `unsupported_construct`/`declaration-form` at the outer repeat for missing progress. Replacing its guard with `a or not a or b or not b` makes false infeasible over the two independent visible atoms, so its skip edge is absent and the outer repeat checks from the nested repeat's entered-body progress. The infeasible case evaluates four valuations, charging each before evaluation; it checks at the measured exact `s3.work_units` limit and, at one unit less, stops at the nested repeat's span with `CheckRefusal`/`stage_limit_exceeded`, cause `work-budget-exceeded`, naming the bound, denied charge count and setting `s3.work_units`, with no enclosing progress refusal. A run raised through each FR-255 entry point checks. | Test (TC-1172) |
| FR-218-AC-33 | A repeat owned by role `c` whose body contains `attempt Load by c as (loaded: Boolean)` checks with `visible (loaded)` and `while { not loaded }`; its first body pass is unconditional, each later guard reads the most recently completed body's binder, and at least one iteration completes. The same post-test repeat with `max 0` refuses `invalid_package`/`invalid-value` at the declaration. A repeat whose guard reads no body binder retains pre-test behavior, so `while { false }` skips its body. The same binder produced by a role in a nested body branch is readable only after that branch is in the repeat body's completed scope; a binder produced by a different role or by a sibling outside the body remains refused with `wrong_snapshot`/`wrong-observation` or `missing_declaration`/`missing-name`, respectively. | Test (TC-1268) |
| FR-218-AC-34 | A bare literal or bare protocol capture in a `visible` list refuses at S3 with `ill_typed`/`operator-ineligible` at that entry; a bare eligible decision fact checks and is individually visible, and a composite expression containing a literal or capture checks without making its operands individually visible. No refused visible entry reaches S4. | Test |

## Dependencies

- ADR-027 §9 DS-1, §1 PS-5 and PS-6, §2.2a RR-4, §2.1 FO-2; ADR-017 PF-7
  (scenario 5).
- [FR-112](FR-112-build-protocol-scoped-anchor-forms.md),
  [FR-113](FR-113-resolve-scoped-anchors-in-nested-control-scopes.md),
  [FR-114](FR-114-bind-a-protocol-attempt-to-its-operation-frame.md),
  [FR-104](FR-104-check-state-clauses.md).
- FR-381 (captures, activation and compensation), FR-308 (the forms),
  FR-383 (the emission of the checked clause), FR-086 (relationship kinds), FR-088 (type identity).
- QSpec owns the grammar and contracts of every construct, and the catalog
  spellings of its refusals: QSpec FR-051, FR-052, FR-053, FR-054, FR-170,
  FR-171, FR-172, FR-228. A predicate join's function is admitted by QSpec
  FR-146 and resolved as FR-065 resolves a call.
  The protocol reference's resolution rules follow QSpec FR-322-AC-66, and
  its production and meaning are QSpec FR-463's.

## AC-16 coverage

- AC-16's binding of a clause with an `on each` activation or a capture (TC-1056
  row 23a) is exercised over the clauses S4 emits.

## References

- Owning ticket: Linear QSL-396. QSpec half: QSpec FR-052, including its loop rule
  (Linear STD-140).
