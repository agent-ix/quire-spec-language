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

Reviewed the branch diff against `origin/main` using the code-review and rust-review checklists. The change removes the specified shell script, its Make and CI calls, and the test fixtures that named those calls. No production Rust behavior changes.

## Findings

No findings in the changed files. The remaining `check-no-committed-binaries` fixture still sets up its executable seam, and the CI ordering tests still name the retained gates. A repository search found no live invocation of `check-index-completeness.sh` after the removal. The script's FR index assertion is intentionally removed under STD-226.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The standalone FR index assertion is removed with the requested ceremony script; this is the explicit STD-226 scope. No replacement is proposed. | tools/check-index-completeness.sh, Makefile |

## Verification

The targeted xtask test groups for spec validation, feature lane ordering, and conformance passed. `cargo fmt --all -- --check` and `git diff --check` passed. The full `make ci` run passed spec validation, conformance vectors, default Clippy and most default-feature tests, then failed in unchanged `qsl-route/tests/it/route_registry.rs`: the installed cargo-deny 0.20.2 rejects that test's `--config` argument position. This failure is outside the changed files; the aggregate gate did not complete.

## Verdict

The scoped deletion is ready for review. Full-gate completion remains blocked by the recorded cargo-deny CLI mismatch.
