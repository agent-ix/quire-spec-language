---
id: SR-1647001
title: "code-review and rust-review of PR #1647 (QSL-583c, CF-2 component and divergence parts and the FR-245 checker)"
type: SpecReview
analysis: code-review
date: 2026-10-26
scope: "agent-ix/quire-spec-language; PR #1647 git diff origin/qsl-583b-zone-liveness...09259275c: qsl-replay/src/certificate/zone_certificate.rs, qsl-replay/src/certificate/zone_certificate/{product.rs,divergence.rs}, qsl-analyze/src/zone_check/{live_certificate.rs,liveness.rs,certificate.rs,search.rs}, qsl-eval/src/automaton/translate.rs, qsl-analyze/tests/it/zone_live_certificate.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-244
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-245
    type: reviews
---
# SR-1647001: code-review and rust-review of PR #1647 (QSL-583c, CF-2 component and divergence parts and the FR-245 checker)

## Summary

Ticket: QSL-583. Head: 09259275c. Soundness came first. I tampered with real certificates in probes in the review worktree, and reverted the probes afterwards.

**No false acceptance found.** Every tampering of a liveness or time-lock certificate was rejected:

| Tampering | Certificates | Result |
| --- | --- | --- |
| Relabel every component reason to each of Acyclic, Lacks(Automaton 0..2 / Divergence / Position), Weak(each constraint) | Rpc response (40 components), Serve weak, Serve strong, Serve weak+weak idle | An alternative was accepted only where it is true for that component (for example, `Lacks(Divergence)` on a singleton with no divergent node, or `Weak(idle)` on the idle-tail component, where no `idle` step is taken). `Acyclic` was never accepted for a component with an inner edge. |
| Flip `divergent`, `positioned` or `tail` on a node mark | Serve weak, first 12 nodes x 3 | 36/36 rejected (`InitialMissing` or `SuccessorMissing`) |
| Swap a silent edge `Wait` <-> `Diverge` | Rpc (73), Serve weak (26) | 99/99 rejected |
| Add a forged `Diverge` self-loop at every node | Rpc | rejected `WitnessFails` at the acyclic component the matching loop lands in. On Serve the forged edges match no successor, so they are ignored. Extra edges cannot hide a behaviour. |
| Number a node that enables `serve` into a `Strong` nested numbering | Serve strong | rejected |
| Move the certificate onto a sibling model where EN-6 says Violated (`serve` guard `x <= 1 ms`, weak and strong) | Serve weak, Serve strong | `WitnessFails` at `ZoneComponent 9`, the idle loop's component: the checker's own cells show `serve` is not enabled everywhere |
| Move it onto the no-fairness sibling | Serve weak | rejected `InitialMissing` (fewer cell atoms, so different cells) |
| Divergence witness of `Quiescent(self)` for every node over locked valuations | `Lock` with `back` | `WitnessFails` at node 1 |

**The checker recomputes and does not trust the certificate.** Edges that match no recomputed successor are ignored (`covers` is `None`). Every recomputed successor must be covered, or the result is `SuccessorMissing`. Each reason is re-derived from the covered edges and the checker's own cells.

**FR-232 enabledness parity holds.** This is the #1643 lesson. `enabling_parts` in product.rs conjoins each guard part with the target invariants on the clocks the step does not reset, and gives no part when a reset value breaks a target bound. The same function feeds the cell atoms (`atoms`), `everywhere` and `somewhere`. Successors apply the part, the resets and then the target invariants, which is equivalent. The divergence fixpoint's `pre` applies the target invariants before it undoes the resets. All of this matches the engine's `enabling_parts`.

**Abstraction.** Coverage on the product is read under bounds that are at least aM on every clock, with `z` at the scale. aLU with L, U >= M is no coarser than aM, which is a bisimulation and keeps each valuation's cell. The time-lock item is not a clause, so `check` forces aM.

**One major defect (FND-001).** A crafted certificate aborts the checker with a stack overflow, within the default byte budget.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Nested `Strong` reasons are recursed without a depth bound. A 200,000-deep chain (about 38 MB, under the 1 GiB `max_certificate_bytes`) aborts the checker: `has overflowed its stack`, SIGABRT. | FR-245; zone_certificate.rs 2112-2147 (`components_bytes`, `numbering_ok`), product.rs 599-664 (`reasons`) |
| FND-002 | low | `automaton()` has two identical match arms (`StateLimit` and `_` both map to `CheckerFault::Automaton`). The checker translates the negation automaton with a `u64::MAX` state limit, so no budget bounds it. | product.rs 102-107; zone_certificate.rs 579-584 |
| FND-003 | low | `components()` and `nested()` skip uncovered edges silently. That is sound, but a certificate padded with junk edges is accepted with no signal, and the junk is charged only through the byte budget. | product.rs 577-590, 693-701 |

## Verdict

Changes requested for FND-001. Bound the nesting (for example, reject a `Strong` whose nested numbering covers the same set of nodes as its component, or cap the depth at the node count, as `Malformed` at `CertificateMember("components")`). Make `numbering_ok`, `components_bytes` and `reasons` iterative or depth-checked. Add a regression test. FND-002 and FND-003 are cleanups.

## Coverage

- Read the whole diff: product.rs, divergence.rs, the checker changes, live_certificate.rs, the liveness.rs idle-tail refactor (tail vertices keep their zone; stutters are `divergent` only under a tick source), and the search and certificate wiring.
- The divergence fixpoint is checked by hand. In `νY.μX`, X is the idle states, plus `z >= 1` reset into Y, plus pre-step of X with delays held to the invariants. Backward zone operations stay within a finite union of regions, so it terminates. Each round is charged.
- `with_z` lifts node zones at `z = 0`.
- The idle-tail divergence change (`divergent: !dense`) keeps tail acceptance through the `z` resets.
- Probe results are summarised above. The probes were reverted with `git checkout --`.
- The baseline `zone_` integration tests pass (79).

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | low | `Components` and `ComponentReason` still derive `Clone`, `Debug` and `PartialEq`, which recurse. The checker never clones, prints or compares a certificate's components, so the check is safe. A caller that clones or prints a crafted certificate before checking it would still overflow. Non-blocking: either note it on the type or make those impls iterative. | zone_certificate.rs 162-200 |

## Dispositions

Round 1. The head is 4842a64a5: fix commit 2967a403b, restacked base-only. Checked in a fresh worktree and target under locked-build at jobs 4:
- `zone_` tests: 82/82 passed (79 baseline, plus the deep-nesting, padding and drift tests).
- clippy `-p qsl-replay -p qsl-analyze --all-targets --all-features -D warnings`: exit 0.

I re-ran the round-0 tamper probes, then reverted them:
- relabelled reasons (Serve weak, strong and weak+idle): 0 implausible acceptances;
- flipped marks: 0/36 accepted;
- swapped `Wait`/`Diverge`: 0/99 accepted;
- moved to a Violated sibling: `WitnessFails` at `ZoneComponent 9`, weak and strong;
- moved to the no-fairness sibling: `InitialMissing`;
- forged `Lock` divergence witnesses: `WitnessFails` at node 1.

Every verdict is unchanged. My original 200,000-deep chain, and a 200,000-deep chain that repeats the same member set, are now both `Malformed` at `components`, with no overflow. `max_automaton_states: 1` stops the check with `LimitReached(MaxAutomatonStates, 1)`. The default is the shared `DEFAULT_MAX_AUTOMATON_STATES` (1,048,576), so it matches the engine's.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | `components_bytes`, `numbering_ok` and `reasons` walk explicit stacks. `Components` has an iterative `Drop`. `numbering_ok` requires each nested numbering to number a strict subset of its own component (members only), so depth is bounded by the node count. Regression test: `a_strong_nesting_deeper_than_its_nodes_is_malformed`. |
| FND-002 | fixed | The duplicate arm is gone. A new `CertificateCheckLimits::max_automaton_states` (default the shared engine constant) bounds the rebuilt automaton, for both the liveness and window paths, and stops the check with `Stopped(LimitReached(MaxAutomatonStates))`. FR-245's Inputs do not list this field; STD-493 8eafddf9 item 4 records that correctly. It is a real resource bound, not ceremony. |
| FND-003 | fixed | Kept by design. FR-245 has no Malformed row for an edge that covers no successor; such edges are charged to `max_certificate_edges`, and `padding_edges_are_charged_against_the_edge_limit` pins that. |

Round 2. The head is e12f4b7d9. The fix commit is on top of 4842a64a5, with no restack. `Components` has hand-written `Clone`, `PartialEq`/`Eq` and `Debug` over a breadth-first flattening (`levels`). A strong reason names the later level of its nested numbering, so every reference points forward. That is why `Clone` can rebuild deepest first and take each built level exactly once. Equal levels hold exactly when the derived equality holds, because the breadth-first indices are fixed by the tree's shape. `Debug` prints the levels: different text from the derived form, but the same information.

My probe, reverted afterwards, used two 200k-deep chains with two reasons at each level:
- a clone equals the original;
- chains that differ only at the deepest leaf compare unequal, and so do chains that differ only at the deepest constraint;
- a branching level with its two strong reasons swapped compares unequal;
- a clone of the branching level keeps each nested numbering under its own reason.

None of these overflowed. `zone_live_certificate`: 7/7 passed. Clippy with `-D warnings`: exit 0.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-004 | fixed | Iterative `Clone`, `PartialEq`/`Eq` and `Debug` over the breadth-first levels, equal in meaning to the derived impls. `a_deep_strong_nesting_clones_compares_and_prints` covers a 200k-deep chain. |
