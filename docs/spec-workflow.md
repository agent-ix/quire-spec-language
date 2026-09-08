# Quoin specification workflow status

Recorded 2026-09-07. The [requirements index](../spec/spec.md) contains 24 draft
artifacts: one master, one StR, four US, eleven FR, four NFR and three IT.
These document the existing syntax implementation and the remaining LC02–LC05
work. The full healthy/violating/refused-or-incomplete state workflow is still
required; syntax success does not satisfy that acceptance.

This opening count describes the original review checkpoint. LR02 adds FR-012,
NFR-005 and IT-004, a scoped TestMatrix, ten TCs and Plan-001. The bounded Rust
audit migration is implemented and locally verified; see the current
[remediation inventory](rust-verification-remediation.md). Historical review
counts and hosted observations below retain their original scope.

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

The owner selected base plus all seven Quoin analyses. The eight formal
SpecReview artifacts are linked from [the base review](../spec/reviews/base.md).
Reviews were performed on the pinned revision recorded in each artifact and
remain conditional on their findings. Human acceptance, TC coverage and a test
matrix are not implied. The optional gap-analysis semantic comparison was
declined and was not run.

Further dependent implementation waits for specification review and the required
shared-contract acceptance. Existing Rust tests and integration evidence remain
scoped to the behavior they actually exercised. No compiler code changed during
this authoring step.

## Authoring remediation from consumer feedback

FR-008 now traces the standard's FR-006/FR-012 and requires the precondition,
immutable-parameter and implication-activation examples identified by Agent C's
FS03 feedback. Interrupted evaluation must preserve observed antecedent events
without inventing consequent entry or a completed Boolean. These are proposed
requirements for the unimplemented evaluator, not current runtime capabilities.

FR-to-story relationships use the catalog's `traces_to` verb because its
`implements` verb denotes interface/contract fulfillment rather than requirement
lineage. IDs are unchanged. The common SpecReview authoring pack was fetched
once for the selected review set; the analyses and raw advice are now recorded.

The installed process manifest has SHA-256
`d08ce71c02ce871fed4c7fb5cea1af1feb5a0b1004f8458fd26deecc2aa7b720`.
Its ADR/Plan/Review/SpecReview/Standard entries appear in both `archetypes` and
`artifact_types`, accounting for the duplicate registrations within one module.
`quire schema --module <process-module> SpecReview` still returns its required
schema and body assertions. The registry diagnostics remain unresolved.

Hosted native CI also passed at the documentation checkpoint `4581641`:
https://github.com/agent-ix/quire-spec-language/actions/runs/34185330391.
This verifies the existing syntax implementation and fixture integrity, not
the proposed evaluator requirements.

## Review validation checkpoint

The eight analysis reports were validated with the explicit repository scope.
All 32 spec/review documents and the separate root code-review artifact were grammar-clean. Quire still emitted the
six installed-registry errors described above despite exit 0. Report bodies
have the required Summary and nonempty Findings tables. No catalog files were
changed. Raw coverage/advisor output and tool/module provenance are retained in
spec/reviews/data; numeric-threshold advice is distinguished from reviewer
method recommendations.

The shared Agent-IX [code/Rust review](../reviews/26-09-07-native-code-review.md)
used the owner's agent-skills/rust-review/SKILL.md content and its rust-style
idioms. Formatting, strict all-feature Clippy and all 21 integration tests passed
at a80a17d. A CLI non-UTF-8 argument panic, formatter-budget discrepancy and
traceability/idiom findings remain. No plan bundle existed at the gap-analysis
target-selection step; no plan-completion verdict was invented.

The owner subsequently required Rust remediation of all four Python verification
helpers, including the two CI paths. This reopens a bounded specification cycle.
The private LC01 campaign audit also requires a separate owner disposition for
fresh TypeSpec/Node producer qualification. Historical results retain their pins;
a Rust runner does not implicitly approve that external executable language.

## LR02 implementation and CI checkpoint

The bounded specify/review/plan cycle preceded Rust implementation. All eight
scoped reviews are in spec/reviews/rust-verification, with later reviewed layout,
budget, canonical-marker and manual-CI clarifications. All four Python audit
helpers and both CI invocations are replaced by Rust. Historical fixture bytes
remain unchanged. Plan-001 and its scoped matrix record actual local execution.

The real shared agent-skills/rust-review/SKILL.md was applied again. NFR-005's
canonical ix-trace-rs attributes take precedence over rust-style's old comment
convention. Quire resolves all 14 new test symbols; FR-012 has 11/11 backed ACs,
and nine executable TCs have bindings. TC-010 is manual inspection, explicitly
classified no_source_symbol. The original 21 untagged tests remain separate debt.

The owner subsequently required local CI until stable. The only owned hosted
workflow now exposes workflow_dispatch exclusively; the specification repository
has none. No hosted run was dispatched. Local formatter, strict Clippy, tests,
private-packet lane, release audit unit tests and separate-target build passed.
Future hosted execution needs access to the private shared trace macro. This
does not weaken the local gates or approve the external TypeSpec/Node producer.

## Native readiness cleanup — 2026-09-08

Plan-002 now resolves SR-009's native implementation findings. Requirement
changes a10ec80/afeeb20 received all eight reviews in f1f6c50; the narrow Error
trait compatibility amendment 5d0c9de received all eight addenda at 03dfc72.
Candidate cbccbb6 preserves runtime source df2d0b5 and corrects one multiline
trace attachment. SR-028 records actual Rust/code review, 38 default tests and
3 selected private tests passing, strict Clippy/rustdoc and a separate-target
build. Quire now binds 41/41 test symbols, 28/28 native ACs and TM-002's 9/9
cases. This supersedes the earlier native trace debt, while preserving its
historical reports. The catalog status-header conflict remains visible;
the final plan audit records its scoped disposition before private landing.
At that readiness review, linking/typechecking and FS02/FS03/FS05 adoption
remained future work. The later adoption below supersedes the decision status.

## LC02 internal adoption and evidence design — 2026-09-08

The owner adopted specification PR8 at
e897f810a7356d4ce8fd19026221ebda7b65596f for internal implementation.
The specification repository records that decision in commit 609790e.
Compiler PR7 merged at 2be73504edff1224d6396da15edca3775905d37b;
LR02 research issue #58 is closed with the Rust audit implementation and evidence.

LC02's IT-005, TM-003 and TC-020–029 cover the existing ten FR-005/006
acceptance criteria. SR-030–037 record the retained base plus seven analyses.
The cases remain planned: Quire reports TM-003 0/10 backed and the existing
41/41 Rust test symbols bound. No new runtime execution is claimed.
The actual shared typed-model/reference interface and qualified rule-model
realization remain prerequisites; A will specify its concrete native API and
resource limits against that shared interface before implementation.
Broader FS acceptance, independent B/C adoption and publication remain separate.
