---
id: SR-7818
title: "Code and Rust review of QSL-720 formatter test/CI changes"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@3db652c6edefe9b9f01519525243a67e94a377f6; Makefile, tests/it/formatter_identity.rs, xtask/src/ci_feature_lane_tests.rs; NFR-013-AC-3, FR-003-AC-9, TC-915"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/NFR-013
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-003
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-915
    type: reviews
---

## Summary

Reviewed the frozen Makefile, Rust integration test, and xtask routing-oracle changes against the repository Rust conventions and the applicable NFR/test text. The 100,000-depth workload, 20,000-volume bound, named `make ci` target, exact-name root skips, and 256 KiB small-stack regression hook are present; the feature-lane argv changes conflict with NFR-013-AC-3.

## Verdict

FAIL pending resolution of FND-001. No builds or tests were run as part of this review; the focused test receipt was supplied by the requester, and the full v2 gate remains queued.

## Scope

- NFR-013 statement and AC-3 examined; AC-1/AC-2 read as context for the routing contract.
- FR-003-AC-9 examined for formatter identity test intent.
- TC-915 examined for stated scope and the separation of process-routing evidence from real Cargo qualification.
- All three changed files reviewed at the exact PR head; Rust test changes reviewed under repository AGENTS.md/CLAUDE.md guidance and the Rust review checklist.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | The feature-lane receipts append three test-harness `--skip` filters, changing the Cargo argument vectors that NFR-013-AC-3 explicitly requires to remain unchanged. The new aggregate target preserves execution in default features, but does not make the feature-lane commands conform to the stated exact argv contract. Amend the owning requirement or preserve the required vectors before approval. | Makefile:202,210; NFR-013-AC-3 | wrong-requirement |

## Examined requirements and bindings

- NFR-013-AC-3: “Feature routing preserves the nine Cargo argument vectors and their order above, default versus all-feature selection, default-before-all reachability from `ci`, and the existing non-feature and clean artifact/feature policy.”
- NFR-013-AC-1: “Without explicit lane overrides, every command selects its lane's distinct child of the owned parent root, including repeated default-to-all and all-to-default switches using the same root.”
- NFR-013-AC-2: “Every feature command receives the selected caller-root child or explicit lane override as literal path data under the environment and GNU Make assignment grammar above, including spaces, double quotes, backticks and dollar forms, without executing path contents.”
- FR-003-AC-9: “For each complete-V1 fixture unit in the repository's tests that checks, checking its formatted source through the S1 to S4 spine yields a checked package identity equal to the original's, and formatting the formatted source again yields identical bytes.”
- TC-915 examined; its description explicitly says the recorder establishes routing only, while real Cargo/compiler and feature-sensitive CLI execution establish compilation and runtime behavior.
- Binding checked: `deep_formatter_target_runs_each_full_size_case_and_is_in_ci` claims NFR-013-AC-1/AC-3; trace present. Existing formatter cases claim FR-003-AC-9; trace present.

## Verification notes

Static checks confirm 100,000 levels remain in the three deep generators and the 100,000-term replay fixture; the 1,000-library chain remains unchanged; only the lifecycle declaration volume changes from 200,000 to 20,000. `test-deep-format` invokes the three retained large cases and appears in `CI_CHECKS`. Both root feature receipts skip those cases by their exact test names. The new 100,000-level sum control formats on a 256 KiB thread stack. No explicit fault-injection run was performed.

