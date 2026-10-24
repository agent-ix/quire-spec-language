---
id: SR-16367
title: "QSL-996 code and Rust review of weak-parallel constraint readers (quire-spec-language PR 1423)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@abcee1ae35d6e8cecc4e267bc6efc8b6e21b94aa; qsl-eval/src/simulation/protocol/access.rs; qsl-eval/src/simulation/protocol.rs; qsl-eval/src/simulation/model/footprints.rs; qsl-semantics/src/check/footprint.rs; qsl-analyze/tests/it/weak_memory.rs; qsl-analyze/tests/it/litmus.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-220
    type: reviews
---

## Summary

Ticket: QSL-996 (PR quire-spec-language#1423). `/code-review` with the
`/rust-review` lane over `git diff origin/main...HEAD` at `abcee1ae3`.

The PR makes an `event`, `effect`, `commit`, `send` or `receive` node inside a
branch of a weak `parallel` a reader of the model locations its constraint
reads (FR-220 "Sharing and classification", ADR-025 MA-1). The reads join the
sharing pass as non-writing touches, and the pre-check settles V-8
`WeakAccessShape{site: node path, transition: None, shape: MultiLocation}` for
a reader of a shared location, after the attempt shapes.

What was checked and found sound:

- **Reader read sets.** `derive_node_reads` keeps a field read through
  `Local(0)` (the `over` input) as a receiver read. `ModelSystem::node_reads`
  lowers that to the one `over` object. Every other holder becomes an
  any-object read: another reference binder (`Param`, rewritten to
  `AnyField`), a non-reference or body-local slot (`Holder::Other`), or a
  navigated reference. `node_reads` expands each any-object read to every
  universe object that declares the field, and keeps `AnyField` when the
  universe cannot be listed. Membership reads stay `AnyMembership`.
  `Membership{Param}` is never produced by `body_reads`, so `lower`'s `Param
  => None` drops nothing. The analysis can over-approximate (a binder that
  holds one fixed object counts as reading every object), but it never misses
  a read. Over-approximation can only cause conservative refusals, and the
  `Cell::set` instantiation over every `Cell` already over-approximates the
  writes on the other side of the same sharing test.
- **Placement.** Each reader's chain comes from the same `chain_of` the
  attempts use. A node outside every weak `parallel`, including a `finish`
  (not in `program.events`/`messages`) and a node after the `join`, has no
  chain and is no reader. `apart` looks the chain up in both maps. Reader
  touches are never writes, so two readers never make a location shared.
- **Red check (reviewer).** In the throwaway worktree, disabling the reader
  loop (`constraints.filter(|_| false)`) fails the new test at
  `weak_memory.rs:817`: no `WeakAccessShape` under `tso`. A second mutation,
  turning the `over` receiver read into an any-object read, also fails it at
  `weak_memory.rs:830` (`[(x,v),(y,v)]` against `[(x,v)]`). So the test pins
  both the reader rule and the precision of the `over` read.
- **API (item 4).** `classify_accesses` is `pub`, but `qsl-eval` is
  `publish = false`, and its only caller (the test harness) is updated.
  `over_value` moved from a private associated fn to a `pub(crate)` free fn
  with an identical body. `WeakAccesses` gains a private field. There is no
  external API break.
- **Gates run by the reviewer at the head:** `cargo fmt --all --check` clean;
  `cargo clippy -p qsl-semantics -p qsl-eval -p qsl-analyze --all-targets -D
  warnings` clean; `cargo test -p qsl-analyze --test it` and `cargo test -p
  qsl-eval --lib simulation`: 725 and 49 passed.

Spec-fidelity probe (item 2): with a third `Cell` `z` (`id = 2`) added to the
fixture universe and `SB` over `z`, the head reports `shared = {(x,v), (y,v),
(z,v)}` and refuses the reader under `tso`, naming `(z,v)`. `attempt_identities`
instantiates `Sx`/`Sy` (`Cell::set`) on every `Cell` row whatever their `IsX`/`IsY`
preconditions. AC-15's "a `Cell` `z` that no attempt accesses" therefore cannot
be built from a `Cell` under the current engine, and the coder's `Spare`
substitution is forced. See FND-001.

## Verdict

Approve with findings: no high finding, and the code is sound. FND-001 needs a
spec ruling, routed to the FR-220 owner. FND-002 needs owner ratification of
two lead defaults. FND-003 and FND-004 are low. Mergeable once the lead
accepts FND-001/FND-002 as routed to the spec owner. No code change is
expected unless the owner rules that EN-1 instantiation should honour attempt
preconditions.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-220-AC-15's admitted variant ("a `Cell` `z` that no attempt accesses", with the event reading `z.v`) cannot be built: EN-1 instantiates `Cell::set` over every `Cell` regardless of `IsX`/`IsY`, so any `Cell` z has `z.v` written by `Sx` and `Sy` and is shared (reviewer probe: shared = {x.v, y.v, z.v}, reader refused). The test substitutes a `Spare` object z and retypes `SB`'s `over` input to `M::Spare`. This keeps the AC's intent (a reader of an unshared location is admitted) but not its text. It needs a spec ruling: either amend AC-15 (and TC-665 step 14) to "an object `z` of a type no attempt's operation touches", or rule that EN-1 instantiation must honour an attempt's `pre` contracts. The second is an engine change, and it would also change AC-11's sharing. | qsl-analyze/tests/it/weak_memory.rs:859-873; qsl-analyze/tests/it/litmus.rs:177-182; spec/functional/FR-220-check-access-orderings-fences-and-access-classification.md:202 |
| FND-002 | low | Two contract choices are lead defaults with no FR-220 text: (a) `first_shape` settles attempt `MultiLocation` shapes before reader shapes, so when both exist the refusal names the attempt; (b) a reader's `WeakAccessShape.transition` is `None`. The test asserts (b) under the FR-220-AC-15 tag (`assert_eq!(shape.transition, None)`), though AC-15 says nothing about `transition`, so the tag binds an unratified default. (a) is untested. The owner should ratify both. If ratified, add the order and the `None` to FR-220's Access shape section; until then, (b) is beyond what AC-15 asserts. | qsl-eval/src/simulation/protocol/access.rs:196-214; qsl-analyze/tests/it/weak_memory.rs:821 |
| FND-003 | low | A reader constraint with no FR-383 footprint rule (a function call, navigation, dispatch) now refuses the whole weak protocol with `Unsupported(AccessFootprint)`, and the `FootprintRefusal` detail (clause and form) is discarded by `map_err(\|_\| ...)`. Before this PR such a protocol was admitted. The path is untested, and the `classify_accesses` `# Errors` doc still says "an application has no footprint" and does not mention node constraints or the `over` binding refusals (`OverBinding`, `OverNotAnObject`) that `over_value` can now return from it. | qsl-eval/src/simulation/protocol/access.rs:383-385; qsl-eval/src/simulation/protocol.rs:695-704 |
| FND-004 | low | Only the `event` reader kind is exercised. `effect`/`commit` (`program.events`) and `send`/`receive` (`program.messages`) readers come from a separate iterator chain with no test. AC-15 names only `event`, so this is a coverage gap against the FR-220 statement, not against the AC. | qsl-eval/src/simulation/protocol/access.rs:368-378 |
