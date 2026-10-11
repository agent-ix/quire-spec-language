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

Scope: FR-353-AC-3, FR-353-AC-4, FR-353-AC-5.

## Test Procedure

Every step runs `cargo run --package xtask -- qualify --corpus <dir>`, adding the `--capability` arguments the step names.

1. Over TC-880 step 2's corpus, run with one `--capability` per capability except the failing one.
2. Run over a corpus location that does not exist.
3. Run over TC-880 step 1's corpus with `contract_version` set to `quire.conformance-corpus/v0`; then with one vector's expected disposition removed.
4. Run over TC-880 step 1's corpus with `--capability CAP-Z`, an id the corpus does not list.
5. Make three independent corpus mutations: duplicate a required key,
   replace a closed-domain value with a value outside that domain, and
   encode a valid document non-canonically under QSpec FR-336. Run each.
6. Over the valid corpus, run separately with `--limit s9.nodes=1`,
   `--limit s3.nodes=ten`, `--limit s3.nodes=-1`, and two occurrences of
   `--limit s3.nodes=5`. Observe the subject invocation boundary in every
   refusal case, including a bad operand after a valid operand.
7. Create three valid corpora using TC-877's outside-default-scope
   capabilities: only monitor-only `CAP-F`, only `CAP-G` with its profile
   absent from QSL's catalog, and both together. Run each with no
   `--capability` operand in JSON and text modes, observing subject
   invocation counts. Give their recorded results passes.
8. Repeat step 7's mixed corpus with `--capability CAP-F` in both output
   modes, so the selected scope is nonempty but has no applicable vectors.

Tag the tests `#[trace("TC-881", "FR-353-AC-n")]`.

## Expected Results

- Step 1: `qualified` for the named scope, no `complete-v1: qualified` line, exit 0.
- Step 2: the I/O refusal, no verdict, exit 2.
- Step 3: a refusal naming `/contract_version`, then one naming the missing member; no verdict, no vector executed, exit 2 each.
- Step 4: `unknown_required_feature`/`unknown-feature` naming `CAP-Z`, no verdict, no vector executed, exit 2.
- Step 5: each corpus validation refusal names its cause and locus; no
  verdict, no vector executed, exit 2.
- Step 6: usage refusal naming the offending operand; no verdict, no
  vector executed, exit 2. A valid earlier operand starts no stage.
- Step 7: cause `empty-scope`, locus the resolved qualification scope,
  zero subject invocations, no verdict or coverage report, exit 2 for
  every corpus and output mode. No complete-V1 qualification is claimed.
- Step 8: the sole capability `CAP-F` is `uncovered`, coverage is zero,
  verdict is `not-qualified`, and exit is 1, with no subject invocation.
