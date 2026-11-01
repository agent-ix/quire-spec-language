---
id: SR-1957023
title: gap-analysis for QSL-1338 formatter lifetime and scheduling slice
type: SpecReview
analysis: gap-analysis
scope: agent-ix/quire-spec-language; Makefile; tests/support/front_end.rs; QSL-1338
review_set: subset
relationships:
- target: ix://agent-ix/quire-spec-language/FR-003
  type: references
---

## Summary

Ticket: QSL-1338. PR: quire-spec-language#1971. Frozen head: aacbdda5eeb2875ceba37db72e21b89d2ca28d4e. Scope is the two-file diff against origin/main: Makefile and tests/support/front_end.rs. No production behavior, dependency, fixture, spec or plan changed.

The parsed CST now drops at the end of the first formatting block; the reparsed CST drops after the second-pass byte assertion, both before emitted_once. The returned formatted String owns its bytes. Original emitted output, original bytes, formatting ceiling, source revision suffix, source/model/dependency/checking authority and retained-source assertions are unchanged. No duplicate helper, new unsafe code, mock, cap, reduced depth, new ignore or weakened assertion was introduced.

The same five exact names are selected; each must list exactly once as an ignored test. Only harness concurrency changes from three to one. Workspace/default-feature target selection and CARGO_TARGET_DIR fallback/cache remain unchanged. The existing ignores remain executed by this dedicated aggregate-gate prerequisite; the four deep fixtures remain 100,000 levels/terms and the dependency chain remains 3 and 1,000 libraries. Serial harness execution prevents overlap between cases, not all internal threads (dependency_fixture_identity still checks its two sides concurrently).

## Verdict

**CONDITIONAL** — No introduced defects found in the two-file change. Scoped mergeability remains conditional on integrating #1930, obtaining all full-depth results and passing required final gates; no full acceptance claim.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Plan completion: not assessed

Reconciliation: computed whole-repository matrix unavailable in this constrained review. Dispatch reports previous identical attempts stalled without output for 3–4 minutes and were stopped; not repeated for this two-file diff. The full repository gap-analysis method is incomplete; manual changed-scope AC/code/test mapping substitutes under reviewer fallback. No whole-repository PASS, trace-count totals or complete reverse-gap/stub inventory is asserted. No new production surface is introduced. Optional whole-repository semantic review not run.

Reviewer reran cargo fmt --all -- --check and git diff --check successfully and inspected make -n test-deep-format; no build or heavy gate launched. No CI workflow changes. Read AGENTS.md, CLAUDE.md, README.md, Cargo.toml and applicable Rust baseline implementation/testing/lifecycle rules. No applicable AssuranceProfile found.

Verified actual logs in /home/peter/dev/.agents/tmp/qsl-1338d-checks: small.log has 1 passed, 0 failed, test 0.13s, command 32m49.54s (cold compile), maxRSS 2,715,812 KiB, exit 0; request 23:28:36Z, acquisition 23:29:19Z from brief, finish 00:02:10Z. dependency_chain_and_order_variants_keep_format_identity.log has 1 passed, 0 failed, test 20.76s, command 21.41s, maxRSS 306,584 KiB, exit 0; request 00:02:37Z, acquired 00:02:38Z, finish 00:02:59Z. Both commands explicitly use -j1 and --test-threads=1 under a 20 GiB bounded runner. Wait time is excluded from command timing. These are focused results, not evidence for all five full-depth cases.

Full four 100,000-level cases, make test-deep-format and the lead aggregate gate are pending integration of #1930. Clippy, cargo-deny and full crate/workspace tests were not rerun during this static review per the dispatch constraint. Prior SIGKILLs do not prove OOM, and no claim is made that this diff alone fixes total memory.

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
  excerpt: cargo test --locked --workspace --tests -- --ignored --exact --test-threads=1
```
