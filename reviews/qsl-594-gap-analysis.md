---
id: SR-1216
title: "QSL-594 gap analysis of PR #599 (FR-284, FR-280-AC-3, TC-767, TC-768)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@b4388c0301217c36a4e3d00665afd5b32d7f817b; PR #599 diff against origin/main; spec/functional/FR-284-keep-the-qualified-core-separable-by-crate.md; spec/functional/FR-280-serve-the-driver-s-lower-generate-and-prove-operations.md; spec/test-cases/TC-768"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-284
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-280
    type: reviews
---
## Summary

Ticket: QSL-594. PR: quire-spec-language#599.

Spec against code:
- FR-284's core list (12 crates, `quire-walk` included) equals
  `CORE_CRATES`.
- The Behavior's four direction categories map onto `offence()`: a QSL
  workspace member outside the core, `ABOVE_CORE`, a CG/RT crate through
  `classify`, and `FRONTEND`'s six categories.
- IR crates are allowed. Proc macros and build/dev edges are not walked.
- The monotonic-counter carve-out matches `MUTABLE_STATIC_TYPES`, which
  holds no atomics.
- The Status names the two live violations exactly as the tool reports
  them.

Trace:
- FR-284-AC-1 (TC-767): the seven `tc_767_*` tests cover a clean graph, an
  above-core crate, an argument parser through a chain, CG and driver
  crates, proc macros skipped, a missing core crate, `parse_graph`, and the
  report. The oracles assert exact pairs, offences and `via` chains.
- FR-280-AC-3: `tc_767_cg_or_driver_crate_in_qsl_route_fails`.
- The static ambient scan: `tc_768_*` (two tests). See FND-002.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The FR-280-AC-3 amendment narrowed "every QSL crate" to "every QSL core crate". FR-280's Behavior, as amended in the same PR, still says "No QSL crate shall depend on a CG, RT or driver crate". The check walks core crates only, so the root crate, `qsl-analyze` or `qsl-bench` could depend on a CG, RT or driver crate unchecked. The AC was narrowed to fit the code, while the ruling only allowed sanctioned IR crates. Either run the CG/RT/driver part of the check over every QSL workspace member (a small extension of `check_direction`), or amend the Behavior to core crates and say why non-core QSL crates may. | spec/functional/FR-280-serve-the-driver-s-lower-generate-and-prove-operations.md:65-66; spec/functional/FR-280-serve-the-driver-s-lower-generate-and-prove-operations.md:74 |
| FND-002 | low | The static ambient-scan tests are traced to FR-284-AC-2 and AC-3. Those ACs describe the runtime child-process run (an arbitrary environment, an empty working directory, equal outcomes, empty stdout and stderr), which waits on FR-275. No FR-284 AC states the static scan the Behavior bullets require, so the tags claim runtime ACs the tests do not exercise. Add an FR-284 AC for the static scan ("no core crate's shipped source reads …, writes …, or exits", TC-768 static step), retag the two `tc_768_*` tests to it, and leave AC-2 and AC-3 to the runtime run. | tools/arch-lint/qualified_core.rs:1013-1048; spec/functional/FR-284-keep-the-qualified-core-separable-by-crate.md:82-83 |

## Verdict

Changes requested: FND-001 is medium, FND-002 is low.

## Dispositions

Round 1, reviewed at `712e03a544b862b0d407f620f896b28265f7bcc0`.

On the team-leader ruling, FR-284's Status states the two `OnceLock` caches
accurately:
- The lines are right (`definition.rs:396`, `diagnostics_catalog.rs:26`).
- Both caches really are built from compiled-in QSpec bytes and read no
  ambient input.
- Removing them really does change `&'static` returns.
- The count of four matches the live run.

The spec.md FR-284 row ("four known violations") and the TC-767/TC-768 rows
in tests.md match.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 2af9dba63b955ff41b3d94c4385972834ae062d5 |
| FND-002 | fixed | 2af9dba63b955ff41b3d94c4385972834ae062d5 |
