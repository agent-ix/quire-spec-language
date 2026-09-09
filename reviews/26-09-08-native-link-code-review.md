---
id: SR-084
title: "Code and Rust review of explicit native model linkage"
type: SpecReview
analysis: code-review
scope: "FR-015 native linkage / TC-044 at 667bf073f94b6925059faf90f1a73c8c6ac75527"
review_set: subset
---

## Summary

The native linking entry point now selects exact admitted model artifacts,
resolves object/reference and operation roles, and preserves original model
provenance on inventory conflicts. The existing formal binding profile retains
its behavior through the shared inventory/import/context/lexical stages.

## Verdict

**PASS** for the native-link implementation and TC-044. Full model admission,
artifact mutation, source-locus and model-limit qualification remain Task-008
work; this review does not complete FR-015, Task-008, native checking or execution.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings | - |

## Context and source correspondence

Reviewed source 667bf073f94b6925059faf90f1a73c8c6ac75527 against FR-015/016's
previously reviewed contract at ceccabb and SR-066–073. This continues the same
LC02 #3 / Plan-005 task after the reviewed FR-017 repairs. Applied the actual
agent-skills/code-review/SKILL.md and agent-skills/rust-review/SKILL.md, portable
rust-style defaults and implementation-gap-analysis discovery. Repository
AGENTS.md, README.md, LICENSE-DECISION.md and Cargo lints remain authoritative.
No applicable AssuranceProfile or deny.toml was found. The owner-declined
optional gap semantic review was not run; ordinary code-review faithfulness
and code/test alignment were inspected. No additional agents were used.

The first native-link test run failed on the absent link_native and profile/
model-correspondence APIs, before their implementation. Expanded controls then
identified an incorrectly located inventory-conflict error and an incorrect
test expectation for the IR digest's unprefixed hexadecimal spelling. The
implementation now reports the conflicting model source at byte zero and
retains both models' sorted related type/value declarations. The digest test
distinguishes malformed native spelling from a syntactically valid wrong
artifact, including deliberate sha256 relabeling of the IR semantic digest.
The default-suite diagnostic-catalog guard also caught the expected 15-to-16
code extension; its updated test asserts the new code's exact spelling and
round trip. These earlier failures are retained in the evidence directory.

## Inspection and executed correspondence

| Area | Evidence and judgment |
| --- | --- |
| Profile and inventory | src/linking.rs:301 uses the existing preflight and exact-import stages. src/linking/native.rs:19 charges every supplied artifact, including unused models, before selection; formal-source and formal-owner maps retain borrowed models and reject conflicts. Duplicate exact candidates remain in the catalog and produce ambiguity. LinkedPackage/LinkedModel expose the explicit profile and original borrowed NativeModel. |
| Context and roles | resolve_context at src/linking.rs:528 requires an explicit native object role; the legacy profile keeps its explicit self State binding. Native self comes from its selected object context. Shape carries borrowed record/reference identities, so dereference does not manufacture a temporary ValueType or expose the carrier's ID field. |
| Operation binding | The selected owner/context/name resolves an actual OperationRole with its original source. Parameters are visible only in that operation. The result keyword selects its declared result; its hidden IR name refuses wrong_snapshot. A resolved precondition result still needs the later checker's post-only availability judgment. No operation is inferred from a pure function. |
| Traversal and provenance | Shared lexical lookup and field resolution retain original ExprIds and spans. Native dereference/reaches resolve the actual target and edge declaration. TC-044 specifically locates the field whose receiver is the dereference AST node and compares the complete formal field source, rather than accepting an unrelated self.n occurrence. Operation, result, enum and parameter targets are inspected through the real public API. |
| Refusal controls | Tests cover absent operation/field, hidden direct or unwrapped carrier access, ordinary enum/record values used as references, an invalid reaches edge, unscoped inputs and result-name bypass. Each failure exposes no package, and subsequent valid linkage succeeds. Invariants and actual pre/post step clauses are exercised. Legacy reference/operation refusals and artifact identity are independently checked. |
| Resource bounds | Native-link tests cover exact/one-below/zero limits for all seven dimensions, 64/65 imports, 256/257 clauses, 10,000/10,003 balanced AST nodes, depth 64/65, exact 1 MiB artifacts, and 8/9 MiB supplied inventories. Elevated options cannot bypass the hard limits. Per-model artifacts beyond 1 MiB are already refused by NativeModel admission; the test explicitly checks that upstream boundary rather than pretending such an admitted model can be passed to link_native. |
| Fixture construction | The source-derived Serde fixture supports optional enum declarations and enum-typed values for the specified adverse reference controls. Enum variants retain borrowed original JSON string occurrences and consume the existing declaration budget. The baseline fixture explicitly verifies the absent enum inventory's default; actual enum/reference and parameter cases use the producer and real IR constructors. Source intake, typed decoding, scalar ownership and lowering remain separate. |

## Rust and implementation-gap checks

Public additions have doc comments and actual callers. Typed IR identity
newtypes and immutable borrowing preserve owner/source identity; no raw-pointer
operation, executable source parser, model-wide scalar inference or general
artifact decoder was introduced. New source contains no todo!, unimplemented!,
debug placeholder, warning suppression or test-only production bypass. Errors
use the existing structured Diagnostic envelope; no request-input unwrap or
unchecked narrowing conversion was added. Iterator/map/set work has the existing
model, node, depth and content bounds. Ordinary exhaustive AST/type matching is
appropriate to this resolution stage; syntax still belongs to Logos/Pratt and
JSON grammar to Serde.

The new code has no async runtime, lock, cancellation, spawned task, persistence
or shared mutable worker state. Loom is therefore inapplicable to this change.
Native tests use real source intake, parsing, model admission and linking, with
independent expected identities/bytes and adverse inputs. They carry imported
bare single-line trace attributes. The private tests retain their named ignored
lane and ran explicitly. No new dependency package, Cargo.lock change, first-
party non-Rust execution, license change or hosted CI trigger was introduced.

No unstated requirement or new source stub was found in the reviewed linkage
scope. The broader model criteria remain explicitly unfinished despite their
partial tests, and the checking API remains planned rather than a passing stub.

## Local gates and retained evidence

Every final command below exited 0. All Cargo phases ran sequentially, offline
and locked, at nice 10 with one job and one test thread, reusing the existing
target and target/clean caches. Logs are under reviews/data/native-checking/;
terminal blank lines may be removed for repository whitespace conformance.

| Command | Result | Log |
| --- | --- | --- |
| `nice -n 10 cargo fmt --all -- --check` | Passed | native-link-fmt.txt |
| `nice -n 10 cargo test --locked --offline --target-dir target -j 1 --no-default-features -- --test-threads=1` | 69 passed; 3 named private cases ignored | native-link-tests.txt |
| `QUIRE_STATE_CORE=/home/peter/dev/worktrees/formalization-a-spec/proposals/state-core nice -n 10 cargo test --locked --offline --target-dir target -j 1 --test fixture_audit -- --ignored --test-threads=1` | 3 selected private cases passed | native-link-private-audits.txt |
| `nice -n 10 cargo clippy --locked --offline --target-dir target -j 1 --workspace --all-targets --all-features -- -D warnings` | Passed | native-link-clippy.txt |
| `nice -n 10 cargo build --locked --offline --no-default-features -j 1 --target-dir target/clean` | Passed using the existing cache | native-link-minimal-build.txt |
| `RUSTDOCFLAGS='-D warnings' nice -n 10 cargo doc --locked --offline --target-dir target --no-deps -j 1` | Passed | native-link-rustdoc.txt |
| `nice -n 10 cargo run --locked --offline --target-dir target -j 1 --bin fixture-audit -- self-test` | 6 negative controls and duplicate-key refusal passed | native-link-audit-self-test.txt |
| `nice -n 10 cargo run --locked --offline --target-dir target -j 1 --bin fixture-audit -- model-bytes tests/fixtures` | 5 byte digests and exact producer pin passed | native-link-audit-model-bytes.txt |
| `nice -n 10 cargo run --locked --offline --target-dir target -j 1 -- parse test:parent fixture:1 tests/fixtures/parent.native` | Parsed | native-link-cli-parse.txt |
| `nice -n 10 cargo run --locked --offline --target-dir target -j 1 -- format test:parent fixture:1 tests/fixtures/parent.native` | Formatted | native-link-cli-format.txt |

Quire spec, plan and review validation passed with the six existing registry
duplicate notices retained in native-link-*-validation.txt. The recorded
native-link-coverage.json binds all 72 Rust test symbols, with no status lies
or untracked symbols. TM-003 has tags for 22/35 cases; only 17 cases are fully
qualified, while five model cases have partial evidence. FR-015's 6/6 tagged
criteria therefore do not establish full qualification. The 18 existing
classifier/catalog diagnostics remain visible in the coverage artifact.

Draft PR #10 remains in progress. This review supplies TC-044's implementation
evidence; it does not authorize completion of the full-plan gap gate, native
definedness checking, finite population validation, reference evaluation,
backend qualification or Quire integration. Hosted CI remains manual-only and
was not dispatched.
