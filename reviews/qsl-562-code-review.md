---
id: SR-12231
title: "Code review of quire-spec-language PR #1108: protocol step footprints"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@74d8f669d5b36bf7817124dfe8a8efcce474238d; PR #1108: qsl-eval/src/simulation/protocol/footprint.rs, qsl-eval/src/simulation/protocol/{identity,channel,steps,program}.rs, qsl-eval/src/simulation/protocol.rs, qsl-eval/src/simulation/footprint.rs, qsl-eval/src/simulation/model/footprints.rs, qsl-semantics/src/check/footprint.rs, qsl-semantics/src/check/mod.rs, qsl-analyze/tests/it/{protocol_footprints,fill,main}.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-216
    type: reviews
---
# Code review of quire-spec-language PR #1108

## Summary

Ticket: QSL-562 (E15b-fp). Code review with the rust-review lane folded in.
The review covers `git diff origin/main...HEAD` only. It was checked against
ADR-027 §5 FT-1 to FT-5, §7, PB-2 and FR-216 on origin/main. Soundness came
first: a footprint must over-approximate what a step really reads and writes.

Narrow tests run through `locked-build.sh`: `cargo test -p qsl-analyze --test it
protocol` gave 35 passed. A scratch diamond test, not committed, reproduced
FND-001.

What was checked and found clean:

- **Branch end writes only its disposition (`ProtocolLocation::Branch`).**
  Sound. `Branch` meets the whole `Instance`, so the end still depends on the
  join that reads the record. After a `continue` join settles, the end of a
  sibling also removes its thread and maybe the record. Two sibling ends still
  commute: the record goes once the last one ends, in either order. A `finish`
  reads `Subtree(root)`, which meets every `Branch`.
- **`finish` reads every thread's control and writes every queue of its
  instance.** Sound. Every other thread is not running and cannot step. The
  only steps left on the instance are channel steps, and those meet the queue
  writes. `activate` is refused, so only one protocol instance is live and
  `pinst` is not needed.
- **A cancelling join writes `Subtree` for a branch that can fork, and
  `Control` otherwise.** Sound. `Subtree` meets the descendants' `Control` and
  every parallel record whose parent lies under the branch. Those are the
  inner instances the cancel settles. `forks_within` uses span containment, so
  a branch body with no `parallel` has no descendants.
- **One queue per channel per protocol instance.** This is coarser than
  ADR-027 §5's "per FIFO key", so it is a sound over-approximation. Queues of
  different protocol instances are separate state.
- **Step identity.** The `channel` member is now built in `enc()` from typed
  `name` and `instance` fields. It is the same `Enc::tagged("channel", [name,
  instance])` with `instance.to_string()`. `Display` of `&u64` and of `u64` give
  the same string, so the bytes are identical. The protocol identity tests pass.
- **`free_ordinal` factoring.** The code moved unchanged (`&text` for
  `&String`), so behaviour is the same.
- **`derive_selected_footprint` / `Operation`.** The frame writes come from
  `effect`, not from the clauses, so selecting clauses cannot drop a write.
  `select` filters on `clause.identity`, the same key `walk.selected` uses for
  `apply_operation`. The checker call site passes `&|_| true`, so its
  behaviour is the same. The cache key adds the selected set.
- **Conflict resolution with QSL-564's `Location::affects`.** `affects` goes
  through to `meets` for protocol locations, and a protocol location meets
  only another protocol location. `visible` uses `affects`, which matches
  FT-5.
- **Attempt anchors.** PB-2 observes an attempt only with its operation's
  anchor, so it is correct that `visible` checks only `AnchorName::Operation`
  for an attempt.
- Rust: no new `unwrap`, `expect` or panic in production code, and the `meets`
  match has no wildcard arm.

## Verdict

Changes requested: one high finding (FND-001), a POR soundness defect that
was reproduced. Two mediums and two lows are recorded and do not block.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The `fork` footprint leaves out what `free_ordinal` reads. The ordinal a fork takes depends on whether a sibling instance of the same `(parent, node)` is still live: its record, and any thread under its branch paths. The footprint reads only the forker's `ctl` and writes `Instance(new ordinal)` and the new threads' `ctl`. Take a `repeat` around `parallel Both … join any [a, b] outstanding continue`. The root can re-fork while branch `b` of ordinal 0 still runs. `fork` (writing ordinal 1) and `b`'s last attempt (writing `Branch{ordinal 0}`, `ctl(b@0)`) are then reported independent. They do not commute: in one order the fork takes ordinal 1, in the other `b`'s end removes record 0 first and the fork takes ordinal 0. Reproduced in a scratch test: at state 5, fork;attempt reaches state 12 and attempt;fork reaches state 10. The fork's identity carries no ordinal, so the two orders give the same labels and different states. This breaks POR-4 (FT-4). Fix: the fork reads `Instance(parent, node, o)` and `Subtree(parent.child(node, label, o))` for every `o` up to and including the chosen ordinal. FT-1's text has the same gap (`fork` reads only `ctl(t)`), so a spec amendment should come with the fix. | qsl-eval/src/simulation/protocol/footprint.rs:291-304; qsl-eval/src/simulation/protocol/steps.rs:511-533 |
| FND-002 | medium | `event`, `receive` and `finish` list their candidate records with `values_of(record_type, &state.model)`. That reads `rows_of(state)`, every object of the model. For a record type that holds a reference, the set of enabled steps then depends on population membership, and their footprints carry no membership read. A create or delete in another thread could then be reported independent of an `event` it enables or disables. Not reproduced: it was not confirmed that the checker admits reference-typed event, receive or finish records. | qsl-eval/src/simulation/protocol/steps.rs:356; qsl-eval/src/simulation/protocol/steps.rs:774; qsl-eval/src/simulation/protocol/channel.rs:391; qsl-eval/src/simulation/protocol/footprint.rs:272-290 |
| FND-003 | medium | `AnchorName` has only `Node` and `Operation`. A `duplicate` or `lose` step has no node, so `visible` never finds it by anchor. PB-2 gives every non-attempt step a `protocol` anchor that names "its node (or channel, template or role)". An atom that reads a channel step's anchor therefore cannot be expressed, and the step is reported invisible. FT-5's text says only "node or operation", so the spec should settle this as well. | qsl-eval/src/simulation/protocol/footprint.rs:153-188 |
| FND-004 | low | FT-1 states that "Settling (FO-1) reads only binders and the thread's own control, so its reads are inside these footprints". `binders_read` adds only the binders read by the stepped node. It leaves out the binders read by the `check`, `choice` and `repeat` guards the thread settles through after the step, and `join` and `timeout` add none. PS-5's heir copies mean no other thread's step can change those bindings while this thread can read them, so independence is not affected today. But `R(s)` is smaller than FT-1 states, and it would be wrong for any later consumer that reads `reads` directly. | qsl-eval/src/simulation/protocol/footprint.rs:425-444; qsl-eval/src/simulation/protocol/settle.rs:73-80 |
| FND-005 | low | `footprint_for` clones the selected `BTreeSet<NodeKey>` into the cache key on every call, including cache hits. `Some(every clause)` and `None` are also cached as separate entries for the same derivation. This costs allocation only and gives no wrong result. | qsl-eval/src/simulation/model/footprints.rs:379-384 |

The findings table holds defects only. The checks that were verified and found
clean are listed under Summary.
