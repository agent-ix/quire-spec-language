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
3. Run the gate over one corpus that QSpec FR-336 rejects for its
   contract version, then another corpus QSpec FR-336 rejects for a missing
   required member.
4. Run over TC-880 step 1's corpus with `--capability CAP-Z`, an id the corpus does not declare.
5. Starting from the valid corpus in TC-880, create four invalid corpora:
   duplicate the root `contract_version` member in raw JSON; set a
   `coverage.primary` entry's `applicability.consumer_types` member to an
   out-of-domain value; remove the `expected.disposition` member from one
   vector; and serialize the otherwise valid corpus with non-canonical JSON
   whitespace. Run each through the QSL gate.
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
- Step 3: the QSL preflight refuses each corpus before vector execution, gives no verdict and exits 2; the schema diagnostic for the unsupported version includes `/contract_version`, and the missing-member diagnostic includes the validator's instance path.
- Step 4: `unknown_required_feature`/`unknown-feature` naming `CAP-Z`, no verdict, no vector executed, exit 2.
- Step 5: each corpus is refused before vector execution, with no verdict and
  exit 2. The duplicate-key diagnostic identifies `contract_version`; the
  out-of-domain and missing-disposition schema diagnostics include their
  validator instance paths; the non-canonical encoding diagnostic identifies
  that the corpus bytes are not canonical JSON.
- Step 6: usage refusal naming the offending operand; no verdict, no
  vector executed, exit 2. A valid earlier operand starts no stage.
- Step 7: cause `empty-scope`, locus the resolved qualification scope,
  zero subject invocations, no verdict or coverage report, exit 2 for
  every corpus and output mode. No complete-V1 qualification is claimed.
- Step 8: the sole capability `CAP-F` is `uncovered`, coverage is zero,
  verdict is `not-qualified`, and exit is 1, with no subject invocation.
