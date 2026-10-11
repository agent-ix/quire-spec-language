---
id: FR-112
title: "Build protocol scoped anchor forms at S2"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-006
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-048
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-113
    type: traces_to
---
# FR-112: Build protocol scoped anchor forms at S2

## Description

When S2 builds the form of a `1-draft` protocol declaration, it SHALL build
one `ScopedAnchorForm { scope, anchor }` for each protocol node reference the
declaration holds, and SHALL attach it to the form of the construct that
holds the reference (ADR-012 §12.2 Form row, §4.1). A scoped
anchor is a clause of its construct with its own refusal causes (FR-113), so
it is a typed subnode, not a string kept inside the construct's form.

`ScopedAnchorForm` is a `ProtocolClause` form (ADR-012 §1, §3) and lives in
`forms::protocol_clause`. S2 records the reference and where it was written.
It resolves nothing: resolution is S3's (FR-113).

A scoped anchor is represented only inside S2 and S3. It has no
`CheckedClauseKind` variant and no checked-package/v2 node: S4 emits nothing
for it, and a resolved anchor reaches later stages only as its target's
identity (ADR-012 §12.2).

## Inputs

- The CST of one protocol declaration (the `qsl-cst` `ProtocolClause`
  production with its `Compensation`, `Control` and `EventNode` children).

## Outputs

One `ScopedAnchorForm` per node reference, with:

- `anchor`: the reference's segments in source order, each segment's text and
  span, and the span of the whole reference. `Main::Applied` has two
  segments; `Tried` has one.
- `scope`: the names, with their spans, of the named controls that enclose
  the reference, outermost first, from the protocol's `run` control to the
  innermost enclosing control. A reference in a protocol-level requirement
  (a `compensate` declaration) has the empty scope.
- `site`: which reference position it fills: `receive-of`, `effect-of`,
  `event-for`, `await-after`, `compensate-for` or `compensate-commit`.

## Behavior

- S2 SHALL build a `ScopedAnchorForm` for exactly these positions: the
  `NodeReference` after `of` in a `receive` or `effect` event node, after
  `for` in an `event` node, after `after` in an `await` control, after `for`
  in a `compensate` declaration, and after `commit` in a `compensate`
  declaration when it is not `never`. No other position yields one.
- S2 SHALL keep a reference's segments exactly as written. It SHALL NOT
  join them into one qualified string, drop a segment, or read a segment as
  a display name.
- S2 SHALL build the scoped anchors of one declaration in source order of
  their references.
- A `NodeReference` the CST holds is always well formed (one or more
  identifiers joined by `::`), so building a scoped anchor adds no S2
  refusal. A CST that carries an error or recovery node refuses as it does
  today (`FormsCause::RecoveringCst`), with no partial form.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-112-AC-1 | The `RecoveryFlow` protocol (TC-510 fixture) builds exactly four scoped anchors, in source order: `compensate-for` with anchor `[Main, Applied]` and the empty scope; `compensate-commit` with anchor `[Main, Committed]` and the empty scope; `effect-of` with anchor `[Tried]` and scope `[Main]`; `event-for` with anchor `[Undo]` and scope `[Main]`. Each segment carries its own span, and each span covers exactly the segment's text in the source. | Test (TC-510) |
| FR-112-AC-2 | A reference inside a nested control records every enclosing named control: an `await Wait after Sent` written inside `branch left` of `parallel Both` inside `sequence Main` records scope `[Main, Both, left]`, anchor `[Sent]` and site `await-after`. The same protocol with `commit never` builds no `compensate-commit` anchor. | Test (TC-510) |
| FR-112-AC-3 | Building the same declaration twice gives equal forms. A form holds no resolved target: two protocols that differ only in whether the named `Applied` exists build equal `ScopedAnchorForm`s. | Test (TC-510) |

## Dependencies

- ADR-012 §4 (typed subnodes), §12.2 (Form row).
- QSpec `choreography-surface.md` ("Control names and declaration paths
  identify static nodes"; "Relative references resolve in their lexical
  control scope").
- FR-048 is the composed lane's choreography rule set, which the spine
  replaces (ADR-011 §7.3 M-6d).
