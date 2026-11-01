---
id: SR-1957022
title: code-review for QSL-1338 formatter lifetime and scheduling slice
type: SpecReview
analysis: code-review
scope: agent-ix/quire-spec-language; Makefile; tests/support/front_end.rs; QSL-1338
review_set: subset
relationships:
- target: ix://agent-ix/quire-spec-language/FR-003
  type: references
---

## Summary

Current disposition: source ae47df82848dbc3e0687550fd87ea901c9453edc; scoped source review clear, with head-specific evidence and limits in Final integration disposition below.

Ticket: QSL-1338. Final frozen head: 2a164fe0e9474f202a6939233a6b80597bb3491e. Same-reviewer disposition round 1. The owned slice relative to integrated parent c19d214682d414ee481bfdea8f982ab4a71bf39b is Makefile, tests/support/front_end.rs and the two original review records. origin/main...HEAD also contains upstream child #1947 changes; those are separately reviewed and are not newly approved here.

The CST lifetime change remains the reviewed change: first parse drops after producing owned formatted bytes; reparse drops after the second-pass assertion, before emitted_once. Formatting idempotence, retained sources, source revision suffix, limits and model/dependency/checking authority remain unchanged. The proposed serialization was removed. The final Makefile retains the ORIGINAL --test-threads=3 schedule and all five full-depth selectors, exact-name/count checks, ignored-test execution and default-lane artifact reuse. Only stale peak/fallback prose changes in the Makefile relative to its parent. No cap, fixture reduction, new skip/ignore, wrapper or assertion weakening was introduced.

## Verdict

**SCOPED CLEAN / SOURCE-REVIEW CLEAR** — No introduced defect or unresolved finding in the owned lifetime/comment slice. Final same-five full-depth lane evidence is now present and supports retaining three threads. Whole-repository assurance, aggregate gate and administrative landing authority are outside this clearance.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Code-review includes rust-review lane. Spec-review remains inapplicable: no spec/plan change. No new source findings.

Plan completion: not assessed

Actual final log inspected: /home/peter/dev/.agents/tmp/qsl-1338d-checks/final-2a164fe0e/test-deep-format.log. Both admitted_source_head and finished_source_head equal the final frozen head. Actual make -j1 test-deep-format completed its five exact-name/count checks and all five named cases: 5 passed, 0 failed, 0 ignored; root harness 363.38s, whole command 15m27.33s including compilation, exit 0. Request and acquisition 04:15:23Z; finish 04:30:50Z. Profile: CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0. Maximum process RSS 8,712,772 KiB; whole cgroup peak 12,789,981,184 bytes (11.91 GiB) under memory.max 21,474,836,480 bytes (20 GiB). Before/after low, high, max, oom, oom_kill and oom_group_kill counters are all zero. The runner banner's environment test_threads=2 does not override the Makefile's explicit --test-threads=3.

Measured older-stack comparison logs under gate-profile-162e8da6f were also inspected: same five serial cases passed at 771.57s harness; three-thread cases passed at 338.13s harness, maxRSS 8,524,240 KiB, cgroup peak 8,741,199,872 bytes (8.14 GiB), 20 GiB cap and zero OOM/kill events. This supports keeping original concurrency over the proposed serial alternative; it is NOT a speedup comparison against original main. Four individual full-100,000-level runs on that older integrated stack were reported passed by the lead; final same-five execution independently supplies the relevant final-depth evidence.

The final measured result establishes this lane fits its observed 20 GiB cgroup bound with unchanged workloads; it does not isolate upstream changes' contributions, prove earlier SIGKILLs were OOM, or guarantee all future loads. Source integration is present, but child #1947's administrative landing still awaits user approval. Full aggregate verification/administrative merge authorization remain separate from this source review. No full CI-green, merged-parent or full FR-003-AC-9 acceptance claim is made.

No builds, gates or source edits were performed in this disposition pass. Formatting/diff checks passed as reported in the dispatch; no reason was found to repeat them. Frozen branch remains unchanged and clean.

## Oracle limits

Pre-existing oracle limitation (outside this diff): FR-003-AC-9 explicitly requires a checked-package identity equal to the original and identical second-format bytes. The inspected assert_format_identity directly asserts second-pass bytes and retained original/formatted source and requires successful compilation, but does not directly compare an original/formatted checked-package identity. dependency_fixture_identity likewise compares selected dependency keys and traverses the closure without directly asserting emitted identity equality. Source revision intentionally changes; this review does not prescribe comparing source-bound IDs or define a replacement semantic identity. Consequently this review establishes assertion preservation, admissibility, idempotence and source retention, not complete AC-9 identity coverage. This unchanged limitation is not an introduced defect or an in-scope fix request.

## Examined units

```yaml
scope:
- id: FR-003-AC-9
  path: spec/functional/FR-003-format-native-source.md
  role: examined
  excerpt: For each complete-V1 fixture unit in the repository's tests that checks,
    checking its formatted source through the S1 to S4 spine yields a checked package
    identity equal to the original's, and formatting the formatted source again yields
    identical bytes.
- id: TC-885
  path: spec/test-cases/TC-885-formatting-keeps-the-checked-package-identity.md
  role: context_only
  excerpt: Each formatted unit checks, with a checked package identity equal to the
    original's.
- id: assert_format_identity
  path: tests/support/front_end.rs
  role: examined
  excerpt: "let formatted = {\n    let parsed = qsl_cst::parse(source.clone(), path,\
    \ bytes, limits.source).unwrap();\n    qsl_cst::format::format_with_limits(&parsed,\
    \ format_limits).unwrap()\n};"
- id: test-deep-format
  path: Makefile
  role: examined
  excerpt: cargo test --locked --workspace --tests -- --ignored --exact --test-threads=3
```

## Dispositions

Round 1: the original Findings table contains only a schema placeholder, not a defect. There are no finding IDs requiring an outcome and no new findings. Final source review is clear within the owned slice.

## Final integration disposition

Same-reviewer disposition round 2. Current frozen source: ae47df82848dbc3e0687550fd87ea901c9453edc, normal merge of exact main 3166c571fd12b4d3707b5f3756078e4b985283e7. No new findings or original defect dispositions are outstanding. Scoped source review remains clear.

Inspected incoming integration and confirmed Makefile, tests/support/front_end.rs, tests/it/formatter_identity.rs and qsl-cst have no diff from the full-depth-qualified 2a164fe0e9474f202a6939233a6b80597bb3491e. The incoming selected exact identity admission/IR revision 154227dcbcda09f3abf635375777893132fc7686 and upstream SMT/member-read-form changes are upstream integration, not a formatter algorithm, fixture, scheduling or own lifetime-patch change. No broad re-review or new builds performed.

Verified actual combined compile log /home/peter/dev/.agents/tmp/qsl-1338d-checks/compile-ae47df828/cargo-check.log: admitted and finished source heads are ae47df82848dbc3e0687550fd87ea901c9453edc; cargo check --locked -j1 --workspace --all-targets passed, exit 0, command 2m44.13s, maximum RSS 1,362,192 KiB. Cgroup memory.max 21,474,836,480 bytes (20 GiB), memory.peak 1,333,456,896 bytes; before/after low/high/max/oom/oom_kill/oom_group_kill counters all zero. Request/acquisition 05:55:12Z, finish 05:57:56Z.

This is compile evidence at ae47df828, not a fresh full-depth test run. The actual five-case three-thread runtime result remains attributed to 2a164fe0e (363.38s harness, all five pass, cgroup peak 11.91 GiB under 20 GiB, zero memory-limit/OOM/kill events). Unchanged formatter source and own patch support carrying the scoped source clearance forward; the current integration's whole-runtime behavior is not thereby fully requalified. No full CI-green, whole-repository gap-audit PASS or administrative merge authorization is implied. The pre-existing direct checked-package identity oracle limitation under AC-9 remains unchanged and explicit in Oracle limits.

Round 2 dispositions: none. Original findings remain placeholder-only; no new findings.
