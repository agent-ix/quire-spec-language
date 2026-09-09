---
id: SR-083
title: "Code and Rust review of qualification pipeline repairs"
type: SpecReview
analysis: code-review
scope: "FR-017 / Task-011 at 08a4fe79a0ed734c2b96acc831ac750a8c1eb56f"
review_set: subset
---

## Summary

The four SR-074 construction findings are resolved in the reviewed FR-017
scope. Original JSON occurrences now supply provenance, typed fixture lowering
has one scalar owner, and the existing linker and auditors retain their
admission order across focused stages.

## Verdict

**PASS** for FR-017 / Task-011. This does not complete Task-008/009/010,
FR-015/016, native runtime evaluation or the full Agent A assignment.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings | - |

## Evaluated context and sequence

Reviewed source revision 08a4fe79a0ed734c2b96acc831ac750a8c1eb56f, including
implementation d1168dd6604100d52fa4cb97877cf6991e68dda4 and the following
regression-trace-only commit. The original architecture evaluation remains
FAIL at its historical 4798508 baseline; this artifact records its repair.
FR-017/TC-054 were specified at a350754 and received the owner-selected base
plus all seven analyses in SR-075–082 at 02b7cc8 before repair implementation.
The preimplementation handoff is recorded on LC02 #3.

Applied the actual `/home/peter/dev/agent-skills/code-review/SKILL.md`, its
Rust dispatch `/home/peter/dev/agent-skills/rust-review/SKILL.md`, the portable
rust-style defaults and implementation-gap-analysis discovery. Read repository
AGENTS.md, README.md, LICENSE-DECISION.md, Cargo lints and hosted workflow.
No applicable AssuranceProfile, repository-specific Rust skill, deny.toml or
additional contributor convention was found. The repository's bare single-line
trace attributes supersede the portable skill's historical doc-tag example.
The owner declined optional gap-analysis semantic comparison; ordinary
code-review faithfulness and test/implementation alignment were still reviewed.
No additional agents, hosted runs or external producer processes were used.

## Disposition of the architecture findings

| Original finding | Repair and inspected evidence |
| --- | --- |
| SR-074 FND-001 | tests/support/located_json.rs:38 accepts the selected borrowed RawValue, checks address subtraction/addition and the original slice, then uses FormalSource for coordinates. There is no pointer dereference, unsafe slice construction, name search or JSON spelling reconstruction. TC-054 independently computes expected byte ranges for repeated identical objects, references preceding declarations, formatting/key permutations, escaped names and Unicode. An identical separate allocation refuses. The actual model fixture test slices each produced record locus and requires its original complete record object; the earlier NodeRef reference locus cannot pass. |
| SR-074 FND-002 | tests/support/native_rule_model.rs:191 has three explicit responsibilities: bind_source, decode_model and lower_model. Located typed declarations enter focused scalar/field/record/value/object/operation conversions. ScalarTable at line 297 owns representation and role together; its Entry check rejects duplicate identity. Original input is bounded to 1 MiB, declaration/field counts are charged before lowering, and nested type lowering is limited to 64. Setup errors preserve source, decoding and IR causes and propagate to the known-fixture harness boundary. |
| SR-074 FND-003 | src/linking.rs:250 runs preflight, formal_catalog, select_models and resolve_clauses. Inventory canonicalization still precedes imports; duplicate clause, unsupported operation and self-binding checks retain their original order. Existing source ownership, exact import, ambiguity provenance, legacy refusals and resource tests pass. No native profile is silently selected by this legacy entry point. |
| SR-074 FND-004 | tools/fixture-audit/roles.rs:79 retains the same Input and reads required root metadata before load_artifacts and the source/model/run/selection checks. Aggregate budgets do not reset. tools/fixture-audit/checkpoint.rs:53 separates profile checks, SyntaxInvocation, observed-result comparison and Display rendering; expected syntax is still read after actual parsing. Positive and independent corruption cases, including all three selected private audits, pass. |

## Rust, faithfulness and implementation-gap inspection

The source-aware helper uses the pinned Serde grammar and structured errors.
Every newly exposed test-support item has a doc comment; production model
access is immutable and fallible. Native role identities use existing IR
newtypes. Exhaustive typed enum conversion remains appropriate; no lexer,
general model reader, parallel semantic table or new dependency package was
introduced. The development-only raw_value feature and original dependency
grant are recorded in docs/dependencies.md; Cargo.lock is unchanged.

Reviewed the new NativeModel admission/artifact modules as the real consumer
used to qualify the producer. They validate supplied roles and source loci,
bound work before normalization, use borrowed declaration indexes, preserve
IR diagnostics where available, and encode typed canonical content through a
bounded writer. The fixture's five model tests call the actual public API and
measure source content, representation, mutation refusal and artifact changes.
They do not substitute a mock model, fabricate runtime objects or treat fixture
setup failure as a checker refusal.

Source/test discovery found no new todo!, unimplemented!, debug placeholder,
unsafe body, panic on the new production admission path, warning suppression,
test-only semantic bypass or empty passing test. The new immutable code has no
async worker, lock, spawned task, connection, cancellation or persistence path;
the concurrency portions of rust-review and Loom are inapplicable to it.
The existing private tests remain explicitly named ignored lanes and were
selected separately. All 64 discovered Rust test symbols carry bound tracking
tags. Ordinary source NUL checks, typed declaration lookups and deliberately
unique expected-marker test controls are not repetitions of the provenance bug.

FR-017-AC-2 is discharged by the structural ownership inspection above, not a
source-scanning pseudo-test. The catalog's advisory Test/Inspection mismatch
remains explained in SR-079. FR-017-AC-1/3/4 have executed tests with explicit
criterion trace attributes. No unstated requirement was discovered in this
repair. Native link integration and the remaining model mutation/budget cases
remain Task-008; contextual checking remains Task-009. TC-040–045 retain
planned completion statuses despite their five initial passing tests.

## Local verification

All Cargo phases ran sequentially with nice 10, one build job, one test thread,
locked offline dependencies and existing target caches. Every listed command
completed with exit 0. Logs are retained under data/native-checking/.
Terminal blank lines were removed from saved Cargo test output for repository
whitespace conformance; test results and diagnostic content are unchanged.
The full test and private-audit suites were rerun after the trace-only change.
Minimal build, strict rustdoc and CLI checks precede only that annotation edit;
no corresponding runtime body changed afterward.

| Command | Result | Log |
| --- | --- | --- |
| `nice -n 10 cargo fmt --all -- --check` | Passed | fmt.txt |
| `nice -n 10 cargo test --locked --offline --target-dir target -j 1 --no-default-features -- --test-threads=1` | 61 passed; 3 private tests explicitly ignored | tests.txt |
| `QUIRE_STATE_CORE=/home/peter/dev/worktrees/formalization-a-spec/proposals/state-core nice -n 10 cargo test --locked --offline --target-dir target -j 1 --test fixture_audit -- --ignored --test-threads=1` | 3 selected private tests passed | pipeline-private-audits.txt |
| `nice -n 10 cargo clippy --locked --offline --target-dir target -j 1 --workspace --all-targets --all-features -- -D warnings` | Passed | clippy.txt |
| `nice -n 10 cargo build --locked --offline --no-default-features -j 1 --target-dir target/clean` | Passed using the existing minimal-build cache | minimal-build.txt |
| `RUSTDOCFLAGS='-D warnings' nice -n 10 cargo doc --locked --offline --target-dir target --no-deps -j 1` | Passed | rustdoc.txt |
| `nice -n 10 cargo run --locked --offline --target-dir target -j 1 --bin fixture-audit -- self-test` | 6 negative controls passed; duplicate keys refused | audit-self-test.txt |
| `nice -n 10 cargo run --locked --offline --target-dir target -j 1 --bin fixture-audit -- model-bytes tests/fixtures` | 5 checkpoint digests and exact producer pin passed | audit-model-bytes.txt |
| `nice -n 10 cargo run --locked --offline --target-dir target -j 1 -- parse test:parent fixture:1 tests/fixtures/parent.native` | Parsed | cli-parse.txt |
| `nice -n 10 cargo run --locked --offline --target-dir target -j 1 -- format test:parent fixture:1 tests/fixtures/parent.native` | Formatted | cli-format.txt |

Actual `quire coverage --scope /home/peter/dev/worktrees/formalization-a-language
--json` completed with exit 0. Its retained report binds all 64/64 Rust test
symbols, reports no status lies or untracked symbols, and binds FR-017's three
Test criteria. AC-2 is correctly identified as Inspection with no required test
symbol; this review supplies that evidence. TM-003 has 21/35 cases with tags,
of which 16 have completed qualification and five are partial model cases.
The engine still reports 18 existing diagnostics; those are retained rather
than suppressed. Spec, plan and review validation commands also completed with
exit 0; their logs retain the existing duplicate registry notices.

The full-plan non-semantic gap gate remains Task-010 work. Existing unmatched
historical IT-004 labels and matrix/classifier limitations are retained in raw
coverage evidence; tag presence alone does not establish complete model or
checker qualification. Hosted CI remains workflow_dispatch-only and was not
dispatched. LC02 remains open.
