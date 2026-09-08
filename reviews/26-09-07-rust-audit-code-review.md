---
id: SR-018
title: "Code review — LR02 Rust fixture audits"
type: SpecReview
analysis: code-review
scope: "tools/fixture-audit/, tests/fixture_audit.rs, Cargo manifests, CI and migration documentation"
review_set: subset
evaluated_revision: "d83b4eaac970231e5b3823240a61fd48bf7ea4e1"
---

## Summary

Reviewed the Rust replacements for the four Python verification helpers against
FR-012/NFR-005 and the reviewed manual-CI policy. Local runtime and trace checks
pass; future hosted execution needs access to the private trace dependency.

## Verdict

**CONDITIONAL** — the low hosted-run prerequisite below is documented. Local
Rust remediation is verified; hosted execution, the external producer and full
native assessment are not qualified by this review.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | A fresh hosted runner cannot fetch the private ix-trace-rs dependency with only this repository's default token. Before a future manual run, provide read access to that dependency. Local checks use the selected cache/access; hosted checks were deliberately not dispatched. | Cargo.toml:26; .github/workflows/ci.yml:17; README.md |

## Scope and assurance context

Implementation revision is d83b4eaac970231e5b3823240a61fd48bf7ea4e1. The owning
specification at 11a9128 and full eight-analysis review/plan at f43ace3 preceded
implementation. Layout/budget and canonical-marker clarifications were reviewed
at 91a4fc0 and 5172f9e. Manual CI specification 1649ef7 was reviewed at 2e5cd9a
before workflow change 8cf5571. The current matrix and task completion updates
record these already-performed runs; they do not change acceptance criteria.

Applied the actual shared `/home/peter/dev/agent-skills/code-review/SKILL.md`,
`rust-review/SKILL.md`, `rust-style/SKILL.md` and implementation-gap-analysis
discovery. Rust-review SHA-256 is
bec67626edf3944397fa6c2c83c1164278c9f86db3efdcc67cf91b2152b2d85a.
The repository's reviewed NFR-005 canonical attributes override the older
rust-style comment/name convention. No AssuranceProfile artifact applies in
this repository. No advisory waiver, gate suppression or subagent was used.
The optional semantic gap review was declined and remains skipped.

## Rust review checks

| Check | Result and evidence |
| --- | --- |
| Conventions and idioms | One private audit executable; contextual thiserror envelope, stable Copy codes/catalog, typed immutable identity key, borrowed path/slice boundaries, private modules and deterministic maps. Existing ByteDigest/Source/parser reused. |
| Tests and tracking | All 14 new functions use imported canonical trace attributes. Unit tests stay beside code; integration tests use the real binary. Three ignored tests name IT-004's explicit private-packet lane and were actually run. |
| Seams | Temporary fixture copies and real process/file/parser boundaries; no production cfg(test) behavior switch or replacement of the unit under test. |
| Source completeness | No production placeholder, panic/unwrap, unsafe block, bypass flag, unimplemented mode or new public API. Model-producer is the specified unconditional refusal. |
| Test completeness | Assertions check observed counts/codes/claims and immutable bytes. Independently changed artifacts, producer pin, metadata, source correspondence, path escape and boundary values discriminate failures. |
| Integrity | No lint weakening. Explicit checks remain enabled in optimized builds. Original model/standard fixture bytes and producer history are preserved. |
| Panic and resource surface | OS arguments remain OsString/Path. File and aggregate bytes, file/value counts, JSON depth and native parser Limits bound work. Recoverable errors return Result and nonzero process status. |
| Conversions | Wire offsets use checked unsigned conversion and checked source ranges; scalar positions come from existing Source. No truncating wire casts. |
| Async/blocking | Synchronous one-shot command; no async runtime, child producer, unjoined work or lock held across suspension. |
| State/lifecycle | Local immutable-tree precondition is explicit. Canonical containment precedes reads; nonregular files are rejected before open. Test tempdirs are RAII-owned and original inputs are compared after execution. |
| JSON and contracts | Serde visitor rejects decoded duplicate keys, invalid scalars/UTF-8, trailing input and bounded-depth/value exhaustion. Required consumed fields are fallible and typed. Unknown historical metadata is retained: this is a selected correspondence audit, not a new full shared wire decoder; default deny_unknown_fields guidance is inapplicable to that reviewed boundary. |
| Gates | Formatter, strict all-target/all-feature Clippy, crate tests, selected private packet tests, optimized audit unit tests and separate-target build actually passed. No deny.toml exists, so cargo deny is not a configured gate. |

## Resolved findings during implementation

The first real review-packet run failed because the selected snapshot was
already empty; replacing its objects with an empty list made no change. The
control now appends a distinct synthetic object and separately requires changed
value/bytes before checking stale and recomputed digests. The real packet and
self-test now pass all six controls.

JSON budget errors initially risked prose-based classification; an explicit
exhaustion flag and depth counter now produce the resource code. Source review
also found open-before-file-type-check could block on a FIFO, even for an
immutable tree. A metadata refusal now precedes open. Directory refusal is
executed; FIFO behavior was inspected, not claimed as an executed FIFO test.

Closure comparisons originally could accept two equally malformed fields.
Required fingerprint/object/array shape checks now precede comparison and a
dedicated adverse test verifies the equal-but-invalid inputs refuse. None of
these fixes changes the reviewed FR-012 behavior or creates a shared producer
schema authority.

## Actual local gates

Rust toolchain 1.94.1; locked Cargo SHA-256
26c8dc235777ff79a5882e01a6c700d2feab9fe4d9f4397662490aa37fb9d444.
Commands used --offline and --locked, with --target-dir target except the
separate build target. The local default suite's substantive result lines were:

```text
cargo fmt --all -- --check: exit 0
cargo clippy --offline --locked --target-dir target --workspace --all-targets --all-features -- -D warnings: exit 0
audit unit tests: test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
audit integration tests: test result: ok. 5 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out
CLI integration: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
parser integration: test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
source-map integration: test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
private-packet lane: test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 5 filtered out
release audit units: test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
cargo build --offline --locked --no-default-features --target-dir target/clean: exit 0
```

Library/main/doc targets contain zero tests and are not added to these counts.
The private lane used QUIRE_STATE_CORE pointing to specification revision
36293bae7f5bcb7ca3b2389ed166e525dc9dba87. Its three tests run actual audits plus
independent mutations; none skipped inside that selected run. All four direct
modes produced the specified 23/7/6, 17/4, 5 and 50/1 counts. Producer refusal
returned exit 3 under the actual binary test with no external runtime PATH.

## Traceability and remaining scope

Quire CLI 0.31.0 / engine 0.46.0 reports 35 Rust candidates, 14 tagged and 14
bound. FR-012 is 11/11 backed and the matrix 9/10; TC-010 is explicitly Manual
and has recorded inspection rather than a source symbol. Raw coverage is in
spec/reviews/rust-verification/data/implementation-coverage.json. No stale tagged
symbol or test-summary status lie is reported. This is trace binding, not a
measured code-coverage percentage or optional semantic review.

The overall rollup is 20/78 because earlier native requirements and 21 existing
tests remain outside Plan-001. Baseline CLI encoding, formatter-budget and Rust
idiom findings remain in SR-009. The catalog also disagrees internally about
the functional table's status header; SR-019 records that limitation. No global
catalog, B/C/TL worktree or external producer was changed.

No new concurrency exists, so Loom adds no current state-interleaving evidence.
Bounded generated cases and targeted mutations were executed; randomized
property campaigns, fuzzing, mutation-score tooling and further fault injection
remain reviewer recommendations for the wider parser/qualification scope,
separate from Quoin's deterministic advice. No recommendation is reported as a
performed test or compulsory newly invented blocker.
