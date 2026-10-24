---
id: SR-13624
title: "STD-226 review: remove the spec index completeness script"
type: SpecReview
analysis: code-review
scope: "STD-226; tools/check-index-completeness.sh, Makefile, .github/workflows/ci.yml, and xtask tests"
review_set: subset
---
# STD-226 index check removal review

## Summary

Reviewed the branch diff after rebasing onto `origin/main` at c16d4be02 using the code-review and rust-review checklists. The change removes the specified shell script, its Make and CI calls, and the test fixtures that named those calls. No production Rust behavior changes.

## Findings

No findings in the changed files. The remaining `check-no-committed-binaries` fixture still sets up its executable seam, and the CI ordering tests still name the retained gates. A repository search found no live invocation of `check-index-completeness.sh` after the removal. The script's FR index assertion is intentionally removed under STD-226.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verification

The targeted xtask test groups for spec validation, feature lane ordering, and conformance passed. `cargo fmt --all -- --check` and `git diff --check` passed. An early full run used local cargo-deny 0.20.2, which rejected a test's `--config` argument position; CI pins 0.19.8, and that test passed with the pinned version. A later run reached `ci-docs`, where the then-current branch linked public documentation to private `same_values`; QSL #1159 removed that link and is included here. The final-head run stopped in the C-14 conformance test because `QSPEC_DIR` pointed to the shared QSpec checkout at a286b81, 38 commits behind main. The same targeted test passed against a separate QSpec origin/main worktree at 014f0ce. The aggregate gate still requires a clean rerun with that current QSpec checkout.

## Verdict

**PASS** for the scoped deletion review. Full gate verification is pending on the rebased branch.
