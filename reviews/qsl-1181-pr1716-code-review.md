---
id: SR-1716001
title: "Code review of quire-spec-language PR #1716: the model-abstract refinement class catalog (QSL-1181 slice 2)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@e39172d9e; PR #1716 diff against qsl-1181a-typed-refinement (12 files), against FR-375-AC-23, FR-375-AC-27, FR-322-AC-130, FR-322-AC-133, FR-464-AC-18 and FR-432 (QSpec main 308596b0)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-specification/FR-375
    type: reviews
---
# Code review of quire-spec-language PR #1716

## Summary

Ticket: QSL-1181 (slice 2 of 6).

**Scope.** `git diff origin/qsl-1181a-typed-refinement...origin/qsl-1181b-model-catalog` is one commit (e39172d9e) and exactly 12 files. Nothing from outside the stack is in the diff.

**What was checked.**

- **Effective operation classes.** `OperationTable::effective` resolves each name through `resolve`, so an ambiguous name gives no class.
  - The class node is the owner's declaration key.
  - The binder record is the receiver (the owner's reference type), then the parameters, then the result.
  - This matches FR-432's `attempt` row: "the reference type of the type the attempt's operation name names ... even when a supertype declares the operation".
- **Catalog rebuild after the protocol clauses are checked** (`catalog_concrete_protocols`, mod.rs). It runs only for model-abstract slots with no selected protocol.
  - Clause selection uses the same rule as lowering's `owner_model_object`: the clause's `over` type is an object type of the concrete model. So the catalog and the clause's `owner_model` edge agree, as FR-375 and FR-464-AC-18 require.
  - Probe: a clause over a third model `T` contributes no classes. The refinement has no catalog and still emits.
- **Lowering to node keys** (`class_catalog_keys`).
  - An operation class lowers to the owner's model node, a finish class to the obligation node, a channel class to the channel node, and `activate` to the clause node.
  - A class with no keyed node gives `None`. `v2_empty_model_body` then turns false and emission refuses with `RefinementNotEmitted`. It is a loud refusal, not a silent drop.
- **Emission and identity.** The `class_catalog` sits in the typed body, so it is in the node-key preimage. Every node and binder key is in `dependencies`, sorted and deduplicated.
- **`activate` binder record** (FR-375-AC-27). Probes:
  - `on each (t: Boolean)` gives `binder_types` `[Boolean]`.
  - `on each (k: Impl::Counter)` gives one reference type.
  - The package emits `activate Local` with one binder.
- **Tests.**
  - `cargo test -p qsl-semantics -p qsl-package --all-features --no-fail-fast` with QSPEC_DIR at 308596b0:
    - qsl-semantics: 800 lib tests and 1327 integration tests pass.
    - qsl-package lib: 351 pass. Two fail, `conformance_c14_source_map_lookup_over_qspec_positive_fixtures` and `conformance_i2_read_over_qspec_checked_package_v2_fixtures`. Both refuse `positive-model-tuple-union.json` at `/semantic_graph/nodes/0/owner`, which is QSpec fixture drift against the IR reader. They are not in this diff and the coordinator reports them failing on main (QSL-1190). The third failure was my own probe before I fixed its source order.
  - The ignored read-back test, run with `--include-ignored`, refuses `invalid_semantic_graph` at `/semantic_graph/nodes/19/body/class_catalog/concrete/1` (`Local::Done`). So `concrete/0` (`Lifecycle::tick`) passes IR's rebuild, as the PR claims.
  - The probes are in `qsl1181-review/probe_1716.diff`. They were not committed.

**The `#[ignore]` on `a_model_refinement_class_catalog_reads_back` is acceptable.** Asserting IR's current refusal would be worse, for four reasons:

- **It asserts the right thing.** The test states FR-375's behaviour: the catalog reads back. IR's refusal is an IR defect. I re-measured it: IR-874's comments assign B2 (empty-abstract fallthrough) and B3 (clause segment taken from the declaration path) to luna:IR:codex3, and its AC-325 closeout is blocked by QSL-1181.
- **The inverted test would test a bug.** A test asserting `invalid_semantic_graph` at `concrete/1` would make an IR bug the expected behaviour. It would also change twice: when B3 lands the refusal moves to the abstract side, and only when B2 lands does it read back.
- **It would not fail when IR-874 lands.** QSL takes `quire-contract-model` from the IR branch `qsl-1181-refinement-exports`, not from main. An IR fix reaches QSL only when someone deliberately moves that dependency, and that person can remove the ignore at the same time.
- **The one real risk is tracking.** An ignored test does not run in `make ci`, so the un-ignore needs a named owner. See SR-1716002 FND-002.

## Verdict

**No soundness or wrong-result defect.** The catalog content, its order, the node keys, the dependencies and the identity are correct for the constructs this slice can lower. There are three low findings.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `ProtocolClasses::of` builds `trigger` with `binders.get(1).map(binder_record).unwrap_or_default()`. A checked `on each` clause with no slot-1 binder would therefore give an `activate` class with an empty binder record. That is the same wrong record this PR removes, and it would happen silently. The case is a broken checker invariant, so raise an `InternalFault` as the other catalog invariants do, not a default. | qsl-semantics/src/check/refinement/internal_list.rs:597-607 |
| FND-002 | low | The `activate` binder record is attributed to IR. `ProtocolClasses.trigger`'s doc says "as IR's FR-464 reader rebuilds them", and the PR says "FR-432 does not say this; the gap is filed on STD-493". At QSpec 308596b0 the rule is stated three times: FR-432's binder-record table has the row "`activate` / as `event`, over the declared type of the clause's `on each` trigger"; FR-375 "Checked class catalog" says it; and FR-375-AC-27 tests it. Cite FR-432 and FR-375-AC-27 in the doc. Withdraw the STD-493 ask "state the binder record of `activate` in FR-432's table", since it is already met. | qsl-semantics/src/check/refinement/internal_list.rs:450-453; QSpec FR-432 "The binder record of an abstract node", FR-375-AC-27; STD-493 |
| FND-003 | low | `ProtocolMembers.nodes` is documented as "its step nodes by declaration index", but the lowering inserts only the `finish` node. That is correct today, because only sequence controls lower and they have no class. But a later slice that lowers a classed control (attempt, event, send, receive, parallel) must also insert it, or every refinement over such a clause will refuse `RefinementNotEmitted`. Make the doc state the current content (the finish node only), so the next slice does not rely on a map that is not filled. | qsl-semantics/src/check/lowering/protocol.rs:662-670, 1388-1398 |

## Dispositions

Round 1, fix commit 32cbba757.

Gates (locked-build, jobs 4, debug info off, `--all-features`, QSPEC_DIR 308596b0):
- qsl-semantics `refinement` integration tests: 93/93.
- qsl-package lib `admission_corpus`: 10 pass, 1 ignored (IR-874).
- clippy on qsl-semantics and qsl-package with `--tests -D warnings`: exit 0.
- `cargo fmt --check`: exit 0.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `ProtocolClasses` now carries `activates` (the clause is `on each`) and a separate `trigger: Option<Vec<ValueType>>`, built from `binders.get(1)` with no default. When `activates` is set and the trigger is `None`, `catalog_classes` returns `InternalFault` `checked-each-clause-has-a-trigger-binder`. The abstract `activate` spelling check now reads `activates`. |
| FND-002 | fixed | The `trigger` doc now cites FR-432's binder-record table ("`activate`: as `event`, over the declared type of the clause's `on each` trigger") and FR-375-AC-27. STD-493 comment b1db40e4 withdraws the earlier `activate` ask and names QSpec's three statements of the rule. |
| FND-003 | fixed | The `ProtocolMembers` doc now says that only the `finish` node is in `nodes` today, and that a slice which lowers a classed control must insert its node there. |
