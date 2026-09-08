---
id: SR-009
title: "code-review review of quire-spec-language"
type: SpecReview
analysis: code-review
scope: "src/, tests/, tools/, Cargo/toolchain/CI and corresponding current requirements"
review_set: subset
evaluated_revision: "a80a17d1dd303b91712df2023fdba8aba83e89c1"
review_date: "2026-09-07"
---

## Summary

The current Rust syntax implementation has real bounded parser/source-map behavior and passes its configured gates. Review reproduced a CLI input panic and identified a formatter contract mismatch, error-idiom gaps and missing evidence traceability; future pipeline stages remain separately unfinished.

## Verdict

**CONDITIONAL** — Findings require disposition before the affected implementation or acceptance gate. This is not owner acceptance.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Non-UTF-8 OS argument/path panics before usage validation: byte FF produces exit 101 instead of a deliberate input/I/O outcome. Use OS-string arguments and specify encoding separately for labels and paths. | src/main.rs:22; FR-010; [actual probe](../spec/reviews/data/cli-nonutf8.json) |
| FND-002 | medium | Formatter cannot accept the selected output budget described by FR-003, and appends before checking the hardcoded default with >=. Reconcile the API and inclusive ceiling, then check prospective growth before allocation. | src/format.rs:8; src/format.rs:46; src/format.rs:55; FR-003; NFR-001 |
| FND-003 | medium | Diagnostic does not implement Display/std::error::Error. A caller using a conventional Error-based Result cannot propagate a parser Diagnostic with ?. The portable Rust-style error-envelope idiom is not satisfied. | src/diagnostic.rs:59; src/parser.rs:8; agent-skills/rust-style: Error Envelopes |
| FND-004 | medium | All 21 Rust integration tests lack recognized TC/FR trace tags. They execute real code, but none binds the authored obligations; matrix/TC setup is also absent. Add real TC identities and resolving tags after specifying the evidence plan. | tests/cli.rs:5; tests/parser.rs; tests/source_map.rs; [binding census](../spec/reviews/data/coverage.json) |
| FND-005 | medium | The CLI suite checks success and an empty-identity refusal, but does not assert usage/I/O exit 2, resource exit 3, or the emitted digest against actual bytes. A change to these branches/field could escape the current CLI suite. | tests/cli.rs:5; src/main.rs:23; src/main.rs:36; FR-010-AC-3/4/5 |
| FND-006 | low | Style note: the code enum has as_str but no all/from_code or owned stable-code catalog. Record code stability before downstream consumers depend on spellings. | src/diagnostic.rs:28; agent-skills/rust-style: Error Envelopes |
| FND-007 | low | Style note: public items and several modules lack required documentation, including coordinate/identity and diagnostic contracts. Apply the default public-item/module-header idiom. | src/source.rs:13; src/syntax.rs:10; src/diagnostic.rs:5; agent-skills/rust-style: Ergonomics & Headers |
| FND-008 | low | Style note: identity and revision are interchangeable String leaves at construction. Consider distinct validated leaf types at the forthcoming trust boundary; existing local labels must not become portable reference authority implicitly. | src/source.rs:7; agent-skills/rust-style: Domain Identity |
| FND-009 | low | Integration setup: optional Python producer helpers invoke child processes without an internal timeout. IT-001 names a 180-second enclosing timeout but the runner ownership/command is not recorded. Specify the runner and missing-tool behavior before adopting this as an automated suite. | tools/check_model_fixture.py:36; tools/check_model_fixture.py:54; tools/check_rule_syntax.py:41; IT-001; NFR-002 |

## Scope and skill provenance

Reviewed `quire-spec-language@a80a17d1dd303b91712df2023fdba8aba83e89c1`: all Rust source, the three Rust integration-test files, Cargo/toolchain/CI configuration, and four optional Python helper scripts. No applicable AssuranceProfile or repository-specific Rust idiom document was present. AGENTS.md, LICENSE-DECISION.md and the documented commands were read first.

The actual shared Agent-IX `agent-skills/code-review/SKILL.md` dispatches Rust review to `agent-skills/rust-review/SKILL.md`; its default idioms come from `agent-skills/rust-style/SKILL.md`. The initially loaded `/home/peter/dev/agent_skills/` Rust review/style copies were verified byte-for-byte identical to the owner's named `/home/peter/dev/agent-skills/` paths. Rust review SHA-256: `bec67626edf3944397fa6c2c83c1164278c9f86db3efdcc67cf91b2152b2d85a`; Rust style SHA-256: `1ed18f352d8a9235e04ea293e94c1ee0cf5e541f4da5908c8ee3af40032b6e69`.

This is code-review plus the discovery phase of implementation-gap-analysis. Python checks apply to the optional scripts only; Rust tests are not judged by Python class/mocker conventions. The owner declined the separate optional gap-analysis intent↔test↔code semantic pass. The current-code boundary/faithfulness checks below do not claim that optional pass ran. No code, test assertions, compiler flags or dependency was changed during this review.

## Rust rubric results

| Check | Observed result |
| --- | --- |
| 0 / 0b: conventions and idioms | Repo has no overriding idiom document. Concrete Diagnostic envelope exists, but standard Error traits/catalog/docs and typed leaf identity idioms have the findings above. |
| 1: test standards and tags | 21 real integration tests; deterministic malformed cases, byte/coordinate/map assertions and a joined constrained-stack test. No recognized trace tags. No clock-based assertion, daemon/database or ignored lane found. |
| 2: seam compliance | Tests exercise actual parser/formatter/source-map functions and real CLI processes. No test-only production bypass or replacement of the unit under test found. |
| 3: source completeness | No todo!/unimplemented!/placeholder implementation found. lib.rs deliberately exposes the crate API. Future linking/evaluation/lowering are documented unfinished work, not concealed stubs. |
| 4: test completeness | Meaningful parser/map assertions exist; CLI missing exit/digest assertions are identified. Authored semantic fixtures are not evaluator tests. |
| 5: integrity | Cargo forbids unsafe code; strict warnings/CI are intact. No blanket allow, weakened threshold, fake waiver or test-skipping workaround introduced. |
| 6: panic surface | CLI env::args panic reproduced. Other inspected expects are on private validated parser/source invariants; no additional input-triggered library panic was established. Review is not proof of panic freedom. |
| 7: numeric boundaries | Inspected spans are bounded UTF-8 offsets. CLI source_bytes as u64 is widening on supported targets and hard bounded before +1; no demonstrated truncating persistence/wire cast. |
| 8: async/blocking | No async runtime or lock-held await in implemented source. CLI filesystem I/O is synchronous by design; optional child-process runner setup is called out. |
| 9: state/concurrency/lifecycle | Arc<Document> is immutable; no application locks/atomics/channels or background evaluator state. The only test-created thread is joined. Loom is conditional on future shared state. |
| 10: untrusted inputs/wire | Native source and string literals are checked. A production shared-reference wire decoder is not implemented; do not claim deny_unknown_fields coverage for a nonexistent reader. CLI selects an explicit local file, not a confined-root file service. |
| 11: resources | Input/token/node/depth/source-map bounds and flat arena storage exist. Formatter budget boundary is the concrete discrepancy; no concurrency limit is required for absent concurrent work. |
| 12: actual gates | Formatting, all-target/all-feature Clippy and locked minimal-feature tests passed. deny.toml is absent, so cargo deny is not an applicable configured gate. |

## Specification-to-code scope

| Requirements | Existing source / qualification |
| --- | --- |
| FR-001 | source.rs / digest.rs: exact immutable bounded source and verified digest intake. |
| FR-002 | token.rs / lexer.rs / parser.rs / syntax.rs: declarative Logos vocabulary, Pratt parse, source spans and bounded flat syntax storage. |
| FR-003 | format.rs: one vocabulary and token-preserving whitespace rewrite; budget discrepancy remains. |
| FR-004 | source_map.rs: checked exact body/original correspondence and explicit layout transforms. |
| FR-010 | main.rs: parse/format outcomes; OS-argument and test-coverage findings remain. |
| FR-005–FR-009, FR-011 | Future native linking, typing, runtime validation, reference evaluation, qualified lowering and extraction integration. No passing implementation coverage claimed. |
| NFR-001 / NFR-002 | Existing resource/build behavior inspected; formatter and optional runner gaps noted. |
| NFR-003 | No completed logical result in current syntax CLI. Whole-pipeline propagation remains unimplemented. |
| NFR-004 | AGPL-3.0-only decision and dependency/fixture inventory exist. This review is not distribution approval or a completed ecosystem rights review. |

## Actual gates

Commands ran in the reviewed worktree, with `--target-dir target` to keep build writes inside Agent A's checkout:

```text
cargo fmt --all -- --check
exit 0

cargo clippy --offline --locked --workspace --all-targets --all-features --target-dir target -- -D warnings
exit 0

cargo test --offline --locked --target-dir target --no-default-features
exit 0
```

Observed test result excerpts (CLI, parser, source-map, then documentation; elapsed-time suffixes omitted):

```text
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

Library and binary unit targets contained zero tests. The documentation-test process was polled to terminal success before recording the overall exit. `cargo deny` was not run because no deny.toml exists. The exact reviewed revision also has [successful hosted native CI](https://github.com/agent-ix/quire-spec-language/actions/runs/34185921148). These passes do not erase the reproducible CLI panic or create formal TC traceability.

## Gap-analysis target preflight

No `plan/*/plan.md` bundle, `spec/matrix.md`, or `spec/tests.md` TestMatrix exists in this new repository. The Quoin `gap-analysis/SKILL.md` target-selection reference states: “If there is no plan bundle, stop and tell the user — gap-analysis audits a plan; it does not invent one”. Therefore the formal plan/Task completion audit stopped at target selection. No fictitious Plan/Task IDs, plan-completion status or matrix verdict were produced. The owner was informed and specifically identified missing setup as expected in the new repo.

The separate read-only implementation-gap discovery did run: OS argument encoding, formatter ceiling semantics, error interoperability, constraint visibility, trace tags and optional child-process environment/timeout ownership are the observed unstated or underspecified boundaries. Raw Quire coverage reports 0/57 backed criteria, 55 unbacked FR verification rows and a Rust census of 21 candidates / 0 tagged / 0 bound, with zero status_lies. This measures missing formal binding; it does not say that the 21 executing tests failed. NFR metrics are included among the 67 obligations. The optional semantic comparison remains explicitly skipped.

## Remediation and architecture direction

Specify the CLI OS-string boundary and formatter output-budget contract first; resolve and re-review those requirements before changing runtime code. Add a conventional error envelope/traits and stable code contract without creating another service or crate solely for errors. Establish actual TC IDs/methods/suites and attach resolving tags to the existing behavioral tests; add the missing meaningful CLI/budget cases. Record public API invariants and introduce typed identity leaves where they prevent incorrect joins.

Keep the current declarative lexer/Pratt parser, immutable source, flat syntax arena and exact source-map boundary. Preserve separate parsed, linked and reference-evaluable representations as the remaining phases arrive. [Evidence review](../spec/reviews/evidence.md) records property/fuzz/mutation/fault-injection recommendations and when a future Loom lane would apply. These are recommendations, not newly executed harnesses. The review remains conditional until findings are resolved or explicitly dispositioned through the required spec cycle.

## Subsequent owner constraint

After the baseline inspection, the owner required Rust remediation of all four
Python audit helpers, including the two invoked by CI. The
[private LC01 campaign audit](https://github.com/agent-ix/quire-spec-language/issues/2#issuecomment-5579283030)
records the same requirement and the separate unresolved TypeSpec/Node producer
language disposition. This is a new explicit verification-language constraint;
the old NFR-002 scoped runtime independence more narrowly. FR-012 and a scoped
review/remediation cycle will address it before any further qualification claim.
