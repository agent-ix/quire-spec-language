---
id: FR-113
title: "Resolve scoped anchors in nested control scopes and refuse missing, ambiguous, shadowing and wrong-kind names"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-006
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-112
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-048
    type: traces_to
---
# FR-113: Resolve scoped anchors in nested control scopes and refuse missing, ambiguous, shadowing and wrong-kind names

## Description

When S3 checks a protocol declaration, the `ProtocolClause` family SHALL
resolve each `ScopedAnchorForm` (FR-112) to the one static protocol node it
names, by its own scope function, and SHALL refuse a reference that names no
node or a node of the wrong kind, a name that names two, and a binder that
shadows a visible name, each
with a catalog code at its span (ADR-012 §3 "anchor scoping", §4.2, §12.2
Check row). A resolved anchor is recorded by the identity of its target, so
no later stage recovers a target from a name (QSL-21 scope).

A protocol with any refusal emits no checked node (ADR-012 §4.2): an
unresolved or ambiguous scope fails before S4, with no partial substitute.

## Inputs

- The protocol declaration's form with its `ScopedAnchorForm`s (FR-112).
- The protocol's declaration collection: for each scope (the protocol's top
  level and each named control), the names of the static nodes it declares
  directly, with their spans. A control declares the names of its direct
  child controls and event nodes; the top level declares the protocol's
  `run` control, its `finish` node and its `compensate` templates.

## Outputs

- For each scoped anchor: the static protocol node it names, recorded on the
  checked protocol by that node's identity.
- Or the refusals below, ordered by builder state, then by source position
  (ADR-012 §4.2).

## Behavior

### Resolution

- The checker SHALL resolve an anchor's first segment in the innermost scope
  of its `scope`, then in each enclosing scope outward, then at the top level.
  The first scope that declares the name decides; scopes further out are not
  consulted.
- The checker SHALL resolve each later segment among the names the previous
  segment's target declares directly.
- A name declared in an inner scope hides the same name declared further out
  for a reference inside the inner scope. A path from an outer control, such
  as `Main::Applied`, reaches the outer node from anywhere in the protocol
  (QSpec `choreography-surface.md`: "fully qualified paths such as
  `Main::Applied` retain the same resolved identity").

### Refusals

- If no scope declares an anchor's first segment, or a later segment names
  nothing its previous target declares, then the checker SHALL refuse
  `missing_declaration`/`missing-name` at the anchor, retaining the segments,
  the scope, and the first segment that failed.
- If one scope declares two static nodes of one name, then the checker SHALL
  refuse `ambiguous_declaration`/`ambiguous-name` at each of those
  declarations, naming both in source order. Each anchor whose resolution
  decides in that scope on that name SHALL also refuse
  `ambiguous_declaration`/`ambiguous-name` at the anchor, naming both
  declarations. No source order or first match picks one.
- Every node record binder (the `as (x: T)` of an event node, a `commit`, a
  `finish` or a `compensate` declaration), capture, the protocol's own
  `over (p)` input parameter, its `activation on each (p)` parameter, and
  every compensation trigger, retry or recovery parameter, SHALL be unique
  across the whole checked protocol declaration (QSpec
  `shared-grammar.md`: binders "are unique in their enclosing declaration"
  -- the protocol as a whole, not merely the lexical scope, sequence,
  choice, parallel, repeat, case, branch, await or compensate declaration
  each one is written in). If a binder has the name of a model or profile
  alias, a native declaration of the package, or another binder visible
  anywhere else in the protocol, then the checker SHALL refuse
  `ambiguous_declaration`/`ambiguous-name` at that binder, naming the
  declaration it would shadow (QSpec `shared-grammar.md`: binders "cannot
  shadow aliases, native declarations or another visible binding", and "the
  same no-shadowing rule applies to binders and captures"). Two separate
  protocols may reuse a binder name.
- The checker SHALL check the kind of node each resolved anchor names,
  by its site (QSpec `choreography-surface.md`'s node and deadline rows):

  | Site | Admitted target |
  | --- | --- |
  | `receive-of` | a `send` event node |
  | `effect-of` | an `attempt` event node |
  | `event-for` | a `compensate` template |
  | `compensate-for` | an `effect` event node |
  | `compensate-commit` | a `commit` node |
  | `await-after` | a `send`, `receive`, `attempt`, `effect` or `event` node, a `commit` node, or a `compensate` template; never a structural control (`sequence`, `parallel`, `branch`, `choice`, `case`, `repeat`, `await`, `check`) |

- If a resolved anchor names a node its site does not admit, then the
  checker SHALL refuse `ill_typed`/`type-mismatch` at the anchor, naming the
  site, the kinds it admits and the target's kind (the catalog row keeps the
  expected and actual kind).
- If a `receive` names a channel other than the channel of the `send` its
  `receive-of` anchor names, then the checker SHALL refuse
  `ill_typed`/`type-mismatch` at the anchor, naming both channels (QSpec: a receive is "related to the exact concrete send
  occurrence" of its channel).
- A refused anchor or binder SHALL NOT stop the checker from checking the
  other anchors and binders of the declaration: each clause that is in order
  but fails its own check still advances the builder (ADR-012 §4.2).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-113-AC-1 | In the `RecoveryFlow` protocol (TC-510 fixture), `Main::Applied` resolves to the `effect Applied` node, `Main::Committed` to the `commit Committed` node, `Tried` to the `attempt Tried` node and `Undo` to the `compensate Undo` template, each recorded by the node's identity; the protocol checks with no refusal. | Test (TC-511) |
| FR-113-AC-2 | Nested scope: with `sequence Inner { effect Applied of Tried ...; }` added inside `Main` after the outer `effect Applied`, an `await` inside `Inner` naming `Applied` resolves to the inner node, and one naming `Main::Applied` resolves to the outer node. An `await` inside `Main` but outside `Inner` naming `Applied` resolves to the outer node. | Test (TC-511) |
| FR-113-AC-3 | Missing anchor and member: `compensate Undo for Main::Missing` refuses `missing_declaration`/`missing-name` at the anchor, naming segment `Missing` and the empty scope; `for Other::Applied` refuses naming segment `Other`; `effect Applied of Absent` refuses naming segment `Absent` and scope `[Main]`. | Test (TC-511) |
| FR-113-AC-4 | Ambiguity: two event nodes named `Applied` in `Main` refuse `ambiguous_declaration`/`ambiguous-name` at both declarations, and `Main::Applied` refuses `ambiguous_declaration`/`ambiguous-name` at the anchor naming both, in source order. Swapping the two declarations swaps nothing but their order in the refusal. | Test (TC-512) |
| FR-113-AC-5 | Shadowing: a capture named `forward` inside `Undo`, where `forward` is already the template's bound parameter, refuses `ambiguous_declaration`/`ambiguous-name` at the capture naming the parameter; an event record binder named `M` where `M` is the unit's model alias refuses at the binder naming the alias. A second protocol in the same unit whose record binder is also `attempted` checks. | Test (TC-512) |
| FR-113-AC-6 | A protocol with one missing anchor and one shadowing binder reports both refusals, ordered by builder state and then source position, and emits no checked protocol node. Checking it twice gives the same refusals in the same order. | Test (TC-512) |
| FR-113-AC-7 | Wrong kind and channel: `effect Applied of Recovered` (an `event` node) refuses `ill_typed`/`type-mismatch` at the anchor naming site `effect-of` and kind `event`; `compensate Undo for Main::Tried` (an `attempt`) refuses naming `compensate-for` and `attempt`; `await Wait after Main ...` (a `sequence`) refuses naming `await-after` and `sequence`. With channels `C` and `D` from `R` to `R`, `send S via C ...` and `receive Got via D of S ...` refuse at the anchor naming `C` and `D`; the same receive `via C` checks. | Test (TC-512) |

## Dependencies

- FR-112 (scoped anchor forms).
- ADR-012 §3, §4.2, §12.2.
- QSpec `choreography-surface.md` (control names, relative and qualified
  paths, "Duplicate or ambiguous declarations refuse") and
  `shared-grammar.md` (binder no-shadowing), and the catalog rows for
  `missing_declaration` and `ambiguous_declaration` in
  `definitions/native-diagnostics.md`.
- The composed lane resolves the same references today in
  `src/linking/composed/scopes/protocol.rs` (`structural`, with
  `WrongTargetKind` and `IncompatibleReference`), with scope issues that
  carry no catalog code; M-6d deletes that checker (QSL-303) once this
  requirement's checker replaces it.

## Status

Specified under QSL-296 (QSL-21a). QSL-298 (QSL-21c) implements anchor
resolution: nested-scope resolution, missing and ambiguous refusals, and
the wrong-kind and channel-mismatch refusals of every site. QSL-306
implements binder no-shadowing (the shadowing clause of "Refusals", AC-5
and the shadowing half of AC-6), enforced protocol-wide (QSpec
`shared-grammar.md`: binders are "unique in their enclosing declaration"),
over a further S2 extension (`qsl_forms::protocol_clause::BinderForm`) that
walks every binder position FR-112 itself does not capture, including the
protocol's own `over (p)` input and `activation on each (p)` parameters.
FR-114's binding half remains open: QSL-309 (binding) and QSL-299
(emission) complete it. A protocol whose anchors and binders all resolve is
still refused `unsupported_construct`/`not-yet-implemented`, since its
other content has no checker and nothing emits it yet (QSL-299), until
those tickets complete protocol checking and emission.
