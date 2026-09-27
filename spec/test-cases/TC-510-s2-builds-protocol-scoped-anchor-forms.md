---
id: TC-510
title: "S2 builds protocol scoped anchor forms with their scope and segments"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-112
    type: verifies
---
# TC-510: S2 builds protocol scoped anchor forms with their scope and segments

## Description

Verify that S2 builds one `ScopedAnchorForm` per protocol node reference,
with its segments, scope and site, and resolves nothing.

Scope: FR-112-AC-1 to FR-112-AC-3.

## Test Procedure

Fixture `RecoveryFlow`: the protocol `tests/it/composed_scopes.rs` names
`COMPENSATION` (role `R`, `compensate Undo for Main::Applied ... commit
Main::Committed`, and `run sequence Main` holding `attempt Tried`, `effect
Applied of Tried`, `event Recovered by R for Undo` and `commit Committed`),
in a `1-draft` unit with model alias `M` and profile alias `S`.

1. Build the forms of `RecoveryFlow`; list its scoped anchors with their
   site, segments, segment spans and scope.
2. Add `parallel Both { branch left await Wait after Sent ...; branch right
   ... } join all [left,right];` inside `Main`; build again. Then replace the
   compensation's `commit Main::Committed` with `commit never`; build again.
3. Build `RecoveryFlow` twice and compare. Remove `effect Applied` from
   `Main` and build again; compare the scoped anchors with step 1's.

Tag the tests `#[trace("TC-510", "FR-112-AC-n")]`.

## Expected Results

- Step 1: exactly four, in source order: `compensate-for` `[Main, Applied]`
  scope `[]`; `compensate-commit` `[Main, Committed]` scope `[]`;
  `effect-of` `[Tried]` scope `[Main]`; `event-for` `[Undo]` scope `[Main]`.
  Each segment's span covers exactly its text.
- Step 2: the `await-after` anchor has segments `[Sent]` and scope
  `[Main, Both, left]`; with `commit never`, no `compensate-commit` anchor.
- Step 3: equal forms; the scoped anchors equal step 1's, since S2 resolves
  nothing.
