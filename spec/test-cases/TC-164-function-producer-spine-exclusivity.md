---
id: TC-164
title: "The checked-package spine is the only function producer; format retargets to the CST"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-065
    type: verifies
---
# TC-164: The checked-package spine is the only function producer; format retargets to the CST

## Description

Verify that native `run` and `compile` no longer produce a checked package
or backend artifact through a native-v1 path, that `package::NativePackage`,
the `lower` command and wire form `native-linked-package/1` are absent from
the repository, and that `format` operates over the CST rather than a
native-v1 parse. Scope: FR-065-AC-5 and FR-065-AC-6.

## Test Procedure

1. Search the compiled crate's public symbols for `package::NativePackage`
   and for a reader or writer of wire form `native-linked-package/1`; search
   the CLI's command table for a `lower` command.
2. Instrument the checked-package-spine entry function (S1-S4) and the
   deleted native-v1 producer function's former call site; invoke `run` and
   `compile` to produce a package and a backend artifact respectively.
3. Construct a source file with a construct that only the deleted native-v1
   parser rejected but whose CST is well-formed; invoke `format` on it.

## Expected Results

- Step 1: none of `package::NativePackage`, a `native-linked-package/1`
  reader/writer, or a `lower` command is found.
- Step 2: the spine entry function is called at least once for each of `run`
  and `compile`; the deleted native-v1 producer function is never called
  (its absence from the compiled symbols makes this a compile-time fact, not
  only a runtime observation).
- Step 3: `format` succeeds, showing it consumed the CST rather than
  rejecting the input the way the deleted native-v1 parser would have.
