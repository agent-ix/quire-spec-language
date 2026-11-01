---
id: SR-12249
title: "QSL-560 PR #1986 code and Rust review"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@d370a0e670f7bc978980a82dab11df3dba23ab8f; PR #1986 diff origin/main...HEAD"
review_set: subset
---

## Summary

Ticket: QSL-560. Code review with the Rust lane against current `origin/main` (`bdbc95f3ac6e33edd3f8ac17bae2defa79cc9d92`). Reviewed the 20 changed Rust files, repository conventions, touched fairness/liveness/replay call paths, and FR-212/FR-217 obligations. No applicable AssuranceProfile was found. The PR emits the STD-616 typed protocol-fairness target union, but the currently pinned IR reader rejects that shape. This is an integration blocker pending IR-1221 and a QSL pin bump.

## Verdict

FAIL for merge at the current dependency pin. FND-001 is a reproduced wire-compatibility failure. FND-002 is a source-order defect against FR-123. The IR reader patch and pin bump are required before merge; rerun the touched crate tests after that update.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Current pinned IR reader rejects the emitted typed protocol-fairness target union, so checked packages containing protocol fairness fail admission. | qsl-package/src/emit.rs:1067-1082 |
| FND-002 | medium | Lowering concatenates operation fairness before protocol fairness, losing source order when the two forms are interleaved. | qsl-semantics/src/check/lowering/temporal.rs:243-335 |

## Examined obligations

| Unit | Role | Excerpt |
| --- | --- | --- |
| FR-212-AC-1 | examined | `eventually holds(k.flag = 1)` under infinite-trace settles `proved`; its fairness set holds three `scheduler` constraints, over `s`, `r` and the root. |
| FR-212-AC-2 | examined | `Chatter` with `scheduling adversarial` ... settles `refuted` with a lasso ...; its fairness set is empty. |
| FR-212-AC-3 | examined | `fair weak whole Chatter::Both::s` names the branch and `fair weak each Chatter::Done` the attempt node ... |
| FR-212-AC-4 | examined | The TP-1 claim ... settles `proved` with and without `scheduling adversarial`, over the same reachable states. |
| FR-217-AC-4 | examined | The same lasso with the scheduler constraints added to its fairness set ... refuses ... as unfair. |
| qsl-package/src/emit.rs:1067-1082 | examined | Fairness members serialize as a typed `target` union, including `operation`, `protocol_root`, `attempt_node`, `parallel_branch`, and `compensation_template`. |
| qsl-analyze/src/model_check/liveness.rs:308-312 | examined | Liveness builds the full set with `fairness_set_with_targets`. |
| qsl-replay/src/model_check/replay.rs:900-915 | examined | Replay rebuilds the same typed protocol fairness set and faults if a checked target is no longer resolved. |
| qsl-semantics/src/check/lowering/temporal.rs:243-335 | examined | Separate loops lower operation and protocol fairness members into one ordered key vector. |

## Coverage and limits

Ran `cargo fmt --all -- --check` (pass), `cargo clippy --workspace --all-targets --all-features -- -D warnings` (pass), and `cargo deny check` (exit 4: existing license/source policy failures; no lockfile or policy changes in this PR). With `QSPEC_DIR=/Users/peter/dev/qspec-std619`, `cargo test --locked -p qsl-package` completed 457 passed / 10 failed / 2 ignored; all 10 failures were reader/admission failures at the typed fairness dependency, including the hybrid target branch and FR-370 refusal-vector tests. `QSPEC_DIR=/Users/peter/dev/qspec-std619 cargo test --locked -p qsl-analyze` completed 1,078 passed / 54 failed; fairness package admissions were rejected by the same pinned reader. The focused PR author tests were reported as passing, but they do not override the admission failures reproduced on the locked reader. `make build-no-std` could not find `core` for `thumbv7em` in this environment despite the target being installed. No CI workflow files changed.

### Finding details

**FND-001.** At `qsl-package/src/emit.rs:1075-1081`, the producer now places the fairness target in a discriminated `target` object. The locked `quire-contract-model` reader (IR revision `154227dcbcda09f3abf635375777893132fc7686`) rejects packages carrying this emitted protocol target at `/semantic_graph/nodes/{n}/dependencies` as `InvalidSemanticGraph`. Reproduced in the QSL package and analyzer test suites. The patch must wait for IR-1221's strict-reader support and a QSL dependency pin bump, then the affected admission/liveness/replay tests must pass.

**FND-002.** The loop over `clause.fairness()` at lines 243-271 appends operation constraints first; the second loop at lines 273-335 appends protocol constraints afterward. FR-123 requires the checked fairness set to retain source order of first occurrence. A clause interleaving the two fairness forms therefore gets a different fairness-node order and identity than its source. Retain a unified ordered sequence through parsing/lowering and cover an interleaved clause.
