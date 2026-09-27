---
id: FR-113
title: "Resolve scoped anchors in nested control scopes and refuse missing, ambiguous and shadowing names"
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
# FR-113: Resolve scoped anchors in nested control scopes and refuse missing, ambiguous and shadowing names

## Description

When S3 checks a protocol declaration, the `ProtocolClause` family SHALL
resolve each `ScopedAnchorForm` (FR-112) to the one static protocol node it
names, by its own scope function, and SHALL refuse a reference that names no
node, a name that names two, and a binder that shadows a visible name, each
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
- If a node record binder (the `as (x: T)` of an event node), a capture, a
  compensation trigger or a retry or recovery parameter has the name of a
  model or profile alias, a native declaration of the package, or another
  binder visible where it is declared, then the checker SHALL refuse
  `ambiguous_declaration`/`ambiguous-name` at that binder, naming the
  declaration it would shadow (QSpec `shared-grammar.md`: binders "cannot
  shadow aliases, native declarations or another visible binding", and "the
  same no-shadowing rule applies to binders and captures"). Two separate
  protocols may reuse a binder name.
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

## Dependencies

- FR-112 (scoped anchor forms).
- ADR-012 §3, §4.2, §12.2.
- QSpec `choreography-surface.md` (control names, relative and qualified
  paths, "Duplicate or ambiguous declarations refuse") and
  `shared-grammar.md` (binder no-shadowing), and the catalog rows for
  `missing_declaration` and `ambiguous_declaration` in
  `definitions/native-diagnostics.md`.
- The composed lane resolves the same references today in
  `src/linking/composed/scopes/protocol.rs` (`structural`), with scope issues
  that carry no catalog code; M-6d deletes that checker (QSL-303) once this
  requirement's checker replaces it.

## Status

Specified under QSL-296 (QSL-21a). Not yet implemented; QSL-298 (QSL-21c)
implements it.
