---
id: SR-1951001
title: "Code review of PR #1951 (QSL-1315): FR-322 anchor ordinal, foreign unit owner, nominal regions"
type: SpecReview
analysis: code-review
date: 2026-10-31
scope: "agent-ix/quire-spec-language; PR #1951 git diff origin/main...c93bba59f (qsl-semantics/src/check/{family.rs,lowering.rs,lowering/model.rs,assemble.rs,assemble/units.rs,mod.rs,time_units.rs,temporal_clause.rs,temporal_clause/timed.rs}, qsl-package/src/emit.rs, three new tests); ticket QSL-1315; rust-review lane folded in"
review_set: subset
relationships:
- target: ix://agent-ix/quire-spec-language/FR-322
  type: reviews
---
# SR-1951001: code review of PR #1951 (QSL-1315)

## Summary

Ticket: QSL-1315. The PR has three changes. (1) A dimension-equivalence `anchor` occurrence now takes the declaration's index as its ordinal, through a new `OccurrenceMap::record_at` over `Vec<Option<S>>`. (2) A dimension that an admitted domain package declares, built on the lowering path, now carries `ModelOwner {identity, node}`, no `declaration`, and a `generated` occurrence at the selecting `model` alias. (3) The timed path places a source nominal's occurrence at `TimeNominalNode::declaration_span`, which is the declared name. The rust-review lane is folded in.

I checked these against QSpec FR-322 at c5deb110. Items 1 and 3 are correct. Item 2 is correct in the in-memory semantic graph, but the change never reaches the wire. The emitter still drops every nominal node that is not source-owned.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| OccurrenceMap record / record_at / iter / resolve / first_origin / order_by | qsl-semantics/src/check/family.rs:1414-1664 | examined |
| unit_nodes, foreign_nominal_owner, dimension_equivalence_nodes | qsl-semantics/src/check/lowering/model.rs:2224-2409 | examined |
| TimeUnits::new spans, TimeNominalNode::declaration_span | qsl-semantics/src/check/time_units.rs:58-180; qsl-semantics/src/check/temporal_clause/timed.rs:90-119 | examined |
| timed nominal source_map region | qsl-package/src/emit.rs:2880-2913 | examined |
| owner_is_locked / omissions | qsl-package/src/emit.rs:1217-1240 | examined |
| package_dimension key, NodeOwner::declaration_key_at | qsl-semantics/src/check/assemble/units.rs:302-335; qsl-semantics/src/value/semantic_node.rs:94-112 | examined |
| equivalence source order | qsl-semantics/src/check/assemble/equivalence.rs:529-675 | context_only |
| FR-322 "Nominal nodes", declaration member exception, owner member, equivalence paragraph, occurrence roles | QSpec spec/objects/interfaces/FR-322-checked-package-artifact.md:840-910, 1574-1623, 1800-1808 | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Item 2 does not reach the wire. `owner_is_locked` keeps a nominal node only when its payload owner is the checked source. A lowering-path dimension owned by a domain package is therefore still dropped, and the omission spreads to every node that names it. A probe built from `dimension Dur = Config::Time; unit dsec : Dur = rational(1, 1); function f using v(d: Quantity<dsec>) ...` over the config-version package with dimensions had no `dimension`/`unit` node and no `f` in the emitted wire, at both the base and the head. No emit or read-back test covers a ModelOwner dimension from the lowering path; the new test checks only the in-memory graph. FR-322 says a model-owned nominal dimension carries its `ModelOwner` on the wire (AC-51). The function is silently dropped. | qsl-package/src/emit.rs:1222-1240; qsl-semantics/src/check/lowering/model.rs:2288-2324 |
| FND-002 | medium | The model-owned dimension's `node_id.path` is `[<FCD identity>, <name>]`: `package_dimension` keys it with `declaration_key_at([name])` under a `NodeOwner::Model`, which prefixes `subject.node`. FR-322 states that a nominal node with a `ModelOwner` and no `declaration` has `node_id.path` `[node_tag, semantic_form, <FCD identity>]` (`["scalar_type", "dimension", "ix://acme/rpc/dimension/Time"]`, AC-81). Now that the PR removes the declaration, that rule applies to this node, and the key does not follow it. The key computation itself predates the PR. | qsl-semantics/src/check/assemble/units.rs:310-328; qsl-semantics/src/value/semantic_node.rs:94-112 |
| FND-003 | medium | `record_at` stores each node's anchor at index `ordinal` in a dense `Vec<Option<S>>`. Every equivalence has its own node key, so equivalence `i` allocates `i + 1` slots, and N equivalences allocate about N²/2 `Option<Location>` slots. With 30,000 equivalences in a 1 MB source, that is about 450M slots, tens of GB. A sparse per-role store, for example `BTreeMap<u64, S>` or `(ordinal, S)` pairs, removes the quadratic allocation and the gap state. | qsl-semantics/src/check/family.rs:1519-1541; qsl-semantics/src/check/lowering/model.rs:2402-2405 |
| FND-004 | low | `record_at` silently keeps the old span when the ordinal is already occupied. It also records nothing when the ordinal does not fit a `usize`. Either way it returns an `Origin` that resolves to another span, or to none. The only caller cannot reach either case today, because `insert_nominal` returns `inserted = false` on a repeat. Still, a broken invariant becomes a wrong origin, not a fault or a debug assertion. | qsl-semantics/src/check/family.rs:1530-1541 |
| FND-005 | low | In `unit_nodes`, a key that is neither source-owned nor matched by `foreign_nominal_owner` becomes a node with a `declaration` and no `owner`. That happens when `ModelOwner::new` fails, a model has no alias, or the key is not found. FR-322 says a node with a `declaration` always carries a `SourceOwner`, and a `DefinitionOwner` is never a node owner, so this node never conforms. The unit table holds only source nodes and imported model base dimensions (units.rs:720-735), so the "definition-owned unit" branch named in the comment has no producer. It masks a lowering invariant failure where it should refuse. | qsl-semantics/src/check/lowering/model.rs:2283-2301 |
| FND-006 | low | `TimeNominalNode` now holds two independent options, `qualified_declaration` and `declaration_span`. Each must be `Some` exactly when `owner` is `Source`, but nothing ties them together, so an inconsistent node can be built. The emitter checks only the first. The doc calls the span "the span of the node's own declaration", but it is the declared name's span. One `Option<(Vec<String>, Span)>`, or a source-declaration struct, would make the bad state unrepresentable. | qsl-semantics/src/check/temporal_clause/timed.rs:96-118; qsl-package/src/emit.rs:2848-2900 |

## Verdict

**CHANGES REQUESTED** (FND-001 to FND-003).

Item 1 is correct. FR-322 (lines 1598-1601) gives the anchor "the ordinal of the declaration among the equivalence declarations of its source in source order, with the declaration's region". The equivalence list is sorted by source span and keeps every declaration when assembly succeeds, so `index` is that ordinal. The test checks the whole declaration text, which matches "the declaration's region".

Gaps in `Vec<Option<S>>` never reach emission or readers:
- `role_occurrences` skips `None` with `filter_map`, so `iter`, `iter_role`, `iter_any_order` and `Debug` see only real occurrences, each with its true ordinal.
- `resolve` returns `None` for a gap, and `first_origin` skips gaps.
- `order_by` sorts `None` first, but it runs only on population `Type` occurrences, which never use `record_at`.
- No qsl-package reader requires anchor ordinals to be dense.
- A collision at one ordinal cannot happen with the current caller.

Item 3: FR-322 does not say whether a source nominal's `declaration` occurrence covers the name or the whole declaration form. The only explicit region rules are the equivalence anchor ("the declaration's region") and a model declaration node ("the region of the declaring construct"). The name span is therefore admissible. It is also what the lowering path records for the same node (assemble.rs:3745-3765, `name.span`), and the emitter skips a timed nominal already written from the graph. So both producers give one region. **Ruling: the name span conforms.**

Rust-review: clippy `-D warnings` over qsl-semantics and qsl-package `--all-targets` with test-support is clean, and `cargo fmt --check` is clean. The `as u64` span casts follow the file's existing convention, and the PR adds no unsafe code or panics.
