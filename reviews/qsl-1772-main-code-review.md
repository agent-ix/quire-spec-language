---
id: SR-20261105101070
title: QSL-1772 main compile code review
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; QSL-1772; PR2458; Exact projection admission, bare/source reader caches, temporal evidence and native report APIs"
review_set: subset
---

## Summary

PR quire-spec-language#2458 frozen e85eaee6e. Rust review folded into code-review. Reviewed the existing ticket worktree only, without source edits, builds, clones, commits or pushes.

The branch-main IR update adopts the merged shared-budget API. Source and bare admission use the same caller NativeJoinBudget; owner ExactTypeProjection results are retained for subsequent evaluator reads, and no IR budget is added to the generic evaluator trait. Invalid identity witnesses stay deferred; native budget denial returns at admission. Optional diagnostic forms are matched with as_deref. All in-repository callers and compile-only fixtures were examined. No CI changes, unsafe code, local dependency pin, shim or mirrored projection representation.

## Examined Scope

- FR-531 native join budget: one finite caller budget per pass; retained identities and projections; witness defects keep their stage; typed native exhaustion.
- FR-531-AC-21: exact native work/storage and one-short refusals for supplied bare admission.
- FR-531-AC-23: cumulative caller ledger; subsequent world and projection reads do not recharge.
- FR-531-AC-24: finite read limits and typed view/evidence denial.
- FR-531-AC-25: helpers take the caller budget; no hidden replacement allowance.
- FR-532-AC-12: capture names and value types match the in-process clause on all three routes; unsupported capture types refuse at the subject stage.
- Changed files: Cargo.lock, qsl-package bare/bare_slot/bare_world/checked_v2/checked_views/subject/reader_protocol_projection/lib; qsl-eval temporal evidence package and caller tests; native report owner/codec; root and emitter test migrations; FR-531 explanatory text.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Public capture_value_types trusts an unbound caller-built projection map: a genuine owner projection from another package, or another node, can be inserted under this view's initializer key and returned as this package's declared capture type without owner revalidation. | qsl-package/src/checked_views/subject.rs:466-483 |

## Verdict

CONDITIONAL; not mergeable until FND-001 is resolved. Preserve provenance by projecting through this admitted package with the explicit caller budget at admission, or by consuming projections through an existing package-bound owner API. A raw caller map is not package-bound merely because its value type has private fields.

## Coverage

Static review of production changes, callers and tests. Historical coder compile and all-target Clippy logs pass; focused tests were still compiling at review time. No independent build or final gate run. The retained BTreeMap bookkeeping overhead is not charged by NativeJoinBudget; this review makes no complete cache-allocation storage claim. Full graph preprojection changes admission work to a bounded whole-pass charge; native denials precede later projection reads as stated by the edited spec. No further concrete defect was established in that ordering.
