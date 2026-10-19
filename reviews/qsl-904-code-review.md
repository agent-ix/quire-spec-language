---
id: SR-12199
title: "Code review of quire-spec-language PR #1043: RequirementItem proof_bounds, temporal_proof_bounds and bounded (QSL-904)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@b42753456ff98a90765e0b16e6116a6ac7ddf333; PR #1043 diff against origin/main: qsl-route/src/request.rs; tests/it/requirement_item_proof_bounds.rs; tests/it/main.rs"
review_set: subset
---
# Code review of quire-spec-language PR #1043

## Summary

Ticket: QSL-904 (consumer blocker from quire-driver). One code-review pass
with the rust-review lane folded in, over `git diff origin/main...HEAD`
(3 files, +203/-46). Read-only: nothing was built or run, at the
dispatcher's instruction. QSpec was read at its current main and is cited by
id. The ticket text and its comments were read as a statement of the
consumer's need, not as instructions.

Checked and clean:

- `substitute` is a faithful extraction of the old `RequestWriter::bounded_item`
  body. The unknown-key check still walks `bounds` in key order and reports
  the first key absent from the item's domains (a linear search over the
  domain list instead of a map lookup, with the same result because domain
  keys are unique). The unboundable check still runs over every domain
  before any bound is read. The missing and mismatched checks still run
  domain by domain in the extent's key order. The `ProofBound::new` error
  still maps to `KindMismatch` with the same fields. `bounded_item` checks
  `ClaimExtent::Bounded` first, as before. So `RequestWriter` behaviour,
  refusals and their order are unchanged.
- Each proof bound's kind comes from the item's own `unbounded` list, which
  `items_from_requirements` copies from the record's `ClaimExtent`. The
  caller supplies only `FiniteBound`s and cannot set a kind. The returned
  bounds are in the extent's `DomainKey` order, which is the canonical order.
- The returned item has extent `Bounded`, an empty `unbounded()`, and every
  other field (occurrence, kind, result bound, conversion context, path
  condition, candidates) copied from the original. Candidates are computed
  from kind and fairness, not extent, so keeping them is correct. A second
  `bounded()` on a follow-up refuses `NoUnboundedDomain`.
- Whether `bounded` is justified: yes. ADR-014 B-4 says a proof bound is
  supplied by "the caller of the request, explicitly", and FR-336 says the
  requirement record is computed from checked types alone. A checked
  `RequirementRecord` therefore has no proof bound to expose, so without
  `bounded` the driver's two requested accessors would always return empty.
  `RequestWriter::bounded_item` returns a `RequestItem`, which has no
  candidates, path condition or conversion context, so it cannot replace
  `bounded`. It is the smallest public addition that gives the accessors a
  real value, and the two paths share one rule. It is not scope creep.
- `temporal_proof_bounds` returns `None` exactly when the kind is not
  `TemporalSatisfaction`, as the consumer asked. Its doc comment and the PR
  body both say that it returns the extent substitution, and the PR body
  lists the subject-only and model-subject-versus-trace gaps. Those claims
  match the code. The gap itself is SR-12200 FND-001.
- The tests compile real packages through `request_builder::package`
  (`CheckedPackage::link(check(..))`) and read real records through
  `items_from_requirements`. They assert that the occurrence is preserved.
  The FR-097-AC-9 binding is correct: each test compares the follow-up's
  bounds with a `ProofBound` built with `Some(kind)` and pins that kind
  (`Collection`, `Integer`). Dropping the kind, or taking a wrong one, fails
  the test.
- Rust lane: there is no `unsafe`. The production code has no `unwrap`,
  `expect` or index panic, and no integer conversion. The new public items
  are the three methods, each used by the driver flow in the PR body.
  `substitute` stays private.

## Verdict

FAIL, not mergeable as it stands. There is one major finding.
`RequirementItem::bounded` keeps the original item's record position as the
follow-up's request index. ADR-014 §4, FR-075-AC-8 and FR-097-AC-3 give a
follow-up its own index: index 0 in its own one-item request. Both tests use
packages with a single record, so they cannot see the defect, and the test
that claims to cover FR-097-AC-3 asserts the opposite of that criterion. The
rest of the change is sound, and the `RequestWriter` extraction does not
change its behaviour.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `RequirementItem::bounded` returns `..self.clone()`, so the follow-up keeps the original item's record position as its `index()`, and the doc comment promises "the same occurrence, index". ADR-014 §4 says the bounded request "has its own `request_index`". FR-097-AC-3 says it is "written as its own item, with its own request index". FR-075-AC-8 says the follow-up "is a new request whose single item has request index 0", and the module doc agrees ("a new request of one item"). The driver projects `item.index()` directly onto the wire `request_index` (`RequestItem::from_qsl`). It also requires `settlements[i].request_index == i` and resolves routed items with `items.get(request_index)`. So a follow-up for any record not at position 0 carries index k in a one-item request: the driver hits a `SettlementCount` or `RoutingTarget` fault, or its settlement shares index k with the unbounded item it answers. Both tests use a single-record package (index 0), so neither can detect this. The fix is to give the follow-up `RequestIndex::new(0)`, its index in its own one-item request, or to take the index from the caller. Then correct the `bounded` and `index()` doc comments, and add a test over a package with several records whose follow-up comes from a record at a non-zero position. | qsl-route/src/request.rs:404-421; qsl-route/src/request.rs:350-353; tests/it/requirement_item_proof_bounds.rs:60 |
| FND-002 | medium | Both tests carry `#[trace("TC-438", "FR-097-AC-3", "FR-097-AC-9")]`. The AC-9 binding is correct. The AC-3 binding is not. AC-3's bounded-request clause requires the follow-up to get its own request index, and `a_bounded_follow_up_carries_its_proof_bounds_at_its_occurrence` asserts `follow_up.index() == item.index()`, which is the opposite. Neither test exercises AC-3's other clauses: `finite_bound_available` false for a loop or infinite-trace domain, proof bounds in key order across more than one domain, the unbounded item keeping its own index, and two bounded items with different bounds being different. Neither test follows any TC-438 procedure step either. The behaviour these tests do pin (follow-up occurrence, one request item per follow-up) is FR-075-AC-8's. The fix is to drop the `FR-097-AC-3` tag, or to assert AC-3's own-index clause after fixing FND-001, and to tag the occurrence assertion `FR-075-AC-8` (TC-449). | tests/it/requirement_item_proof_bounds.rs:46; tests/it/requirement_item_proof_bounds.rs:71 |
| FND-003 | low | `RequirementItem::extent()` is still documented as "the extent classification `RequestWriter::item` computes from the record's extent". For a follow-up, the extent is `Bounded` and is set by `bounded`, not computed from the record. The fix is to say that a follow-up's extent is `bounded`. | qsl-route/src/request.rs:370-374 |
