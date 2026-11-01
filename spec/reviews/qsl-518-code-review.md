---
id: SR-202610310155
title: "Code and Rust review of QSL-518 PR #2042 AC10"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@1f6bb469dd75a3de1bf8f8e87eee900f6bb11f3a; PR #2042 diff: xtask/tests/refinement_layering.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-345
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-868
    type: reviews
---

## Summary

Ticket: QSL-518. Reviewed PR #2042 at `1f6bb469dd75a3de1bf8f8e87eee900f6bb11f3a`. The change adds one integration test for FR-345-AC-10. Code review and the Rust lane found no defects. The test exercises the generated corpus, FR-278 compile composition, production registry snapshot, supported negotiation, CLI/core report identity without negotiation, unchanged case and edge results, six settled witnesses, and requests per candidate.

## Verdict

**PASS** — the AC10 test is correctly traced and verifies the negotiated and non-negotiated paths. Focused test, Clippy, formatting, and dependency-tree checks passed.

## Assurance Context

No applicable AssuranceProfile for `xtask/tests/refinement_layering.rs` was found under `spec/`.

## Scope

- `FR-345-AC-10`: examined the test's required composition, registry, negotiation outcomes, report equality, silence, and dependency condition.
- `TC-868`: examined the test-case mapping and trace tag.
- `xtask/tests/refinement_layering.rs`: examined the full changed integration test and Rust test idioms. No production, dependency, or CI files changed.

## Checks

- `cargo test -p xtask --test refinement_layering production_comparison_matches_cli_and_negotiates_each_witness -- --exact`: passed (1 test).
- `cargo clippy -p xtask --test refinement_layering -- -D warnings`: passed.
- `cargo fmt --all --check`: passed.
- `cargo tree -p xtask -e normal,build,dev`: no `quire-contract-codegen` entry.
- Inspected the comparison implementation and found no stdout/stderr writes in the comparison path.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Recheck at fix head

Rechecked PR #2042 at `6d13fb005a2fe7b18d3a3f40ab605fbe5136cc58`, including the new `gag` dev dependency and `assert_silent` helper. The targeted test, xtask integration-test Clippy, and formatting checks pass. A separate minimal libtest probe demonstrated that `gag::BufferRedirect` does not capture Rust `println!` output under the default test harness capture: the text was emitted by the test harness while the gag buffer was empty. The test helper therefore does not close the existing stream-capture gap.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | medium | `assert_silent` uses `gag::BufferRedirect`, which captures OS file descriptors but misses Rust `println!` output intercepted by libtest. A reproduction emitted `stream-capture-probe` while the gag buffer was empty. The test can therefore pass despite a direct `compare` call writing stdout/stderr under the normal test harness. Capture the calls in a subprocess or use a method that observes the test harness's captured output. | xtask/tests/refinement_layering.rs:45-72 |
