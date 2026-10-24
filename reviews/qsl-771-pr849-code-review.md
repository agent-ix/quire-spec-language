---
id: SR-9133
title: "Code review of quire-spec-language PR #849: population_v2_fixtures CLI"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@cc7aeef5b7e30a065ebe94fd677c2a20693704a6; src/population_fixtures.rs, examples/population_v2_fixtures.rs, Cargo.toml, src/lib.rs, qsl-package/src/emit.rs, qsl-package/src/lib.rs, qsl-package/src/emit/tests/admission_corpus.rs"
review_set: subset
---
# Code review of quire-spec-language PR #849

## Summary

Ticket: QSL-771. Code review with the rust-review lane folded in, scoped to
`git diff origin/main...HEAD` (3 commits, base 63c678156). Read in place, no
builds run. Acceptance criteria from the ticket are the oracle.

Examined:
- `src/population_fixtures.rs` (feature `population-fixtures`):
  `with_population_package`, `assemble`, `fixture`, `emit_and_read`, `write`,
  the typed `Error`, and its five tests (examined)
- `examples/population_v2_fixtures.rs`: thin CLI, usage and error to stderr,
  exit 1; matches the `native_protocol_v2_handoff` precedent (examined)
- `qsl_package::emit_package` changed from `#[cfg(test)] pub(crate)` to `pub`
  and re-exported. It takes `&CheckedPackage`, so it cannot emit an unchecked
  stage; the caller only chooses occurrence regions, each validated by
  `SourceRegion::new` and refused when empty (examined)
- The admission corpus after the move: `Fixture::ModelPopulation`
  (bound 3) still runs in `qsl-package` tests; it emits through
  `Emitted::whole`, which places every region at `0..TEXT.len()` (66 bytes)
  (examined)
- `Cargo.toml` `[[example]] test = false` against xtask `test_suites.rs`
  `WORKSPACE_EMPTY`/`LANES` (examined)
- Embedded `tests/fixtures/spine-model.*` are this repo's own fixtures, not
  another repo's artifact; `spine-model.semantic-ir.json` is written verbatim
  from the embedded bytes at run time (examined)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `population-3.json` bytes differ from the admission corpus's `Population<M::Gadget>[3]` bytes: the new module places regions at `0..749` (whole unit), the corpus `ModelPopulation` row still at `0..66` (`TEXT.len()`). The corpus keeps its own builder, so AC1 (one shared path, no second copy) and AC2 (written bytes equal the corpus's bytes) are not met; the only byte-equality test compares `write` output with `fixture()`, the same function | src/population_fixtures.rs:172-180, qsl-package/src/emit/tests/admission_corpus.rs:211-226, qsl-package/src/emit/tests.rs:183-189 |
| FND-002 | medium | The moved test lost its typed IR-admission assertion: main counted `read.admitted.node_kinds()` equal to `CheckedNodeKind::BoundedDomain(BoundedDomainForm::ModelPopulation)` per bound (TC-416 step 11: decode each node through IR's node kinds). The new code discards the `AdmittedPackages` it reads into, and `each_bound_is_a_typed_population_node` only matches JSON strings on the emitter's wire | src/population_fixtures.rs:189-197, src/population_fixtures.rs:284-313 |
| FND-003 | high | Broken intra-doc link: the module doc names `[population_package]`, renamed to private `with_population_package` in 36461fe19. `ci-docs` runs `cargo doc --workspace --all-features` with `RUSTDOCFLAGS="-D warnings"`, so `make ci` fails; only `ci-all-features` was rerun on the final head | src/population_fixtures.rs:5 |

## Verdict

**FAIL: 3 blocking findings.**

What is sound: `write` builds and admits all four cases before
`create_dir`, so a refused emission leaves nothing; an existing path is
refused as `OutputExists` and left untouched (tested); admission goes
through the real `read_import_view` (IR's v2 reader plus the `package_id`
cross-check); the four cases `[0]`, `[1]`, `[3]`, `[2, 5]` match TC-416 step 11
and the ticket; no vendored copy; `emit_package` exposes no unchecked stage.

`test = false` on the example is a legitimate fix, not a hidden gate: the
example has no tests, `clippy --all-targets --all-features` still compiles
it, and `WORKSPACE_EMPTY` is shared with the default-workspace lane, where
the feature-gated example never builds, so declaring it there would fail as
"declared empty, but it did not run".

Recorded, not reported: medium, `emit_package` is new public API whose
caller picks every occurrence region (it could be `#[doc(hidden)]` or
feature-gated); low, a mid-write I/O failure after `create_dir` leaves a
partial directory (typed `Io`, same as the precedent); low, the CLI exit path
is not tested as a process (same as the precedent); low, `assemble`
duplicates the test helper `assemble_with_models` (forced by TC-390).

## Dispositions

Round 1, reviewed at 08d57a8da7fba3c32009cc02c551b6654333d1cc. Full `make ci`
log `qsl771-ci4.log` ends `exit=0` and was written at 23:20, after the
commit (21:34). It runs `ir_admits_one_model_population_node_per_bound ... ok`.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | deferred | The obstacle is real. TC-390 (`tests/it/family_outcome_layering.rs:319-328, 400-455`) fixes qsl-package's `[dependencies]` without qsl-cst or qsl-forms, which it admits as dev-only. The root crate depends on qsl-package, so the corpus cannot reach the root builder. The corpus N = 3 row is a separate fixture: parameter `p` against `p0`, regions 0..66 against 0..749. Nothing asserts it equals the written bytes, so no assertion claims an equality it does not prove. The deviation from ticket AC1 and AC2 is recorded on QSL-771 by the lead; the planner decides on amending TC-390/ADR-011 X-7. The PR body does not yet state it. |
| FND-002 | fixed 08d57a8da | `AdmittedPackages::package` exposes IR's admitted `CheckedPackageV2`. `Fixture.node_kinds` comes from its `node_kinds()`. `ir_admits_one_model_population_node_per_bound` counts `CheckedNodeKind::BoundedDomain(BoundedDomainForm::ModelPopulation)` per bound for every case, so a wrong or missing kind fails the count. |
| FND-003 | fixed 08d57a8da | The doc names `with_population_package` as plain code and links `write()`; `ci-docs` passed inside the full `make ci`. |
