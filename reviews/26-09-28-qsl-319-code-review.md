---
id: SR-782
title: "QSL-319 code review (with rust-review lane) of PR 520"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@00e411c88065195926487cb5b0b8e1f4343fb598; qsl-semantics/src/check/lowering.rs; qsl-semantics/src/check/claims.rs; qsl-package/src/emit/tests.rs; qsl-semantics/src/check/lowering/state.rs (unchanged, context); qsl-semantics/src/check/family.rs (unchanged, context); spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md (unchanged); spec/test-cases/TC-416-emission-writes-the-nodes-check-lowered.md (unchanged); spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md (unchanged)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: reviews
---
## Summary

Ticket: QSL-319. PR: quire-spec-language#520 at 00e411c8, base origin/main.
Methods: code-review with the rust-review lane folded in.

What the PR changes: `parameter()` in `lowering.rs` now records a
`value`/`parameter` node's own occurrence as `expression`, not `anchor`
(QSpec FR-341-AC-10 at e56756f: "every occurrence has role `expression`").
`key_claims` in `claims.rs` skips the `(node, location)` pairs that the
`binders` map names as a binder's own declaration site. The emit round-trip
test now asserts role `expression` on every parameter occurrence.

Checks the dispatcher asked for, with results:

1. Exclusion scope. All three callers of `parameter()` (the binders in
   `bind()` for let, query and fold; function parameters; state-clause
   parameters) pass the same `location` to `parameter()` and to
   `record_binder`, with the same key. Draft keys are resolved by the same
   `resolve` for occurrences and for `draft_binders` in `settle`. So
   `own_declarations` is exactly the set of parameter declaration occurrences.
   The only other occurrence it can drop is a read of the same parameter at
   the same location. That happens only when a function or clause body root
   is a bare parameter read. That occurrence is never a scalar site, and a
   body root is never a guard, so nothing changes.
2. The regression is real. Mutation M1: I restored main's `claims.rs` at head.
   `a_sibling_may_reuse_a_binder_name` failed with "Forall beside itself
   checks: ... UnkeyableRequirements", and
   `keying_each_content_once_keeps_every_key_and_preimage` failed. The cause
   is `outermost()`: a binder form used as a guard (`(forall ..) and ..`) has
   a parameter declaration occurrence at the same location that it does not
   name. The scalar-site and narrow lookups filter by operation role, so
   they were never affected. Only guard resolution was.
3. Precision is pinned. Mutation M2: I widened the exclusion to every
   occurrence of a binder's parameter node, ignoring location. Six
   qsl-semantics tests failed, among them
   `tc_160_guards_carry_their_outcome_outermost_first` and three region
   tests. So tests do check the location half of the key, and SR-758
   FND-006's gap (a compare no test depends on) does not recur here. All
   binder forms go through the same three `parameter()` callers, so no
   binder form is missed. No other node kind records an occurrence at its
   enclosing form's location under role `expression`: `typed_application`,
   `value_node` and the Local read each record at their own checked node's
   location.
4. No other role drifted. The diff touches one `record` call. The remaining
   `Anchor` recorders are the operation-anchor bindings in `state.rs`
   (clause and attempt), which are unchanged.
5. The new assertion goes through the real path: parse, forms, assemble,
   check, link, `emit_checked`, the wire `source_map`. Mutation M3: I
   reverted only the role change. The test failed on parameter `b`'s
   ordinal-0 entry with role `anchor`.
6. Gates. I re-ran `make ci` at 00e411c8 myself and it exited 0. The log has
   93 `test result: ok` lines and 0 `FAILED`. qsl-semantics lib passed 410 and
   412 (default and all features), and qsl-package lib passed 92. The three
   named tests pass in every run.

SR-758 FND-006 history: that finding showed that the `operation_role ==
Narrow` compare in `key_claims`' narrowed-bound lookup is not caught by any
test when mutated. This PR does not touch that lookup, and its own exclusion
is caught by tests (M1, M2).

## Verdict

Changes requested. The code change is correct and tightly scoped, the
claims-keying fix is exactly as narrow as claimed and mutation-pinned, and
`make ci` is green. The blocker is spec, not code. QSL's own FR-093 (binder
bullet and AC-9), TC-416 and ADR-013 QC-24 still say a binder site is an
`anchor` occurrence, so after this PR the repo's spec contradicts its code
(FND-001). FND-002 is a new ordinal-order divergence from FR-093's stated
rule. It can be settled in the same spec edit.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | QSL's own spec still says a binder's site is an `anchor` occurrence of its parameter node. That appears in the FR-093 binder bullet, in FR-093-AC-9 ("`a`'s parameter node has an `anchor` occurrence over `a: Boolean`"), in TC-416's expected results, and in ADR-013 QC-24 ("whose binder site is an `anchor` occurrence"). After this PR the code emits `expression`, per QSpec FR-341-AC-10. No test asserts the old role, so nothing fails, but the spec now contradicts the code. Fix in this PR: reword those four places to `expression`, citing QSpec FR-341-AC-10. | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:79-81; spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:690; spec/test-cases/TC-416-emission-writes-the-nodes-check-lowered.md:72; spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md:1124 |
| FND-002 | medium | FR-093 says ordinals per (node id, role) follow (source document, region start, region end). The declaration occurrence now shares its `expression` ordinal sequence with the reads, and it is always recorded first. Measured at head on the spine fixture: parameter `a` has ordinal 0 at 322..329 (the body root, the declaration) and ordinal 1 at 322..323 (the read `a`). By FR-093's rule the read should be ordinal 0. Before the PR the two were in separate `anchor` and `expression` sequences, so this did not happen. It happens whenever a binder's region starts at the same byte as the first read of its parameter. A reader that derives ordinals by FR-093's rule would get different occurrence keys. Fix: align FR-093's ordinal sentence with `OccurrenceMap::record` (insertion order, ADR-013 O-07) in the FND-001 edit, or order ordinals by region. | qsl-semantics/src/check/lowering.rs:2169; qsl-semantics/src/check/family.rs:1220-1235; spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:73-76 |

## New findings (disposition pass 1)

Reviewed at agent-ix/quire-spec-language@9dd43d1d278dfd0f9b8782b4a7cdb6d7f82f2af3 (fix commit 9dd43d1d).

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | medium | The FND-002 fix adds: "A binder's declaration occurrence is recorded before its reads, so it is always ordinal 0 of that sequence." That is false whenever two binders share one parameter node. Parameter nodes are content-addressed on (name, level, type) (`insert_node` dedups, and `parameter()` records one `expression` occurrence per call), so AC-7's own package shares P1 between `both`'s `a` and `nb`'s `a`. Measured at 9dd43d1d with a scratch probe over `both` + `nb`: P1 has ordinal 0 at 234..241 (`both`'s binder site), 1 at 234..235 (read `a`), 2 at 292..305 (`nb`'s binder site), 3 at 297..298 (read `a`). `nb`'s binder site is ordinal 2, not 0. The insertion-order rule itself is correct. Only the "always ordinal 0" claim is wrong. Fix: say that each binder's site is recorded before that binder's own reads, and that binders sharing a parameter node share one ordinal sequence in lowering order, so only the first binder's site is ordinal 0. | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:74-78; qsl-semantics/src/check/lowering.rs:2147-2170; qsl-semantics/src/check/lowering.rs:1407-1428 |
| FND-004 | low | Pre-existing, not caused by this PR, but in text the fix round rewrote. The FR-093 binder bullet ("a binder's site (a parameter's `name: T`, ...)"), FR-093-AC-9 ("an `expression` occurrence over `a: Boolean`") and TC-416 step 4 ("over its binder site") place a function parameter's own occurrence at `a: Boolean`. The code records it at `function.location`, which the emitted source map puts at the function body. The same probe gives 234..241 = `a and b` for `both`'s binder site, and 292..305 = `both(a, true)` for `nb`'s. Round 1 measured the same on the spine fixture (322..329, the body root). The role is now right, but the region the spec names is not the one emitted. Fix: either say the site of a function parameter is the function body's region, or record the occurrence at the parameter's own span. Deferring this to a follow-up ticket is fine, since it predates QSL-319. | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:81-84; spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:693; spec/test-cases/TC-416-emission-writes-the-nodes-check-lowered.md:71-74; qsl-semantics/src/check/lowering.rs:2479 |

## Dispositions

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 9dd43d1d: all four spots now say `expression` and cite QSpec FR-341-AC-10 (FR-093 binder bullet and AC-9, TC-416 step 4, ADR-013 QC-24). No residual `anchor` occurrence wording for parameter nodes remains in FR-092, FR-093 or TC-416. The remaining `anchor` occurrences in FR-105/FR-114 belong to `operation_anchor` nodes, which is correct. |
| FND-002 | fixed | 9dd43d1d: FR-093 now says ordinals follow insertion order, which matches `OccurrenceMap::record` (family.rs:1220-1235: ordinal = length of the (identity, role) vec before push). The extra "always ordinal 0" sentence added with it is wrong for shared parameter nodes. That is recorded separately as FND-003. |
