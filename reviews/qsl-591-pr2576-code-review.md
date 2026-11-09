---
id: SR-164205
title: QSL-591 code-review PR2576 execution seam
type: SpecReview
analysis: code-review
review_set: subset
scope: "agent-ix/quire-spec-language; QSL-591 PR2576 @883d75ea5 qsl-eval/src/execution.rs, qsl-replay/src/spine/call.rs, qsl-replay/src/spine/lifecycle.rs, qsl-replay/src/spine/native_result.rs, qsl-replay/src/outcome.rs, qsl-replay/src/limits.rs, src/command.rs, src/command/output.rs, xtask/src/qualification/adapters.rs and test call-site updates"
relationships:
- type: references
  target: ix://agent-ix/quire-spec-language/FR-279
- type: references
  target: ix://agent-ix/quire-spec-language/FR-294
- type: references
  target: ix://agent-ix/quire-spec-language/FR-296
---

## Summary

Ticket: QSL-591. Code review with the rust-review lane folded in (rust-style loaded: API design, implementation, safety and testing rules). Scope is `git diff origin/main...HEAD` at 883d75ea5, all 26 files. The PR moves S6a `execute` out of `qsl-replay` lifecycle into a new `qsl_eval::execution` module that defines `ExecutionBackend`, `PreparedEntries`, the object-safe `SuppliedBackend` with its blanket impl, `SuppliedBackends`, `PrepareMeter`, the `Interpreter` backend and `execute`. It routes the spine `run` through it, adds `Call::backend` and `run_with_backends`, and maps `CallFailure::Unsupported` and `CallFailure::Limit` to the new `RunRefusal::Unsupported` and `RunRefusal::PrepareLimit` instead of a fault. Builds were not rerun, per the brief: the coder reported full qsl-eval/qsl-replay tests passing on this head and clippy -D warnings clean.

## Verdict

CONDITIONAL. The seam's shape matches FR-294/FR-296 for the function half. `PrepareMeter` checks before it charges and cannot overflow. The not-supplied refusal comes before any stage. The interpreter keeps the FR-277 single-run cancel contract. Two medium defects need fixing: the prepare-setting mapping (FND-001) and prepare limits that never reach the spine (FND-002). The clause half and FR-296-AC-4's `Default` clause are known to be out of scope. They are not findings.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `call_failure` maps every `LimitExceeded` setting other than `PrepareCodeBytes` to `PrepareSetting::CompileWorkUnits` through a `_` arm. A backend that returns `PrepareFailure::Limit` naming any other setting is reported as a compile-work-units limit, which is a valid but wrong value. Match the two prepare settings explicitly and return `CallFailure::Fault` for any other. The `u64::try_from(actual).unwrap_or(u64::MAX)` on the next line also saturates the counter silently and should fault in the same way. | qsl-eval/src/execution.rs:450-459 |
| FND-002 | medium | `run_reading` passes `PrepareLimits::default()` to `execute`, and `run_with_backends` takes only `SpineLimits`. `CallerLimits.prepare`, which the FR-255 settings operation sets from `prepare.compile_work_units` and `prepare.code_bytes`, therefore never reaches a supplied backend run through the spine. FR-279 requires prepare to run under the caller's `limits.prepare`. | qsl-replay/src/spine/call.rs:622-626 |
| FND-003 | low | The native-result refusal record for `RunRefusal::Unsupported` writes `backend`, `construct` and `construct_name`, but drops the `Untranslatable` cause's location. FR-294 requires the cause to name the construct's location. | qsl-replay/src/spine/native_result.rs:2381-2391 |
| FND-004 | low | `ExecuteLimits` and `CallLimits` add a `supplied: SuppliedLimits` field that the FR-279 and FR-294 signatures do not have. FR-279 states `ExecuteLimits::default()`, which cannot exist because `ScalarLimits` has no `Default`. Code and spec disagree. Align the FR-279/FR-294 signatures with the code; the FR-277 field is needed. | qsl-eval/src/execution.rs:123-130 |
| FND-005 | low | `run` repeats `run_with_backends`'s body except for the backends argument. `run` could delegate with `&SuppliedBackends::default()`. | qsl-replay/src/spine/call.rs:497-550 |

## Coverage

Examined every hunk of the 26 changed files. `PrepareMeter` checks the bound before it adds the charge, so `work` and `bytes` stay within their u64 bounds. `SuppliedBackends::member` matches `ExecutionBackendChoice` exhaustively. `execute` prepares exactly one entry and looks up `unsupported` before it calls. `Interpreter::prepare` polls the cancel handle and charges nothing. The `#[allow(clippy::too_many_arguments, reason)]` matches the file's existing convention. No unsafe code, no new panics on caller data, and no new public item without documentation.

```yaml
scope:
- id: FR-279
  path: spec/functional/FR-279-execute-a-checked-entry-as-a-library-operation.md
  role: examined
  excerpt: "QSL SHALL expose `execute`, which calls a checked function or evaluates a checked clause through the execution-backend seam (FR-294, ADR-029 OP-1)."
- id: FR-294
  path: spec/functional/FR-294-run-every-execution-backend-behind-one-seam.md
  role: examined
  excerpt: "One trait in layer 5 (`qsl-eval`, module `execution`) holds every execution backend (ADR-029 EB-1)"
- id: FR-296
  path: spec/functional/FR-296-select-the-execution-backend-in-the-request.md
  role: examined
  excerpt: "The caller picks the execution backend in the `execute` request (ADR-029 EB-8). The choice is one of the seam's closed set: `interpreter`, `aot` or `jit`, with `interpreter` as the default."
```

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | 9d3a3f2af: `call_failure` matches both prepare settings explicitly and returns `CallFailure::Fault` for any other setting or for a counter past u64. The new test `a_limit_that_is_not_a_prepare_limit_is_a_fault` covers both cases. |
| FND-002 | fixed | 9d3a3f2af: `Call.prepare` (default `PrepareLimits::DEFAULT`) reaches `execute` through `run_reading`. `the_calls_prepare_limits_reach_the_supplied_backend` shows a JIT stand-in refusing one below each count and completing at the count, through `run_with_backends`. The CLI and xtask still use `Call::new`'s default limits. That has no observable effect, because neither can supply an aot/jit backend (the driver owns `--engine`) and the interpreter charges nothing. It is outside this PR's scope and is not a defect. |
| FND-003 | fixed | 9d3a3f2af: the Unsupported refusal record writes a `location` field (`CallLocationWire` as JSON), and the new native-result test checks it. |
| FND-004 | deferred | The coordinator is routing this to the spec lane: the FR-279/FR-294 signatures should gain `supplied` and drop `ExecuteLimits::default()`. No code change is needed. |
| FND-005 | fixed | 9d3a3f2af: `run` delegates to `run_with_backends` with `&SuppliedBackends::default()`. |
