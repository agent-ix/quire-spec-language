---
id: SR-16323
title: "Code review of quire-spec-language PR #1394 hybrid emission and read-back"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; QSL-1017 PR #1394 against qsl-1017-pr2; qsl-forms/src/{hybrid_declaration,syntax}.rs; qsl-package/src/{checked_v2,emit,lib}.rs and checked_v2/hybrid.rs and emit/tests/hybrid.rs; qsl-semantics/src/check/{assemble,check,hybrid,lowering,mod,node_key}.rs and value/member.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-447
    type: reviews
---
# Code review of PR #1394

## Summary

Ticket: QSL-1017. Reviewed the PR's own Rust diff, its call sites and tests, FR-447-AC-5, AC-12 through AC-16, and QSpec FR-481's hybrid node contract. The code review includes the rust-review lane. The final aggregate gate belongs to the code lead.

## Verdict

**FAIL.** A `fair S` claim whose transitions all target one mode emits the wire member reserved by QSpec FR-481 for `fair S::T`. The package reader can admit this member because it denotes the same transition set, so successful admission does not establish fidelity to the source form. The edit to FR-447 attempts to permit this encoding, but the cited QSpec contract still distinguishes the two spellings.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `hybrid_fairness` chooses a mode-node/target member whenever all transitions have one target, including source `fair S`. QSpec FR-481 requires model-node/mode for `fair S` and mode-node/target only for `fair S::T`. Because `HybridFairness` retains only transitions, the checker cannot tell these forms apart. Preserve the distinction in the checked form or obtain and land a QSpec contract change before claiming FR-481 conformance. | qsl-semantics/src/check/lowering/hybrid.rs:433; qsl-semantics/src/check/hybrid.rs:348; QSpec FR-481:186-192 |

## Coverage

Examined FR-447-AC-5, AC-12, AC-13, AC-14, AC-15, AC-16 and the FR-447 S4 emission clause; QSpec FR-481 hybrid model nodes, hybrid claims, fairness members and reader order. Inspected the changed emitter, read-back, checked-form, wire-member, and tagged-test paths for the introduced behavior, including API exposure, malformed-input handling, typed boundaries, resource limit, and trace tags. No CI workflow file changed. No full aggregate gate was run in this review lane.
