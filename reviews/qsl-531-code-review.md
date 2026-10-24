---
id: SR-12185
title: "Code review of quire-spec-language PR 1026: FR-339 component witnesses and the core partition check"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@d839169621a52beaf1ac2b76b8728607f0e35fd3; PR 1026: qsl-replay/src/certificate/scc.rs, qsl-replay/src/certificate.rs, qsl-replay/src/lib.rs"
review_set: subset
---
# Code review of quire-spec-language PR 1026

## Summary

Ticket: QSL-531 (partial, E8b). Code review with the rust-review lane folded
in, over `git diff origin/main...HEAD` only (three files, one commit). No build
was run; the review reads the code against FR-339, ADR-019 AM-7 and the
`qsl-eval` fairness `Graph` and `Constraint` it builds on.

- Rule order. `check_partition` runs one level at a time: `partition`
  (NotPartition), then `edges` (BackwardEdge), then each witness in listed
  order (WitnessFails). The `sub` levels of `UnfairStrong` witnesses are pushed
  reversed onto an explicit `Vec<Level>` stack and popped, so they run after
  their level, depth first, in listed order. A deep certificate cannot overflow
  the call stack. This matches the module doc.
- Located rejection state per rule (FR-339-AC-2). NotPartition: the duplicated
  or foreign state itself, else the least uncovered state by `ProductStateRef`
  order, else (empty component) the parent component's first state or the
  least reached state at the top level. BackwardEdge: the source state of the
  first computed edge, in component, state and edge-insertion order, whose
  target is owned by an earlier component. WitnessFails: the component's first
  listed state. UnfairStrong does not have a rule of its own. A failure of its
  listed constraints is WitnessFails at the component's first state, and a
  failure inside `sub` uses the sub level's own rules and loci.
- Soundness. No bogus-certificate acceptance was found.
  - Partition. Every listed state must resolve to a reached vertex inside the
    level's scope, and a second claim of a vertex is rejected. Coverage is
    checked over the whole scope. Empty components are rejected.
  - Topological order. Edges are read from the recomputed graph, never from
    the certificate. Only edges with both ends in the scope are read, so at a
    `sub` level an edge into a non-quiet state is correctly ignored.
  - Trivial. One state, and no self-edge of any label, stutter included. A
    terminal state's stutter self-loop is a cycle, and the code treats it as one.
  - MissingAcceptance. The set index must exist, and the vertex set must be
    disjoint from that acceptance set.
  - UnfairWeak. The constraint must be `Weak`, enabled at every state by the
    vertex's FR-120 enabled set (`is_enabled_in`: any member of a `whole`
    constraint), and taken by no inside edge. Stutter takes no constraint.
  - UnfairStrong. Every listed index must be a `Strong` constraint taken by no
    inside edge, and `sub` must partition exactly the quiet states, where none
    is enabled. A fair cycle that takes none of the listed constraints inside
    the component can only visit quiet states, so it lies in one
    sub-component. A nested `sub` re-enters the same three rules over a
    strictly checked scope.
  - Edge cases. `UnfairStrong{constraints: [], sub}` refines to the whole
    component, and an empty `sub` is accepted only when the quiet set is
    empty. Both are sound. The recursion is bounded by the finite certificate.
- Completeness. Every certificate EN-1 would emit under FR-339's rules is
  accepted. `UnfairWeak` refuses a strong constraint even when that
  constraint would also starve the component. That matches the FR ("that weak
  constraint"), and EN-1 emits `UnfairStrong` for strong constraints.
- Cancel. `poll` is called per component, per listed state, per edge read and
  per quiet-set state. A cancelled handle returns `Stopped(Cancelled{source})`
  and is never `Accepted` (tested). No budget is charged (FND-004).
- ProductGraph. It wraps `qsl_eval::fairness::Graph` with a state-to-vertex
  map and refuses duplicate states. No `qsl-analyze` code is used (ADR-018
  PC-5).
- Public wire types. `Component` and `ComponentWitness` are `pub`, re-exported
  at the crate root, and have no production caller yet. The export is
  justified: `qsl-analyze` (EN-1) depends on `qsl-replay` and already builds
  `ClosureCertificate` from it, so EN-1 needs these to be constructible. They
  follow `ClosureCertificate`'s shape (pub fields, `Clone, Debug, Eq,
  PartialEq`, no serde). Keeping `ComponentCertificate`/`ProofCertificate` out
  until their caller exists is consistent with that.
- Rust idiom. The production path has no `unwrap`, `expect`, panic, `unsafe`
  or lossy cast (`u32` to `usize` goes through `try_from`). The one
  `unwrap_or(fallback)` is on a branch the partition check makes unreachable,
  and it is commented. Iteration order is deterministic (`BTreeMap`,
  `BTreeSet`, insertion-ordered edges). No ticket ids appear in comments,
  nothing is vendored and there is no compatibility path.
- Declared remainder, not blocking: `check_components`,
  `ComponentCertificate`/`ProofCertificate` and the FR-127 settlement arm wait
  on E9 (QSL-530). "Delete uncertified for liveness" has nothing to delete,
  and the code confirms that.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `the_weak_each_partition_is_accepted` (TC-526) also asserts `ProductGraph::add_state`'s `DuplicateState` refusal. That is a construction-helper behaviour unrelated to TC-526 step 1, bundled into the acceptance test. It belongs in its own untagged test. | qsl-replay/src/certificate/scc.rs:568-572 |
| FND-002 | low | `lib.rs`'s `#[allow(dead_code, reason = "the zone certificate checker is its first caller and has not landed yet")]` on `mod certificate` now also hides the unused crate-private `check_partition`, `ProductGraph`, `ProductGraphError` and `EmptyProduct`. The stated reason does not cover them. It should name `check_components` as their first caller too, or the allow should move to the items. | qsl-replay/src/lib.rs:37-40; qsl-replay/src/certificate/scc.rs:94-212 |
| FND-003 | low | The module doc says the check polls "at every component, state and edge it reads". `MissingAcceptance`'s `is_disjoint` reads states unpolled, and `edges` polls per edge but not per state, so a state with no out-edges is read without a poll. The work is still linear and bounded, so only the wording is wrong. | qsl-replay/src/certificate/scc.rs:39-41,298-310,331-333 |
| FND-004 | low | FR-339 Behavior says the checker "SHALL charge those same existing budgets during component enumeration, partition and acceptance/fairness work". `check_partition` takes no limits and charges nothing. The module doc argues that the existing budgets are materialization budgets (states, transitions, automaton states), which only the product's construction consumes. That reading is defensible, but the spec text and the code disagree. Nested `sub` levels can re-read the same states and edges once per nesting level, bounded only by the certificate's size and the cancel handle. Either the FR sentence should say partition work charges nothing, or `check_components` (remainder) should state where the charge falls. | qsl-replay/src/certificate/scc.rs:38-41,185-212; spec/functional/FR-339-check-an-en-1-component-certificate.md:Behavior |

## Verdict

PASS. No major. The partition check is sound: every route by which a bogus
certificate could be accepted was traced and is closed. This covers foreign,
duplicate and uncovered states, empty components, backward edges, a
self-looping `Trivial`, a weak constraint that is not enabled everywhere or is
taken inside, a strong constraint taken inside, and a `sub` that keeps a
non-quiet state or omits a quiet one, at any nesting depth. The rule order and
loci match the module doc and FR-339-AC-2. Nesting is iterative, cancellation
is polled and never accepted, and the new public types are justified by
EN-1's crate dependency. The four findings are low. Nothing blocks.
