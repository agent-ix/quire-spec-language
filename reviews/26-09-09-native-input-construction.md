---
id: SR-096
title: "Code and Rust review of native input construction"
type: SpecReview
analysis: code-review
scope: "FR-018 / Task-012 at c8fa41f6e172e58b9406792d1b9f6b6bd85e52cc"
review_set: subset
---

## Summary

Snapshot and invocation constructors now preserve exact flat input, emitted bytes,
role-specific references and bounded structural errors. Twenty-one public API
tests and a compile-fail role check qualify the three constructor cases.

## Verdict

**PASS** for FR-018 and Task-012. FR-007 population validation, FR-008 execution
and the complete native/backend/Quire workflow remain required downstream work.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings | - |

## Review context and sequence

Evaluated source c8fa41f6e172e58b9406792d1b9f6b6bd85e52cc against FR-018,
NFR-006-M-1..5 and docs/native-runtime-inputs.md. Initial specification 045025f
received all eight QUOIN reviews SR-088–095 before runtime implementation.
The review later found a copied model/checker setup sentence in TC-055–057;
dc63f33 corrects that sentence to the existing construction-only boundary and
29922d7 records all eight review addenda. Existing code was preserved during
that correction; no acceptance result, byte encoding, limit or API changed.

Applied the actual /home/peter/dev/agent-skills/code-review/SKILL.md,
rust-review/SKILL.md, rust-style/SKILL.md and implementation-gap-analysis
discovery. AGENTS.md, README.md, LICENSE-DECISION.md and Cargo lints govern.
No applicable AssuranceProfile, local Rust style override or deny.toml exists.
The optional semantic gap comparison remains declined. Ordinary code/spec
faithfulness and assertion quality were inspected; no subagents were used.

## Requirement and assertion correspondence

| Criteria | Executed evidence |
| --- | --- |
| FR-018-AC-1/5 | All ten ValueNode variants, signed extrema, empty containers, shared earlier children, all snapshot/invocation roots and ordered duplicate metadata survive construction. Model-invalid shapes remain available for later validation. |
| FR-018-AC-2/7 | Self, forward and out-of-range children for every container and invalid State/object-field/parameter/result roots refuse through the public constructors. Tests check original typed draft paths, supplied labels, no emitted artifact and unused-node inspection. |
| FR-018-AC-3 | Hand-authored complete snapshot/invocation envelopes check field order and raw lowercase digest spelling. A separately precomputed SHA-256 literal checks the snapshot bytes. Fifty-four label/revision/observation combinations repeat exact bytes and usage; ten snapshot and eleven invocation mutations change their byte-bound content. |
| FR-018-AC-4 | Distinct snapshot/invocation reference APIs preserve exact labels/digests. Empty labels refuse for both artifacts and both expected-ref constructors; a compiled doctest rejects interchanging reference roles. ValueId is explicitly a local numeric index, without an invented cross-arena identity promise. |
| FR-018-AC-6 | Independent text totals and every metadata-entry class, generated shared-depth families, exact/lowered/zero ceilings, hard-limit clamping, escaped output expansion and small-stack construction/destruction execute. Usage and structured stop locations distinguish preflight refusal from encoded-byte exhaustion. |

## Architecture and Rust checks

The public runtime facade owns distinct immutable Snapshot/Invocation types.
Flat draft data, typed references and errors are separate from construction
inspection/encoding. A closed private Body trait has two real implementations
sharing one bounded artifact constructor. No new lexer, text scanning semantic
parser, external reader, shared evidence authority or model-role inference is
introduced. Serde handles the documented emitted representation.

The arena length is checked before derived depth allocation. Vector entries are
charged before iteration; text lengths before scanning or encoding. Every node
is checked, then a linear depth pass processes each node once without expanding
shared subtrees. Child/root conversions use try_from and checked bounds. The
writer checks output length before appending, and SHA-256 is computed only after
complete emission. Failed construction moves original labels into InputError,
avoiding an unadmitted clone of oversized strings. It returns no partial artifact.

Production changes have no input-driven panic, unwrap, expect, unsafe, unchecked
boundary cast, global state, I/O, async worker, lock, recursive owned input,
test-only behavior or new lint suppression. The writer's debug assertion checks
its private append-length invariant. Its no-op flush is correct for a memory
buffer. InputError implements the standard Rust error interface with a stable
native code; it deliberately has draft provenance rather than a fabricated
source span. Public APIs have docs and public integration callers.

Tests reach only public crate/IR APIs; path modules organize test cases and do
not expose implementation internals. Assertions use actual bytes, values, paths
and counters, including independent oracles. Existing serde_json string escaping
is reused only where that primitive is itself the specified encoding. No clock,
network, semantic double or ignored new test supplies a result. One explicitly
joined 512-KiB test thread checks flat construction and destruction; this is not
a concurrency campaign. Loom, fuzzing and mutation adequacy are not claimed.

All first-party source/tests remain Rust and AGPL-3.0-only. Direct serde derive
enablement makes an already resolved IR feature explicit; Cargo.lock and the
package/license inventory are unchanged. Hosted CI still has workflow_dispatch
only and was not dispatched.

## Resource evidence limits

Exactly 100,000 child entries and depth 64 succeed; the next unit refuses even
with elevated options. Exactly 1 MiB encoded content succeeds, while one more
byte refuses. A 100,000-node arena passes node/depth inspection but necessarily
hits the coupled encoded-byte ceiling; 100,001 nodes stop at preflight with zero
node/output usage. This is not reported as a successful maximum-sized artifact.
Independent lowered node ceilings also execute. Text and output counters are
separate even though their option is shared. Counters describe admitted content
and structural work, not allocator capacity, caller allocation cost or latency.

## Actual local gates

Final commands below completed successfully against the source/test bytes in
the evaluated commit. Cargo phases ran serially, nice 10, locked/offline, one
job and one test thread, reusing the explicit existing target caches. Evidence
paths are relative to reviews/data/native-runtime/.

| Command | Result | Evidence |
| --- | --- | --- |
| `nice -n 10 cargo fmt --all -- --check` | Passed | input-fmt.txt |
| `nice -n 10 cargo test --locked --offline --target-dir target -j 1 --test runtime_inputs -- --test-threads=1` | 21 passed | input-complete-cases.txt |
| `nice -n 10 cargo test --locked --offline --target-dir target -j 1 --no-default-features -- --test-threads=1` | 131 ordinary tests and 1 compile-fail doctest passed; 3 named private tests ignored in this lane | input-default-tests.txt |
| `nice -n 10 cargo clippy --locked --offline --target-dir target -j 1 --all-targets --all-features -- -D warnings` | Passed; the workspace has one member and no optional features | input-clippy-all-features.txt |
| `RUSTDOCFLAGS='-D warnings' nice -n 10 cargo doc --locked --offline --target-dir target -j 1 --no-default-features --no-deps` | Passed | input-rustdoc.txt |
| `nice -n 10 cargo build --locked --offline --target-dir target/clean -j 1 --no-default-features` | Passed using existing cache | input-minimal-build.txt |
| `QUIRE_STATE_CORE=/home/peter/dev/worktrees/formalization-a-spec/proposals/state-core nice -n 10 cargo test --locked --offline --target-dir target -j 1 --test fixture_audit -- --ignored --test-threads=1` | 3 selected private fixture audits passed | input-private-audits.txt |

The initial missing-API test failure, private-interface warnings, useless_concat
Clippy failure and old diagnostic-count assertion (18 rather than the reviewed
19) are retained in the earlier logs. They were corrected before the final
gates; no warning or acceptance assertion was suppressed. Raw command output
retains its original trailing blank lines. The first template command omitted
the current CLI's required repository argument; the corrected scoped command
succeeded. These initial failures are not qualification successes.

## Traceability and remaining work

Quire 0.31.0 / engine 0.46.0@ca7362d4 binds 134/134 Rust test symbols, with
FR-018 7/7 criteria and TM-004 3/23 cases backed. Root backing is 141/208,
not complete runtime coverage. No untracked symbols or reported status lies
appear. Twenty catalog/classifier diagnostics, three preexisting unmatched
IT-004 tags and six registry duplicate notices remain explicit limitations.
In particular the catalog expects Status where its matrix skeleton uses
Coverage Status, so that functional-row status classifier is skipped. This
review checks the seven construction rows against real tests manually; an
empty status_lies list alone does not establish that result. NFR-006 uses its
actual metric rows, not the catalog's inapplicable optional AC section.

The scope scan found no constructor stubs or unowned behavioral constraints.
Task-013 must still validate populations, closure, invocation captures, deltas
and frames before producing a ValidatedContext. Task-014 must execute the source
AST, and Task-015 must qualify IT-006. LC02/FS03 acceptance, backend qualification
and Quire integration remain separate required steps of the full assignment.
