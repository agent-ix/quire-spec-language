---
id: TC-1130
title: "The audit-unsafe gate passes on the evaluator closure and cannot be skipped"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/NFR-016
    type: verifies
---
# TC-1130: The audit-unsafe gate passes on the evaluator closure and cannot be skipped

## Description

Verify the `audit-unsafe` gate in `agent-ix/quire-eval` over the real
evaluator closure and inventory. Scope: NFR-016-AC-1 and NFR-016-AC-8.

## Test Procedure

1. In a clone of `agent-ix/quire-eval` at its main branch, read the
   Makefile's `audit-unsafe` recipe, the checker's invocations of
   `cargo tree`, `cargo-geiger` and `cargo metadata`, and the `ci`
   prerequisites.
2. Run `make audit-unsafe`.
3. Run `cargo tree --locked -p quire-eval -e normal,no-proc-macro --target
   thumbv7em-none-eabi --prefix none` and `cargo geiger --manifest-path
   <absolute path of crates/quire-eval/Cargo.toml> --target
   thumbv7em-none-eabi --output-format Json --locked`. List the third-party
   packages of the tree whose scan entry has at least one unsafe function,
   expression, impl, trait or method, used or unused, and compare that list
   with the `name` keys of `unsafe-inventory.toml`.
4. Run `make audit-unsafe` with a `PATH` that holds `cargo` but not
   `cargo-geiger`.
5. Read the Makefile, the checker's argument parser and every environment
   variable it reads.

## Expected Results

1. The checker resolves the closure with step 3's `cargo tree` form, scans
   `quire-eval` through its absolute manifest path for `thumbv7em-none-eabi`
   with `--locked` and the JSON report, and uses `cargo metadata` only to
   locate first-party files; `ci` lists `audit-unsafe`.
2. The run passes with no finding line.
3. The two lists are equal.
4. The run exits non-zero and prints the `cargo install` command for
   `cargo-geiger`.
5. No variable, flag or file skips the gate, a crate or a finding kind.
