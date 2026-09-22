---
id: TC-396
title: "S2 refuses unrepresented constructs and constructs owned by another family"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: verifies
---
# TC-396: S2 refuses unrepresented constructs and constructs owned by another family

## Description

Verify that S2 refuses the whole unit, and never yields a partial form, in
two cases (ADR-011 §2.3 E2). A construct with no `Expression` variant is
refused with `UnrepresentedConstruct`. A construct whose variant another
family owns is refused with `ForeignFamilyConstruct`.

Step 1 depends on FR-091-OQ-5. Step 2 depends on FR-091-OQ-1.

Scope: FR-091-AC-7, FR-091-AC-8.

## Test Procedure

Every fixture unit below starts with the complete-V1 header
(`language "ix:native" edition "1-draft";`) and one profile selection whose
alias is `v`.

1. For each body below, build an admissible unit whose one function body is
   that construct, and run S2: `"text"`; `decimal(1, 2)`; `a mod b`;
   `xs[0]`; `none`; `collect(x in c: x)`.
2. Do the same for `deref(r)`, `allInstances<M::T>(p)` and `pre(a)`.
3. Run S2 on the body `if a then b else a mod b`.

Tag the test `#[trace("FR-091-AC-7", "FR-091-AC-8", "TC-396")]`.

## Expected Results

- Every step-1 case refuses with cause `UnrepresentedConstruct` and names
  the construct's CST production and exact span.
- Step 2 refuses with cause `ForeignFamilyConstruct`. The owning family is
  `StateModel` for `deref` and `allInstances`, and `ProtocolClause` for
  `pre`. The refusal names the construct's span.
- Step 3's span is the `a mod b` sub-construct, not the whole body.
- No case returns a parsed unit.

## Status

Planned; no test backs this case.
