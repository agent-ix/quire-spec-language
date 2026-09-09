---
id: SR-086
title: "Code and Rust review of native clause checking"
type: SpecReview
analysis: code-review
scope: "FR-006/016 at cdb6560ea26b9baad725b17ab82a315ae7c39a30"
review_set: subset
---

## Summary

The native checker establishes contextual types and guarded definedness through
actual IR proofs, retaining source, authored identities and runtime assumptions.
Twenty-four public API tests qualify the thirteen checker cases. A discovered
nested-population omission was reproduced and fixed before final verification.

## Verdict

**PASS** for FR-006/016 and Task-009. Runtime validation, reference execution,
backend qualification and Quire integration remain required downstream work.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings | - |

## Review context

Reviewed cdb6560ea26b9baad725b17ab82a315ae7c39a30 against FR-006/016 and the
concrete docs/native-model-checking.md contract reviewed at ceccabb by the
owner-selected all-review set SR-066–073 at 3cdeb59. Applied the actual
/home/peter/dev/agent-skills/code-review/SKILL.md and rust-review/SKILL.md,
portable rust-style defaults and implementation-gap-analysis discovery.
AGENTS.md, README.md, LICENSE-DECISION.md and Cargo lints govern. No applicable
AssuranceProfile, custom Rust idiom document or deny.toml exists in this scope.
The owner's optional semantic gap comparison was declined; ordinary code/spec
faithfulness and assertion quality were inspected. No subagents were used.

## Requirement correspondence

| Cases | Executed behavior |
| --- | --- |
| TC-025/026 | Actual optional references distinguish preceding implication/conditional guards from absent, right-hand and alternative-only guards. Pre/post facts remain distinct, including captured aliases and pre(alias). Undefined refusals retain the exact native unwrap locus and real IR PotentiallyUndefined cause. |
| TC-027/047 | Guarded Version addition produces actual IR CheckedRange discharge. Missing/weakened/late guards, possible zero divisors and full-i64 minimum divided by -1 refuse. Signed division/remainder constant expressions typecheck and discharge; their runtime numerical results are not claimed. |
| TC-028/029/048 | Exact nominal inference propagates through let/arithmetic/branches. Ambiguous literals and size refuse. Count must contain all possible lengths and be dimensionless. Equal-representation nominal scalars do not coerce. Tests cover record/object/reference/enum/Bool equality and ordering eligibility, conditional mismatch, dimensioned operators, Unicode scalar text maxima and the actual Boolean operation result. |
| TC-046 | Result/pre availability is checked in actual invariant/pre/post contexts, retaining the actual Link versus Check refusal phase. Underlying result-name bypasses refuse. Invocation parameters remain pre-captured and result post-captured; pre on either captured alias does not retag it. |
| TC-049 | Complete source-ordered output preserves original expressions/spans, model pointers, formal loci, authored RequirementRef/ClauseId and execution points. Missing/extra/foreign/duplicate mappings, source identity/revision/path/bytes, formal source collision and wrong clause/operation anchors refuse. Different requirement owners may reuse one local clause ID; duplicate qualified identities refuse. |
| TC-050 | Stable optional reads/aliases and compound let-bound values share the appropriate keys; repeated unbound compound text does not. Initializer goals cannot use body guards. Both quantifiers check arbitrary elements under local guards even though an eventual sequence can be empty. Active shadowing, own initializer/domain use and escaped locals refuse. Unreachable unsafe work omits goals while name/type errors still refuse. Boolean let guards, common alternative facts and contradiction controls execute. |
| TC-051 | Four bounded alias families test every caller budget at measured exact work, one less and zero. Shared expansion reaches the hard per-goal limit despite elevated options. Expanded proof depth exactly 64 succeeds; 63 and an attempted depth above 64 refuse before IR. Distinct goals accumulate materialization charges. Repeated successful and smaller-budget calls expose no cached partial package. |
| TC-052 | Checked clauses retain context/model/universe/observation/frame correspondence even for true postconditions. Reachability requires the same target/universe/observation; cross-observation identity comparison is admitted for an exact type. Nested structural references, optional/sequence inputs, skipped context fields and unused parameter/result values retain transitive target populations. Native reference cycles terminate. No runtime population or Boolean is supplied. |
| TC-053 | An independently authored Rust Boolean evaluator checks 202 guard formulas against 808 assignments. Every one of the 62 admitted guards implies presence. Mandatory direct, false, common-join and contradictory controls are admitted. Refusals retain actual IR diagnostics; the native and IR evaluators do not supply the oracle. This is a bounded soundness family, not an exhaustive proof over all formulas. |

## Architecture and Rust inspection

The public API orchestrates separate binding validation, model indexes, type
constraints, population requirements, shared proof construction, presence facts
and bounded IR materialization. Exhaustive native AST visitors consume parsed
syntax; no source-text parser or textual semantic identity is added. The checker
retains the original AST for later execution. Nonnumeric proof carriers and
unknown identity/reachability predicates implement the documented conservative
abstraction; they are not placeholder runtime implementations.

The constraint solver uses union by rank/path compression and queued wrapper
relations. Native type identity includes the admitted model owner and exact
nominal role. Linking already refuses inconsistent duplicate owners. Proof graph
and value handles have distinct types; optional keys include receiver, declaration,
observation and lexical identity. Numeric ranges and definedness belong to the IR
checker. Outcome joins intersect common facts and discard contradictory paths;
derived premises retain native guard/outcome/source correspondence.

All public refusal paths return the existing typed diagnostic envelope. New
production modules contain no unwrap, expect, panic, unsafe, test bypass, new
dependency, worker spawn, I/O, global cache or warning suppression. Internal
arena indices derive from immutable parser/linker-owned IDs or private appended
tables, never caller-authored integers. AST/proof recursion is bounded at 64;
model wrapper traversal consumes the admitted depth-64 model. Union ranks are
bounded by the at-most-20,000-variable table. Counts are checked before graph,
fact-set and IR-node construction, including witness/guard wrappers. A failure
may retain local scratch only until return; it exposes no CheckedPackage.

Population discovery uses a visited record/observation set across all clause
roots and invocation inputs. It follows reference targets as well as structural
fields; each admitted record is expanded once per observation, preventing native
reference cycles or shared record diamonds from recursive expansion. The model's
separate node/entry/content ceilings bound this discovery. No runtime closure or
truth is inferred from these static requirements.

Tests use deterministic Rust fixture mutation and actual public constructors,
parser, linker and checker. Model setup must succeed before a judgment is tested.
Imported bare single-line trace attributes resolve in the current Quire engine.
No semantic mock, network dependency, timing assertion or new ignored test is
introduced. Loom is inapplicable to this immutable serial checker; no fuzz or
concurrency qualification is claimed. Rust and AGPL-3.0-only remain unchanged;
hosted CI has only workflow_dispatch and was not dispatched.

## Resolved review findings and evidence limits

The initial population collector inspected only direct object/reference types.
The valid expression pre(envelope)=envelope omitted other_nodes when Envelope
contained an OtherRef. checker-nested-runtime-before.txt records the failing
public regression. inputs.rs now traverses nested records/reference targets and
all context/frame/invocation inputs. The final suite also checks skipped inputs
and a cross-population cycle. This was an implementation gap under the existing
FR-016-AC-9/FR-007 boundary, not a new requirement or semantic decision.

Strict Clippy caught an oversized discharge argument list, subsequently replaced
by Request, and a test err().expect() pattern, replaced by let-else. Initial
qualification failures also exposed invalid zero-maximum IR sequence setup and
two insufficient generated boundary examples. The corrected tests use an admitted
sequence maximum, actual measured graph expansion and an exact depth-64 control.
No setup failure is presented as proof of refusal.

checker-complete-cases-first.txt retains the first 20-pass/3-fail result with
lines clipped to 500 characters; the enormous CheckedPackage Debug line is an
excerpt. The original full log is retained locally at
/tmp/agent-a-checker-complete-cases-first-full.txt. Terminal trailing whitespace
and final blank lines were normalized in selected logs. Final commands/results
are retained intact. The usage figures measure the documented work counters,
not allocator capacity, execution time or arbitrary maximum-sized inputs under
all simultaneous ceilings. Native linkage already enforces its 10,000-node
ceiling before the checker can receive an over-limit unit.

## Local validation

All final commands exited 0. Cargo phases ran serially with nice 10, locked and
offline, one build job and one test thread, reusing explicit target caches.
Paths below are relative to reviews/data/native-checking/.

| Command | Result | Evidence |
| --- | --- | --- |
| `nice -n 10 cargo fmt --all -- --check` | Passed | checker-fmt.txt |
| `nice -n 10 cargo clippy --locked --offline --target-dir target -j 1 --workspace --all-targets --all-features -- -D warnings` | Passed | checker-clippy-all.txt |
| `nice -n 10 cargo clippy --locked --offline --target-dir target -j 1 --all-targets --no-default-features -- -D warnings` | Passed | checker-clippy-minimal.txt |
| `nice -n 10 cargo test --locked --offline --target-dir target -j 1 --no-default-features -- --test-threads=1` | 110 passed; 3 named private tests ignored | checker-full-tests.txt |
| `QUIRE_STATE_CORE=/home/peter/dev/worktrees/formalization-a-spec/proposals/state-core nice -n 10 cargo test --locked --offline --target-dir target -j 1 --test fixture_audit -- --ignored --test-threads=1` | 3 selected tests passed | checker-private-audits.txt |
| `nice -n 10 cargo build --locked --offline --no-default-features -j 1 --target-dir target/clean` | Passed using existing cache | checker-minimal-build.txt |
| `RUSTDOCFLAGS='-D warnings' nice -n 10 cargo doc --locked --offline --target-dir target --no-deps -j 1` | Passed | checker-rustdoc.txt |
| `nice -n 10 cargo run --locked --offline --target-dir target -j 1 --bin fixture-audit -- self-test` | 6 negative controls and duplicate-key refusal passed | checker-audit-self-test.txt |
| `nice -n 10 cargo run --locked --offline --target-dir target -j 1 --bin fixture-audit -- model-bytes tests/fixtures` | 5 digests and exact producer pin passed | checker-audit-model-bytes.txt |
| `nice -n 10 cargo run --locked --offline --target-dir target -j 1 -- parse test:parent fixture:1 tests/fixtures/parent.native` | Parsed | checker-cli-parse.txt |
| `nice -n 10 cargo run --locked --offline --target-dir target -j 1 -- format test:parent fixture:1 tests/fixtures/parent.native` | Formatted | checker-cli-format.txt |

Quire 0.31.0 validates spec and plan with exit 0. Coverage binds 113/113 Rust
symbols, reports no status lies or untracked symbols, FR-006 5/5, FR-016 9/9 and
TM-003 35/35. Eighteen existing catalog/classifier diagnostics, three extra IT-004
tags and six registry duplicate notices remain explicit tool limitations. The
root rollup is 131/158; unimplemented runtime/backend/Quire criteria remain
unbacked, and Manual/Inspection rows do not require source symbols. Mechanical
trace backing supplements the actual runs and inspection above.
