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
