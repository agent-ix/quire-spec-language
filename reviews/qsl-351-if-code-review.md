---
id: SR-1304
title: "Code review of PR #635 (QSL-351: InternalFault re-export, EmptyBackendIdentity code and category)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@0ce20489b71bbe0e2f13581e68118b08c5a43bb7; qsl-replay/src/lib.rs, qsl-replay/tests/terminal_record_facade.rs, qsl-route/src/lib.rs"
review_set: subset
---

## Summary

Ticket: QSL-351. PR agent-ix/quire-spec-language#635, two commits on main
`c8f0c2818`. Code review with the rust-review lane over
`git diff origin/main...HEAD`.

Checked:

- **`InternalFault` re-export.** `pub use qsl_foundation::{Code,
  InternalFault, RequestIndex}` at the `qsl-replay` root. It has a real
  consumer: `CallSiteRefusal::Fault(InternalFault)` and
  `ReplayRefusal::Fault(InternalFault)` are public variants, and CG's open
  branch `code/qsl-c8f0c28-declined-map` (`tests/it/terminal_map.rs:376`,
  `:930`) records "QSL's `InternalFault` cannot be built here", leaving the
  fault halves of its FR-029-AC-10 and FR-030-AC-10 untested. With the
  re-export CG can build both fault refusals through `qsl_replay` alone.
- **`EmptyBackendIdentity::code` / `category`.** `const fn`s returning
  `Code::InvalidIdentifier` and `Category::Refusal`. Same code as
  `ReplayRequestRefusal::EmptyBackendIdentity`, and the same `code() -> Code`
  shape the replay refusals use, so a driver forwards a route refusal and a
  replay refusal of an empty identity alike. The test checks code, category
  and exit 20 (`Category::Refusal.exit_code()`).
- **Not re-exporting `BackendId` / `EmptyBackendIdentity` from
  `qsl-replay` is correct.** ADR-011 §6.1 gives layer 6 `replay` "layers 1
  to 5, F, SV, K" (plus `quire-contract-model` and I3 under a feature); R
  (`route`) is not a numbered layer, so it is outside "1 to 5", and R's own
  row is "4, 3, F, SV, K". `qsl-replay/Cargo.toml` has no `qsl-route`
  dependency. The only rows that may depend on both are the root crate's
  `command` ("every layer above, including I3, R, tool and `replay`") and
  the orchestrating driver. CG reads no backend member from a manifest (the
  driver does, ADR-013 C-28), so CG needs neither type.
- No `unsafe`, no `unwrap` outside tests, no allocation change.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

Clean. Both additions have a consumer, the layering reading is right, and
the route refusal now carries the same code as its replay twin.

Focused run at the reviewed sha through `locked-build.sh`: `cargo test --locked -p qsl-route -p qsl-replay --lib --test terminal_record_facade -- empty_backend internal_fault`: 5 passed, 0 failed.

Disposition pass 1, reviewed at `a933ea7e3`: no finding was open; the fix round touches only the facade test, reviewed under SR-1305.
