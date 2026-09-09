---
id: SR-099
title: "Code and Rust review of the complete native runtime workflow"
type: SpecReview
analysis: code-review
scope: "Task-015 / IT-006 / TC-077 at 4ac3597ffbed031c9e0773fc9971370ddce25186"
review_set: subset
---

## Summary

Five public integration tests qualify the source-derived native model through
parsing, linking, checking, input construction, validation and reference execution.
Healthy, violating, refused and incomplete results retain the actual selected
source, model, authored clause and immutable input correspondence.

## Verdict

**PASS** for IT-006/TC-077 and the code/Rust qualification part of Task-015.
Plan reconciliation and the private handoff remain Task-015 work. LC02 strict
packages, FS03 acceptance, backend qualification and Quire integration remain
required by the original assignment.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings | - |

## Scope and process

Applied the actual /home/peter/dev/agent-skills/code-review/SKILL.md,
rust-review/SKILL.md, rust-style/SKILL.md and implementation-gap-analysis
discovery. Read AGENTS.md, README.md, LICENSE-DECISION.md, Cargo lints and the
owning LC03 issue. No applicable AssuranceProfile, repository Rust style
override or deny.toml exists. Cargo-deny is therefore not a required gate here.
No optional semantic gap comparison or additional agent was used.

The reviewed implementation gate remains 045025f with all eight QUOIN reviews
SR-088–095 at 1603f97/f7ed193 and the recorded construction-setup correction at
dc63f33/29922d7. No language, encoding, API, acceptance criterion or limit changed
in Task-015. This review examines the new integration tests and actual public
composition boundaries; SR-096/097/098 retain the detailed construction,
validation and evaluation reviews and their independent negative controls.

Qualification source is 4ac3597ffbed031c9e0773fc9971370ddce25186. The full
regression and build/style/CLI/audit gates ran at its parent
46f5e70f5cc675fd82cd1c5a9bc7b29ed26cfea7. The only subsequent changes are five
test comments and status prose. All five affected tests and formatting passed
again at 4ac3597; executable source and assertions are identical to the full run.
Rust is 1.98.1 (48a229cea 2026-09-01). Cargo.lock SHA-256 is
4c1f974585e6b99e22801cc023b6bdaf7ba7698263afd371e857c30085b071f0.
The IR dependency remains 690bde7f2dc58662cf9ff0595c2c0e3b17107c6f and the
adopted state semantics remain e897f810a7356d4ce8fd19026221ebda7b65596f.

## Integration criteria and actual observations

| IT-006 step | Executed assertions in tests/runtime_workflow.rs |
| --- | --- |
| SC-01 | Source-derived model admission, real parse/link/check, exact borrowed native model, authored requirement/clause and matching formal/native source digests. |
| SC-02 | Complete current input validates; healthy parent yields true at exactly 12 expression steps. Empty aggregate is true at 3 steps; three repeated values require 15 steps and three comparisons. |
| SC-03 | Equal parent version yields false; a cycle can satisfy local parent ordering while positive reachability distinguishes cycle from acyclic input at 3 expression/2 graph steps. A middle aggregate violation returns false after 11 steps/two comparisons. |
| SC-04 | Construction succeeds before dangling_reference, stale_dependency and wrong_snapshot are observed in validation. Exact phase, authored/native/runtime provenance and no successful context are asserted. Dangling targets include model-related loci. |
| SC-05 | Incomplete population is classified incomplete. Four-step exhaustion retains only actual antecedent entry/completion; cancellation before entry retains zero expression work/events. Exact 12-step retry succeeds; a subsequent smaller budget reproduces the incomplete result, usage and events without changing bytes. |
| SC-06 | A permitted deletion and n update validate before post execution. A pre-captured parameter still resolves the deleted target, pre(self) reads pre, self/result read post, and a false post result changes truth. Wrong roles, forbidden signed-field change and missing deletion inventory refuse during validation. |
| SC-07 | Rust test output records actual outcomes, usage, versioned costs, original event handles/spans, source/model digests and selected snapshot/invocation references. Commands and compiler/IR/standard revisions are recorded here. No setup failure or static proof is counted as a runtime outcome. |

The complete pipeline reuses tests/support/runtime_setup.rs and the qualified
source-aware JSON model reader. It constructs real immutable artifacts and
calls public APIs; it does not mock the evaluator or implement another
interpreter. The mathematical small-graph closure oracle remains the independent
Task-014 control, included in the full regression.

## Rust, architecture and discovery checks

The runtime returns stage-specific typed failures and constructor-private
successes; evaluate requires a borrowed ValidatedContext. Context/report getters
retain original checked source and immutable input. The new tests assert both
the data correspondence and actual truth/usage, rather than only is_ok/type
checks. Test unwraps fail fixture setup immediately; no request-path panic,
unsafe code, error suppression, new dependency or unchecked boundary cast was
introduced. Arena test indices use u32::try_from.

Operation cases use a closed named enum and exhaustive matches. Test setup
mutation is separate from independently authored expected values/costs. There
are no test-only behavior branches in production; the existing private invariant
module exercises deliberately corrupted private storage without exposing an
unchecked public constructor. Tests require no network, daemon, fixed port,
wall-clock threshold or ambient state beyond the separately selected audit lane.

Reverse discovery covers nine behavior families: snapshot construction,
invocation construction, exact bytes/reference roles, artifact/clause selection,
typed values/population closure, operation frames/captures/deltas, original-AST
execution, events/provenance, and per-stage limits/immutable retries. FR-018,
FR-007, FR-008 and NFR-006 own these respectively. No new unowned behavior,
source/test stub, I/O in evaluation or hidden semantic constraint was found.
The synchronous immutable API has no shared scheduler/lock state for a Loom
model. B retains portable result ownership; no result/evidence framework is added.

All new executable qualification is Rust under AGPL-3.0-only. Existing dependency
grants and manifest/lock pins remain unchanged. The only hosted workflow still
has workflow_dispatch as its sole trigger; it was not dispatched.

## Repairs during review

The first coverage scan found five untracked IT-006-SC-* identifiers extracted
from ordinary Rust comments. Those procedure labels are not minted criteria.
The comments now name workflow step numbers while actual bare single-line
trace attributes retain valid TC/FR IDs. Both scans are retained; the final
scan has no untracked symbols. Stale FR-007/018 and README status text was also
corrected. No result or failing assertion was weakened.

## Actual local gates

Each Cargo phase ran serially at nice 10, locked/offline, one job and one test
thread, using existing explicit target caches. Evidence is under
reviews/data/native-runtime/; every command below exited zero.

| Command | Result | Log |
| --- | --- | --- |
| `nice -n 10 cargo test --locked --offline --target-dir target -j 1 --no-default-features -- --test-threads=1` | 202 ordinary tests and one compile-fail doctest passed; three named private tests ignored in this lane | workflow-full-tests.txt |
| `nice -n 10 cargo test --locked --offline --target-dir target -j 1 --test runtime_workflow -- --test-threads=1 --nocapture` | All five integration tests passed, including final comment repair | workflow-final-tests.txt |
| `nice -n 10 cargo clippy --locked --offline --target-dir target -j 1 --workspace --all-targets --all-features -- -D warnings` | Passed; this crate has no optional features | workflow-clippy.txt |
| `nice -n 10 cargo fmt --all -- --check` | Passed at final source | workflow-final-fmt.txt |
| `RUSTDOCFLAGS='-D warnings' nice -n 10 cargo doc --locked --offline --target-dir target -j 1 --no-default-features --no-deps` | Passed | workflow-rustdoc.txt |
| `nice -n 10 cargo build --locked --offline --target-dir target/clean -j 1 --no-default-features` | Passed using existing minimal-build cache | workflow-minimal-build.txt |
| `QUIRE_STATE_CORE=/home/peter/dev/worktrees/formalization-a-spec/proposals/state-core nice -n 10 cargo test --locked --offline --target-dir target -j 1 --test fixture_audit -- --ignored --test-threads=1` | Three private audits passed | workflow-private-audits.txt |
| `nice -n 10 cargo run --locked --offline --target-dir target -j 1 -- parse test:parent fixture:1 tests/fixtures/parent.native` | Parsed four clauses/one import with exact source digest | workflow-cli-parse.txt |
| `nice -n 10 cargo run --locked --offline --target-dir target -j 1 -- format test:parent fixture:1 tests/fixtures/parent.native` | Formatted source successfully | workflow-cli-format.txt |
| `nice -n 10 cargo run --locked --offline --target-dir target -j 1 --bin fixture-audit -- self-test` | Six independent digest/content controls and duplicate-key refusal passed | workflow-audit-self-test.txt |
| `nice -n 10 cargo run --locked --offline --target-dir target -j 1 --bin fixture-audit -- model-bytes tests/fixtures` | Five historical checkpoint digests and producer pin passed; no producer/evaluator executed | workflow-model-bytes.txt |

## Traceability and method limits

Quire 0.31.0 (engine ca7362d4) binds 205/205 Rust symbols. FR-018 is backed
7/7, FR-007 15/15, FR-008 20/20 and TM-004 23/23. Root backing is 196/208;
this is not full-assignment completion. The final scan has no untracked
symbols or reported status lies. Twenty existing classifier/catalog diagnostics,
six registry duplicate notices and three old unmatched IT-004 tags remain
visible. In particular, the configured Status selector does not match the
functional matrix's Coverage Status header; manual assertion/run reconciliation
is required and was performed. NFR metric ordinals and IT procedure labels are
not invented trace IDs. Manual/Inspection no-symbol rows are not missing tests.

`quoin advise --repo /home/peter/dev/worktrees/formalization-a-language --json`
exited 2 because its Quire version probe failed, although a direct quire
--version reports 0.31.0. The actual failure is retained in
workflow-advice-diagnostics.txt; no advice result is claimed. Explicit method
assessment retains examples, adverse mutations, generated bounded families,
independent closure/cost oracles, cancellation/panic controls and integration.
No fuzz, concurrent model-checking or mutation-adequacy run is inferred from
these tests or the incomplete adviser.
