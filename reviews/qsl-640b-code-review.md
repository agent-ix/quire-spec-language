---
id: SR-1356
title: "Code review of quire-spec-language PR #645: composite WitnessValue replay and composite bounded_shadow settlement (QSL-640)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@6ec5705d9187c030367108c3078c3297490b1a96; PR #645 diff against origin/main: qsl-replay/src/composite.rs, qsl-replay/src/execute.rs, qsl-replay/src/execute/argument.rs, qsl-replay/src/execute/composite_domain.rs, qsl-replay/src/execute/composite_parity.rs, qsl-replay/src/execute/composite_site.rs, qsl-replay/src/execute/scalar_site.rs, qsl-replay/src/execute/parity_identity.rs, qsl-replay/src/execute/operator_parity.rs, qsl-replay/src/witness.rs, qsl-replay/src/witness/value.rs, qsl-replay/src/witness/value_text.rs, qsl-replay/src/request.rs, qsl-replay/src/lib.rs, qsl-foundation/src/bound.rs, qsl-eval/src/value/expression/*, qsl-semantics/src/model/object_environment.rs, and the tests under qsl-replay/src/execute/tests/ and qsl-replay/src/witness/value_text/tests.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-358
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-070
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: reviews
---
# Code review of quire-spec-language PR #645

## Summary

Ticket: QSL-640. Reviewed head 6ec5705d9 (731c576dd plus one spec-only
Status commit). The Rust lane (rust-review) is folded into this file.

Focused tests run at this head (locked-build, one target dir):
`cargo test -p qsl-replay -p qsl-foundation -p qsl-eval --lib -- tc_90 tc_736 tc_18 value_text witness bound`
passed (qsl-replay 122 passed, 0 failed).

Mutation of the identity tie, in a scratch worktree: forcing
`identity_tie` to accept any digest (keeping the preimage build and the
encoder call) made `tc_907_a_tampered_obligation_digest_refuses` and
`tc_907_a_preimage_the_encoder_refuses_never_yields_an_identity` fail; the
other 39 tc_907/tc_904 tests passed. So the tie is load-bearing and tested.

Checked and correct:

- `identity_tie` builds ADR-013 O-09's one parity preimage
  (`parity_preimage`) and recomputes it through `parity_obligation`, then
  compares with the request's identity. A mismatch refuses
  `ScalarIdentity(Obligation { claimed, recomputed })`, an encoder refusal
  `ScalarIdentity(Encoding)`. There is no `Ok(())` short cut and no sentinel
  identity. The membership checks compare two independent views of the
  package: the checked body's parameter slot (via `locate`) against the
  wire preimage's argument reference digest (`operand_identities`). The claim
  carries no operand identities, so a caller cannot invent one; the check
  guards against the two package views drifting.
- A composite literal operand is `graph_child` with an empty `bounds`
  domain. This is sound for identity: the literal node is content-addressed,
  so two different literals at one position give different `node_id`s.
- Common steps run in FR-358's order: recompile and package tie, node
  selection (Node, Function, Equality causes), derive, declare, harness,
  identity tie. Every refusal precedes F-1 and V-1..V-5, as the rulings say.
- F-1..F-6 and V-1..V-5 are in ruling order. `Refinement` is one enum for
  both entries. `NotEqual` negates the verdict and keeps the pair count. The
  pair count comes from `plan_equality`, with no early exit.
- `composite_domain::derive`: enum leaves get `Variants` (coverable);
  text, rational, decimal, float, quantity, reference and population leaves
  get `Whole` (never covered, harness bound refuses `HarnessKind`); a
  recursive declaration keys `Depth` at the path where it was first
  entered; a request `DeclaredDomain` over an authored position, of the
  wrong kind, unknown or repeated refuses; the position count is limited by
  `s3.nodes` with `PositionLimit`.
- `FiniteBound::variants` sorts, refuses empty and duplicate sets;
  `parity_identity` encodes `{"tag":"variants","variants":[...]}` as O-09
  states.
- Deep values (question 6): `WitnessValue`'s `Clone`, `Drop`, `Debug`
  (redacted, `node_count`) and `PartialEq` are all hand-written on explicit
  heap stacks; `WitnessField`/`WitnessSlot` derive `PartialEq`, which calls
  the iterative `WitnessValue::eq`. The value-text encoder and decoder,
  argument conversion and the domain walk are explicit-stack loops. TC-736
  runs a 100,000-long list on a 512 KiB thread through decode, clone, eq,
  Debug and drop, and through replay.
- Rebase onto B5 (question 4): replay limits now come only from
  `ReplayLimits` (`with_input_bytes`) and `StageLimits`; the tests raise
  `replay.input_bytes` through it and pass. No behaviour was lost in the
  code; only two spec Status sections still name the dropped
  `replay_within` / `reconstruct_within` (SR-1358 FND-006).
- Integer leaves in value texts decode to `ExactInteger`; transcript
  entries must ascend strictly (`EntryOrder`); set duplicates are found by
  the kernel's equality in `form_collection`; a float in a set is refused as
  a kind mismatch.

## Verdict

Request changes, for FND-001 and FND-002. The identity tie is real and
tested, the settlement rows follow the rulings, and the deep-value paths
are iterative.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | A composite claim cannot name which occurrence of its node it is about. `CompositeParityClaim` has no occurrence key, and `locate` always takes the first occurrence of the node in the function body. Probe at this head: `function d(a: Q, b: Q): Boolean { (a = b) and (a = b) }` has ONE equality node with occurrences `expression` 1 and 2 in `d` (and 0 in `f`, which shares the node). ADR-013 O-09 says two occurrences of one application node are two obligations. So CG's correctly minted identity for the second `a = b` (ordinal 2) always refuses `ScalarIdentity(Obligation)`, and the recomputed identity is always ordinal 1's. Fix: carry the occurrence key in the claim as `OperatorClaim` does, check it is an occurrence of the node in the selected function (`Occurrence` cause), and use it in the preimage (spec side: SR-1358 FND-003). | qsl-replay/src/composite.rs:88-106, qsl-replay/src/execute/composite_site.rs:93-118 |
| FND-002 | medium | An accounting-limit stop while admitting the operands (the request's `accounting_limits`, not the claim's `limits`) settles `Incomplete { stage: ExactEvaluation }` at F-2's place, before F-3 is checked. So a native `ExecutionFault` or `Incomplete` (F-3, `Failed`) is reported as `Incomplete(ResourceExhausted)` whenever the request's limits are too small for the conversion. The stage is also mislabelled: no exact evaluation ran. FR-358's rows do not place this case (SR-1358 FND-004). Fix: decide the row in FR-358, then either check F-3 before an admission limit stop or give the admission stop its own stage, and add a test with a native fault under limits too small for conversion. | qsl-replay/src/execute/composite_parity.rs:250-263, qsl-replay/src/execute/composite_parity.rs:325-342 |
| FND-003 | low | `check_elements` re-encodes every element of every set, bag and ordered set to JCS bytes, even a one-element collection where no order check applies. For a value nested through sets, each level re-encodes its whole subtree, so the work grows with depth times size and is charged to no meter. Fix: skip collections with fewer than two elements, and compare elements from the bytes already in the document (the entry is JCS text) instead of re-encoding nested subtrees. | qsl-replay/src/witness/value_text.rs:625-646 |
| FND-004 | low | `composite_domain::child` clones the parent's `path` and `composites` vectors for every derived position, so a deeply nested type costs positions times depth in time and memory before `PositionLimit` stops it (the limit counts positions, not the cloned length). Fix: keep one shared path stack (push on enter, pop on exit), or store parent links. | qsl-replay/src/execute/composite_domain.rs:343-362 |
| FND-005 | low | `replay_composite_parity` and `settle_verified_shadow` take no `ReplayLimits` and decode with `ReplayLimits::default()`, so a request beyond the default `replay.input_bytes` cannot be raised for these entries. FR-263-AC-3 says the library replay entry's limit raises it. The FR-357 entries share the pattern. Fix: take `replay_limits: ReplayLimits` as `replay` and `replay_frame` do. | qsl-replay/src/execute/composite_parity.rs:59-60 |

## Dispositions

Round 1, reviewed at dfc72b6d120b4bc914933300007032a7c6c67537 (fix commits 6ec5705d9..dfc72b6d1). Focused tests pass at this head (qsl-replay tc_904..tc_908, tc_736: 69 passed; compile_package_facade: 6 passed). Mutation: making `locate` skip the claimed-occurrence membership check fails `tc_907_each_occurrence_of_a_shared_node_settles_under_its_own_identity`.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 8b0247a83 |
| FND-002 | fixed | 8b0247a83 |
| FND-003 | deferred | Partly fixed in 74d7fca97 (collections under two elements are no longer encoded); encoding nested elements once needs a byte-span API or a bottom-up encoder, moved to QSL-647 (sub-ticket of QSL-640) |
| FND-004 | fixed | 74d7fca97 |
| FND-005 | fixed | 8b0247a83 |
