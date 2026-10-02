---
id: SR-1227
title: "Code review of quire-spec-language PR #607: iterative Clone, PartialEq, Hash, Debug and Drop for quire_exact::ValueType"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@3147f80640a171fe6e30ce049c0751eca382e92c; PR #607 diff against origin/main: quire-exact/src/value/value_type.rs, quire-exact/src/value.rs, quire-exact/src/collection.rs, quire-exact/src/decimal.rs, qsl-semantics/src/check/{check.rs,check/typing.rs,identity.rs,mod.rs,type_form.rs}"
review_set: subset
---
# Code review of quire-spec-language PR #607

## Summary

Ticket: QSL-484 (slice B3). The PR replaces `ValueType`'s derived `Clone`,
`PartialEq` and `Debug` with hand-written non-recursive impls, and adds a
non-recursive `Hash` and `Drop`. `DecimalType` now derives `Hash`. Nine
qsl-semantics sites stop moving out of a `ValueType`, because a type with a
`Drop` impl cannot be destructured by move (E0509). The Rust lane
(rust-review) is folded into this file.

What the coordinator asked to check:

- **Charge AC.** quire-exact charges nothing when it builds a `ValueType`.
  The cover is upstream, and it exists:
  - A type written in source is bounded by S1. `qsl_cst::Limits` has
    `source_bytes` (1 MiB), `tokens` (100,000) and `nodes` (50,000)
    (qsl-cst/src/lexer.rs:21-36). Each `Option<..>` or collection link takes
    at least one CST production and several tokens. `type_form::resolve_form`
    builds one `ValueType` link per form.
  - The checker adds at most one `Option` wrap per checked expression
    (check.rs:1573, 1770, 2124). Each checked expression passes
    `Typer::enter`, which charges `CheckingLimitKind::Nodes`.
  - A replayed type is read under `qsl_replay::bounds::MAX_ENCODED_BYTES`.
  - The `Debug` worklist grows by at most 3 steps per link, plus 4 for the
    leaf. `tc_735_the_debug_worklist_grows_by_a_constant_per_type_node`
    measures this at 100,000 links. So D-1 item 3 holds: the growth is a
    constant per node that S1 has already charged. `Clone`, `PartialEq`,
    `Hash` and `Drop` use a cursor and keep no stack at all.
- **ADR-030 citation.** D-1 item 2 says `quire-exact`, a leaf crate with no
  dependency, "keeps hand-written iterative traits (D-4.7)". The Alternatives
  section accepts hand-written traits for `ValueType` in particular. FR-262
  Behavior 3 and AC-2 also require `Hash`, so the new `Hash` is required and
  not an extra.
- **Drop safety.** quire-exact has `unsafe_code = "forbid"`. `Drop` detaches
  each child with `mem::replace(slot, ValueType::Boolean)`. Each detached
  link drops at the end of its loop iteration, with only a `Boolean` below
  it. That nested drop runs one more iteration on the placeholder and stops.
  Nothing can leak or drop twice: there is no unsafe code, no `ManuallyDrop`
  and no `mem::forget`.
- **Clone, PartialEq and Hash.** `shallow_clone` keeps the variant, so the
  `(source.child(), target.child_mut())` cursor always pairs matching links.
  `shallow_eq` compares the same fields that `shallow_hash` hashes
  (discriminant, then the link's own data; kind and bound for a collection),
  so equal types hash equal. The hash is self-delimiting, because each
  discriminant settles whether another link follows. `CollectionType` keeps
  its derived traits. Each of them calls the `ValueType` impl once, and these
  impls never call back into `CollectionType`'s traits, so nothing recurses.
- **Debug is the derive's output.** I checked this against the real derive:
  a scratch binary was built once against origin/main's quire-exact and once
  against this head. It printed 5 types (unit, `Option` leaf, struct leaf,
  `Decimal`, and the nested Option/Set/Option/Int type) under `{:?}`,
  `{:#?}`, `{:x?}`, `{:>12?}`, `{:#x?}`, `{:#>12?}` and `{:#04X?}`. All
  compact output and all plain `{:#?}` output is byte-identical. Alternate
  mode with formatting flags is not identical (FND-001). `Value`'s `Debug`
  already had this difference and documents it.
- **The nine qsl-semantics edits are minimal.** Five are tests that add
  `ref` to a `let-else` pattern (identity.rs ×4, mod.rs, type_form.rs). Three
  are production sites that borrow and then clone only the inner part
  (check.rs ×2, typing.rs ×1). These also clone less than the old
  whole-`ValueType` clone did. The compiler forces each one, and
  `cargo check --workspace --all-targets --all-features` is clean, so no
  other site was missed.
- **Value test.** There is no compatibility layer, no shim, no depth cap and
  no ceremony. `CollectionType::debug_fields` destructures the whole struct,
  so adding a field breaks the build until `Debug` prints it. That is a real
  guard. `DebugWriter::write` now returns its peak worklist length, and the
  charge test needs it. The cost is one `max` per step.

Gates run at this head with `CARGO_TARGET_DIR=<worktree>/target`:
- `cargo test -p quire-exact`: 86 lib tests and 2 doctests passed, including
  all four `tc_735_*`.
- `cargo test -p qsl-semantics`: 420, 250 and 23 tests passed.
- `cargo clippy -p quire-exact -p qsl-semantics --all-targets -- -D warnings`
  was clean.
- `cargo check --workspace --all-targets --all-features` was clean.
- `make quire-exact-no-std` built.

## Verdict

Approve with two low findings. Neither one is a wrong result on a
production path. Both can be fixed in this PR.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The `ValueType` `Debug` doc says it "Prints what `#[derive(Debug)]` printed, in compact and alternate mode". In alternate mode, leaves get plain `{:#?}`, so hex, width, fill and zero-pad flags are dropped. Measured: `format!("{:#x?}", ValueType::Population(Some(255)))` prints `0xff` with the derive and `255` here. `Decimal` scales and `CardinalityBound` fields differ the same way. Fix: copy `Value`'s caveat into this doc: "In alternate mode a leaf gets plain `{:#?}`, so the width, fill, precision and hex flags of the caller's format spec reach compact-mode leaves only." Also qualify the `ValueType` type doc. | quire-exact/src/value/value_type.rs:213-214 |
| FND-002 | low | The `Hash` oracles assert only that equal types hash equal. A `Hash` that writes nothing, or that ignores everything below the root link, passes all four `tc_735_*` tests. Fix: in `tc_735_value_types_equal_and_hash_equal_their_clones_only`, assert that distinct samples hash distinct (`hash_of(a) != hash_of(b)` for `index != other_index`) and that `bounded(3)` and `bounded(4)` hash differently. In the deep test, assert `hash_of(&value_type) != hash_of(&ValueType::option(copy.clone()))`. | quire-exact/src/value/value_type.rs:463, 494-512 |

## Dispositions

Round 1, reviewed at 1871a38092fada9e4f163cebb26ffebe51cfe8f0.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 1871a38092fada9e4f163cebb26ffebe51cfe8f0 |
| FND-002 | fixed | 1871a38092fada9e4f163cebb26ffebe51cfe8f0 |
