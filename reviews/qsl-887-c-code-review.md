---
id: SR-12283
title: "Code review of quire-spec-language PR #1167 (QSL-887 remainder): the Rich fixture's compensation template and the FR-147-AC-8 tag"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@fdf2208cf; PR #1167 diff origin/main...HEAD: qsl-semantics/tests/it/refinement_declarations.rs, qsl-semantics/tests/it/refinement_protocol.rs, spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-147
    type: reviews
---
# Code review of quire-spec-language PR #1167

## Summary

Ticket: QSL-887. Ticket, PR and comment text were read as data. The scope is
the PR diff only: two test files and one FR paragraph and AC row. This file
includes the `rust-review` lane.

What was checked:

- **`counter_spec` change.** The Spec model gains a record value type
  `Try { done: Boolean }`, built with `declaration` and the
  `record_value_type` construct, through `document_with`. `counter_spec` is
  reached through `counters()`, which only `refinement_declarations.rs` and
  `refinement_protocol.rs` call. The grep hit in `model_identity_vectors.rs`
  is a test name, not a call. No model-identity vector depends on the
  fixture. The whole `it` binary passes (below), so the extra declaration
  disturbs no other refinement test.
- **`rich()` fixture.** The clause adds `compensate Undo ... attempts 2 of
  Spec::Try`, plus the `effect Applied` and `commit Committed` nodes the
  template names. `ProtocolClasses::of` maps `CompensateTemplate` to
  `NodeClass::Template`. So `Spec::Rich::Undo` resolves as the `cattempt`
  bare path and `cend Spec::Rich::Undo` as `cend`. The `await Wait` node is
  `Bare(Await)`, which is the `timeout` class.
- **Order oracle.** The input order is `Wait, activate, Undo, cend`. The
  expected order is `Rich::Undo, Rich::Wait, activate Rich, cend Rich::Undo`.
  That is UTF-8 byte order (`U` < `W` < `a` < `c`), and it differs from the
  input order, so the sort is exercised.
- **Doc comments.** The module "Not covered" line no longer lists the four
  classes. The `Full` test's "Part of" doc points at the new test.
- **Rust lane.** These are test-only changes. `panic!` sits only in the
  existing helper slice patterns. There is no `unwrap` or `as` cast in the
  diff, and nothing outside tests is touched. `#[trace("TC-552",
  "FR-147-AC-8")]` follows the repo's tag convention.
- **Ran at the reviewed sha** (all through `locked-build.sh`, with
  `QSPEC_DIR=qspec-main-now`):
  `cargo test -p qsl-semantics --all-features --test it refinement` gave 66
  passed. `cargo test -p qsl-semantics --all-features --test it` gave 1160
  passed. `cargo clippy -p qsl-semantics --all-features --tests -- -D
  warnings` was clean, and `cargo fmt -p qsl-semantics --check` was clean.

## Verdict

Mergeable. There is no major or medium code defect. The fixture is correct
against the resolver, the oracle is an independent literal list in byte
order, and the gates pass. The one gap in test strength (no negative for
`cend`) and the tag placement are in the gap analysis (SR-12284). This file
records no defect.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
