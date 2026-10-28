---
id: TC-471
title: "S3 routes accepted execution points by authored state clause"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-104
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-104-AC-15
    type: verifies
---
# TC-471: S3 routes accepted execution points by authored state clause

## Description

Verify the supported S3 check input resolves accepted execution points to
authored clauses without inferring missing bindings.

Scope: FR-104-AC-15.

## Test Procedure

1. Using TC-459 step 7's package, add an invariant `Init` and an invariant
   `Handle` on `Config::Flow`, plus a `pre` clause `Before` and a `post` clause
   `After` on `Config::Flow::go`.
2. Submit the S3 check input with entries keyed by the exact authored names
   `Init`, `Handle`, `Before` and `After`, carrying their accepted
   `ClauseBinding` execution points. Also include an otherwise valid clause
   `Unbound` with no entry.
3. Repeat with the entry for `Before` omitted. Repeat with an entry keyed by
   `NotAuthored` and no corresponding clause.

Tag the tests `#[trace("TC-471", "FR-104-AC-15")]`.

## Expected Results

- Step 2: each supplied point is attached unchanged to the checked result for
  its exact authored clause. `Unbound` has no execution point; S3 does not
  infer one from its kind or context.
- Step 3, omitted entry: `Before` has no accepted point and no point is
  synthesized.
- Step 3, unknown key: S3 refuses `missing_declaration`/`missing-name` at
  `NotAuthored` before lowering or minting clause identity; no checked clause
  or package is produced.
