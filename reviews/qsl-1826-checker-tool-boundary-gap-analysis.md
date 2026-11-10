---
id: SR-20261109172912000131
title: "QSL-1826 checker tool-boundary scoped semantic gap review"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; QSL-1826; PR #2642; Cargo.toml; README.md; tools/qsl-probability-check; replay/analyze test fixture includes; FR-287-AC-1; FR-282-AC-1"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-287
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-282
    type: references
---

## Summary

Manual scoped trace, ownership and semantic review of executable placement and fresh-process preservation. No scoped gap found.

## Verdict

**PASS for the scoped gap-analysis review.** No findings. The lead still owns the final affected aggregate and exact-head merge; this is not program or whole-repository qualification.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Review Evidence

Frozen head: ef0cc79d32a0d087512a7ece871364e085c556b0. Ticket: QSL-1826; downstream unblock: QSL-1813 / PR #2641.

The complete former qsl-probability-check source, README, integration tests and fixtures are moved within this repository. A Git base-versus-head comparison independently verified every moved non-manifest blob byte-for-byte; the old directory has no tracked entry. The package and implicitly named binary remain qsl-probability-check. Cargo dependency paths point to the same workspace packages; versions, features, dependency kinds, protocol, checker algorithms and test bodies are unchanged. The only executable change is its placement under tools/, exactly the existing FR-287 developer-tool rule. There is no guard allowlist, exemption, new format, compatibility alias or duplicated checker.

The root workspace lists the new member path once. Source fixture includes in replay and analyze are inside existing test code and refer to the same moved fixture blobs. All shipped references outside historical reviews were searched; no stale root-relative checker path remains. The README link resolves. The qualified checker remains in qsl-replay; moving its frontend does not place filesystem, process-stream or argument handling in a core library. The normal dependency tree contains core QSL libraries and their existing support dependencies, and excludes qsl-analyze, qsl-inspect, the driver and qsl-command; the analyzer remains a dev dependency only. Existing CARGO_BIN_EXE process invocations still name the same binary.

No assurance profile applicable to this move was found under spec/. No CI lane, linter, test, dependency-direction rule, wire schema, admission limit, cancellation behavior, trace tag or public API was weakened or changed. No new unsafe code or semantic conversion is introduced. Existing test tags survive byte-for-byte.

Independent confined checks passed with the required thermal flock, nice/ionice, Rust 1.98.1, jobs=2, RUST_TEST_THREADS=2 and this worktree's own target: formatting for checker/replay/analyze; all-target/all-feature Clippy for those three packages; all 25 checker integration tests; and the normal dependency tree. Logs are /tmp/qsl1826-tool-boundary-review/check-{fmt,lint,process,tree}.log and checks.time. The author's separate record also shows the unchanged no_binary guard passing 1/1 as a direct compiled binary after cargo/rustc were quiet, with no parent Cargo or flock (/tmp/qsl1826-tool-boundary-no-binary.log). This review did not launch a metadata subprocess. No full aggregate, cargo-deny, broad workspace runtime or large unchanged core test was repeated here; the final affected gate remains the lead's checkpoint.

## Coverage

Plan completion: not assessed

The dispatch explicitly authorized a manual scoped semantic gap pass. FR-287-AC-1 is directly examined through the unchanged architecture guard and actual Cargo target/member placement. FR-282-AC-1 is examined for preservation of the existing fresh-process boundary, production checker and normal dependency closure, with independently executed success and mutation/refusal process tests. Other EN-5 proof profiles, numeric algorithms and their full semantic acceptance coverage are unchanged and are not newly qualified by this review.

No new behavior or test requires a new owning requirement; existing FR-287 and FR-282 own the placement and process guarantees. Existing trace attributes remain unchanged. The actual guard can fail (and did on the old root placement); it passes because the package is now under the already allowed tools directory, not because an assertion or whitelist was relaxed. The process tests launch the real built binary and check accepted exact values and refused mutated inputs rather than substitute the checker.

Whole-repository computed matrix and runtime coverage were NOT RUN or claimed. The dispatch prohibits repeating earlier repository-wide probes that took 521/295 seconds and returned no JSON. This scoped artifact makes no repository-wide gap PASS, QSL-1813 completion, matrix completion or driver qualification claim. The exact model identifier is unavailable in the harness metadata and is recorded as unavailable.

## Examined Units

```yaml
{
  "scope": [
    {
      "id": "FR-287-AC-1",
      "path": "spec/functional/FR-287-reach-qsl-through-the-driver-cli.md",
      "role": "examined",
      "excerpt": "`cargo metadata --no-deps --format-version 1` in the QSL repository lists a `bin` target only in the developer-tool packages (`xtask`, the packages whose manifest directory is under `tools/`, `qsl-string-edge` and `qsl-bench`); the root package `quire-spec-language` has no `bin` target and no `default_run`; and no workspace member outside those packages lists one of them as a normal or build dependency."
    },
    {
      "id": "FR-282-AC-1",
      "path": "spec/functional/FR-282-check-analyze-certificates-in-the-qualified-core.md",
      "role": "examined",
      "excerpt": "The EN-5 certificate from FR-281-AC-1, written to bytes by `analyze` and passed with the byte provision and the subject binding to the EN-5 checker in a fresh process that links only qualified-core crates, is accepted: the process re-checks the provision's sources into a `CheckedPackage`, admits the `ModelSystem` and runs the checked-subject form."
    },
    {
      "id": "FR-284-AC-1",
      "path": "spec/functional/FR-284-keep-the-qualified-core-separable-by-crate.md",
      "role": "context_only",
      "excerpt": "The direction check passes over the QSL workspace."
    }
  ],
  "bindings": [
    {
      "test_id": "only_developer_tool_packages_list_a_bin_target",
      "ac_id": "FR-287-AC-1",
      "trace": "correct"
    },
    {
      "test_id": "an_actual_en5_proof_is_checked_in_a_fresh_process_using_its_source",
      "ac_id": "FR-282-AC-1",
      "trace": "correct"
    },
    {
      "test_id": "a_fresh_checker_refuses_altered_values_objectives_identities_ranks_and_provisions",
      "ac_id": "FR-282-AC-1",
      "trace": "correct"
    }
  ]
}
```
