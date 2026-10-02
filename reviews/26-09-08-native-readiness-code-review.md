---
id: SR-028
title: "Native syntax merge-readiness code and Rust review"
type: SpecReview
analysis: code-review
scope: "Plan-002 native source, formatter, CLI, tests and SR-009 dispositions"
review_set: subset
evaluated_revision: "cbccbb61e9bf15690af386d857b6255ed2bfb948"
review_date: "2026-09-08"
---

## Summary

The native cleanup resolves SR-009's current implementation findings. Actual
local checks pass; all 41 Rust test symbols bind under the installed Quire
reader. This is a syntax/fixture milestone, not a linked or evaluated state
workflow. LR02's unchanged Rust audits retain SR-018's reviewed boundary.

## Verdict

**CONDITIONAL** for external tooling/hosted setup below; no open medium or high
finding prevents the owner-authorized private merge under local-only CI.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Installed process catalog requires Coverage Status but its functional coverage reader selects Status; status checking is skipped on that table. Actual TC-summary statuses and execution are reconciled explicitly. Six duplicate registry diagnostics remain visible. | spec/native-readiness/tests.md:18; spec/reviews/native-readiness/data/implementation-coverage.json |
| FND-002 | low | A future hosted runner cannot fetch the private ix-trace-rs pin without repository read access. No hosted run is an acceptance gate under the owner's current local-only policy; configure access before any later dispatch. | Cargo.toml:31; .github/workflows/ci.yml:3; NFR-002 |

## Scope and skills

Used the actual agent-skills/code-review/SKILL.md, its Rust dispatch to the
owner-named agent-skills/rust-review/SKILL.md, default rust-style, and the
implementation-gap-analysis discovery phase. Rust review SHA-256 remains
bec67626edf3944397fa6c2c83c1164278c9f86db3efdcc67cf91b2152b2d85a.
AGENTS.md and the dependency/license decisions govern the review. There is no
applicable AssuranceProfile or overriding Rust idiom file in this repository.
No subagent or optional gap-analysis semantic pass was used.

Native refinements were specified at a10ec80/afeeb20 and reviewed in SR-020–027
at f1f6c50. Compilation exposed thiserror's implicit source-field inference;
the exception was specified at 5d0c9de and reviewed at 03dfc72 before the manual
trait implementation. Source implementation is df2d0b5; cbccbb6 only splits one
multiline trace into two canonical attributes after Quire failed to bind it.

## SR-009 dispositions

| Previous finding | Disposition and observed evidence |
| --- | --- |
| FND-001: OS argument panic | args_os plus separate UTF-8 command/label validation; real FF arguments exit 2 before missing-file I/O, while an FF file path parses exact bytes and preserves valid labels. TC-017. |
| FND-002: formatter ceiling | Additive format_with_limit checks every append with checked_add; zero/one-short/exact/above/default/usize::MAX boundaries execute. A 1 MiB admitted source that expands refuses. Existing format API remains. TC-016. |
| FND-003: Error interoperability | Diagnostic implements Display/Error directly, preserving source provenance. Pinned thiserror 2.0.20 infers source as a nested error, so the reviewed compatibility exception avoids renaming fields or adding a false error cause. Actual boxed Diagnostic propagates through ?, downcasts intact and has no cause. TC-018. |
| FND-004: untraced native tests | Nine real TC artifacts and TM-002 cover the native boundary. All 21 old native tests and six added tests have imported canonical trace attributes. Quire now binds 41/41 repository test candidates including LR02. |
| FND-005: missing CLI outcomes | Real processes assert usage/missing-file/directory-read exit 2, resource exit 3, refusal exit 1 and parse exit 0 with an independently selected source digest. TC-015/017. |
| FND-006: code catalog | Code all/from_code/Display and the owned native catalog retain all ten spellings. Enumeration, uniqueness, lookup and unknown-code refusal execute. TC-018. |
| FND-007: API documentation | Native modules, public types, variants, fields and accessors describe implemented ownership/coordinate bounds. Strict missing_docs rustdoc passes. |
| FND-008: identity leaf types | Reviewed deferral: source/revision remain compatible opaque diagnostic labels. Validated distinct shared refs belong at LC02's future trust boundary; they are not silently inferred from these Strings. |
| FND-009: Python child timeout | Superseded by LR02 removal of all four Python executables and unapproved producer refusal with no child launch. Historical producer bytes are retained; fresh qualification is not claimed. |

## Rust rubric and implementation discovery

| Check | Result |
| --- | --- |
| Errors, ownership and traits | One structured native diagnostic boundary; borrowed OS path/labels, checked code lookup and the documented derive exception. No public dynamic/string error API or new trait abstraction. |
| Tests and seams | Real public APIs, actual CLI processes and isolated tempdirs; formatter unit test uses the production lexer to compare ordered raw spellings. No replacement of the tested implementation, clock threshold or test-only production branch. |
| Completeness and integrity | No todo/unimplemented placeholder, lint suppression, unsafe addition or lowered gate. lib.rs exposes the real crate API. Forward requirements are unfinished work, not implemented stubs. |
| Panic and arithmetic | Formatter's token expectation and brace decrement rely on private immutable ParsedUnit invariants. Every output append checks overflow and the selected content ceiling first. Parser source/token/node/depth ceilings and checked external spans remain intact. The bounded malformed corpus now asserts actual outcomes, labels and ranges. |
| Conversions and I/O | CLI uses a widening conversion of its fixed 1 MiB read ceiling plus one sentinel; no truncating wire conversion was introduced. Lossy path text is explicitly display-only. Caller-selected local file I/O is outside an async runtime and does not claim cancellation/deadline guarantees. |
| State and lifecycle | No production task, lock, mutable shared state or child producer was added. The existing constrained-stack test joins its worker; immutable sources and RAII tempdirs retain ownership. Loom has no production interleaving to explore here. |
| Untrusted input | Source/refusal handling stays separate from later domain/portable wire decoding. Native parse rejects BOM at lexical admission; the Source abstraction also retains original non-native document bytes for correspondence. Existing strict audit JSON/path rules are unchanged. |
| Reverse requirement ownership | Intake/digest/coordinates: FR-001; declarative tokens, syntax and arena: FR-002; bounded whitespace formatter: FR-003; source maps: FR-004; CLI/error catalog: FR-010; all ceilings: NFR-001; audit behavior: FR-012/NFR-005; local CI and rights: NFR-002/004. No new unowned executable surface found. |

## Actual local gates

Rust 1.94.1, Cargo.lock SHA-256
26c8dc235777ff79a5882e01a6c700d2feab9fe4d9f4397662490aa37fb9d444.
All Cargo build/test commands below used --offline --locked and --target-dir
target, except the separate build target target/clean.

```text
cargo fmt --all -- --check: exit 0
cargo clippy --all-targets --no-default-features -- -D warnings: exit 0
cargo clippy --workspace --all-targets --all-features -- -D warnings: exit 0
cargo test --no-default-features: 38 passed, 0 failed, 3 explicitly ignored
formatter units: 1 passed
audit units: 6 passed
CLI integration: 3 passed
audit integration: 5 passed, 3 ignored
native boundary integration: 3 passed
parser integration: 11 passed
source-map integration: 9 passed
selected private-packet lane: 3 passed, 0 failed, 0 ignored, 5 filtered out
cargo build --no-default-features --target-dir target/clean: exit 0
RUSTDOCFLAGS='-D missing_docs' cargo doc --no-deps --lib: exit 0
fixture-audit self-test: 6 content/digest negative controls; duplicate keys refused
fixture-audit model-bytes tests/fixtures: 5 checkpoint byte digests and exact producer pin
quire-spec parse test:parent fixture:1 tests/fixtures/parent.native: parsed, 4 clauses
```

The selected private lane uses QUIRE_STATE_CORE pointing to
formalization-a-spec/proposals/state-core at e897f810a7356d4ce8fd19026221ebda7b65596f.
An initial invocation incorrectly used spec/state-core and failed on the absent
directory; correcting the explicit path made all three tests execute and pass.
Two initial boundary-test setup errors were also corrected before the passing
full suite. No failed attempt is counted as a pass. After cbccbb6's attribute-only
edit, formatter/strict Clippy and all three CLI tests passed again. No runtime
source or dependency changed after the full suite.

No deny.toml exists. No hosted CI was dispatched. Fixture bytes are unchanged
from 5fbc96e. Random fuzzing, mutation-score campaigns and broader fault
injection remain recommendations, not completed evidence.

## Coverage and claim limits

Actual Quire reconciliation: 41 candidates, 41 tagged, 41 bound; zero untracked
symbols and zero reported status lies. FR-001/002/003/004/010 total 28/28 backed;
TM-002 is 9/9. FR-012 remains 11/11, TM-001 9/10 with TC-010 explicitly Manual.
The global 57/93 includes unimplemented future requirements and stakeholder
workflow acceptance. Three unmatched IT-004 mentions are existing ignore-lane
labels, not unresolved canonical TC attributes. Optional NFR AC selectors and
empty future SuiteRegistry/Inspections are setup diagnostics, not new features
to invent. The known future NFR-003 method spelling remains outside Plan-002.
Raw coverage and visible diagnostics are retained with the native spec reviews.
