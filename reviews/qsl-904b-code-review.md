---
id: SR-12259
title: "Code review of quire-spec-language PR #1146: caller-supplied temporal subject proof bounds (QSL-904)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@d70693be10d7e284808eea618c782d717b586c48; PR #1146 diff against origin/main: qsl-route/src/request.rs; tests/it/requirement_item_proof_bounds.rs"
review_set: subset
---
# Code review of quire-spec-language PR #1146

## Summary

Ticket: QSL-904. This is the second PR for the ticket. PR #1043 landed
`proof_bounds()`, `bounded()` and `temporal_proof_bounds()`. In #1043,
`temporal_proof_bounds()` returned the extent substitution for every
`temporal-satisfaction` item. This PR adds
`RequirementItem::with_subject_proof_bounds` and a separate
`subject_proof_bounds` field. `temporal_proof_bounds()` now reads that field.
It is `None` for a non-temporal item and `Some(&[])` until a caller supplies
subject bounds. Two refusals are new: `BoundRefusal::NotTemporal` and
`DuplicateSubjectDomain`. Both map to `invalid_runtime_input`/`invalid-value`.
The Rust lane (rust-review) is folded into this file.

What was checked against QSpec FR-331 at quire-specification main 8998f318:

- Behaviour change. A bounded temporal follow-up's extent bounds no longer
  appear in `temporal_proof_bounds()`. This matches FR-331: a subject's
  `proof_bounds` "are independent of `extent`", and a subject bound "moves no
  entry into `extent.bounds`". The reverse also holds now, and the updated
  test pins `Some(&[])` on a bounded follow-up. It also matches the driver's
  stated contract on QSL-904, which asked for "exact QSL-owned temporal
  subject proof_bounds", `None` for a non-temporal item and `Some(&[])` for a
  temporal one with none.
- Nothing calls the old behaviour. On this head, `temporal_proof_bounds` and
  `with_subject_proof_bounds` are called only from
  `tests/it/requirement_item_proof_bounds.rs`. quire-driver `origin/main`
  (5e23b70) and its `ir-672*` / `ir672*` branches do not call
  `temporal_proof_bounds()`. Their only hit is a test name in
  `quire-cli/src/backend_provider.rs`.
- `bounded()` builds from `..self.clone()`, so a follow-up keeps any subject
  bounds already supplied and never writes into them. `with_subject_proof_bounds`
  leaves index, occurrence, extent, `unbounded` and `proof_bounds` unchanged,
  and the new test asserts each of these.
- The `PR #1043` interim doc caveat was removed. Its subject-only half is now
  implemented. Its model-subject half is not, and the gap analysis (SR-12260)
  records that.
- Rust lane: there is no `unsafe`. The production code has no `unwrap`,
  `expect` or panic. `windows(2)` indexes only `pair[0]` and `pair[1]`, which
  always exist. The sort is stable and keyed on `DomainKey`'s `Ord`, the same
  order `proof_bounds()` uses. There is no integer conversion. The method
  returns `Result<Self, _>` from `&self`, the same shape as `bounded()`.
- Focused run at the reviewed head: `cargo test -p quire-spec-language --test
  it requirement_item_proof_bounds` passed 5 of 5. The brief's `-p qsl-route`
  form selects qsl-route's own `it` binary and runs 0 tests.

## Verdict

PASS with no major finding. The extent and the subject are now independent,
as FR-331 requires, and no caller depends on the old mirroring. One medium
finding should be fixed in this PR. The method accepts any `ProofBound` the
caller builds. That includes one with no kind (a harness `Variants` bound)
and one with a non-population domain or kind. So the subject bound's kind is
the caller's, not QSL's, and QSL can emit a subject bound that FR-331 refuses
and FR-097-AC-9 forbids. One low finding: the catalog code of the new
refusals is not tested.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `with_subject_proof_bounds` accepts any `ProofBound`, so the caller chooses the kind and the domain. This admits two things it should not. First, a bound built with `kind: None` and a `FiniteBound::Variants` bound, which `ProofBound::new` allows for the FR-358 harness. FR-331 says `variants` "is a harness bound, never a B-4 substitution kind" and that "an absent `kind` refuse[s] before consumption". FR-097-AC-9 says every proof bound the request writer emits carries its domain kind, "never none". Second, a `DomainKey::Node` domain with a `collection`, `integer` or `recursive` kind. FR-331 says a subject's `proof_bounds` are "the proof-bound substitutions for the subject's own populations", and that the kind is "written by QSL from its own classification". `bounded()` already does this right: it takes `&BTreeMap<DomainKey, FiniteBound>` and QSL writes the kind. The fix is to give `with_subject_proof_bounds` the same shape. Take `&BTreeMap<DomainKey, FiniteBound>`, accept only a `DomainKey::Population` key with a `cardinality` bound, write `DomainKind::Population`, and refuse anything else with a typed `BoundRefusal`. A `BTreeMap` also removes the need for `DuplicateSubjectDomain`. Add a test for each refused case. | qsl-route/src/request.rs:432-460 |
| FND-002 | low | `bounded_refuses_a_bounded_item_and_an_infinite_trace_item` asserts the `invalid_runtime_input`/`invalid-value` catalog code for its refusals. The new test checks `NotTemporal` and `DuplicateSubjectDomain` by variant only, so the two new `CatalogCoded` arms are not tested. The fix is to assert `catalog_code()` on both refusals in `subject_proof_bounds_are_independent_of_the_extent`. | tests/it/requirement_item_proof_bounds.rs:207-219; qsl-route/src/request.rs:204-205 |
