---
id: TC-745
title: "The checked-input gate rejects pre-check signatures and reconstruction and passes the workspace"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-270
    type: verifies
---
# TC-745: The checked-input gate rejects pre-check signatures and reconstruction and passes the workspace

## Description

Verify FR-270 by planting forbidden signatures and stage calls in copies of
the scanned trees, planting look-alikes that are allowed, and running the
gate over the real workspace.

Scope: FR-270-AC-1 to FR-270-AC-4.

## Test Procedure

Copy the scanned crates' `src/` trees into a temporary directory and run the
gate's scan over the copy, planting one item per run unless a step says
otherwise.

1. In `qsl-eval`: `pub fn plant(e: &qsl_forms::Expression)`; the same after
   `use qsl_forms::Expression as Expr;`; the same through `type Node =`
   a CST node type. In `qsl-route`: a `pub` method taking
   `Option<Vec<&PackageDeclarations>>`. In `qsl-replay` outside `spine`: a
   `pub fn` taking a `quire_contract_model` package type.
2. In `qsl-eval`: a private function with only checked parameters whose
   body calls a `qsl-cst` parse function. In `qsl-route`: one that calls
   `PackageDeclarations::check`. In `qsl-replay`'s `spine`: the same parse
   call.
3. In `qsl-eval`: `pub fn plant(k: StateClauseKind)`; a `#[cfg(test)]`
   module with `fn t(e: qsl_forms::Expression)`. In `qsl-replay` outside
   `spine`: a function that builds `qsl_cst::Limits`. Then step 1's first
   two plants together.
4. Run `cargo xtask checked-input` over the workspace. Plant step 1's first
   item in the real `qsl-eval` tree in a scratch worktree and run
   `make checked-input`.

Tag the tests `#[trace("TC-745", "FR-270-AC-n")]`.

## Expected Results

- Step 1: five runs, each exiting non-zero with one `signature` finding
  naming the planted file and line, the entry and the type.
- Step 2: two `reconstruction` findings naming the function and the called
  stage function; no finding for the `spine` plant.
- Step 3: no finding for the three look-alikes; two findings for the two
  plants together.
- Step 4: no finding and exit 0; the make target exits non-zero.
