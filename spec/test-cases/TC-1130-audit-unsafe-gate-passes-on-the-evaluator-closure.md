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
   Makefile's `audit-unsafe` recipe and its `ci` prerequisites.
2. Run `make audit-unsafe`.
3. Run `cargo geiger -p quire-eval --target thumbv7em-none-eabi
   --output-format Json --locked`, list the third-party closure packages it
   reports with at least one unsafe function, expression, impl, trait or
   method (used or unused), excluding proc-macro crates and packages reached
   only through one, and compare that list with the `name` keys of
   `unsafe-inventory.toml`.
4. Run `make audit-unsafe` with a `PATH` that holds `cargo` but not
   `cargo-geiger`.
5. Read the Makefile, the checker's argument parser and every environment
   variable it reads.

## Expected Results

1. The recipe runs the step 3 command and the checker; `ci` lists
   `audit-unsafe`.
2. The run passes with no finding line.
3. The two lists are equal.
4. The run exits non-zero and prints the `cargo install` command for
   `cargo-geiger`.
5. No variable, flag or file skips the gate, a crate or a finding kind.
