---
id: SR-054
title: "Code and Rust review of native formal linkage"
type: SpecReview
analysis: code-review
scope: "src/linking.rs, native diagnostics, tests/linking.rs, Cargo and local qualification"
review_set: subset
evaluated_revision: "d44e97424ecfe42343edb869e9203275aca26db1"
---

## Summary

The native library resolves exact formal imports and scoped declaration names
through the existing Contract IR API. Its immutable output preserves native
source and formal provenance without claiming type correctness or evaluability.

## Verdict

PASS for FR-005/013 native linking and its local qualification. The full LC02
typing and state-workflow acceptance remain incomplete.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No unresolved code finding in the implemented linking scope. | src/linking.rs; tests/linking.rs |

## Applied review and provenance

Applied the actual agent-skills/code-review/SKILL.md and
agent-skills/rust-review/SKILL.md, rust-style defaults and repository AGENTS.md.
No applicable AssuranceProfile is declared in spec/. The owner's canonical
single-line imported trace attributes override the historical doc-tag examples.
All ten new tests have compiler-checked tags bound by Quire's declared grammar.
No additional agents or optional gap-analysis semantic comparison were used.

The concrete API was specified at c78792a and all eight QUOIN reviews preceded
implementation at 8dc6b48. Corrections to the example at ecaf4cf were re-reviewed
through all eight retained records at 01e597d before corrected tests continued.

## Rust and architecture checks

The API consumes ParsedUnit and borrows a slice of immutable validated IR
environments. LinkedPackage owns the exact source and keeps selected model
references; no model is cloned into a second type authority. Public getters
cannot replace the source, declarations or linkage. Formal identity uses the
complete RequirementRef and a closed type/field/value/variant key. Related
locations sort by identity and provenance; lexical locations stay native spans.

One existing Box<Diagnostic> boundary carries stable codes, phase, source and
structured upstream context. The existing manual Display/Error implementation
is retained because its public source field denotes provenance, not Error::source.
There is no public string/dynamic error, new decoder, IO, async worker, shared
mutable cache, unsafe code, feature bypass, hidden fallback or alternate binder.

The ExprKind and operator dispatches are exhaustive. Scalar arithmetic does not
propagate a record receiver; unknown shape needed for field lookup refuses.
Name resolution remains separate from guards, type inference, Boolean roots,
observation availability and the later reference/operation mapping.

Counts are checked before traversal; depth is checked before descending. The
canonical emitter receives the minimum remaining aggregate/per-model budget.
The retained bounds count emitted bytes, not allocator capacity or a new IR
heap guarantee. Total subtraction/addition is within the 8 MiB ceiling enforced
by the pinned emitter. The usize-to-u64 conversion is checked after a 1 MiB clamp.
The two expect sites concern a bounded conversion and private ParsedUnit handles;
callers cannot construct invalid handles or mutate the arena through this API.
No recoverable caller failure reaches an expect or library panic.

## Tests and resolved observations

Real IR constructors and native parser calls reach the actual linker. Assertions
check exact owner/key/source loci, raw-byte versus semantic digest substitutions,
an unused declaration changing the closure, scope shadowing and isolation,
option/collection/conditional receiver shape, enum identities and missing names.
Permutation families reject missing/stale/ambiguous input in every position;
successful neighboring clauses and prior calls cannot expose a partial package.
Zero/lowered/hard limits, exact byte ceilings, aggregate exhaustion and deep flat
syntax are exercised with real data. No compiler/evaluator logic is mocked.

The initial fixture attempts failed before linkage because source/package names
used URI spellings rejected by IR, then because value is a reserved native token.
The example now uses an actual PackageId namespace and count field, without
changing either grammar. Those failed setup runs are not qualification evidence.
The first full regression run then found the legacy ten-code census; it now
expects the specified fifteen codes and asserts empty legacy related/upstream
context. Initial record-shape propagation through unary/arithmetic syntax was
also corrected before qualification; adverse receiver tests prevent recurrence.

## Actual local gates

All Cargo work after the owner's resource report used nice -n 10, one build job
and serial test execution. No builds overlapped and no hosted run was dispatched.

```text
cargo +1.98.1 fmt --all -- --check
exit 0
cargo +1.98.1 test --offline --locked --target-dir target -j 1 --no-default-features -- --test-threads=1
48 passed; 0 failed; 3 named private-lane tests ignored
QUIRE_STATE_CORE=/home/peter/dev/worktrees/formalization-a-spec/proposals/state-core cargo +1.98.1 test --offline --locked --target-dir target -j 1 --test fixture_audit -- --ignored --test-threads=1
3 passed; 0 failed
cargo +1.98.1 clippy --offline --locked --target-dir target -j 1 --all-targets --all-features -- -D warnings
exit 0
RUSTDOCFLAGS='-D warnings' cargo +1.98.1 doc --offline --locked --target-dir target -j 1 --no-deps
exit 0
cargo +1.98.1 build --offline --locked --no-default-features --target-dir target/clean -j 1
exit 0
quire validate --scope . 'spec/**/*.md' 'plan/**/*.md' 'reviews/**/*.md' --summary
134/134 docs grammar-clean before these two review artifacts; 0 grammar findings
git diff --check
exit 0
```

[Actual logs](data/native-linking/formalization-a-link-all-tests.txt) and sibling
files retain default/private tests, Clippy, rustdoc, build and coverage output.
The rights snapshot has 138 packages including this crate, all with declared
licenses. IR remains MIT OR Apache-2.0; new source is AGPL-3.0-only; ICU and
other dependency grants remain intact. No deny.toml exists and no cargo-deny
or vulnerability-scan result is claimed. Hosted credential setup is unqualified.

## Implementation gap discovery

Seven behavior groups are owned by FR-005/013 and the detailed binding contract:
exact imports, declaration identity, original provenance, lexical scopes,
receiver shapes, resource limits and atomic diagnostics. No unowned behavior,
source stub or test stub was found in the changed surface. The companion gap
review preserves incomplete typing, runtime, backend and extraction acceptance.
