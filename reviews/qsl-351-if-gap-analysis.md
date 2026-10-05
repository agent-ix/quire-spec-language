---
id: SR-1305
title: "Gap analysis of PR #635 (QSL-351: InternalFault re-export, EmptyBackendIdentity code and category)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@0ce20489b71bbe0e2f13581e68118b08c5a43bb7; qsl-replay/tests/terminal_record_facade.rs, qsl-route/src/lib.rs, spec/functional/FR-069-implement-typed-proof-result-envelope.md, spec/functional/FR-075-compute-candidates-from-registered-backends.md"
review_set: subset
---

## Summary

Ticket: QSL-351. PR agent-ix/quire-spec-language#635. Manual
AC-to-test-to-code check; the PR changes no spec.

- `an_empty_backend_member_is_an_invalid_identifier_refusal` traces to
  TC-433 / FR-075-AC-6 ("an empty identity refuses"). It refines that
  refusal with its code and category; correct binding, literal oracles.
- `an_internal_fault_is_built_and_read_through_the_facade_alone` traces to
  TC-177 / FR-069-AC-1, the category map of terminal records, which names no
  `InternalFault`. Its asserts read `stage`, `invariant`,
  `catalog_code` and `category`, which are `qsl_foundation`'s behaviour; the
  QSL behaviour this PR adds is that the type is reachable from the
  `qsl_replay` root, which compiling the import already shows.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The `InternalFault` facade test traces to FR-069-AC-1, which says nothing about `InternalFault`, and asserts `qsl_foundation`'s own accessors rather than the re-export. Trace it to an AC that states the facade reach (FR-121-AC-13's "built through `qsl_replay`'s root paths alone" pattern, extended to the fault type CG builds `ReplayRefusal::Fault` / `CallSiteRefusal::Fault` from), or build one of those fault refusals through the root and check it settles `Failed` | qsl-replay/tests/terminal_record_facade.rs:77-90 |

## Verdict

Approve with one low finding: a mis-targeted trace on the facade test.
