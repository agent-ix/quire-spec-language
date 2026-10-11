---
id: SR-4960
title: "QSL-669 code and Rust review of PR #720"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@4e16d437ec13a70de86aee6e8f5ea21076a7e3a4; base bdf259f008c6577f4235b06ab1d87c32db83099a; qsl-semantics/src/model/domain_package.rs; qsl-semantics/src/model/key.rs; qsl-semantics/src/model/normalize.rs; qsl-semantics/src/model/population.rs; qsl-semantics/tests/it/identity_golden_vectors.rs; qsl-semantics/tests/it/model_dispatch.rs; qsl-semantics/tests/it/model_normalization.rs; reviews/qsl-669-code-review.md; reviews/qsl-669-gap-analysis.md; QSpec FR-150-AC-1, FR-150-AC-4, FR-150-AC-6, FR-150-AC-7, FR-150-AC-8; NFR-012"
review_set: subset
---
## Summary

Ticket: QSL-669. PR #720 at `4e16d437ec13a70de86aee6e8f5ea21076a7e3a4`, compared with `bdf259f008c6577f4235b06ab1d87c32db83099a`. Disposition round 2 rechecked the code/Rust review and all changes since `8eb14aec90fabb189ccfdf485d87099db326e68a`, especially the new ordering of inherited-fact charges relative to `work_step()` and the adjusted bounded-work regression test.

The new vacant-entry path admits the qualify-fact charge before inserting the inherited declaration preimage; every inherited path then admits its inherit-fact charge before incrementing the tracked work step and constructing the path inputs/fact. Existing entries skip a duplicate qualify fact but still charge each inherit fact first. A denied charge returns before the tracked work step and before adding the denied fact; no partial view escapes. The revised narrow/wide fixture test verifies the admitted work remains `(2, 2)` when a later qualify-fact charge is denied. This agrees with NFR-012's rule that normalization stop before the work priced by a denied charge. No new code/Rust defect or test-oracle issue was found in this delta. The prior FND-001 for this artifact is the no-findings placeholder; there are no code-review findings to disposition.

The CI log `/home/peter/dev/worktrees/logs/batches/qsl720-final-ci-4e16d43.log` was still in `cargo test --locked --workspace` when inspected. Its footer was absent, so no gate result is claimed here. `git diff --check bdf259f008c6577f4235b06ab1d87c32db83099a...HEAD` passed. No build or test was run by this reviewer.

Scope examined includes the closed identity wire shapes and their exact vectors; normalization provenance, fact-charge order, resource-work accounting, and changed tests; and QSpec FR-150-AC-1, AC-4, AC-6, AC-7, AC-8 plus NFR-012's charge-before-work requirement.

## Verdict

No new code/Rust findings in disposition round 2. Review verdict is clean for the inspected changes; mergeability remains pending the exact-head gate footer.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
