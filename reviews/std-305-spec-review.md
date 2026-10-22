---
id: SR-15750
title: "Spec review of quire-spec-language PR #1239: STD-305, the ordering-mismatch weak access shape"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@1698295c0c710bd07391c2a33ac8e2e075206db5; PR #1239 (git diff origin/main...1698295c0): spec/functional/FR-220-check-access-orderings-fences-and-access-classification.md, spec/test-cases/TC-665-access-orderings-and-classification.md, spec/decisions/ADR-025-weak-memory-models-for-parallel.md (MV-1); read for context: ADR-025 MA-1 to MA-4, ADR-018 V-8, FR-219-AC-4, ADR-027 (no shape list), agent-ix/quire-specification main 5ee0071fc FR-435, FR-439"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-220
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-665
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-025
    type: reviews
---
# SR-15750: Spec review of quire-spec-language PR #1239: STD-305

## Summary

Ticket: STD-305. Sub-analyses folded in: integrity, EARS, failure-domain. The head 1698295c0 is behind main 43f4e2086 but merges cleanly. `make check-spec-validation` on the merge result exits 0 with no diagnostic on FR-220, TC-665 or ADR-025. No other open PR touches FR-220, TC-665 or ADR-025; #1262 edits ADR-027, not ADR-025. The files do not overlap #1238 or #1241.

What checks out:

- The contradicting sentence ("explored with the semantics of the ordering it carries") is deleted.
- ADR-025 MA-3 already sends the instantiated mismatch to EN-1's pre-check, so the one-line MV-1 edit agrees with it.
- The five AC-14 wire vectors are exactly MA-3's mismatches: a load with `release`/`acq_rel`, a store with `acquire`/`acq_rel`, an RMW with none.
- `ordering: null` occurs only in the RMW case.
- "Settles before any state is explored" matches FR-219-AC-4's `JoinPolicy` wording.
- Admitting the mismatch under `sc` follows from "orderings change no behaviour under `sc`".

## Verdict

**NOT MERGEABLE.** The precedence vector in AC-14 is refused at S3 before the pre-check runs (FND-001). FND-002 and FND-003 need a planner decision or a corrected follow-up statement.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | [major] The precedence vector "an attempt that writes two shared fields with `acquire`" never reaches the pre-check. FR-220 Admitted orderings (ADR-025 MA-3): "S3 SHALL refuse `acquire` or `acq_rel` on an attempt that reads nothing". A write-only attempt with `acquire` therefore refuses `ill_typed`/`operator-ineligible` at S3, and TC-665 step 13's expected `multi-location` cannot occur. There is a second problem: MA-2 and QSpec FR-435 define a class only over at most one shared location. A two-shared-location attempt has no load, store or RMW class, so "both a `MultiLocation` shape and an ordering mismatch" is undefined as written. Fix: make the vector an attempt that reads a branch-local register and writes two shared fields with `acquire`. S3 admits it, because the attempt reads something. On instantiation its shared part is written without being read, which is a store-like mismatch. State the rule as "the pre-check tests the one-location shape first; an attempt with a `MultiLocation` shape is not tested for an ordering mismatch". | spec/functional/FR-220-check-access-orderings-fences-and-access-classification.md:124,153-154,195; spec/test-cases/TC-665-access-orderings-and-classification.md:41-44,66 |
| FND-002 | medium | [minor] The ordering rule and the closed wire enum `"access": "load"\|"store"\|"rmw"` leave out an instantiated `Local` attempt that carries an ordering. MA-3 and the QSpec FR-435 table say "a local attempt takes none". S3 admits an ordering on any attempt that touches a data place, and on instantiation that attempt can turn out to touch only branch-local registers (the AC-8/AC-13 register shape). The spec does not say whether that attempt is admitted or settles a mismatch. Planner decision: admit it (ordering on a branch-local access is unobservable), or add `"local"` to `access` with a vector. | spec/functional/FR-220-check-access-orderings-fences-and-access-classification.md:136-155 |
| FND-003 | low | [minor] The QSpec follow-up is understated. QSpec FR-435 (main 5ee0071fc, class table paragraph, and FR-435-AC-2) says an ordering the class does not take "refuses at check time `ill_typed`/`operator-ineligible`". That is a check-time refusal, not a pre-check V-8. So QSpec needs FR-435's refusal moved to the pre-check outcome as well as a third FR-439 shape. FR-220 References already says QSpec FR-439 and FR-435 "mirror" the shape, which is not true on QSpec main. Reword it as the follow-up QSpec owes (Linear STD-305), and name the FR-435 change in the PR body. | spec/functional/FR-220-check-access-orderings-fences-and-access-classification.md:211-214 |
