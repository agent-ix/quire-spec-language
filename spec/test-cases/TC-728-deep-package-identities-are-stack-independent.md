---
id: TC-728
title: "A deep package's identities do not depend on the stack, and byte limits report as limits"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-259
    type: verifies
---
# TC-728: A deep package's identities do not depend on the stack, and byte limits report as limits

## Description

Verify that identities over a 100,000-deep package are minted without
native recursion, equal those minted on a large stack, and that a byte
overrun is reported as the calling stage's byte limit.

Scope: FR-259-AC-1, FR-259-AC-2, FR-259-AC-4.

## Test Procedure

1. On a thread with a 512 KiB stack, compile a package holding a
   100,000-term sum under limits raised to fit it, and read its checked
   package identity, every node key and its v2 package identity.
2. Compile the same source on a thread with an 8 MiB stack and read the
   same identities.
3. Check a declaration whose preimage is longer than `s3.input_bytes`.
4. Drive the intake digest site of FR-260 and the observation digest site of
   FR-261 with a byte limit below the document's encoded length.
5. Check each FR-258 expression form at 4 and at 63 levels, a function with
   a parameter typed with 4 and with 100 nested `Option`s, and a chain of 4
   and of 30 records, at the default limits. Measure the deepest JSON
   array or object nesting over every node preimage of each checked package.
   Emit a package whose function body is a 4-level and a 100-level `and`,
   `else if` and `let` chain, and measure the same over its v2
   `identity_preimage`.

Tag the tests `#[trace("TC-728", "FR-259-AC-1")]`, `#[trace("TC-728", "FR-259-AC-2")]`, `#[trace("TC-728", "FR-259-AC-4")]`.

## Expected Results

- Steps 1 and 2: every identity is equal across the two threads.
- Step 3: `stage_limit_exceeded`/`input-bytes-exceeded` naming that bound,
  the preimage length and setting `s3.input_bytes`.
- Step 4: each site reports its own input-bytes limit with its setting, and
  no malformed-input cause.
- Step 5: for each pair, the deepest nesting at the shallow depth equals
  the deepest nesting at the deep depth.

## Status

Planned.
