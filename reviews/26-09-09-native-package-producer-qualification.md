---
id: SR-111
title: "Code and Rust review — qualified native package producer"
type: SpecReview
analysis: code-review
scope: "Task-016 producer construction, static identity and complete producer qualification; reader/integration portions excluded"
review_set: subset
evaluated_revision: "c195950140f05abe33fabb30b74a1210e387b5ee"
review_date: "2026-09-09"
---

## Summary

The producer now passes the remaining correspondence, static-mutation,
runtime-independence and type-role controls. Together with SR-109/110's fixed
vectors, schema, exhaustive features and measured limits, the evidence closes
Task-016. Verified reconstruction and independent ecosystem acceptance remain
required work.

## Verdict

**PASS** for Task-016 and the producer portions of FR-019/021 and NFR-007.
Task-017 may consume the qualified producer. This does not qualify the reader,
close Plan-007/LC02 or establish independent B/C acceptance. PR #12 stays draft.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The installed trace engine still lists NFR metric obligations without minting their targets, leaving 20 references untracked. Explicit producer metric evidence below closes the local producer measurement gate; reader metrics remain pending. The tool diagnostic is retained, not suppressed. | SR-109 FND-004; SR-110 FND-002; data/native-packages/correspondence-coverage.json; TC-088 |

SR-109 FND-001/003 were resolved by cffcaa4 / SR-110. SR-109 FND-002 and
SR-110 FND-001 are resolved by c195950's correspondence and identity evidence.
No medium/high implementation finding remains for the producer task.

## Scope and method

Applied the actual /home/peter/dev/agent-skills/code-review/SKILL.md,
rust-review/SKILL.md, rust-style/SKILL.md and implementation-gap-analysis
discovery. Repository guidance, Cargo lints and shared trace attributes govern.
No applicable AssuranceProfile, repository-specific Rust override or deny.toml
exists. The installed Quoin SpecReview authoring pack supplies this artifact's
form. This is the author's code/Rust review; independent consumer acceptance is
not inferred. Optional semantic gap review remains declined.

Specification 41da6e5 / all-eight review 69588ad and the source-setup correction
2c6b9b8 / all-eight supplements 1c3aa50 remain the reviewed contract. This
increment adds assertions and compile-fail documentation, not production
semantics or a changed interface. The original missing-API failure preceded
implementation, but corrected successful fixed fixtures followed the first
producer. TC-090's reviewed correction requires that true sequence to remain
visible; the task history records it rather than claiming prior passing data.

## Producer criterion audit

Every Task-016 deliverable was inspected against its actual implementation and
executed assertions. Package case files below are under
tests/package_construction_cases/; private controls are under src/package/.

| Obligation | Authoritative evidence and result |
| --- | --- |
| FR-019-AC-1 | package_construction_cases/inventory.rs compares complete source-ordered invariant/pre/post records across distinct model and authored owners, with permuted external bindings. fixed.rs compares complete one/two-clause artifacts. |
| FR-019-AC-2 | Fixed vectors and identity.rs preserve native labels/formal revisions, including u64::MAX and control labels; static_changes.rs changes exact source bytes and formal document identity against independent output expectations. Display-path-only changes preserve bytes. |
| FR-019-AC-3 | Whole selected model artifacts/digests and aliases are compared. features.rs preserves repeated aliases and omits unselected models from the closure. static_changes.rs qualifies used context, unused value/enum, scalar, object-universe and unused operation-role changes through real admission and exact expected artifacts. |
| FR-019-AC-4 | correspondence.rs enumerates native byte positions and typed/wire targets for parent reads, the implicit dereference target and nested aliases; ordered used/unused parameters, post result, frame and observations agree with retained checked state. Constant predicates retain nested optional-sequence references and transitive populations, including cycles. |
| FR-019-AC-5 | Fixed complete vectors enumerate all semantic selectors and the distinct adopted base/rules revisions and digests. Their source selections remain the inspected adopted pins, not a newer checkout's contents. |
| FR-019-AC-6 | features.rs generates every unary/binary/builtin mapping, inferred literal/size types, unreachable syntax, unused declarations, nested wrappers, aliases and finite object cycles. Expected sets are named independently of the producer's exhaustive matches. |
| FR-019-AC-7 | Complete fixed and multi-owner records retain both ordered dispositions, with native availability and exact original-root unlowered IR spans. No executable binder/proof conversion exists in package production. Reader rejection of forged dispositions remains Task-017. |
| FR-019-AC-8 | Fixed/repeated constructions match exact bytes. The new private runtime test executes three distinct populations at four budgets, including actual true/false and exhausted results, then compares canonical content, full artifacts, raw/static hashes and pass usage after fresh construction. |
| FR-019-AC-9 | Immutable private fields and distinct NativePackageRef/NativePackageIdentity/ByteDigest and authored/IR types remain intact. The two new compile-fail examples reject native identity at raw selector and IR CanonicalDigest boundaries. Raw/IR/JCS wire substitutions remain reader controls. |
| FR-019-AC-10 | limits.rs independently counts expected JSON work, tests inclusive/zero/one-below/elevated/coupled limits and retries, and compares retained model/source bytes. encoding/tests.rs isolates hard byte/string/entry/depth ceilings before retention. Impossible coupled public maxima are explicitly unqualified. |
| FR-021-AC-1 | Three frozen canonical and full-artifact families have independently prefixed SHA-256 values. A private test compares actual canonical bytes separately before hashing; Unicode/control and large exact revision cases pass. |
| FR-021-AC-3 producer portion | identity.rs and static_changes.rs compare admissible changed outputs with independent complete expectations. Sixteen raw canonical mutations cover all semantic selectors, both definition revisions/digests, source loci, resolved targets and runtime requirements; they remain raw controls, not fabricated CheckedPackages. |
| FR-021-AC-4 producer portion | Fixed canonical files omit identity/projection members while complete artifacts retain them. Full reader comparison of a forged excluded claim remains unimplemented. |
| FR-021-AC-5 producer portion | Compile-fail role separation passes; unsupported domains and wrong/foreign digests must still be exercised by the actual reader. |
| FR-021-AC-6 producer portion | All derive/canonical/encode limits are independently measured; fixed/path/runtime controls show excluded data does not enter canonical bytes. Reader request passes remain pending. |
| NFR-007 M-2..5 producer portion | Emitted bytes, decoded string bytes, aggregate entries and container depth are separately measured in derive/canonical/encode, with exact/lowered/hard controls and unentered-pass absence. Offered bytes M-1 and reader recognition/decode/compare/frontend limits belong to Task-017. |
| NFR-005 | Production, fixture authoring and all new qualification remain Rust under AGPL-3.0-only. No dependency, feature, grant or executable helper changed in c195950. The sole hosted workflow remains workflow_dispatch-only; all observed checks were local. |

## Rust, architecture and gap discovery

The production manifest still borrows actual checked inputs and uses the
bounded Serde formatter, with exhaustive feature matches and native domain
hashing. The new occurrence helper only appends known fixture fragments and
records their expected positions; it contains no parser or source-search join.
Expected formal declarations are explicit qualified model inputs, not exporter
output. Opaque ExprIds are observed through the public AST arena rather than
constructed by tests. The implicit dereference-target expectation now includes
the whole composed expression span in retained traversal order.

All runtime identity controls execute real validate/evaluate calls; wrong
truth or resource classification fails before comparing package identity.
Mutation fixtures alter one named input dependency, retaining independent
record order and hash expectations. No production test branch, bypass,
unchecked wire conversion, unsafe, panic on external input, shared mutable
state, lock, async task or new I/O exists in the change. New tests use compact
shared trace attributes; the type-level doctests are explicitly mapped above.
Source scanning found no unfinished implementation placeholder.

Discovery categories map identity distinctions, constructor privacy, bounded
walks and absence of runtime inputs to existing FR-019/021/NFR-007. Setup
failures revealed missed expectations, not new semantics: ResourceExhausted
spelling, private ExprId construction, forbidden active-name shadowing, both
postcondition context observations, a misplaced fixed occurrence offset and
the dereference target. All failed logs are retained; production admission,
linking and checking were not relaxed. No unstated requirement or shared
repository change was needed.

## Executed gates and matrix reconciliation

At c195950, all final gates terminated with exit 0: formatting, strict
all-target Clippy, 232 ordinary tests, three compile-fail doctests, three
separately selected private audits at immutable adopted standard e897f81,
cached minimal build, strict rustdoc, audit self-test/model bytes, and actual
CLI parse/format commands. Complete commands and limitations are in
[correspondence-verification.txt](data/native-packages/correspondence-verification.txt).
Cargo used nice 10, one job, offline/locked dependencies and the existing cache;
the final regression used one test thread. No hosted dispatch occurred.

Applied QUOIN spec-matrix/SKILL.md to the existing TM-005. Quire 0.31.0 was
verified, and the explicit-scope EARS precondition passed 227/227 spec documents
before reconciliation. TC-078/079/080/090 are now qualified. Ten cases retain
pending reader/integration portions; an individual passing producer assertion
does not mark its shared case complete. The whole matrix is not Complete.
The pre-reconciliation census binds 235/235 Rust candidates and backs 223/249
rows; it retains 20 metric references, 22 catalog diagnostics, six registry
diagnostics and three historical unmatched IT-004 tags. No status lie is
reported. Full Plan-007 gap-analysis remains Task-018 after reader/integration.

Task-016 is done, with Task-017 next and Task-018 dependent on it. Independent
FS05/domain registration and A/B/C acceptance, actual IR lowering, qualified
backend/compiled ConfigVersion and Quire integration remain mandatory in the
original assignment. This producer gate does not redefine that completion.
