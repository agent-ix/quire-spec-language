---
id: TC-881
title: "A scoped run's verdict names its scope, and a refused run has no verdict"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-353
    type: verifies
---
# TC-881: A scoped run's verdict names its scope, and a refused run has no verdict

## Description

Verify that a verdict over a caller scope names that scope and is not a complete-V1 verdict, and that each run the `qualify` gate refuses before any vector exits 2 with no verdict.

Scope: FR-353-AC-3, FR-353-AC-4.

## Test Procedure

Every step runs `cargo run --package xtask -- qualify --corpus <dir>`, adding the `--capability` arguments the step names.

1. Over TC-880 step 2's corpus, run with one `--capability` per capability except the failing one.
2. Run over a corpus location that does not exist.
3. Run over TC-880 step 1's corpus with `contract_version` set to `quire.conformance-corpus/v0`; then with one vector's expected disposition removed.
4. Run over TC-880 step 1's corpus with `--capability CAP-Z`, an id the corpus does not list.

Tag the tests `#[trace("TC-881", "FR-353-AC-n")]`.

## Expected Results

- Step 1: `qualified` for the named scope, no `complete-v1: qualified` line, exit 0.
- Step 2: the I/O refusal, no verdict, exit 2.
- Step 3: a refusal naming `/contract_version`, then one naming the missing member; no verdict, no vector executed, exit 2 each.
- Step 4: `unknown_required_feature`/`unknown-feature` naming `CAP-Z`, no verdict, no vector executed, exit 2.
