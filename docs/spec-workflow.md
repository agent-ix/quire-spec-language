# Quoin specification workflow status

The sections below retain their historical revisions and run evidence. For
current matrix status interpretation and the verified candidate tool/module
stack, see [matrix status checks](matrix-status.md). Bound tags, classified
statuses and executed tests are separate observations.

## LC02 native package review — 2026-09-09

The [native package specification](../spec/native-packages/index.md) is authored
at 9a80805 with the canonical control-escape clarification at
41da6e5eb86bb727fbad5370fd23b38d330bcfea. It covers FR-019/020/021,
NFR-007, IT-007 and TM-005's 14 planned cases for 27 functional criteria.
The owning ticket is tracked privately; the authoring branch starts from
merged runtime 789c636. No package implementation preceded review.

Agent A used the actual installed QUOIN 0.22.5 specify, spec-matrix,
spec-review and all seven analysis skills from the installed Quoin
plugin 0.22.5 skills cache. The authoring pack
was fetched once for FR,NFR,IT,TC,TestMatrix,SpecReview; the installed catalog
resolved agent-ix and supplied the discrete artifact skeletons/schemas.
No applicable AssuranceProfile was found. The existing owner choice is all
eight analyses; the optional semantic gap pass remains declined.

[SR-101–108](../spec/reviews/native-packages/index.md) record the actual serial
reviews at 41da6e5 and PASS for planning/implementation of this slice. Draft
repairs define format-before-version-shape ordering, exact native canonical
input, cyclic feature traversal, independent vectors and honest per-pass
accounting. Native input, static identity, raw bytes and projection availability
retain distinct roles. No independent B/C acceptance is asserted.

Strict Quire 0.31.0 validation reports 218/218 specification documents clean
before review and 226/226 after the eight reports. Raw adviser/catalog/coverage
outputs and the sandboxed adviser failure are retained beside the reviews.
All 27 FR methods match at Test-class level; five benchmark recommendations for
exact resource limits have explicit negative-abuse-testing dispositions.
Coverage still binds 205/205 existing Rust symbols, while all 41 new functional/
case rows remain unbacked. Twenty-two classifier and six registry diagnostics
remain disclosed, including the matrix header mismatch and historical IT-004
tags. The schema has had JSON syntax and structural inspection only; actual
Rust Draft 2020-12 qualification is planned in TC-083.

[Plan-007](../plan/Plan-007-native-packages/index.md) now carries three serial
tasks: construction/identity, verified reconstruction and workflow qualification.
It was authored using QUOIN spec-to-plan after all eight reviews, with the
installed Plan/Task schemas and index/log skeletons. Strict validation reports
233/233 scoped specification/review/plan documents grammar-clean; all tasks
remain not_started. The next executable work is Task-016's independent Rust
fixtures and genuine missing-API test, followed by implementation.
Static native-domain
registration, FS05 consumption, compiled ConfigVersion/backend
parity and Quire integration are tracked as separate full-assignment work. No Cargo
build, additional agent, hosted dispatch or public posting ran in this review.

Recorded 2026-09-07. The [requirements index](../spec/spec.md) contains 24 draft
artifacts: one master, one StR, four US, eleven FR, four NFR and three IT.
These document requirements spanning syntax, linking, checking and runtime
evaluation as drafted so far. The full
healthy/violating/refused-or-incomplete state workflow is still
required; syntax success does not satisfy that acceptance.

This opening count describes the original review checkpoint. LR02 adds FR-012,
NFR-005 and IT-004, a scoped TestMatrix, ten TCs and Plan-001. The bounded Rust
audit migration is implemented and locally verified; see the current
[remediation inventory](rust-verification-remediation.md). Historical review
counts and hosted observations below retain their original scope.

## Authoring contract

Authoring follows the installed Quoin plugin 0.20.0 skills:

- `specify/SKILL.md` from the installed Quoin plugin 0.20.0 skills cache
  (SHA-256 `6f9075e93e35bb46dc23140bcfe648d9f56d621472f7f567cd0404c1c225cdbd`).
- `spec-review/SKILL.md` from the installed Quoin plugin 0.20.0 skills cache
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
Fresh TypeSpec/Node producer qualification also requires a separate owner
disposition. Historical results retain their pins;
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
At that readiness review, linking, typechecking and shared-evidence adoption
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
A will specify its concrete native API and resource limits against the actual
shared typed-model/reference interface and qualified rule-model realization.
Broader FS acceptance, independent B/C adoption and publication are tracked
as separate work.

## Current ownership correction — 2026-09-08

The owner clarified current work ownership. A's earlier request for a transfer
was based on stale records and is withdrawn. IR #54 belongs in the current
A/B/C work allocation. It does not assert that C has begun implementation
merely because A posted it.

This corrects work ownership only. The adopted semantics, IT-005 prerequisites,
ten planned LC02 cases and SR-030–037 technical findings are unchanged. Actual
adapter availability still requires code and qualification evidence; it is no
longer described as a dependency on an inactive work track.

## IR #54 resolution — 2026-09-08

C landed Contract IR PR61 at 690bde7f2dc58662cf9ff0595c2c0e3b17107c6f and
closed #54. Accepted ADR-0054 removes the incorrect universal Filament-model
reader dependency. FR-005, IT-005, TM-003 and TC-020–024 now describe native
resolution over public formal declarations. The existing FR-013/019/023 APIs
are the integration target; no external #54 delivery remains outstanding.

The native request/result API, resource policy and concrete source/revision
correspondence are A's next specification/implementation work. The selected
object-reference/state case remains required by the full goal, with its narrow
semantic mapping owned by A rather than inferred from generated datatypes.
Earlier conditional reviews are historical and are superseded for the adapter
prerequisite by the new boundary reviews; no planned TC is marked executed.

IR FR-019 and NFR-005 record Rust 1.98.1; ADR-0055 remains marked proposed at
the merged revision and the actual IR Cargo/toolchain files have not migrated.
Native compatibility runs explicitly select +1.98.1. They do not silently
reattribute older 1.94.1 results or claim upstream policy implementation.

## Native formal linker qualification — 2026-09-08

FR-013's concrete API was specified at c78792a and reviewed through all eight
QUOIN analyses at 8dc6b48 before implementation. The fixture corrections at
ecaf4cf use the actual IR PackageId namespace and nonreserved native count
field; all eight retained reviews re-evaluated that correction at 01e597d.
The initial setup failures are explicitly retained in the implementation review.

The actual Rust library now borrows exact IR environments and retains the owned
source, scoped declarations and typed formal provenance. Ten public linker tests
qualify TC-020–024/030–034; 48 default tests and three private audit tests pass
on Rust 1.98.1, alongside strict Clippy and rustdoc. Code::all now includes the
five specified linkage codes, and the legacy error propagation test checks
empty related/upstream context. (`related`/`upstream` since moved off
`Diagnostic` onto `LinkingError`/`CheckingError` themselves per ADR-011
§6.1; the historical record above describes the shape at the time this
entry was written, not the current one — see docs/native-error-codes.md.)
No CLI linking/typing/evaluation is implied.

Actual Quire coverage reports FR-005 5/5, FR-013 6/6 and TM-003 10/15 backed.
FR-006's five cases remain planned. Current catalog diagnostics also identify
uncatalogued historical NFR methods and broad property shapes; the global
coverage rollup does not qualify those methods or complete the full workflow.

Specification PR8 has separately merged at 8ab058b, with the B consumer handoff
posted. The compiler does not wait on B's complete reader or closed IR #54.
After the owner's resource report, all Cargo checks use one job, tests use one
thread, and build/check phases run serially at low process priority.

## Native formal source correspondence — 2026-09-08

FR-014 was specified at 4eb4ef6, reviewed with actual QUOIN base plus all seven
selected analyses at 257f787 (SR-056–063), and posted to private LC02 before code:
https://github.com/agent-ix/quire-spec-language/issues/3#issuecomment-5595066820.
Plan-004 at 6085159 preceded implementation 7464c9a. The first focused Rust run
failed on the missing module; five new public-API tests then passed. No new
dependency, execution language, model reader or shared identity domain was added.

SR-064 applies the actual user-selected agent-skills/rust-review/SKILL.md and
reports PASS for the bridge. SR-065 preserves the incomplete full LC02 matrix
and PR/landing task at its evaluated revision. The optional semantic gap review
remains declined. Full default tests: 53 passed; selected private audit lane:
3 passed; strict Clippy, fmt, rustdoc and cached separate minimal build pass.
Quire reports FR-014 5/5 and TM-003 15/20 backed, with zero status lies or
untracked symbols and 18 retained catalog diagnostics. FR-006 remains 0/5.
Validation reports 156/156 scoped spec/plan/review documents grammar-clean.

The explicit FormalSource binding retains original bytes and native labels,
while checking both directions against a separately assigned formal identity.
Native model semantic roles, actual typing/definedness, reference evaluation,
backend qualification and existing-extractor integration remain work.

## LC03 reviewed runtime packet — 2026-09-09

PR10 landed the qualified native model/checker at bfac17d; Plan-005 and
SR-083–087 retain its actual implementation evidence. The next owning task is tracked privately.

The actual QUOIN 0.22.5 skills specify, spec-matrix, spec-review and all seven
selected analysis skills authored the runtime packet at 045025f. It contains
expanded FR-007/008, new FR-018/NFR-006/IT-006, two concrete API contracts and
TM-004 with 42 functional criteria, 17 resource metrics and 23 planned cases.
Quoin 0.23.1 supplied the installed catalog contracts; Quire 0.31.0 validated
the scoped artifacts. The complete advisor/catalog/properties/coverage output
is retained in spec/reviews/native-runtime/data.

Reviews SR-088–095 at 1603f97 approve this specified API scope. The first EARS
review used an empty findings table, which the installed schema rejected;
f7ed193 records the repair and successful validation while retaining the first
failure. Resolved contract findings include conditional post self, caller-poll
panic behavior, exact draft-error provenance, local ValueId meaning and atomic
entry/event capacity checks. Resource-metric benchmark recommendations and
parser/temporal characteristic matches have explicit evidence-review dispositions.

The actual QUOIN spec-to-plan skill produced Plan-006 after those reviews.
Tasks 012–015 cover input construction, model-aware validation, independent
evaluation and full qualification/handoff in one serial Agent A track.
No runtime source, executable test or Cargo/CI behavior changed in this packet.
Runtime rows remain planned; existing tests are not relabeled as runtime evidence.
The actual local code-review/rust-review skills and non-semantic gap reconciliation
remain mandatory during implementation qualification.

This native API milestone is scoped to LC03. LC02/FS03 acceptance, compiled
ConfigVersion/backend qualification and Quire integration for the original
assignment are tracked as separate work. It introduces no B/C/TL/Filament
ownership change, new portable result authority, decoder, producer-language
execution or hosted CI run.

## LC03 input construction qualified — 2026-09-09

Task-012 is qualified at c8fa41f6e172e58b9406792d1b9f6b6bd85e52cc. SR-096
records the actual code/Rust review, 21 constructor tests, role-separation
compile-fail doctest and passing serial local gates. The three constructor TC
setup notes were corrected at dc63f33 and received all eight review addenda at
29922d7 before this qualification; the existing FR-018 contract was unchanged.

FR-018 is 7/7 backed and TM-004 is 3/23 backed. Quire binds all 134 Rust symbols;
its matrix status-header and other catalog/classifier limitations are explicitly
recorded in SR-096. Only construction cases and NFR-006-M-1..5 are qualified.
Task-013 population validation is unblocked; evaluator, integration, backend and
Quire acceptance remain outstanding. PR11 stays draft while that work proceeds.
