---
id: SR-12269
title: "QSL-925 PR #1155 code review: single invalid-fairness-kinds registration cause (FR-134-AC-3..5)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@2f2e1f7770a7aa29f54230a35c3eee881ad20fdd; qsl-route/src/lib.rs, qsl-route/tests/it/fairness_kinds.rs"
review_set: subset
---

## Summary

Ticket: QSL-925. PR quire-spec-language#1155 at head 2f2e1f777, diff `origin/main...HEAD` (2 files, +71/-47). Code review with the rust-review lane folded in. Checked against FR-134-AC-3 to AC-6 and TC-535 on main, and QSpec FR-272 and FR-368-AC-8/AC-9 (cited by id). One focused test run: `cargo test -p qsl-route --test it fairness_kinds`. Only majors block.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The `descriptor_from_manifest` doc comment still says a defective member "refuses `unknown-fairness-kind`", a repeat "refuses `duplicate-fairness-kind`" and an absent member "refuses `absent-fairness-kinds`". This PR deletes all three causes, so the public doc of the sole manifest entry point now names codes the code never emits. Reword it to `invalid-fairness-kinds` with the defects `absent`, `not-a-list`, `unknown-label`, `duplicate-label`. | qsl-route/src/lib.rs:180-185 |

## Verdict

PASS (no major). One low doc finding, fixable in this PR.

Checked and clean:

- **Single cause and code.** `RegistrationCause::InvalidFairnessKinds(FairnessKindsDefect)` is the only fairness variant. `as_str` gives `invalid-fairness-kinds`, so `catalog_code` is `invalid_capability/invalid-fairness-kinds`, matching the FR-272 row and FR-368-AC-8.
- **Precedence.** The code returns `Absent`, then `NotAList`, then maps every element (the first unknown in member order wins), then checks repeats (the first element that repeats an earlier one). This is the order FR-134 and FR-368 state: `absent`, `not-a-list`, `unknown-label`, `duplicate-label`. Unknown still wins over a repeat (V10), and the single cause keeps which defect was reported in the typed detail.
- **NotAList fix.** A non-list member was `UnknownFairnessKind` before. It is now `NotAList`, as FR-134-AC-3 requires.
- **Deletion with no shim.** The three old variants and their `as_str` spellings are gone. No alias, re-export or legacy mapping. A repo-wide grep (code, tests, docs, spec, all crates including the eval, replay and spine paths) finds no remaining consumer. The only hits are the stale doc comment (FND-001), the stale FR-134 "Remaining work" line (SR-12270 FND-001) and historical review files.
- **Typed enum.** `FairnessKindsDefect` mirrors `DomainsDefect`: same derives, closed variants, each carrying the received value where the spec requires it (`NotAList`, `UnknownLabel`, `DuplicateLabel`), and none for `Absent`. No stringly-typed defect label.
- **Rust.** No new `unwrap`, `expect`, indexing or panic on a production path. The `panic!` in the test helper is test-only and reports the unexpected cause.
- **Test oracles.** `refused` now asserts identity, the full catalog code and the exact defect with its value. V10 tightened from a code-only check to `UnknownLabel("eventual")`. V16/V17 use `refused_code` and assert the earlier pair/domains cause. Each oracle fails if the defect, value or code is wrong.
- **Tags.** `TC-535` with FR-134-AC-3 (V4-V6, V11-V14), AC-4 (V7, V8, V10, V15), AC-5 (V9) and AC-6 (V16, V17). Each tag matches what its test asserts.

## Dispositions

| Finding | Disposition |
| --- | --- |
| FND-001 | Fixed: the `descriptor_from_manifest` doc comment now names `invalid-fairness-kinds` with its four defects. |
