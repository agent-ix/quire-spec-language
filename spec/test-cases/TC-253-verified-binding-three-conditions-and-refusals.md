---
id: TC-253
title: "The verified binding admits VerifiedPackage only under all three conditions, refusing each failure independently"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-087
    type: verifies
---
# TC-253: The verified binding admits VerifiedPackage only under all three conditions, refusing each failure independently

## Description

Verify `library`'s verified binding (ADR-011 §4): a `VerifiedPackage` is
constructed from v2 wire bytes only when all three named conditions hold —
(1) the schema version is a supported v2 version, (2) the recomputed
FR-322 `package_id` (the `quire.package.semantic/v2` digest of the JCS
bytes of the read `identity_preimage`) lexically equals the declared
`package_id`, and (3) that identity is listed in the consumer's library
lock or pinned request — and that each of the three failure modes refuses
independently, with a named cause and no partial output, never falling
back to a digest of the file bytes, the lock file, or the source. Scope:
FR-087-AC-3.

## Test Procedure

1. Construct a valid v2-encoded package whose declared `package_id` equals
   the digest recomputed over its own `identity_preimage`, at a supported
   schema version, and listed in the test's library lock. Confirm `library`
   admits it as a `VerifiedPackage`.
2. Condition 1 (version): construct the same package at an unsupported
   schema version, all else held equal. Confirm `library` refuses with a
   named "unsupported version" cause and produces no `VerifiedPackage`
   (not a partial one with the unsupported version silently accepted).
3. Condition 2 (digest equality): construct the same package with its
   declared `package_id` field altered so it no longer equals the
   recomputed digest over the identity preimage, all else held equal.
   Confirm `library` refuses with a named digest-mismatch cause and
   produces no `VerifiedPackage`.
4. Condition 3 (lock/request membership): construct the same package,
   valid version and matching digest, but omit its identity from the
   library lock and pinned request. Confirm `library` refuses with a named
   "not listed" cause and produces no `VerifiedPackage`.
5. Adverse test: construct a package whose file bytes' own digest (not the
   `identity_preimage` digest) happens to equal some accepted value, while
   the actual recomputed `package_id` does not match the declared one;
   confirm `library` still refuses (the file-byte digest never substitutes
   for the `identity_preimage` digest).
6. Adverse test: construct a package whose lock-file digest matches some
   accepted value while the recomputed `package_id` does not; confirm
   `library` still refuses (the lock-file digest never substitutes).
7. Adverse test: construct a package whose source digest matches some
   accepted value while the recomputed `package_id` does not; confirm
   `library` still refuses (the source digest never substitutes).
8. Confirm each of steps 2-4 fails independently: combining two failure
   conditions in one input still refuses cleanly (not a crash, not an
   admission, no partial `VerifiedPackage`), and the refusal's cause
   identifies at least one of the failing conditions. This step does not
   require the refusal count itself to be exactly one — FR-087-AC-3 states
   no such count — only that the input is refused, not admitted or
   silently accepted in part.
9. Graph order (QSpec FR-322-AC-14): read two v2 wires that hold the same
   two declaring nodes, one in ascending and one in descending node-id
   order, each pinned at its own recomputed `package_id`. Under
   `make conformance`, read every QSpec positive fixture
   (`$QSPEC_DIR/proposals/checked-package-v2/fixtures/positive-*.json`)
   through the whole I2 read. Then check an identity preimage that repeats a
   node id at adjacent positions, and one that repeats it two positions
   apart (`[R, S, R]`).
10. E4 dependency binding (FR-087-AC-14): link a graph with an import whose
    recorded `package_id` is not its package's, and link a graph importing
    `test/units` at version `3` and a package `mid` that selected
    `test/units` at version `2`.

## Expected Results

- Step 10: the first link refuses `DependencyIdentityMismatch`
  (`stale_dependency`) naming the recorded and the recomputed `package_id`;
  the second refuses `invalid_package`/`conflicting-definition` naming both
  selections of `test/units`. Neither yields a package.

- Step 1: the valid input is admitted as a `VerifiedPackage`.
- Steps 2-4: each of the three conditions, failed independently, produces a
  named refusal and no `VerifiedPackage`; an admission (partial or full)
  for any of these inputs fails this test.
- Steps 5-7: no alternate digest (file bytes, lock file, source) is ever
  accepted as a substitute for the `identity_preimage` digest; an admission
  in any of these three cases fails this test.
- Step 8: a doubly-failing input still refuses cleanly, with no partial
  output and no crash; the number of refusals recorded is not itself
  scored by this step.
- Step 9: both graph orders are admitted as a `VerifiedPackage`, under two
  different `package_id`s, with the same exports; every positive fixture is
  admitted, at least one of them with a non-ascending `identity_projection`;
  each repeated node id refuses as `DuplicateNode`, naming the repeated id
  and the index of its second occurrence.
