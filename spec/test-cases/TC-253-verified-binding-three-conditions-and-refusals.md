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
(1) the schema version is a supported v2 version, (2) the package's preimage
is admitted (exercised by TC-282's `InvalidPreimage` and `UndeclaredExport`
fixtures), and (3) that identity is listed in the consumer's requested set —
and that a failure of condition 1 or 3 refuses independently, with a named
cause and no partial output, never falling back to the file bytes or the
source. Scope: FR-087-AC-3.

## Test Procedure

1. Construct a valid v2-encoded package at a supported schema version, with
   an admitted preimage, and listed in the test's requested set. Confirm
   `library` admits it as a `VerifiedPackage`.
2. Condition 1 (version): construct the same package at an unsupported
   schema version, all else held equal. Confirm `library` refuses with a
   named "unsupported version" cause and produces no `VerifiedPackage`
   (not a partial one with the unsupported version silently accepted).
3. (Retired: the lock-content equality check is withdrawn.)
4. Condition 3 (requested-set membership): construct the same package,
   valid version, but omit its identity from the requested set. Confirm
   `library` refuses with a named "not listed" cause and produces no
   `VerifiedPackage`.
5. (Retired: withdrawn with the lock-content comparison.)
6. (Retired: withdrawn with the lock-content comparison.)
7. (Retired: withdrawn with the lock-content comparison.)
8. Confirm each of steps 2 and 4 fails independently: combining two failure
   conditions in one input still refuses cleanly (not a crash, not an
   admission, no partial `VerifiedPackage`), and the refusal's cause
   identifies at least one of the failing conditions. This step does not
   require the refusal count itself to be exactly one — FR-087-AC-3 states
   no such count — only that the input is refused, not admitted or
   silently accepted in part.
9. Graph order (QSpec FR-322-AC-14): read two v2 wires that hold the same
   two declaring nodes, one in ascending and one in descending node-id
   order, each read under its own identity and version. Under
   `make conformance`, read every QSpec positive fixture
   (in place from the QSpec checkout at
   `proposals/checked-package-v2/fixtures/positive-*.json`)
   through the whole I2 read. Then check a node list that repeats a
   node id at adjacent positions, and one that repeats it two positions
   apart (`[R, S, R]`).
10. E4 dependency binding (FR-087-AC-14): link a graph with an import of a
    package, and link a graph importing a package as `test/units` and a
    package `mid` that selected a different package as `test/units`.

## Expected Results

- Step 1: the valid input is admitted as a `VerifiedPackage`.
- Steps 2 and 4: each of conditions 1 and 3, failed independently, produces a
  named refusal and no `VerifiedPackage`; an admission (partial or full)
  for any of these inputs fails this test.
- Steps 3 and 5-7: retired.
- Step 8: a doubly-failing input still refuses cleanly, with no partial
  output and no crash; the number of refusals recorded is not itself
  scored by this step.
- Step 9: both graph orders are admitted as a `VerifiedPackage`, as two
  distinct packages, with the same exports; every positive fixture is
  admitted, at least one of them with a non-ascending `identity_projection`;
  each repeated node id refuses as `DuplicateNode`, naming the repeated id
  and the index of its second occurrence.
- Step 10: the first link records the import's identity and version as
  the selection of its identity; the second refuses `invalid_package`/`conflicting-definition` naming both
  selections of `test/units` and both dependency paths (`test/units` and
  `test/mid -> test/units`). Neither yields a package.
