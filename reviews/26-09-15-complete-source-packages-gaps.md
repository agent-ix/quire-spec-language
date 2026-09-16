---
id: SR-454
title: "Complete source packages gap analysis"
type: SpecReview
analysis: gap-analysis
scope: "quire-spec-language#117; Task-047; FR-131/134/302/303/339; TC-180/184/222; src/complete; tests/complete_*"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/Task-047
    type: reviews
  - target: ix://agent-ix/quire-specification/FR-131
    type: reviews
  - target: ix://agent-ix/quire-specification/FR-134
    type: reviews
  - target: ix://agent-ix/quire-specification/FR-302
    type: reviews
  - target: ix://agent-ix/quire-specification/FR-303
    type: reviews
  - target: ix://agent-ix/quire-specification/FR-339
    type: reviews
  - target: ix://agent-ix/quire-specification/TC-180
    type: reviews
  - target: ix://agent-ix/quire-specification/TC-184
    type: reviews
  - target: ix://agent-ix/quire-specification/TC-222
    type: reviews
---

## Summary

The final `/gap-analysis` finds QSL #117 complete for Task-047. Complete-V1
source parsing, exact profile/model/package selections, the dependency-closed
FR-131 graph, lossless CST identity, recovery separation, incremental edits,
formatting and editor diagnostics are implemented and exercised at their real
Rust boundaries. Later checked-reader, diagnostic-catalog, manifest, extension
and checked-package authorities remain explicitly allocated to Task-054/#123.

## Verdict

**PASS** — no scoped requirement, implementation, test, traceability, stub,
allocation or reverse-trace gap remains.

## Coverage

The scoped corpus binds FR-131/134/302/339 and TC-180/184/222 to substantive
Rust tests. FR-303 is intentionally foundation-only here and is traced to
Task-047 rather than falsely claiming an acceptance criterion or concrete
TC-223 result. Package tests execute as crate-local tests because semantic
constructors require an unforgeable crate-issued reader proof; public
source/editor tests exercise the authority-free exact-reference catalog.

Npm Quire 0.32.0 reports repository-wide coverage of 502/524 matrix rows. Its
remaining rows, unmatched tags and suspicions predate or lie outside #117; the
manual scoped reconciliation and independent semantic/Rust reviews found no
unbacked #117 obligation. Native Quoin 0.23.1 does not yet expose `coverage`
([quoin#538](https://github.com/agent-ix/quoin/issues/538)); the permitted npm
fallback supplied this census. Native `quoin validate --repo . --strict`
reports no repository finding.

## Reverse trace

| Changed behavior | Owning requirement | Executing evidence |
| --- | --- | --- |
| Complete exact definition closure, required facets/capabilities, backend-independent identity and located refusal | FR-131 | crate-local `complete::package_tests`; TC-180 |
| Exact edition/profile/import/model syntax and declarative complete grammar | FR-131, FR-339 | `complete_grammar`; TC-180 |
| Byte-exact source/CST, separate recovery and stable revision-bound identities | FR-302 | `complete_cst`; TC-222 |
| CRLF/LF/final-newline source-location preservation | FR-134 | `complete_editor`, `source_map`; TC-184 |
| Incremental/formatter/editor foundations under exact catalog and caller limits | FR-303 foundation | `complete_editor` and `complete_cst`, tagged Task-047; concrete checked-package TC-223 remains Task-054 |
| Existing historical/composed lexing remains unchanged | compatibility boundary in #117 | `parser`, `complete_cst`, full all-feature suite |

Source stubs: 0. Test stubs: 0. Untraced scoped production behavior: 0.
Caller-mintable checked semantic authorities: 0. The only reader-proof mint is
crate-private and test-only; Task-054 must connect the concrete checked reader.

## Allocation reconciliation

The review found and corrected an upstream allocation drift before recording
PASS. QSpec PR
[#67](https://github.com/agent-ix/quire-specification/pull/67) (merge
`56dd9315929ce870bc9ab2b799d7a11cedfdca43`) now assigns V1-SRC-010/011/013/014
and V1-EXPR-023 primary delivery to QSL #123. V1-EXPR-023 now uses TC-047 for
typed diagnostic classification while retaining TC-231 final qualification.
The central manifest, matrix, local Plan-013 and executable allocation fixture
therefore agree.

## Gates

| Gate | Result |
| --- | --- |
| Native `quoin validate --repo . --strict` | pass |
| Npm `quire coverage --scope .` | 502/524 repository-wide; no scoped #117 gap |
| Focused complete package/CST/grammar/editor and source-map suites | pass |
| Full all-target/all-feature Rust suite, serial | pass; only documented environment-dependent examples/private-packet tests ignored |
| Strict all-target/all-feature Clippy | pass |
| All-feature doctests | 12/12 pass |
| Formatting and diff hygiene | pass |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No scoped implementation, allocation or traceability gap remains after remediation. | #117; Task-047; QSpec #67 |
