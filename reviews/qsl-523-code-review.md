---
id: SR-12067
title: "QSL-523 PR #932 code review: qsl-analyze graph core (retained edges, iterative SCC, generic product)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@0284181836139f07147116d0aa86fd0e7328ae8c; PR #932: Cargo.lock, qsl-analyze/Cargo.toml, qsl-analyze/src/lib.rs, qsl-analyze/src/model_check/mod.rs, qsl-analyze/src/model_check/graph.rs, qsl-analyze/src/model_check/product.rs, qsl-analyze/src/model_check/scc.rs, qsl-analyze/tests/it/graph_core.rs, qsl-analyze/tests/it/main.rs"
review_set: subset
---

## Summary

Ticket: QSL-523. PR quire-spec-language#932, diff `origin/main...028418183`
(9 files, +1526/-3). Code review with the rust-review lane folded in.
Read-only: no build or test was run. The author's gate (fmt, clippy
`-D warnings`, `cargo test -p qsl-analyze --all-features`, xtask
import_graph) is taken as given per the brief, and gates are not reviewed.
QSpec is cited by id only.

Checked hard, per the brief:

1. Iterative SCC (`scc.rs`). Two-pass Kosaraju with explicit stacks and no
   recursion. Pass 1 is a `(node, next-edge)` DFS that pushes a node to
   `finished` only after all its retained out-edges are consumed, so the
   finish order is a correct postorder. Every state is a pass-1 root
   candidate, so every state is in `finished`. Pass 2 walks the transposed
   graph (`incoming` built from every retained edge, multi-edges included)
   in decreasing finish order, so each tree is exactly one SCC. Members
   and components are then ordered by one discovery-order scan, so
   ordering is deterministic and linear. `cyclic` is true for more than one
   member or a retained self edge. Parallel edges do not change it. The
   30,000-node chain and 30,001-node cycle on a 256 KiB thread would
   overflow under a recursive DFS. The all-512-three-node-graphs test
   checks the partition against an independent transitive-closure oracle.
2. Retained edges (`graph.rs`). Every examined successor that has an
   admitted target is pushed to `edges` and `outgoing`. Coalescing by full
   RFC 8785 key bytes only picks the target id and never skips the edge.
   The only edge not retained is the one whose new target hits the state
   cap. It has no admitted endpoint, and the doc comment says so. Every
   edge carries `IdentityPermutation`. There is no dedup of successors.
   Duplicate `(transition, key)` pairs stay as parallel edges, as the
   first test asserts.
3. Generic product (`product.rs`). `ProductKey` encodes
   `[model key, property key]` as a canonical array. Both components are
   always present, and the array framing rules out ambiguous
   concatenation, which the `key("ab","c") != key("a","bc")` test checks.
   `ProductState` keeps both components. `ProductTransition` tags model
   steps `["model", id]` and stutter `["terminal-stutter"]`, so no model
   identity can alias the stutter step. The counters test compares the
   full retained edge multiset against an independently written relation,
   so a property-key omission would merge `(m,0)` and `(m,1)` and fail it.
   Successor order is `retain`'s own sort over (transition bytes, key
   bytes), the same comparator as qsl-eval's `order::expand`, so the order
   is independent of the port's listing order and the property's
   materialization order.
4. Limits, cancel and work accounting against FR-101's seam. `retain`
   matches qsl-eval `explore` field for field. Inclusive caps.
   `max_transitions` is checked before each examined successor and counts
   coalescing edges. The state cap pushes the rejected key's digest onto
   the frontier. The too-many-initial path reports `states = max_states`
   with every initial digest. The depth horizon is checked before poll,
   and poll before expansion. `Stopped` puts the head state first in the
   frontier. A state that was cut off keeps its partial out-edges, is
   marked `expanded: false` and has no findings. EN-1 asks for EN-1's own
   exploration in `qsl-analyze` in FR-101 order with FR-101 keys, so a
   second BFS here is what the spec requires, not a copy.
5. Dependency graph. `qsl-analyze` adds `qsl-eval`, `qsl-foundation` and
   `quire-canonical` (workspace). These are layers 5, F and the shared
   encoder, all allowed by ADR-018 LA-1 and ADR-011 §6.1 layer A. No crate
   depends on `qsl-analyze`. The root `no_core_crate_depends_on_qsl_analyze`
   test still guards that. Nothing is vendored, and no new external crate
   was added.
6. Panic surface. The production paths index with `states[id.0]`,
   `outgoing[node]`, `edges[edge]` and `incoming[target]`. Every
   `ProductStateId` is minted only inside `retain` (`pub(super)` field).
   Every edge endpoint is an admitted id, and `RetainedGraph`'s fields are
   private outside `model_check` and never mutated after `retain` returns.
   So these indexes are in range by construction. The public accessors
   (`state`, `outgoing`) take foreign ids and use `get`. No `unwrap` or
   `expect` on a production path. `#![forbid(unsafe_code)]` holds.
7. Test oracles. The expected state orders, edge lists, SCC partitions,
   product pair sets, edge multisets and SCC sets are written out by hand
   or come from independent oracles (transitive closure, a separately
   written product relation). None is derived from the implementation.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

PASS. No major bug in the changed code. The iterative Kosaraju SCC is
correct and stack-safe. Retained edges are unreduced: parallel, self, back
and coalescing edges are all kept, each with `IdentityPermutation`. The
product key carries both components. Limits, cancellation and work
accounting match the FR-101 seam. The crate's dependency direction is
clean.

Not raised (below the brief's major-only bar, recorded for the next
EN-1 slice):

- `PropertyComponent::findings` and the bounded `close` run only when a
  product state is expanded. `retain` never expands states at the
  `max_depth` horizon, or states left in the frontier. So a position
  finding, or a TP-2 closure at a terminal state at depth `k`, is not
  produced by the product. ADR-018 UE-4 has EN-1 read the letter of every
  product state it creates up to depth `k`. The future `model_check`
  caller must evaluate horizon states itself, or this seam should move
  position findings to state creation.
- `ProductTransition::Model(id)` sorts by the bytes of `["model", id]`.
  This keeps the model's own FR-101 order unless one identity's encoding
  is a proper prefix of another's. Only bare top-level JSON numbers can be
  (for example `1` and `12`). Then the product order and the model order
  disagree. It is still deterministic.
- `retain` returns a bare `quire_canonical::Error`, while qsl-eval's
  explorer maps the same failure to `EncodingRefusal`. The caller of
  `retain` will need that mapping when it settles a refusal.
