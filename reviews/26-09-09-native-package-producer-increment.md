---
id: SR-109
title: "Code and Rust review — initial native package producer"
type: SpecReview
analysis: code-review
scope: "Task-016 initial increment: src/package.rs, src/package/, checking type iterator, native Code vocabulary and package constructor tests"
review_set: subset
evaluated_revision: "22e3c5de657d06985db86202317a26ad1132749f"
review_date: "2026-09-09"
---

## Summary

The initial producer retains actual checked source/model correspondence and
uses bounded Serde encoding with separate native identity. Local gates pass,
but the reviewed Task-016 qualification is incomplete; the reader must not
treat this increment as a completed producer qualification.

## Verdict

**CONDITIONAL** — continue the remaining producer tests and fixture/schema
qualification under Plan-007 before Task-017 depends on a completed Task-016.
No high-severity implementation finding remains in this inspected increment.
This author's review is not independent B/C acceptance or whole-epic closure.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | TC-090 still needs fixed complete canonical byte fixtures, including multiple clauses; current positive expectations are independently composed in Rust, while the old fixed header-only JSON is adverse data. | tests/package_construction_cases/vectors.rs:71; tests/fixtures/native-package/README.md; TC-090 |
| FND-002 | medium | Complete lexical occurrence, ordered parameter/result, transitive skipped-input, static-mutation, runtime-independent identity and type-role controls remain before the producer claims all planned correspondence/identity coverage. | plan/Plan-007-native-packages/tasks/Task-016-package-construction.md:73; TC-079; TC-082; TC-090; TC-091 |
| FND-003 | medium | The structural package schema has not been compiled/exercised with the reviewed Rust Draft 2020-12 development feature; Markdown/schema syntax inspection does not supply this evidence. | schemas/native-linked-package-1.schema.json; Task-016; TC-083 |
| FND-004 | low | The installed trace engine lists NFR-007 metric obligations but does not mint their trace targets: 20 exact metric references remain reported as untracked. Preserve the diagnostic and reconcile the explicit TC-088 evidence rather than deleting the IDs or claiming zero untracked references. | data/native-packages/producer-coverage-final.json; src/package/encoding/tests.rs:10; tests/package_construction_cases/limits.rs |

## Review scope and method

Applied the actual local agent-skills/code-review/SKILL.md,
agent-skills/rust-review/SKILL.md and rust-style/SKILL.md. Read the owning
AGENTS.md, README, LICENSE-DECISION and Cargo lints; their shared trace attributes
take precedence over the portable skill's legacy doc-line convention. The
Rust lane replaces Python-specific checks. No applicable AssuranceProfile,
repository-specific Rust skill, deny.toml or asynchronous/concurrent boundary
exists in this scope. The installed Quoin authoring pack resolved org agent-ix
and supplied the SpecReview skeleton/schema used here.

Reviewed against specification 41da6e5 / all-eight reviews 69588ad and the
admitted-source correction 2c6b9b8 / all-eight supplements 1c3aa50. Changes to
the reviewed contract reopen specify/spec-review. This increment changes no
FR/NFR behavior; its status notes distinguish implemented work from remaining
qualification. Source, model, runtime, native static and IR identities retain
their stated boundaries. The optional semantic gap pass was not run.

## Code and Rust checks

- Errors use the existing stable Code and a thiserror-derived PackageError
  with typed native/JSON causes, field/index path and request-local pass usage.
  No string is parsed to recover classification. Unknown wire/feature codes
  are declared for the reviewed reader contract; no reader stub is exposed.
- NativePackage owns a constructor-produced CheckedPackage and final bytes;
  models remain immutable borrowed authorities. Manifest views borrow inputs,
  preserve source import/clause order and retain all selected artifact bytes.
  No separate binder, expression wire, proof-as-executable conversion or model
  interpretation is introduced. IR dispositions remain explicitly unlowered.
- Feature matches exhaust native syntax/operator/type enums. Complete selected
  declarations contribute features even when unused; exact owner visits and
  non-expanding named references keep object cycles finite. The new crate-only
  type iterator avoids scanning every expression against every clause.
- Serde owns punctuation, escaping and integer spelling. Formatter events
  charge entries, decoded string bytes and container depth; the writer checks
  bytes before appending. Derive, canonical and final encoding have separate
  actual counters; an unentered pass stays absent. Feature discovery is bounded
  by admitted source/model/AST limits, not presented as JSON-entry or elapsed
  performance measurement. The private hard-ceiling tests explicitly isolate
  counters where shared public limits would stop an earlier pass.
- Canonical preimage includes the named domain and exact NUL separators and
  excludes only the stated identity/projection members. Complete package bytes
  retain dispositions. The Unicode/u64 oracle and independently changed
  preimages detect global key sorting, slash escaping, normalization, rounding,
  dropped domain segments/separators and a final newline.
- No unsafe, panic/unwrap/expect, unchecked wire casts, TODO/unimplemented
  placeholder, new allow lint, lock, async task or I/O exists in package
  production code. The two-element windows indexing has a fixed internal
  cardinality; counters use checked arithmetic. Public items are documented;
  private views/adapters remain module-local. Tests use real parse/link/check
  and admitted models, without replacing the unit under test.
- Quire initially omitted six multiline trace-attribute candidates. Splitting
  the same IDs into compact standard attributes restored all six while
  preserving rustfmt and macro checks. Every new test is tagged and all 224
  discovered Rust candidates bind. Metric-target limitations remain FND-004.

## Defects and setup corrections resolved in the increment

The new Code variants initially had as_str mappings but were absent from all(),
making from_code return None. A new exact-membership test failed at that public
boundary before the enumeration fix. The existing vocabulary count was then
updated from 26 to the reviewed 29; its round-trip/uniqueness assertions remain.
Clippy's redundant single-argument concat! was removed, and observation output
now uses an iterator instead of allocating a temporary small Vec.

The original header-only positive fixture error triggered the separate
specification correction and all-eight review before implementation resumed.
Two later test setup/expectation errors are retained in the logs: differing
model source bytes reused one formal source identity, and a constant
precondition omitted post populations needed by its existing frame contract.
Neither native admission nor checking was relaxed. Corrected positive oracles
follow the first producer implementation; that sequence is explicit.

## Actual gate evidence

All final commands below completed with exit 0 at the evaluated Rust source.
The full command/pin record and original failures are in
[producer-verification.txt](data/native-packages/producer-verification.txt).
Only terminal blank lines in saved logs were normalized for git diff --check.
Cargo phases ran serially at nice 10, one job, one test thread, locked/offline
and using the existing target cache. Rust 1.98.1 was verified. No CI dispatch ran.

| Gate | Actual result | Evidence |
| --- | --- | --- |
| Formatting | cargo fmt --all -- --check passed | data/native-packages/producer-format.txt |
| Strict Clippy | all targets / no default features passed with -D warnings | data/native-packages/producer-clippy.txt |
| Normal regression | 221 tests plus one compile-fail doctest passed; three named private tests selected separately | data/native-packages/producer-regression-final.txt |
| Producer increment | 16 public tests and three private encoding controls included in the normal regression | data/native-packages/producer-regression-final.txt |
| Cached minimal build | no-default-features build passed; no clean-cache claim | data/native-packages/producer-minimal-build.txt |
| Rustdoc | no-deps/no-default-features passed with -D warnings | data/native-packages/producer-rustdoc.txt |
| Audit controls | six content/digest controls, duplicate-key rejection and five historical model-byte checks passed | data/native-packages/producer-audit-self-test.txt; data/native-packages/producer-audit-model-bytes.txt |
| Private audit lane | all three selected tests passed against an immutable archive of adopted standard e897f81 | data/native-packages/producer-private-audits.txt |
| Trace inspection | 224/224 Rust candidates bound; 221/249 matrix rows backed; all package rows remain Planned | data/native-packages/producer-coverage-final.json |
| Scoped document validation before this report | 253/253 grammar-clean | data/native-packages/producer-spec-validation.txt |

Trace output retains 22 catalog/classifier diagnostics, six registry diagnostics,
three historical unmatched IT-004 tags and FND-004's 20 metric references. No
status lie is reported, but the installed Status/Coverage Status mismatch means
that alone cannot qualify a matrix. No fuzz campaign, Loom run, benchmark,
mutation-adequacy result or unexecuted hard public maximum is claimed. There is
no deny.toml, so no cargo-deny lane applies. Dependency versions/features and
grants are unchanged; new first-party source and fixtures are AGPL-3.0-only Rust.

## Implementation gap discovery

Applied the actual implementation-gap-analysis discovery categories to this
increment and scanned all src Rust files for placeholders. Defensive cardinality
checks, exact identities, pure ownership and resource ceilings map to existing
FR-019/021 and NFR-007. No new unstated requirement or unrelated repository edit
was needed. Remaining findings are already scoped Task-016 evidence obligations
and a retained tool discrepancy, not permission to claim the producer task done.
No new retro/skill artifact is required for an undiscovered requirement.

Task-016 remains in_progress; Task-017 and Task-018 remain not_started. PR #12
stays draft. Independent native-domain registration/FS05 adoption, actual IR
lowering, qualified backend/compiled ConfigVersion and Quire integration remain
mandatory parts of the original objective.
