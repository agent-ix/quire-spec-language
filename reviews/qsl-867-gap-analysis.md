---
id: SR-202610310140
title: "Gap analysis — QSL-867 hyper typer unification"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; QSL-867, FR-172-AC-6, TC-597, qsl-semantics/src/check/check.rs, qsl-semantics/src/check/hyper_clause.rs, qsl-semantics/src/check/hyper_clause/lowering.rs, qsl-semantics/src/check/hyper_typing.rs, qsl-semantics/src/check/mod.rs, qsl-semantics/tests/it/hyper_clauses.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-172
    type: references
---

## Summary

The scoped manual trace check found FR-172-AC-6 tagged in the new parity test and a remaining evidence gap: its required refusal code/tag/locus are not asserted. Repository-wide matrix and reverse-gap checks could not complete because both installed matrix commands remained silent and running for over two minutes.

## Verdict

**CONDITIONAL** — the scoped AC-6 gap is concrete; repository-wide coverage remains unverified because the matrix commands did not return.

## Coverage

- `FR-172-AC-6` examined against the new `relation_and_hyper_atom_bodies_share_value_typer_refusals` test. The `#[trace("TC-597", "FR-172-AC-6")]` binding is present and the test exercises rational range, ambiguous literals, and root shadowing.
- `TC-597` step 6 examined: its expected result requires exact refusal code and cause plus the corresponding source location for the value-clause, relation, and hyper cases.
- Changed Rust files manually checked for stubs and behavior lacking an owning requirement; no additional reverse gap was identified in the changed code.
- Static matrix verification was unavailable: `quoin matrix --repo . --json` (Quoin 0.29.1) and `quire matrix --scope . --format json` (Quire 0.37.1) produced no completed output and remained running for more than two minutes; both were interrupted with Ctrl-C (exit 130). The first Quire stderr included duplicate archetype/inverse-label diagnostics, but no matrix result.
- Repository-wide reverse-gap inventory is not claimed because matrix analysis did not finish. No plan was supplied; Plan completion: not assessed.
- Optional semantic review was skipped; it was not requested.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | The AC-6 parity test is tagged, but it does not assert the required refusal codes, tags, or exact loci for `RationalRange` and `AmbiguousLiteral`; TC-597 step 6 requires those outputs. | FR-172-AC-6 | correct-requirement-no-evidence |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The regression test now checks the required codes, tags, and exact spans. |
