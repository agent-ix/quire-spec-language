---
id: SR-085
title: "Code and Rust review of complete native model qualification"
type: SpecReview
analysis: code-review
scope: "FR-015 / TC-040–045 at 0cd679c7b8491d002ad96524e59155073b52b048"
review_set: subset
---

## Summary

Seventeen new public-API tests complete the model admission, artifact identity,
source-locus and resource qualification families. Together with the existing
model/link tests and SR-084, they qualify FR-015 and complete Task-008. Expression
checking and the remaining state workflow are still unimplemented milestones.

## Verdict

**PASS** for FR-015 / Task-008. This does not complete Plan-005 or LC02.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings | - |

## Review context

Reviewed source 0cd679c7b8491d002ad96524e59155073b52b048 against FR-015 and
docs/native-model-checking.md, previously reviewed at ceccabb in SR-066–073.
The production admission and artifact implementation is unchanged; native
linkage retains its SR-084 inspection and reran in the full suite. Applied the
actual /home/peter/dev/agent-skills/code-review/SKILL.md and rust-review/SKILL.md,
portable rust-style defaults and implementation-gap-analysis discovery. The
repository's AGENTS.md, README.md, license decision and Cargo lints govern.
No applicable AssuranceProfile, custom Rust style or deny.toml was found.

This qualification changes no production interface, semantic requirement or
acceptance criterion. Ordinary code/test faithfulness was inspected; the owner's
declined optional semantic gap comparison was not run. Task-010 still owns the
full model/checker gap and final PR gate. No additional agents were used.

## Requirement and implementation correspondence

| Criterion | Actual evidence and judgment |
| --- | --- |
| TC-040 / AC-1 | tests/native_model.rs and native_model_cases/admission.rs inspect all seven nominal scalar/site mappings, signed bounds including full i64 extrema, units, optional reference and sequence wrappers, exact object/carrier/universe roles, operation anchor/result/frame, real State/Input kinds, owner and source identities. Every fixture field/value/role span is checked against its parsed original JSON occurrence. An additional scalar value retains Option/Collection/Option wrappers and a maximum of three. The model supplies the profile's sequence representation; runtime order/multiplicity evaluation is not claimed here. |
| TC-041 / AC-2 | The original eight mutations plus admission.rs cover missing/empty/duplicate scalar assignments, mismatched one-site bounds/kinds, missing/Bool/enum/record leaves, ordinary text maxima zero/one/IR maximum, absent/excessive text roles, used and unused unsigned/saturating/rational data and pure functions. Carrier cases isolate missing/extra/non-Text/optional/zero-max IDs and duplicate/conflicting/missing object owners. Operation cases cover absent/duplicate contexts, missing/State/duplicate/result-overlapping parameters, invalid/missing results, orphan Input values, and invalid/duplicate field/created/deleted frame entries. Each negative call returns the expected native code after successful IR construction and is followed by successful fresh admission. |
| TC-042 / AC-3 | identity.rs generates seven source-derived semantic mutations, checks their changed structured artifact payloads and digest, verifies exact i64 extrema, and exercises six permutations of types, values, fields, variants, scalar/site/object/operation/frame inventories. Ordered parameters change the digest and retain their input order in the actual artifact. Native identity/revision and formal owner mutations participate; existing tests retain path independence and source-byte sensitivity. Native-link tests select exact current artifacts and reject formal-only, semantic-IR and foreign digests. |
| TC-043 / AC-4 | provenance.rs tests false line, column, formal source and revision against seven locus classes, including an actually unused record field, enum variant and value, plus scalar/object/operation roles. All IR spans and environments construct successfully first. Seven additional cases put a point inside a UTF-8 scalar in an explicitly appended synthetic provenance region; the valid prefix-bound model admits before mutation. Related original declaration spans are checked where the diagnostic contract supplies them. Native-link tests retain conflicting source/owner inventory, duplicate ambiguity and independent-owner controls. |
| TC-044 / AC-5 | SR-084's actual reference/operation/enum/parameter linkage and legacy compatibility controls all ran again in the complete suite. No linker behavior changed in this qualification. |
| TC-045 / AC-6 | limits.rs independently counts the fixture's 9 roles, 10 entries, 32 nodes and depth 2, testing exact/one-below/zero for every dimension. A richer fixture's 18 entries checks scalar sites, parameters and all three frame inventories together. Valid exact hard-boundary fixtures execute at 10,000 nodes, 10,000 entries and depth 64; the next item fails with elevated caller limits. The existing native-link tests retain exact 1 MiB artifact and 8 MiB aggregate boundaries and every LinkLimits dimension. |

## Resource-bound interpretation

The 10,000-node case uses 153 bounded optional Boolean values and a shorter
tail, rather than thousands of source-bearing declarations. The 10,000-entry
case uses finite fields and operation frames. Both fit the artifact limit and
execute through actual IR constructors and NativeModel admission. The new suite
completed in 0.69 seconds in the recorded serial full run; this is an observation,
not a wall-clock assertion or performance guarantee.

At 10,001 coherent unique roles, the native roles preflight refuses before
artifact work despite an elevated caller option. At 10,000 roles, this fixture
instead exhausts the 1 MiB artifact ceiling. These are simultaneous ceilings:
no exact-10,000-role success is claimed. The independent valid nine-role case
proves inclusive role-budget behavior. The bounded byte writer was inspected:
it checks length before extending its output, and failed encoding exposes no
partial NativeModel. The limits are content/count guarantees, not exact allocator
capacity promises. No mass Cartesian family or concurrent worker is introduced.

## Rust, test and gap inspection

New test orchestration has four concern modules and focused shared helpers; it
uses the existing source-derived producer instead of adding another decoder or
model authority. IR reconstruction succeeds explicitly before native assertions.
The tests call public APIs with deterministic bounded fixtures, imported bare
single-line trace attributes, typed identities and checked numeric conversions.
They introduce no unsafe code, new dependency, production test bypass, mock,
ambient network/environment dependency or ignored case. Panic assertions and
indexing are confined to qualification setup and assertions.

Strict Clippy initially rejected a complex tuple/function-pointer table. A
named Dimension structure resolved it without warning suppression. Inspection
also strengthened an unused-field control and replaced serialized-text searches
with structured bounds/unused-value assertions before the final run. The first
16-test run and initial Clippy failure remain in the evidence directory;
model-qualification-tests.txt is the authoritative final 17-new-test run.

Discovery found no new hidden contract or source stub in the implemented model
scope. Planned checking remains explicitly absent rather than a passing stub.
There is no async, lock, cancellation or shared mutable worker state in this
change, so Loom does not apply. New executable qualification is Rust under
AGPL-3.0-only; Cargo.lock, dependency grants and manual-only CI are unchanged.

## Local validation

Every final command below exited 0. Cargo phases ran one at a time at nice 10,
locked/offline with one job and one test thread, using existing target caches.
Logs are under reviews/data/native-checking/; only terminal blank lines may be
normalized for repository whitespace conformance.

| Command | Result | Log |
| --- | --- | --- |
| `nice -n 10 cargo fmt --all -- --check` | Passed | model-qualification-fmt.txt |
| `nice -n 10 cargo clippy --locked --offline --target-dir target -j 1 --workspace --all-targets --all-features -- -D warnings` | Passed | model-qualification-clippy.txt |
| `nice -n 10 cargo test --locked --offline --target-dir target -j 1 --no-default-features -- --test-threads=1` | 86 passed; 3 named private cases ignored | model-qualification-tests.txt |
| `QUIRE_STATE_CORE=/home/peter/dev/worktrees/formalization-a-spec/proposals/state-core nice -n 10 cargo test --locked --offline --target-dir target -j 1 --test fixture_audit -- --ignored --test-threads=1` | 3 selected private cases passed | model-qualification-private-audits.txt |
| `nice -n 10 cargo build --locked --offline --no-default-features -j 1 --target-dir target/clean` | Passed using existing cache | model-qualification-minimal-build.txt |
| `RUSTDOCFLAGS='-D warnings' nice -n 10 cargo doc --locked --offline --target-dir target --no-deps -j 1` | Passed | model-qualification-rustdoc.txt |
| `nice -n 10 cargo run --locked --offline --target-dir target -j 1 --bin fixture-audit -- self-test` | 6 negative controls and duplicate-key refusal passed | model-qualification-audit-self-test.txt |
| `nice -n 10 cargo run --locked --offline --target-dir target -j 1 --bin fixture-audit -- model-bytes tests/fixtures` | 5 digests and exact producer pin passed | model-qualification-audit-model-bytes.txt |
| `nice -n 10 cargo run --locked --offline --target-dir target -j 1 -- parse test:parent fixture:1 tests/fixtures/parent.native` | Parsed | model-qualification-cli-parse.txt |
| `nice -n 10 cargo run --locked --offline --target-dir target -j 1 -- format test:parent fixture:1 tests/fixtures/parent.native` | Formatted | model-qualification-cli-format.txt |

Quire spec/plan/review validation exited 0; the six existing registry duplicate
notices remain in model-qualification-*-validation.txt. Coverage binds all 89
Rust test symbols, with no status lies or untracked symbols. FR-015 has 6/6
backed criteria; TM-003 has 22/35 backed cases, matching the 22 qualified and
13 planned checker cases. The 18 existing classifier/catalog diagnostics remain
in model-qualification-coverage.json. Execution and the criterion inspection
above establish qualification; trace presence alone does not.

Task-008 is complete and Task-009 is next. Draft PR #10 remains open for checking
and the final plan gate; no merge or hosted CI dispatch occurred. Population
validation, healthy/violating/refused-or-incomplete reference execution, backend
qualification and Quire integration remain required downstream work.
