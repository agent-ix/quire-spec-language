---
id: SR-2026111011431564766309
title: "QSL-1826 document-budget main-fix code-review"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; QSL-1826; PR #2665; exact-probability document-budget test and its production key-admission/candidate/document boundaries"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-201
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-282
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-764
    type: references
---

## Summary

Independent scoped code review including the Rust lane of PR #2665. No findings.

## Verdict

PASS for the reviewed single-test delta and its examined budget-stop contract. No repository qualification or complete compound-criterion coverage is claimed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Reviewed frozen source head `5766163bac26e985b8759b38fbfd539eddf3cb7e`, based on main `14c0d809138ebe041aabfb474c22f296e5be6cc2`. The diff is one existing integration test file, 13 additions and six deletions. There is no production, public API, fixture, dependency, lock, specification or workflow change.

Independently traced the eventual Link fixture into `qsl-analyze/src/analyze/exact.rs:341`: eventual claims use CandidateBudget, whose constructor takes the caller's document working-storage ceiling. The graph builder uses that borrowed budget for model-state key admission. `qsl-replay/src/probability_certificate/key_admission.rs:99` retains the actual StorageBytes denial before reservation succeeds; zero storage therefore cannot retain a candidate. The callback at `exact.rs:591` runs only after a candidate certificate exists. Output byte limits remain enforced by the subsequent document construction/writing path, so zero output bytes still reaches the callback. FR-201's cumulative key/graph/storage controls do not promise that all budget denials occur after solving. The former universal callback assertion contradicted the current bounded path.

The revised test is stronger than simply removing the failing assertion: it positively requires callback=true for the zero-output-byte arm and callback=false for the zero-working-storage arm. The real positive baseline still requires nonempty certificate bytes. Both failure arms still require the exact typed Incomplete limit, bound zero, and setting name. The new assertions additionally prohibit an accepted typed certificate or certificate document, alongside the existing absence of exact probabilistic evidence. A wrong phase, wrong resource cause, missing stop, changed bound or leaked accepted evidence can fail this test. No ignored test, enlarged limit, fabricated verdict, replacement engine or production accounting bypass was introduced. Relaxed atomics record a synchronous callback; no thread or timing oracle was added.

Inspected the coder's actual focused logs: the original exact test failed once with exit 101 at the reported assertion; the renamed exact test passed once; the related module passed 13 tests with zero failures or ignored tests. Formatting and focused test-inclusive Clippy passed. These are supplied executed evidence, not reviewer reruns. Per the explicit focused/no-Cargo review instruction, this reviewer ran no Cargo, full suite, Kani, make ci or full matrix. The lead still owns the final affected gate. Repository-wide assurance, all branches of compound criteria and plan completion are not established by this review. Plan completion: not assessed.

Ownership comparison found codex2's PR #2664 changes unrelated mixed-infinite expected-reward handling in the production adapter; it does not change candidate budgeting or the reviewed test file. The cause is already on main; no unmerged codex2 code or protected branch was edited. No applicable AssuranceProfile was selected; no safety, Tokio, ABI, wire-format or dependency surface is changed.

## Examined Scope

- `FR-201-AC-11` — role: examined; `spec/functional/FR-201-check-a-probability-certificate.md`; excerpt: Pre-cancelled input and exhausted byte/state/edge/arithmetic budgets stop without a truth value.

- `FR-201-AC-26` — role: examined; `spec/functional/FR-201-check-a-probability-certificate.md`; excerpt: AC-23/24 independently check with no solver dependency. Source/document/key byte, state/edge, graph work/storage and scalar/rational budgets are exercised at exhaustion, including SCC, reverse reachability and distance construction; actual typed limit and consumption are retained. Pre-cancellation and cancellation during each pass or exact sum stop without acceptance or truth values; no fabricated upstream key-budget API or cyclic refutation is claimed.

- `FR-282-AC-12` — role: examined; `spec/functional/FR-282-check-analyze-certificates-in-the-qualified-core.md`; excerpt: With a source provision or certificate document one byte above its caller ceiling, the fresh checker stops/refuses before reading beyond that ceiling; pre-cancellation and cancellation during graph expansion stop without acceptance or a truth value. Exhausted state, edge and arithmetic budgets retain the actual named setting and consumption.

- `FR-201` — role: examined; `spec/functional/FR-201-check-a-probability-certificate.md`; excerpt: Original/stopped expansion, all action/draw reconstruction, internal raw/current/candidate/post/full keys, exact action sums, zero support, progress passes and retained rows SHALL preserve the real caller's byte, state/edge, representation/arithmetic and cumulative work/storage limits and actual cancellation cause.
