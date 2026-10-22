---
id: TC-1113
title: "Allow CG's evaluator corpus producer the facade's corpus module and refuse every other QSL crate"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-060
    type: verifies
---
# TC-1113: Allow CG's evaluator corpus producer the facade's corpus module and refuse every other QSL crate

## Description

Verify FR-060 T12-A's allowances for CG's evaluator corpus producer:
`qsl_replay::corpus` is reachable only from CG's `evaluator_corpus` module,
a reference to any other QSL workspace crate fails from every CG module, and
a crate from its own repository is outside the rule. Scope: FR-060-AC-8 and
FR-060-AC-9.

## Test Procedure

1. Run `arch-lint api-surface` with a QSL fixture tree whose
   `qsl-replay/src/lib.rs` exists and a CG fixture tree holding
   `use qsl_replay::corpus::encode;` and a
   `qsl_replay::compile_package(..)` call in `src/evaluator_corpus/mod.rs`.
2. Add a `qsl_replay::corpus` reference to `src/replay/mod.rs`, then
   separately a `qsl_replay::corpus::load` path to `src/oracle/mod.rs`, and
   rerun.
3. Separately add a `qsl_eval::`, a `qsl_semantics::`, a `qsl_package::`
   and a `quire_spec_language::` reference and a `use qsl_eval as e;`
   alias, each once in `src/evaluator_corpus/mod.rs` and once in
   `src/replay/mod.rs`, and rerun.
4. Add the member path `crates/extra` to the QSL fixture's workspace
   manifest, with its own `Cargo.toml` naming the package `qsl-extra`, and
   a `qsl_extra::` reference to `src/evaluator_corpus/mod.rs`, and rerun.
5. Add `qsl_eval::` inside a comment and inside a string literal, and a
   `quire_exact::` reference, to `src/evaluator_corpus/mod.rs`, and rerun.

## Expected Results

- Step 1: T12-A passes.
- Step 2: each run fails T12-A, naming the file, line and module of the
  `qsl_replay::corpus` reference.
- Step 3: each run fails T12-A, naming the crate, file, line and module;
  `quire_spec_language` is derived from the root member `.`.
- Step 4: the run fails T12-A on `qsl_extra::` with no change to the rule
  data.
- Step 5: T12-A reports no finding for the comment, the string literal or
  the `quire_exact::` reference.

## Metadata

- Priority: P1
- Target Integration: `tools/arch-lint/api_surface.rs`
- Automation: Automated Rust unit tests

## Dependencies

**Upstream:** [FR-060](../functional/FR-060-check-qsl-api-surface-boundary.md).
**Downstream:** none.
