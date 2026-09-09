---
id: TC-090
title: "Qualify native canonical bytes and domain vectors"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-021
    type: verifies
---
# TC-090: Qualify native canonical bytes and domain vectors

## Description

Property, priority P1. Verifies FR-021-AC-1 and FR-021-AC-6. Qualified at
c195950; SR-111 records actual evidence. The oracle is independently
authored fixed byte data and Rust hash assertions, not a second invocation of
the production canonicalizer.

## Test Procedure

Use exact independently authored canonical-content byte fixtures for actual
checked source with an imported model and a constant clause, followed by a
source with multiple checked clauses. A header-only source cannot pass native
parsing and must not supply positive setup. Enumerate every fixed record member
in the documented order. Keep the true fixture/implementation sequence in the
qualification report: correcting a setup error is not a previously passing oracle.
Include quote, backslash, slash, b/f/n/r/t controls, another U+0000–001F control,
non-ASCII and supplementary Unicode in admitted opaque native source labels;
use the exact positive formal revision 9007199254740993 without a float cast.
Compute the domain-prefixed SHA-256 expectation from these independent literal
bytes using the existing Rust hash primitive, and compare production content
and digest separately. Record fixture provenance and expected values.

Change display paths and runtime populations/budgets while keeping the static
source/model/authored bindings fixed. Independently mutate the preimage by
omitting each NUL/domain segment, adding a final newline, sorting object keys
globally, escaping slash, changing Unicode normalization and rounding the large
revision. Keep the valid native package setup separate from malformed vectors.

## Expected Results

Production canonical bytes equal the independent fixtures and the exact named
preimage produces their recorded digests. Each nonconforming encoding/preimage
control differs from its expectation. Display paths, runtime data and evaluator
budgets do not change either canonical content or native identity. A matching
raw artifact hash or an encoder/reader round trip alone cannot pass this case.
