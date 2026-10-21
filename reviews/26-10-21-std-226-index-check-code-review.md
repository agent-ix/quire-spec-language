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

Reviewed the rebased branch diff against `origin/main` using the code-review and rust-review checklists. The change removes the specified shell script, its Make and CI calls, and the test fixtures that named those calls. No production Rust behavior changes.

## Findings

No findings in the changed files. The remaining `check-no-committed-binaries` fixture still sets up its executable seam, and the CI ordering tests still name the retained gates. A repository search found no live invocation of `check-index-completeness.sh` after the removal. The script's FR index assertion is intentionally removed under STD-226.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verification

The targeted xtask test groups for spec validation, feature lane ordering, and conformance passed. `cargo fmt --all -- --check` and `git diff --check` passed. The first full `make ci` run stopped in `qsl-route/tests/it/route_registry.rs` because local cargo-deny 0.20.2 rejected its `--config` argument position; CI pins cargo-deny 0.19.8, and the specific test passed with that version. The pinned-version full rerun reached `ci-docs`, where the then-current branch linked public documentation to private `same_values` in `qsl-eval/src/simulation/model/probabilistic.rs:145`. QSL #1159 removed that link on main and is included in this rebased branch. The aggregate gate still requires a clean rerun.

## Verdict

**PASS** for the scoped deletion review. Full gate verification is pending on the rebased branch.
