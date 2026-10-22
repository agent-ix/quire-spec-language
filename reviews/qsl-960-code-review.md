---
id: SR-12307
title: "Code review of quire-spec-language PR #1197: StageLimits::with_named at the replay root (QSL-960)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@ae46f1869a9416e06a7ae44519c7e306a902b4c3; qsl-replay/src/request.rs; qsl-replay/src/lib.rs; qsl-replay/tests/stage_limit_named_root_only.rs"
review_set: subset
---
# Code review of quire-spec-language PR #1197

## Summary

Ticket: QSL-960. Covers `StageLimits::with_named`, `StageLimitNameRefusal`,
the request decoder's move onto `with_named`, the root re-export, and the
root-only test. Includes the rust-review lane.

Measured at ae46f1869:

- The new test passes (4/4).
- `cargo run --locked -p xtask -- string-edge` reports no unmarked string
  comparison. The name lookup goes through `SettingName::from_name`, which
  already exists.
- `make arch-lint-api-surface-qsl` passes for T12-B through T12-F and
  FR-100-AC-8. T12-A is skipped under `--qsl-only`.
- `ReplayRequest` decode now folds over `with_named`. The two paths share
  one name check and cannot drift. The first refused entry still wins, in
  BTreeMap order, the same as before.
- A refusal consumes `self` and returns only `StageLimitNameRefusal { name }`.
  No partial value escapes.
- With no entries, `StageLimits::default()` is unchanged.
- Probe (throwaway, deleted): generated sources of 2,000,167 and 2,600,071
  bytes of real declarations (enum, tuple, record, union and function per
  group). Using root APIs only, each compiles through `compile_admitted`
  when `s1.input_bytes`, `s1.tokens` and `s1.nodes` are raised by name and
  `ReplayLimits::with_input_bytes` is raised. Release build: 9.1 s and
  12.7 s.

## Verdict

Mergeable. No high findings. FND-001 is the rustdoc trap for consumers.
Fix it in this PR if it is cheap.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The rustdoc of `with_named` does not say that `s1.input_bytes` above `ReplayLimits.input_bytes` refuses as `AdmittedResource::AboveReader`, so the reader limit must be raised with it. It also does not say that a source of real declarations reaches the `s1.tokens` (100000) and `s1.nodes` (50000) defaults at about 267 KB and 300 KB, far below 1 MiB. Only FR-399 and a test comment state the pairing. A consumer such as CG reads the rustdoc, and with only `s1.input_bytes` raised it gets AboveReader, then a token-limit refusal. | qsl-replay/src/request.rs:91-95 |
| FND-002 | low | The tests match `AdmittedResource::StageLimit(_)` and do not check which setting fired. AC-1's refusal would still pass if `s1.tokens` fired instead of `s1.input_bytes`. | qsl-replay/tests/stage_limit_named_root_only.rs:48-51, 107-114 |
