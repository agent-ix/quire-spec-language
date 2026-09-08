# Quoin specification workflow status

Recorded 2026-09-07. The [requirements index](../spec/spec.md) contains 24 draft
artifacts: one master, one StR, four US, eleven FR, four NFR and three IT.
These document the existing syntax implementation and the remaining LC02–LC05
work. The full healthy/violating/refused-or-incomplete state workflow is still
required; syntax success does not satisfy that acceptance.

## Authoring contract

Authoring follows the installed Quoin plugin 0.20.0 skills:

- `/home/peter/.claude/plugins/cache/quoin/quoin/0.20.0/skills/specify/SKILL.md`
  (SHA-256 `6f9075e93e35bb46dc23140bcfe648d9f56d621472f7f567cd0404c1c225cdbd`).
- `/home/peter/.claude/plugins/cache/quoin/quoin/0.20.0/skills/spec-review/SKILL.md`
  (SHA-256 `8c23b0f67e77d16a21a034885e7f78c835dff3c819ad9d7b23f121fe4489741d`).

The `quoin write` authoring pack was requested once for
`master-requirements,StR,US,FR,NFR,IT` using Quoin 0.23.1. It resolved the
organization to `agent-ix` from Git and selected the installed
`spec-artifacts-iso` module. Its manifest SHA-256 was
`f88971dfd4fa86e873c1216a8f607beee79258d2e88e78631a591050580249f0`.
The returned skeletons/schemas supply artifact structure; requirement prose and
examples are newly authored. Existing template grants are not reassigned.

## Observed validation

Quire 0.31.0 was run with this repository as its explicit scope:

```sh
quire validate --scope "$PWD" 'spec/**/*.md' --strict --summary --diagnostics-format json
```

All 24 documents are grammar-clean after wording corrections. A separate local
check found unique artifact identities and resolved every local Markdown file
link and frontmatter relationship target in the two Agent A spec trees.

The installed catalog still emits error-severity diagnostics despite exit 0:
duplicate ADR/Plan/Review/SpecReview/Standard registrations and a duplicate
`part_of` inverse for `contains` and `aggregates`. Selecting the exact ISO module
removes the duplicate archetype diagnostics but still emits `DuplicateInverseEdge`;
both verbs declare that inverse in its manifest. No catalog definitions were
changed, and these runs are not recorded as error-free validation.

## Review and implementation status

`/spec-review` is requested. Its required user selection of base, all, or a subset
of analyses remains pending. No formal `SpecReview` document, completed review,
human acceptance, formal TC coverage or test matrix is claimed by this record.
Existing freeform architecture notes are design inputs to that review.

Further dependent implementation waits for specification review and the required
shared-contract acceptance. Existing Rust tests and integration evidence remain
scoped to the behavior they actually exercised. No compiler code changed during
this authoring step.
