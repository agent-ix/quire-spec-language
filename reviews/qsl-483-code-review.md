---
id: SR-1260
title: "Code review of quire-spec-language PR #615: arena checked node, no fixed depth (QSL-483, B2)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@220d8fa2b2028aad8e7379ed6550707900fbda24; PR #615 diff against origin/main (merge base 8d1deba43): qsl-semantics/src/check/{ir.rs, check.rs, check/typing.rs, facts.rs, observation.rs, claims.rs, termination.rs, field_refinement.rs, state_clause.rs, family.rs, mod.rs, refusal.rs, region.rs, checked_dispatch.rs, lowering.rs, lowering/model.rs, lowering/state.rs, lowering/wire.rs, node_key/mod.rs, node_key/shape.rs} and their tests; qsl-semantics/src/family/{contract.rs, requirements.rs}; qsl-semantics/tests/it/state_clauses.rs; quire-semantic-value/src/{checking.rs, semantic_node.rs}; qsl-replay/{Cargo.toml, src/result/wire.rs, src/result/wire/value.rs, src/spine.rs, src/witness/derivation.rs, src/spine/clause/tests.rs, src/spine/clause/tests/witness.rs}; qsl-eval/src/value/expression/{evaluate.rs, mod.rs, s6a/separation.rs}; qsl-eval/tests/it/{dispatch_calls.rs, total_functions.rs}; qsl-package/src/emit.rs, emit/extent_agreement.rs, emit/tests.rs, emit/tests/admission_corpus.rs, emit/tests/deep_bodies.rs; qsl-bench/src/deep_input.rs; xtask/src/definition_scan.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-258
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-030
    type: reviews
---
# Code review of quire-spec-language PR #615

## Summary

Ticket: QSL-483 (B2). PR: quire-spec-language#615, draft, one commit on main
8d1deba43. Reviewed head 220d8fa2b. The Rust lane (rust-review) is folded into
this file. AC coverage is in SR-1261.

What the PR does, checked against the code:

- **Arena.** `CheckedBody { nodes: Vec<Node> }` with the root last, `NodeId`
  children and the `CheckedNode` view (ir.rs:106-294). `Clone`, `PartialEq`
  and `Debug` are derived over the flat vector, and drop is the vector's drop.
  `Node` holds `ValueType` (hand-written iterative traits in quire-exact),
  `Location` (flat `Vec<usize>`), `Box<CheckedEquality>` (not recursive) and
  `CheckedLiteral(Value)` (shallow clone, worklist `Debug`, `compare_keys`
  equality). No derived impl in the arena recurses.
- **Typer.** One `BodyBuilder` per pass, taken with `mem::take` on success
  (typing.rs:425-447). Every caller propagates a refusal with `?`, so a failed
  pass never leaves nodes in a later body. `Frame::Leave` and the depth counter
  are gone. Nodes are pushed only when a parent names them, so each is stored
  after its operands.
- **Walks.** Facts (`stable_path`, `subject`, `literal`, `interval`,
  `outcomes`), observation, claims, termination, field refinement, state
  clauses, `descendants` and `catalogued_step` keep explicit stacks. I traced
  each against the recursive version it replaces. The evaluation order, the
  `If`/`Let`/`Query`/`Fold` result plumbing in observation, and the
  connective `under` facts in `outcomes` match the old code.
- **LeafWalk** runs on `quire_walk::walk` (lowering.rs:955-1037). quire-walk
  enters children in push order (walker.rs:83-104), so leaf order is
  unchanged. Open composites are a `BTreeMap`. `reaches_text` answers every
  composite reached in one search, with a backward pass over `named_by`
  (lowering.rs:1124-1200). That removes the per-link re-search. The claim
  holds.
- **Stratified terms.** `LeafTerm`, `Binding<V>`, `GroupTerm`, `MemberTerm`,
  `BindingValue`, `ApplicationTerm`, `AggregateTerm`, `FrameTerm`, `BodyTerm`.
  `Walk::body` calls only lower strata, so keying is a fixed-depth match.
  `PreimageTerm` keeps a derived recursive `Serialize`, but it is built from a
  stratified body, so its depth is fixed. The pinned-bytes tests are unchanged,
  so node ids did not move. `shape.rs` drops `disable_recursion_limit`; that
  is correct because the preimage depth is now fixed.
- **Deletions verified.** `MAX_CHECKING_DEPTH`, `DepthAboveMaximum`,
  `CheckingLimitKind::Depth`, the fallible `CheckingLimits::new`,
  `StageLimits.nesting_depth`, `enter_nesting`/`leave_nesting`,
  `NodeKeyRefusal::TooDeep`, the recursive `LeafWalk::walk`/`within`/`fields`,
  `node_key::term`, `TypeWire::of`, `CollectionTypeWire::of`, `ValueWire::of`
  and the nesting-depth tests are gone. A grep of `*.rs` at 220d8fa2b finds no
  reference left. Nothing under `qsl-semantics/src/check` depends on
  `stacker` or `qsl-walk-grow`. No test asserts a deleted name's absence (the
  xtask list only drops two names).
- **No ceremony, compat layer or vendoring** in the diff. The one dependency
  change is the serde_json `raw_value` feature (FND-003).
- **Behaviour change.** `PackageDeclarations::check` now returns at the first
  `StageFailure::Limit` (check/mod.rs:999-1005). That matches FR-258 Behavior 6.

**The serde_json `raw_value` feature.** It is needed for this design: it is
how the hand-written element text is spliced into the serde-written
`RecordWire`. It brings back no recursion limit. `RawValue::from_string`
parses through `deserialize_raw_value`, which calls `ignore_value`. In
serde_json 1.0.151 that is a loop over `self.scratch` that never touches
`remaining_depth` (src/de.rs:1102-1200, :1285-1293). The 100,000-deep test
passing confirms it (make ci log line 1052). It does add a second encoder,
though (FND-003).

**Test oracles of the 100,000-deep tests.** These are real-output checks, not
just "it didn't crash":

- `a_100000_deep_checked_body_clones_compares_formats_and_drops_on_a_small_stack`
  checks the length, the descendant count, clone equality, inequality at the
  deepest node and the `Negate` count in `Debug`.
- TC-726 checks the leaf path length and its segments.
- The deep option keying test compares every node key on a 512 KiB stack with
  the keys on an 8 MiB stack, and counts the `option` nodes.
- The deep deciding element test counts the per-level substrings.

The 1,000-level form tests compare outcomes at 1,000 with those at 2, which is
what FR-093-AC-14 asks for.

**Gate.** The coder's `make ci` log (`~/dev/worktrees/logs/qsl-b2-make-ci.log`)
ran in the PR worktree and ends `exit=0`, with fmt, clippy `-D warnings`, the
tests, docs and arch-lints. Its mtime is 21:06, and the commit was authored at
21:07, so the gate ran on the working tree just before the commit. I did not
re-run any test.

**Reported, not ruled (with plan).**

- **Location.** `Location::child` clones the whole parent path
  (quire-semantic-value/src/location.rs:85-92). So a node at depth d costs
  O(d), and a 100,000-deep chain needs about 5×10⁹ path entries.
- **Strata.** `BindingValue::Binding` lets a Member binding hold a binding of
  a Leaf (node_key/mod.rs:388-404), one level beyond ADR-030 D-2's Member row
  ("binding of a Leaf or a Group"). The AC-5 oracle's `is_binding_value`
  mirrors the code rather than D-2, and its `is_leaf` admits `group_reference`.
- **FR-269 reader.** The reader still deserializes the derived recursive
  `ValueWire`. serde_json caps that at 128 levels, and the implicit drop of a
  deep `ValueWire` recurses within that cap. The plan has ruled that this moves
  to `quire_canonical::read` after #613.
- **Pre-check.** The ruled per-declaration `node_count` pre-check is at
  check/mod.rs:932-935 → family.rs:1486 → family/contract.rs:182-190. See
  SR-1261 FND-003.

**Outside the diff, for routing (not findings of this PR).**

- The root crate's `src/checking.rs:94-112` holds `Limits::depth` ("at most
  64"). `src/checking/constraints.rs:187-200` recurses natively and refuses
  with "native checking depth limit exceeded". ADR-030's inventory does not
  list this depth cap.
- `qsl-replay/src/spine.rs:760-870, 1117-1124` keeps
  `DependencyLimits::depth` (64, `nesting-depth-exceeded`). ADR-030 D-7 slice
  1 lists it for deletion. Confirm which slice-1 ticket owns it.

## Verdict

Changes requested. The arena, the stratified terms, the LeafWalk conversion
and the deletions are correct, and I found no recursion left in a derived impl
or on a converted path. Three medium findings: the walks are not on the
toolkit as FR-258 and ADR-030 slice 1 require, the 100,000 `let` chain has
quadratic costs beyond `Location`, and the hand-written element encoder has
untested branches. There are also four low findings. Not mergeable at this
head: the two ruled changes are still to come (pre-check removal in this PR,
and the reader move after #613 and a rebase), and AC-1 is unmet (SR-1261).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-258 Behavior 3 says every checker walk "SHALL run in arena order or on the walker toolkit (FR-356)", and ADR-030 D-7 slice 1 says "the remaining checker walks on the walker toolkit". This PR adds new hand-written stacks instead: `Definedness::interval` and `outcomes`, the observation `Walk::run` task stack, `catalogued_step` and `CheckedNode::descendants`. Typing, lowering and claims keep their own frame machines. Only `LeafWalk` uses `quire_walk::walk`. Each walk is correct and stack-safe. But FR-356's qualification argument ("one Kani-verified traversal instead of a hand-written stack in each walk") fails for every one of them. Fix: move these walks onto `quire_walk`. `descendants`, `callees`, `losses` and `dereferences` can be one forward loop over the arena. Or have the plan amend FR-258 Behavior 3 and D-7 slice 1 to accept explicit stacks for this slice. | qsl-semantics/src/check/facts.rs:502; qsl-semantics/src/check/facts.rs:658; qsl-semantics/src/check/observation.rs:108; qsl-semantics/src/check/check.rs:2370-2410; qsl-semantics/src/check/ir.rs:234 |
| FND-002 | medium | The `#[ignore]` reason on the AC-1 test is accurate but incomplete. Besides `Location`, the 100,000-deep `let` chain is quadratic in four more places, and none of them is charged per level. (1) `Typer::bind` scans every local for a duplicate name (check.rs:936). (2) `Typer::name` scans locals from the top for each `a` (check.rs:1266). (3) `Definedness` clones the whole `Facts` at every `let`, and each `let b = a` adds an alias, so the clone grows by one entry per level (facts.rs:874-878). (4) Lowering finds each `Local` by a reverse scan of the binder scope (lowering.rs:2654, :2794). Separately, `outcomes` keeps two `Facts` clones per pending connective (facts.rs:1131-1144), so its heap grows by O(\|facts\|) per level, not by the constant FR-258 Behavior 3 states. A `let` chain over 100,000 levels does about 5×10⁹ steps in each of these places, so fixing `Location` alone will not make the test runnable in a debug build. Fix: name these sites in the ignore reason and in the follow-up. Index locals and binders by name or slot, and share `Facts` across levels (persistent maps or a scoped undo log), so each level costs O(log n). | qsl-semantics/src/check/check.rs:936; qsl-semantics/src/check/check.rs:1266; qsl-semantics/src/check/facts.rs:874-878; qsl-semantics/src/check/facts.rs:1131-1144; qsl-semantics/src/check/lowering.rs:2654; qsl-semantics/src/check/lowering.rs:2794; qsl-package/src/emit/tests/deep_bodies.rs:65 |
| FND-003 | medium | `encode_element` is a second encoder for the FR-269 deciding element. It writes the JSON by hand, repeating as string literals the serde layout of `ValueWire`, `TypeWire`, `CollectionTypeWire` and `SlotWire`, while the reader still uses the derives. A rename on either side compiles. The round-trip test `tc_744_every_deciding_element_kind_round_trips` reads back: options over scalar or composite types, composites with present slots, and collections of scalars. It never reads back the hand-written type-level `option` (value.rs:849) or `collection` (value.rs:854), or the `absent`/`null` slots (value.rs:783-784). The 100,000-deep test only counts substrings. So a typo in those literals passes every test and writes documents the reader refuses. Fix: add round-trip cases for an `Option<Option<Int>>` payload type, an `Option<Sequence<Int>>` payload type, and a composite with `Absent` and `Null` slots. Move to quire-canonical's event writer when it exists; the locked 9572a21 has none. | qsl-replay/src/result/wire/value.rs:715-876; qsl-replay/src/result/wire/value.rs:783-784; qsl-replay/src/result/wire/value.rs:849; qsl-replay/src/result/wire/value.rs:854; qsl-replay/src/spine/clause/tests/witness.rs:1343-1502 |
| FND-004 | low | `ElementWire` puts the writer and the reader in one enum. Its `Read` state cannot be serialized: `Serialize` returns an error for it (value.rs:674-683). Its `Written` arm in `read` has no caller, and it re-parses through serde's recursive derived reader (value.rs:669). Fix: write through a plain wrapper over `Box<RawValue>`, read `ValueWire` directly in `RecordWire`, and delete the dead arm. | qsl-replay/src/result/wire/value.rs:654-689 |
| FND-005 | low | Doc comments still describe deleted depth bounds. family.rs:7-9 tells the history of `check` ("Before, ... charged the contract's own nesting-depth limit once"), naming a limit that no longer exists. family.rs:741-753 talks about charging "`CheckContext`/`StageLimits.nesting_depth`" and says "`Typer`'s pre-existing, separate CheckingLimits depth bound (unchanged, checked at every real nesting step already) is what bounds a call's own nesting". This PR deletes both. check.rs:744-746 and :1012-1014 still say "no checking limit has bounded its depth yet". Fix: reword them to the node and work limits, or delete the paragraphs. | qsl-semantics/src/check/family.rs:7-9; qsl-semantics/src/check/family.rs:741-753; qsl-semantics/src/check/check.rs:744-746; qsl-semantics/src/check/check.rs:1012-1014 |
| FND-006 | low | `CheckedLiteral::eq` returns false whenever `compare_keys` returns `None`, which it does for a value that holds a float. A `CheckedBody` holding such a literal would then compare unequal to its own clone, which breaks reflexivity and AC-1's "compares equal to its clone". Today the typer builds only Boolean, Integer, Rational and Enum literals, so this cannot happen yet, but the field is `pub`. Fix: fall back to encoding equality, as replay's `same_element` does, or restrict the literal kinds by type. | qsl-semantics/src/check/ir.rs:335-344 |
| FND-007 | low | Two new tests have no trace tag in a traced repo: `the_stratum_walk_refuses_a_nested_composite` and `the_text_leaf_walk_enters_two_visits_per_charged_record`. The second backs FR-258 Behavior 3/4's charged-growth bound, which no AC names. Fix: tag the oracle self-test with TC-725/FR-258-AC-5. For the charged-growth test, add an AC or tag the closest one. | qsl-semantics/src/check/lowering/tests/deep_bodies.rs:106; qsl-semantics/src/check/lowering/tests/depth.rs:224 |

## Dispositions

Round 1, reviewed at d3fe0a5a629876c960646cc7e1047be62c1ece09: the fix round
rebased on main fbf69cb9c, so this round reviews `git diff
origin/main...d3fe0a5` and the fix commits 302494b2e..d3fe0a5a6. I ran no
build. The coder's `make ci` log (`~/dev/worktrees/logs/qsl-483-make-ci.log`)
ends `exit=0`, after the head commit, with the AC-1 test passing in 446.84 s
(debug).

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 21458cd9f, with 302494b2e and 669a068ba. `Definedness::interval` and `outcomes` now run as `IntervalWalk` and `OutcomeWalk` on `quire_walk::walk` (facts.rs:511-519, 620-632, 1066-1263). The observation walk is a `quire_walk::Walk` with typed `Step` and `Exit` frames (observation.rs:118-125, 236-330). `catalogued_step` scans its operand with `FreeOfAccumulator` on the toolkit (check.rs:2412-2447). `CheckedNode::descendants` is a toolkit walk, and `callees`, `losses` and `dereferences` read it (ir.rs:236-306). I traced the observation result stack: every node's exit leaves exactly one result, and `Let`, `Query` and `Fold` bind between their operands as the old code did. The older frame machines are a separate point (FND-012). |
| FND-002 | fixed | 302494b2e, with 5e56f6b7a. `Location` is an `Arc` parent chain. `child` is O(1), `Drop` is a loop, and equality, hash and order go by depth and fingerprint (quire-semantic-value/src/location.rs:85-283). `Typer::bind` and `Typer::name` use a `local_names` `HashMap` (check.rs:935-975, 1276-1283). `Definedness` keeps one `aliases` table and shares `Rc<Facts>` (facts.rs:399-404, 802-806). Lowering's `Binders::find` is a `HashMap` by slot (lowering.rs:358-404). Emission memoizes regions (region.rs:33-67, 286-306). The AC-1 test now runs, 100,000 deep. A guard chain that adds facts still copies `Facts` at every level (FND-008). |
| FND-003 | fixed | 08de27836. The element is written through `quire_canonical::to_vec` by an explicit-stack `ElementEncode` (value.rs:682-859), with the fixed-shape forms through their `FixedShape` serde encoding. It is read by `quire_canonical::read` and an explicit-stack `decode_element` (value.rs:970). `tc_744_every_deciding_element_kind_round_trips` adds the three named cases (witness.rs:1451-1472) and round-trips bounded and unbounded collection types, so every hand-written member name is exercised on both sides. |
| FND-004 | fixed | 08de27836. `ElementWire` is gone. `RecordWire<E>` is generic: `Box<RawValue>` on write, and on read `ElementSlot`, an index into the elements decoded from the document (wire.rs:364-421). No dead arm remains. |
| FND-005 | fixed | b03a80050 (family.rs) and f5fb83bc4 (check.rs). family.rs:7-12 no longer tells the nesting-depth history. check.rs:744-747 and the second site now say the stack grows by one entry per node, which the typer then charges. The new text at family.rs:733-735 is inaccurate in another way (FND-011). |
| FND-006 | fixed | ad942518e. `CheckedLiteral::eq` falls back to `same_encoding`, an explicit-stack structural comparison that takes floats by width and bits (ir.rs:356-432). `a_float_literal_equals_its_clone_and_differs_by_bit_pattern` checks a NaN alone and inside an option. The float case this finding named is fixed. A population is still not equal to itself (FND-010). |
| FND-007 | fixed | f5fb83bc4, with 302494b2e. `the_stratum_walk_refuses_a_nested_composite` is tagged `#[trace("TC-725", "FR-258-AC-5")]` (stratum.rs:86). `the_text_leaf_walk_enters_two_visits_per_charged_record` is tagged `#[trace("FR-258-AC-6", "TC-726")]`, against a new FR-258-AC-6 and its TC-726 step. |
| FND-008 | fixed | Round 2, aed7b2df0: 732a93a8e (test corrected in aed7b2df0). `Facts` now holds three `PersistentMap`s, which are hash tries with `Rc` nodes (facts.rs:64-72; facts/persistent.rs). `Facts::clone` is three reference counts. An insert copies one root-to-leaf path, bounded by the 64 hash bits and about log2(n) in practice. `join` uses `intersect_with`, which skips every subtree the two sides share. In the guard chain this finding described, each level now adds O(log n) trie nodes to each outcome instead of a copy of size n, so live memory and time are O(n log n). The maps expose no iteration, so their hash order reaches no output. `every_version_of_a_growing_map_shares_its_nodes` asserts the sharing on 2,000 versions. The new `Form::DistinctGuards` runs the guard chain at 2 and 1,000 levels. The definedness walk also moved onto `quire_walk` with the old order kept (the `then` branch before `otherwise`, the source, identity or reduction obligation, then the step). |
| FND-009 | fixed | Round 2, aed7b2df0: 732a93a8e. `SpanMemo` keys on (link address, span-tree address) and keeps a clone of every location whose links it remembered (region.rs:41-70). No remembered address can be freed and reused while the memo lives, and a chain read under another origin uses that origin's tree. The span trees are borrowed from the graph for the closure's life. A dedicated test is not needed: address reuse depends on the allocator and cannot be forced deterministically, and the fix makes it impossible by construction. The memo's main path is covered by every emission test, including the 100,000-deep AC-1 test. |
| FND-010 | fixed | Round 2, aed7b2df0: 732a93a8e. `same_encoding` compares two populations by identity (ir.rs:392-396). The doc now states the relation it implements and says it needs no agreement with any writer's encoding (ir.rs:374-381). The float test also checks a population: equal to its clone, unequal to another id, and `compare_keys` is `None`. The function keeps the name `same_encoding`, which is now a misnomer; it is cosmetic. |
| FND-011 | fixed | Round 2, aed7b2df0: 732a93a8e. refusal.rs:584-585 no longer lists a preimage depth bound. family.rs:733-735 says the `Typer` node charge bounds a call's nesting. |
| FND-012 | still-open | Round 2, aed7b2df0: awaiting the ruled reword. 732a93a8e moved `Definedness::walk` and the claims walk onto `quire_walk`, and rewrote Behavior 3 to name the toolkit walks, with a Status remainder and a follow-up slice for `Typer::run` and lowering's frame stacks (FR-258:51-63, 117-122). The plan session has ruled that Behavior 3 states the behaviour, that typing, definedness and lowering run at any depth without native recursion, with no Status remainder and no follow-up ticket. The current text does not match that ruling yet. |
| FND-012 | fixed | Round 3, 83efb2a9b: b6e47deb1. FR-258 Behavior 3 is now "No native recursion". It requires typing, definedness facts, obligation collection, expression lowering, the FR-092 type-keying walk and every other walk to run at any depth without native recursion. It names no toolkit and the Status remainder is gone (FR-258:51-57), as the plan session ruled. Typer and lowering meet it with heap-stack frame machines. Behavior 4 still names the walker toolkit for the text-leaf walk, which is true of the code. |
| FND-013 | fixed | Round 3, 83efb2a9b: b6e47deb1. `intersect_with`'s `combine` now returns `V`, so it cannot refuse. The doc says an entry only one map holds is dropped, and that `combine` must be idempotent because a shared subtree is kept without calling it (facts/persistent.rs:118-126). The callers in `join` (hull, and `()`) are idempotent. `intersect_keeps_common_keys_combined_and_drops_the_rest` covers shared subtrees, maps built apart, and disjoint keys. |

## New findings (disposition pass 1)

These are defects in the fix round's new code: the Location redesign, the
`Facts` sharing, the literal equality and the reworded docs.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-008 | medium | Definedness still copies the whole fact set to add one fact. `relate` clones `Facts` for each outcome of a literal comparison (facts.rs:587), `present(p)` clones it for its true outcome (facts.rs:645), and `join` builds another copy per connective (facts.rs:333-352). Each copy stays on the pending stack until its branch is walked (facts.rs:786-789). The 128-level cap is gone, so this now has no bound. Take a chain of nested guards on distinct subjects: `let b1 = x + 1 in if b1 > 0 then (let b2 = x + 2 in if b2 > 0 then … else 0) else 0`. Level `i` makes two copies of size `i`, and every `else` copy is live at once, so time and live heap grow as n²/2 fact entries. Neither `s3.nodes` nor `s3.work_units` charges them. At S1's defaults (100,000 tokens, 50,000 CST nodes) such a chain reaches about 5,000 to 7,000 levels, which is on the order of 10⁷ live entries (gigabytes). On main the cap held it to 128 levels. FND-002's named sites are fixed, and the AC-1 forms add no facts, so no test reaches this. I traced this by reading; I did not run it. Fix: share facts across levels, either with a persistent map or with one mutable `Facts` and a scoped undo log that the pending steps restore. Or charge each copied entry to the work budget. Add a deep guard-chain case to the deep tests. | qsl-semantics/src/check/facts.rs:587; qsl-semantics/src/check/facts.rs:645; qsl-semantics/src/check/facts.rs:333-352; qsl-semantics/src/check/facts.rs:780-790 |
| FND-009 | low | `SpanMemo` keys each resolved span by the raw address of a `Link` (region.rs:40-62), and the public `CheckedGraph::memoized_regions` accepts any `&Location`. Its doc says the closure "borrows the graph, which keeps the locations it was asked about alive" (region.rs:286-290). That holds only for locations the graph owns. A caller that resolves a temporary `Location`, drops it, and then resolves another one allocated at the same address gets the first one's span. The key also ignores `origin`, which is a `pub` field, so the same chain under another origin would read another tree's span. Today's only callers pass graph-owned locations (emit.rs `Recorded<'g>`), so no wrong region is produced yet. Fix: keep a clone of each memoized link in the memo, so no address is reused while it lives, and key on origin plus address. Or narrow `memoized_regions` to the package's own occurrences. | qsl-semantics/src/check/region.rs:33-67; qsl-semantics/src/check/region.rs:286-306; quire-semantic-value/src/location.rs:195-214 |
| FND-010 | low | `CheckedLiteral::eq` is still not reflexive for a population. `same_encoding`'s leaf arm requires `compare_keys == Some(Equal)` (ir.rs:424-428), and `compare_keys` has no key for a population pair (quire-exact/src/key.rs:147-161, TC-297). So a population literal, or a composite, option or collection holding one, is unequal to its clone. The typer builds none today, and lowering refuses one (lowering.rs:3791), but the field is `pub`. The doc also says it compares "exactly the members a replay deciding element writes" (ir.rs:374-378). It does not: decimals and texts compare by key, so `1.0` equals `1.00` and texts of different types with one payload are equal, while the encoder writes the scale and the text type. Fix: compare populations by id, and describe the actual relation (key equality at keyed leaves, width and bits for floats, ids for populations). The relation does not need to track the encoder, and nothing depends on that, so no test tying the two together is needed. | qsl-semantics/src/check/ir.rs:356-432; quire-exact/src/key.rs:147-161 |
| FND-011 | low | Two docs are stale after this PR. refusal.rs:584-586 lists "a body nested past the preimage depth bound" as a `NodePreimage` cause, which is the `NodeKeyRefusal::TooDeep` this PR deleted. family.rs:733-735 (reworded for FND-005) says `Typer`'s "node and work charges, made at every node it enters" bound a call's nesting. `Typer` charges only nodes (check.rs:1137-1160). The work budget is charged once per declaration (FR-258-AC-4). Fix: delete the depth clause, and say "node charge". | qsl-semantics/src/check/refusal.rs:584-586; qsl-semantics/src/check/family.rs:733-735 |
| FND-012 | low | FR-258 Behavior 3 says typing, definedness facts, obligation collection and expression lowering "SHALL run in arena order or on the walker toolkit (FR-356)". The fix round moved the five walks FND-001 listed. The older machines still run their own explicit stacks: the `Typer` frame machine (typing.rs:452-456), `Definedness::walk` (facts.rs:711-727), lowering's `LowerFrame` and type-frame stacks (lowering.rs:1694, 2790) and the claims walk (claims.rs:495-496). Each is stack-safe, so nothing misbehaves. But the requirement, and FR-356's argument of one Kani-verified traversal, do not hold for them. ADR-030 D-4.3 covers only walks "that recurse", so the ADR and FR-258 also disagree. Fix: move these machines onto `quire_walk`, or narrow Behavior 3 to what this slice delivers and route the rest to the slice ticket. | spec/functional/FR-258-check-and-lower-expressions-at-any-depth.md:51-57; qsl-semantics/src/check/check/typing.rs:452-456; qsl-semantics/src/check/facts.rs:711-727; qsl-semantics/src/check/lowering.rs:2790; qsl-semantics/src/check/claims.rs:495-496 |

## New findings (disposition pass 2)

Round 2, reviewed at aed7b2df09e20f15f7437c04d392ea99e2200695.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-013 | low | `PersistentMap::intersect_with` takes `combine: &dyn Fn(&V, &V) -> Option<V>` and documents that "an entry `combine` refuses is dropped" (facts/persistent.rs:118-130). `merge` keeps a subtree the two maps share without calling `combine` (persistent.rs:210-212), so a refused entry in a shared subtree survives. The coder's own test hit this: the `\|_, _\| None` assertion failed in qsl-483-r2-test1/test2.log and was deleted in aed7b2df0. Every production caller returns `Some` (facts.rs:347, 356, 359), so nothing is wrong today. But a future caller that refuses would keep a fact on the joined path, and definedness could then discharge an obligation it should refuse. The `Option` is unused generality. Fix: make `combine` return `V`, or call it on shared subtrees when it can refuse. Either way, drop the "refuses is dropped" sentence. | qsl-semantics/src/check/facts/persistent.rs:118-130; qsl-semantics/src/check/facts/persistent.rs:200-212; qsl-semantics/src/check/facts.rs:340-361 |
