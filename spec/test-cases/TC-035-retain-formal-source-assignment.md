---
id: TC-035
title: "Retain explicit native and formal source assignment"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-014
    type: verifies
---
# TC-035: Retain explicit native and formal source assignment

## Description

Integration, P1; verifies FR-014-AC-1 through the actual IR source constructors.

## Test Procedure

Read verified native bytes with identity ix://example/source and opaque revision
draft:alpha. Bind them to formal document NativeClause and positive revision 91.
Drop the caller's Source handle and inspect the retained binding. Repeat using
two other independently selected positive formal revisions, including u64::MAX.

## Expected Results

The binding retains the exact text, path, native labels, byte digest and selected
formal identity. No relationship between the revision spellings is inferred.
IR constructors refuse zero revision and an invalid document identifier before
either can be supplied to the binding.
