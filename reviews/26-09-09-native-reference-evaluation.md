---
id: SR-098
title: "Code and Rust review of native reference evaluation"
type: SpecReview
analysis: code-review
scope: "FR-008 / Task-014 at 48f53aed5990f7daf15ae6871c1c081d8974c64f"
review_set: subset
---

## Summary

The native evaluator executes the retained AST over a constructor-private
validated context, preserving captured observations, exact work counters and
original implication events. Twenty-nine public API tests and two private
invariant controls qualify reference execution and its resource limits.

## Verdict

**PASS** for FR-008, NFR-006 evaluation metrics M-12..17 and Task-014.
Task-015 still owns complete IT-006 integration and Plan-006 qualification.
Backend qualification, remaining LC02/FS03 acceptance and Quire integration
remain required by the original assignment.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings | - |

## Scope and review process

Reviewed source 48f53aed5990f7daf15ae6871c1c081d8974c64f against FR-008,
NFR-006, NFR-003/NFR-005 and docs/native-runtime-evaluation.md. The governing
specification and eight actual QUOIN reviews remain those recorded in Plan-006:
045025f, SR-088–095, 1603f97/f7ed193, with the earlier construction-test setup
correction and review addenda. This implementation changes no admitted syntax,
native scalar/observation meaning, runtime input encoding or model authority.

Applied the actual /home/peter/dev/agent-skills/code-review/SKILL.md,
rust-review/SKILL.md, rust-style/SKILL.md and implementation-gap-analysis
discovery. AGENTS.md, README.md, LICENSE-DECISION.md and Cargo lints govern.
No applicable AssuranceProfile, Rust style override or deny.toml was found.
The owner-declined optional semantic gap comparison was not run. Ordinary
code/spec faithfulness, negative controls, assertion quality and reverse
code-to-requirement discovery were checked.

## Requirement and executable evidence

| Criteria | Actual controls |
| --- | --- |
| FR-008-AC-1/2 | Parent version ordering on a healthy parent and equal/higher violating parents; healthy, violating, empty and repeated aggregate members after successful checking and validation. |
| FR-008-AC-3/11 | All 75 functional graphs on one to three vertices, all source/target pairs, two identity spellings and both object/reference operands: 2,456 actual results agree with a separate Boolean adjacency-matrix closure oracle. Isolated self, self-loop and two-object cycle have independent exact cost vectors. |
| FR-008-AC-4/16 | All ten mandated cost vectors complete at their exact expression/graph budgets and stop one below each nonzero dimension. Grouping costs no step; five-step let stops at four; a true antecedent at two steps retains completion but no consequent entry. Boolean short circuiting and skipped conditionals emit no events or skipped work. |
| FR-008-AC-5/19 | Deterministic cancellation and caller panic at every observed poll in a let/implication/reaches execution; retries restore the same truth, counts and events, preserve bytes and obey a subsequently smaller budget. Private fault controls separately exercise retries after Refused. |
| FR-008-AC-6/14 | Precondition/pre and postcondition/post self; nested pre; lexical and conditional optional/reference captures; immutable pre parameters pointing to an object deleted in post; post result capture inside pre; large sequence binding and repeated local reads under zero comparison/text budget. |
| FR-008-AC-7/8/9/10/17 | False, true, grouped, nested and repeated implications; every event-capacity boundary; atomic expression/depth/event entry; original implication/operand ExprIds and exact operand regions in LF/CRLF sources; independent byte, line and scalar-column checks for multibyte operands. |
| FR-008-AC-12 | Independent signed arithmetic vectors for every division/remainder sign pair, zero numerators, add/subtract/multiply/negate and i64 endpoints. Undefined source refuses in Check; invalid supplied integers refuse in Validate. Private storage corruption exercises runtime range, checked-division overflow, wrong-shape and unavailable-capture refusal without a public unchecked context. |
| FR-008-AC-13 | All 40 sequences of length zero through three over three scalar values, independently expected forall/exists truth and implication-event occurrence order. Size counts duplicates. A separately admitted reference-sequence model preserves duplicate targets in order. |
| FR-008-AC-15 | Integer comparison truth tables; exact Unicode equality/order, empty/prefix/composed/decomposed/supplementary cases; reordered eligible nested records and enum variants; declaration-name comparison order; distinct equal-valued objects and same identity across observations. Option/Seq equality and cross-universe equality refuse in Check. |
| FR-008-AC-18 | Separate expression, graph, comparison, text, event and depth ceilings. Exact maximum graph/comparison/text/event work can complete, and elevated options cannot disable any ceiling. Recursive record comparisons reach depth 64 independently of expression depth; a 64-frame expression completes, while its 65-frame source is refused upstream. |
| FR-008-AC-20 | Actual collect syntax refuses as unsupported_construct in Profile; the supported sequence control retains three duplicate occurrences and all nine implication events. |

All ordinary runtime assertions first establish native source parsing, exact
model admission/linkage, static checking and runtime validation. Setup errors
are retained separately and never accepted as evaluation refusals.

## Architecture and Rust review

The runtime separates its public report/limits, work accounting, immutable value
views, AST execution and type-directed comparison. It uses the retained native
AST and checked types; it does not execute the IR proof witness, add another
parser or infer types from runtime values. A borrowed report privately retains
its exact context, outcome, usage and ordered events. No result adapter competing
with B's portable contract is introduced.

Values are small copied scalars or borrowed text, arena/container and identity
views. Let/quantifier bindings use the exact linked lexical declaration span;
State and parameter reads use the exact linked formal owner. Snapshot/object
lookups reuse the validated context's indexes. The existing checked model
catalog gains a bounded borrowed field-order index for record comparison.
Input field/parameter name lookup scans already bounded metadata with cancellation;
it never expands a large value on a local/field read. No constant-time lookup
or allocator-capacity guarantee is claimed.

Groups and reaches traverse iteratively. Expression and comparison recursion
are checked before entering a frame, separately capped at 64. Reaches charges
before a fresh expansion and tests the reached target before suppressing a
repeat. Every call has its own visited set. Native integer operations use
checked i64 arithmetic followed by the exact checked native result interval.
Text compares Unicode scalar iterators with separate per-side/end-check charges.

Implication entry preflights expression, depth and event capacity together.
Antecedent completion checks event capacity after actual operand completion.
Failure preserves only actual work/events and never manufactures a Boolean.
Caller panic follows normal Rust unwinding. No global state, async work, locks,
I/O, unsafe code, source/test lint waiver or production panic was added. Arena
indices use checked conversion and lookup. Private fault tests compile beneath
validation::api and reuse the same public fixture producer; they add no callable
production bypass and no test-dependent branch inside production functions.

The initial generated graph test found a real evaluator defect: it rejected
reference operands already admitted by the checker. The repaired implementation
accepts object/reference handles with their exact capture, and the full 2,456-case
oracle passes. Separate setup repairs replaced a non-admitted block comment,
corrected collect's expected Profile phase and recognized the coupled shallow
expression/comparison depth stop. Clippy found stale imports after shared setup
extraction; they were removed without suppressing warnings. A final review
replaced an arena index cast with checked conversion.

Discovery found no new unowned semantic constraint or placeholder. FR-008 owns
execution, FR-007 owns input validity and NFR-006 owns the explicit ceilings.
All new production and qualification code is Rust, AGPL-3.0-only. Dependencies,
licenses and hosted workflows are unchanged. CI remains workflow_dispatch-only;
no hosted run was dispatched. The API has no scheduler or shared mutable state
for a Loom model. No fuzz or mutation-adequacy result is inferred from these tests.

## Actual local gates

Cargo phases ran serially with nice 10, locked/offline resolution, one job,
one test thread and the existing explicit target caches. Logs are under
reviews/data/native-runtime/.

| Command | Actual result | Evidence |
| --- | --- | --- |
| `nice -n 10 cargo test --locked --offline --target-dir target -j 1 --no-default-features -- --test-threads=1` | Full checkpoint regression: 192 ordinary tests and one compile-fail doctest passed; three named private tests ignored in this lane | evaluation-checkpoint-full-tests.txt |
| `nice -n 10 cargo test --locked --offline --target-dir target -j 1 --test runtime_evaluation --lib -- --test-threads=1` | Final affected scope: all 29 evaluator tests and three library unit tests, including both private invariant controls, passed | evaluation-qualified-tests.txt |
| `nice -n 10 cargo clippy --locked --offline --target-dir target -j 1 --workspace --all-targets --all-features -- -D warnings` | Passed on final source and tests | evaluation-qualified-clippy.txt |
| `nice -n 10 cargo fmt --all -- --check` | Passed | evaluation-qualified-fmt.txt |
| `RUSTDOCFLAGS='-D warnings' nice -n 10 cargo doc --locked --offline --target-dir target -j 1 --no-default-features --no-deps` | Passed | evaluation-rustdoc.txt |
| `nice -n 10 cargo build --locked --offline --target-dir target/clean -j 1 --no-default-features` | Passed with the existing minimal-build cache | evaluation-minimal-build.txt |
| `QUIRE_STATE_CORE=/home/peter/dev/worktrees/formalization-a-spec/proposals/state-core nice -n 10 cargo test --locked --offline --target-dir target -j 1 --test fixture_audit -- --ignored --test-threads=1` | Three selected private audits passed | evaluation-private-audits.txt |

After the full checkpoint suite, five evaluator controls were added and the
arena-index conversion was tightened. The final affected tests and Clippy passed;
the full checkpoint log is not presented as a fresh 197-test run. Subsequent
test-index conversion/formatting changes were compiled by the final Clippy gate.
Failed and successful earlier logs retain the actual sequence. No failed gate
was waived or relabeled as a successful semantic observation.

## Traceability and remaining work

Quire binds 200/200 Rust symbols, FR-008 is backed 20/20, TM-004 is 22/23 and
root backing is 195/208. The report has no untracked symbols or reported status
lies. Twenty existing catalog/classifier diagnostics, six registry duplicate
notices and three old unmatched IT-004 tags remain known limitations. Functional
matrix status inference is incomplete because of the existing Coverage Status
header; this review manually reconciled all twenty FR-008 rows with assertions.
NFR-006 metric ordinals are prose, not invented trace IDs. The event test uses
two bare single-line trace attributes, both bound by the declared grammar.

TC-061's evaluation portion and TC-067–076 are qualified at the evaluated source.
Task-014 is complete. Task-015 must execute IT-006/TC-077, complete Plan-006's
gap/review handoff and determine PR readiness. PR11 remains draft. Remaining
LC02/FS03, compiled-model/backend and Quire requirements remain open; this
reference milestone does not complete the original assignment.
