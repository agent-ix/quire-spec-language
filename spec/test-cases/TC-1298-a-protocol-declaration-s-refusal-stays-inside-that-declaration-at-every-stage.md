---
id: TC-1298
title: "A protocol declaration's refusal stays inside that declaration at every stage"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-503
    type: verifies
---
# TC-1298: A protocol declaration's refusal stays inside that declaration at every stage

## Description

Verify FR-503-AC-1 to AC-6 and FR-002-AC-9: S1 confines a syntax error to its
protocol declaration, S2 and S3 attribute their refusals by locus, the
operations continue for clean declarations, and the failure carries
`protocol_results` without changing its category.

Catches an S1 recovery that spoils a neighbour's CST, a refusal located in a
preceding source, a compile that stops at the first refusal, a payload that
changes the exit, and a non-protocol refusal marked on a protocol.

Fixtures: **W**, **X**, **Z** are TC-1297's (X mutated per row). Declarations are
in one source unless a row says two.

## Test Procedure

| # | Unit and operation | Expected |
| --- | --- | --- |
| 1 | `qsl_cst::parse` over W, X, Z with X missing its closing brace before Z's `protocol` keyword | recovered `ParsedSource`; `invalid_syntax` attached to X's span; W's and Z's CST equal their CST in W, Z alone |
| 2 | `parse` over row 1's unit | refused at S1; results [W `Cleared{S2}`, X `Refused` `invalid_syntax`, Z `Cleared{S2}`] |
| 3 | `parse` over a unit with a syntax error in the header; over a unit whose last brace is missing so X runs to the end of the source | refused, no `protocol_results` each |
| 4 | `compile` over row 1's unit | results [W `Emitted`, X `Refused`, Z `Emitted`]; W's and Z's nodes byte-identical to compiling each alone; category `refusal`, exit 20; no package |
| 5 | X with an S2 form refusal (a recovery node in its own subtree only); then X with an S3 refusal; each in a two-source unit where X is in the second source | X `Refused` with the second source's index and X's span; W `Emitted`; failure code is X's |
| 6 | `check` over a unit with X refused at S3 | results [W `Cleared{S3}`, X `Refused`]; category and exit equal X's cause alone |
| 7 | `check` under an `s3.work_units` budget of exactly W's work, units W, Z, V (equal work) | `StageFailure::Limit`, results [W `Cleared{S3}`, Z `Incomplete`, V `Incomplete`], exit 22 |
| 8 | a refused state clause in a unit holding W, through `compile` and through `check` | W `Emitted`; W `Cleared{S3}`; never `Refused` |

## Expected Results

- Each row asserts results, loci, category and exit as fields.
- The payload is removed from each failure in rows 4, 6 and 7 and the category
  and exit are unchanged.
