---
id: SR-097
title: "Code and Rust review of native runtime validation"
type: SpecReview
analysis: code-review
scope: "FR-007 / Task-013 at 45ed1b4d11eb162ac6bab9d94e2dca6113545037"
review_set: subset
---

## Summary

The native validator now admits immutable contexts only after exact binding,
typed-value, closure, capture, delta and frame checks. Thirty-five public API
tests qualify the validation obligations, including mixed failures and every
validation resource ceiling.

## Verdict

**PASS** for FR-007, NFR-006 validation metrics M-6..11 and Task-013.
FR-008 predicate execution, IT-006, backend qualification and Quire integration
remain required work. TC-061's evaluation/captured-read portion remains planned.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings | - |

## Review scope and sequence

Evaluated source 45ed1b4d11eb162ac6bab9d94e2dca6113545037, including the
c4f7c74 validation checkpoint, against FR-007, NFR-006-M-6..11,
NFR-003/NFR-005 and docs/native-runtime-inputs.md. Initial specification
045025f received the eight actual QUOIN reviews SR-088–095 before runtime
implementation; their commits and dispositions remain in Plan-006. No new
requirement, snapshot encoding, model role or frame permission is introduced
by the qualification fixes.

Applied the actual /home/peter/dev/agent-skills/code-review/SKILL.md,
rust-review/SKILL.md, rust-style/SKILL.md and implementation-gap-analysis
discovery. AGENTS.md, README.md, LICENSE-DECISION.md and Cargo lints govern.
No applicable AssuranceProfile, Rust style override or deny.toml was found.
The optional semantic gap comparison remains declined. Ordinary code/spec
faithfulness, assertion quality and reverse code-to-spec discovery were reviewed.
This is a Task-013 code review; it does not claim Plan-006 is complete.

## Requirement and executable evidence

| Criteria | Actual controls |
| --- | --- |
| FR-007-AC-1/2/8/9 | Complete versus incomplete missing targets, missing self/State, wrong owner/type/universe, empty keys and composed/decomposed Unicode at the scalar maximum, equal-valued distinct objects and reference cycles. Missing observations do not become empty populations. |
| FR-007-AC-3/6 | Two actual authored clauses, foreign clause/requirement, stale selected input/model bytes, conflicting inventory identities, snapshot/invocation namespace collisions, wrong context/name/anchor and pre/post role. Unselected bytes count without validating their object contents. |
| FR-007-AC-7 | Eleven generated native shapes across object fields, State roots, parameters and results, covering all ten ValueNode variants. Controls include signed endpoints/outside bounds, Unicode bounds, nominal enum/record owners and names, missing/extra/duplicate fields, options, nested records, sequences and references inside structural records. Shared nodes are checked separately under each nominal site. |
| FR-007-AC-4/10/11 | Permitted and forbidden field/create/delete effects, correct/wrong/missing/duplicate/intersecting delta sets, absent/present declared results and missing/extra/duplicate parameters. A precondition can retain a pre-captured reference to deleted self; a postcondition still needs post self. Independently sourced second-model populations do not inherit the selected model's permissions. |
| FR-007-AC-5/13/14 | Mixed invalid/unavailable/frame defects, duplicate populations/objects/fields and eight combinations of inventory/population/object/field permutations. Storage comparison detects nested values, option tags, sequence order and removed duplicates, preserves State roots, and retains known later-element changes when an earlier element is ambiguous. |
| FR-007-AC-12/15 | Fresh exact/lowered/zero limits, hard clamping, zero detail capacity with valid/invalid/incomplete inputs, cancellation at every observed poll for current and invocation requests, caller-panic unwind, immutable source/model/input bytes and smaller-budget retries after success. |

All setup goes through actual source parsing, native model admission, linkage
and static checking before runtime assertions. The fixture reader's new
from_source entry point reuses its existing occurrence-aware decode/lower path
with an explicitly supplied FormalSource. This permits independent models to
retain distinct formal source identities; it does not create a new parser.

## Architecture and Rust findings resolved before qualification

The validator separates inventory/selection, value/capture checks, frame
comparison and request-local budget/report handling. Its immutable success type
has a private constructor and retains the original CheckedPackage/input. The
checked package keeps the existing model catalog; bounded field, enum and frame
indexes are reused instead of repeated declaration/permission scans. Frame
validation borrows completed population maps rather than cloning whole object
indexes. The moved maps are restored on every returned outcome; callback panic
unwinds the request without returning a context.

Actual regressions found and fixed during implementation were:

- Duplicate population order changed the retained incompleteness facts. Conflicting entries now retain all observed incomplete flags without selecting one population as authoritative.
- Incomplete populations and unrelated duplicate objects/fields suppressed known unauthorized changes. Ambiguity is retained per object/field, available unique values still compare, and unavailable populations do not establish inferred deltas.
- Equal diagnostic sort keys preserved discovery order. Related locations and retained message content now deterministically break ties after the specified stage/location/span/code keys.
- An ambiguous first sequence element suppressed a known later difference. Record and sequence comparison retain ambiguity while continuing to detect available unequal elements.
- The first implementation passed an unnecessary boxed local to the diagnostic accumulator. It now moves the value without a lint suppression.

The red/green logs are retained. Separate setup failures caught incorrect
model-byte accessor use, recursive formal containment, unused scalar/Input
roles, foreign formal-source identity reuse, a forward arena edge and invalid
authored value spelling. Those failures did not count as runtime refusals.

Production changes add no unsafe code, input unwrap/expect/panic, unchecked
boundary cast, test-only behavior, global mutable state, I/O, async worker,
lock or lint waiver. Indexing follows privately built positions into immutable
validated structures; externally supplied ValueIds use checked conversion and
lookup. The value-kind mismatch arms explicitly refuse incompatible expected
types, and frame equality is separate from source-language equality eligibility.
Recursive container traversal inherits the constructor/model depth bounds;
reference closure uses finite indexes without recursively expanding objects.

Discovery found no new unowned semantic constraint or runtime stub. FR-018 owns
structure/bytes, FR-007 owns model-aware validation and reports, NFR-006 owns
bounds/cancellation, and the existing native model owns permissions and types.
First-party implementation/tests remain Rust and AGPL-3.0-only. No dependency,
license or hosted-workflow change was made; CI remains workflow_dispatch-only
and no run was dispatched.

## Limits and claim boundaries

Exact successful controls reach 64 offered artifacts, 8,388,608 aggregate bytes
and 10,000 selected object occurrences. Complete invalid-input enumeration can
retain exactly 256 diagnostics; one additional needed detail stops separately.
Shared-value controls reach the hard 1,000,000-work and 8,388,608-text ceilings
and refuse the next charged unit despite elevated caller options. Independently
lowered valid fixtures admit exact measured work and stop below it. Construction
fuel is not charged again; unselected object contents do not count as selected work.

Permutation tests first check each diagnostic against that permutation's actual
artifact reference. They then compare logical diagnostic content using a common
reference mapping: reordered payload bytes necessarily have different digests.
This is not a claim that distinct artifact digests are equal. Interrupted
traversals promise only their observed prefix. Poll panic follows ordinary Rust
unwinding; no callback duration, allocation-capacity or wall-clock SLA is claimed.

No fuzz, mutation-adequacy, Loom or concurrency qualification is inferred from
these tests. This API is serial and immutable, with no lock or scheduler to model.
A completeness flag remains an input assumption, not a deployment guarantee.

## Actual local gates

The following commands completed successfully for the evaluated source/test
bytes. Cargo phases were serial, nice 10, locked/offline, one job and one test
thread, with the existing explicit caches. Evidence is under
reviews/data/native-runtime/.

| Command | Result | Evidence |
| --- | --- | --- |
| `nice -n 10 cargo test --locked --offline --target-dir target -j 1 --no-default-features -- --test-threads=1` | 166 ordinary tests, including all 35 runtime validation tests, and 1 compile-fail doctest passed; 3 named private tests ignored in this lane | validation-qualified-full-tests.txt |
| `nice -n 10 cargo clippy --locked --offline --target-dir target -j 1 --workspace --all-targets --all-features -- -D warnings` | Passed | validation-final-clippy.txt |
| `RUSTDOCFLAGS='-D warnings' nice -n 10 cargo doc --locked --offline --target-dir target -j 1 --no-default-features --no-deps` | Passed | validation-qualified-rustdoc.txt |
| `nice -n 10 cargo fmt --all -- --check` | Passed | validation-qualified-fmt.txt |
| `nice -n 10 cargo build --locked --offline --target-dir target/clean -j 1 --no-default-features` | Passed using the existing cache | validation-qualified-minimal-build.txt |
| `QUIRE_STATE_CORE=/home/peter/dev/worktrees/formalization-a-spec/proposals/state-core nice -n 10 cargo test --locked --offline --target-dir target -j 1 --test fixture_audit -- --ignored --test-threads=1` | 3 selected private audits passed | validation-qualified-private-audits.txt |

The earlier validation-complete-cases.txt is an unsuccessful setup run, not
the final qualification result. The complete passing 35-test run is included
in validation-qualified-full-tests.txt. No failed gate was waived.

## Traceability and remaining work

Quire binds 169/169 Rust test symbols. FR-007 is backed 15/15, FR-008 is 0/20,
TM-004 has 12/23 backed cases and root backing is 165/208. All new tests use
bare single-line shared trace attributes with resolving TC/FR criterion IDs.
NFR metric ordinal labels are descriptive: NFR-006 does not mint corresponding
metric trace IDs, so validation tests bind through TC-065 and FR-007. Initial
unresolving suffix tags and their correction remain in the checkpoint evidence.

There are no untracked symbols or reported status lies. Twenty existing
catalog/classifier diagnostics, six registry duplicate notices and three old
IT-004 unmatched tags remain explicit limitations. The matrix's Coverage Status
header is not the classifier's Status header, so functional status inference is
incomplete; this review manually checked FR-007's fifteen rows against assertions.
Neither a backed tag nor an empty status_lies list alone establishes qualification.

Task-013's validation scope is complete. TC-061's predicate/captured-read
behavior still needs FR-008/TC-071 execution evidence. Task-014 must implement
the source-AST reference evaluator; Task-015 must qualify IT-006 and the complete
plan. Remaining LC02/FS03 acceptance, backend qualification and Quire integration
remain required by the original assignment.
