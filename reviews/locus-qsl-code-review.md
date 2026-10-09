---
id: SR-2454
title: "Code review of the QSL certificate-locus wire conformance at 84fba37c8 (spec-only diff)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@84fba37c8a342218e99aa0f2cb66b89204e3e879; diff against origin/main b24dbda01d56843cd14d8cab9233ec3cff67535f: spec/functional/FR-314-check-an-smt-proof-certificate.md, spec/functional/FR-338-check-an-en-1-closure-certificate.md; context: unmerged branch task/490-e4-proof-result-types qsl-replay/src/certificate.rs, qsl-replay/src/outcome.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-314
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-338
    type: reviews
---
# Code review of the QSL certificate-locus wire conformance

## Summary

Prepublication review, no PR and no ticket assigned yet. PR: pending. The
diff touches two spec files and no code: no `.rs`, `Cargo.toml`,
`spec/tests.md` or test file. The rust-review lane does not apply. Nothing
in the diff adds a pin, digest, SHA, version record or vendored copy, so
nothing fails the repository's ceremony rule. The diff is 8 lines of
Markdown inside existing sections, and the frontmatter is unchanged.

The unmerged E4 branch was read for context only; it is not part of this
diff. At `task/490-e4-proof-result-types`, `CertificateLocus` has no
`ProductState` variant, and its `Serialize` refuses `ProofStep` with "QSpec
FR-331 defines no wire spelling for a proof-step certificate locus". Once
this contract merges, that writer must emit `{part, index}`, with `index` as
a decimal string. That work belongs to E4's coder and is recorded here only
as a downstream consequence.

## Verdict

PASS for code. There is no code in the diff. The spec defects are in
SR-2452 and SR-2453.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
