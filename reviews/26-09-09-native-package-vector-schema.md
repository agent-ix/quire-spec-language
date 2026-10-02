---
id: SR-110
title: "Code and Rust review — fixed package vectors and structural schema"
type: SpecReview
analysis: code-review
scope: "Task-016 fixed producer/canonical vectors, Rust fixture author, schema development feature and shared private-test setup"
review_set: subset
evaluated_revision: "cffcaa413f800dd3840eb002d894d3a4f38f2a42"
review_date: "2026-09-09"
---

## Summary

The fixed minimal, control and multiple-clause vectors now match the actual
producer and its private canonical pass. The selected Rust Draft 2020-12 schema
compiles and passes positive and structural adverse cases. This resolves
SR-109 FND-001 and FND-003 within their stated scope; Task-016 still has other
correspondence and identity obligations.

## Verdict

**CONDITIONAL** — this tested increment is ready in draft PR #12; Task-016
remains in progress before reader work depends on completed producer
qualification. No high-severity implementation finding remains in this change.
This is the author's review, not independent B/C acceptance.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Lexical/parameter/result/transitive correspondence, further static dependency mutations, runtime-independent identity and type-role controls remain. A producer omitting these claims could still pass the current fixed constant-clause vectors. | SR-109 FND-002; Task-016; tests/package_construction_cases/fixed.rs; TC-079; TC-082; TC-090; TC-091 |
| FND-002 | low | The installed engine still reports 20 exact NFR metric references as untracked despite listing their obligations. A census alone cannot establish full metric qualification; preserve explicit evidence and the diagnostic. | SR-109 FND-004; data/native-packages/vector-schema-coverage.json; TC-088 |

## Review method and scope

Applied the actual /home/peter/dev/agent-skills/code-review/SKILL.md,
rust-review/SKILL.md, rust-style/SKILL.md and implementation-gap-analysis
discovery categories. Read the repository guidance and Cargo lints. No applicable
AssuranceProfile, repository-specific Rust idiom skill or deny.toml exists.
The existing Quoin SpecReview authoring pack supplies this artifact's form.
Shared trace attributes follow repository conventions; legacy doc-line examples
in the portable skill do not replace them. The optional semantic gap pass was
not run.

Scope remains specification 41da6e5 / all-eight review 69588ad and correction
2c6b9b8 / all-eight supplements 1c3aa50. No production API, accepted language,
wire field, authority or acceptance criterion changed. The source/model fixture
bytes are upstream inputs; parser/linker/checker outputs never author the
expected package fields. Fixed data follows the first implementation, and all
original setup failures remain recorded.

## Code and Rust observations

- Frozen canonical and complete artifact bytes are distinct files. The public
  test compares exact output bytes, raw hash and independent domain-prefixed
  SHA-256 against the frozen digest. The private test compares actual canonical
  pass bytes before hashing. These detect a wrong production member order,
  source offset, integer/control spelling or omitted second clause without
  relying solely on producer/reader agreement. TC-090 runtime controls remain
  unqualified; fixed vectors alone do not close the whole case.
- Shared setup invokes real model admission, parse, link_native and check with
  external authored bindings. The unit-test alias existed already. Clippy
  caught duplicate module loading; the existing runtime setup is now compiled
  once at the test crate root and reused. Production functions have no test
  branches or bypasses. The old private runtime corruption controls still pass.
- Serde continues to own JSON grammar. The independent fixture recipe is a
  fixed record composition, not another general parser. Exact upstream model
  source/artifact files prevent unnoticed input regeneration. Its maintenance
  executable returns io::Error for argument/filesystem refusals and creates a
  fresh directory before writing fixed filenames. Constant fixture assertions
  are not on a request-input path. Existing-directory and argument refusals
  leave candidate files unchanged; no ambient environment or network input
  supplies expected semantics.
- One scoped expect(dead_code) documents the example's deliberate import of
  the generic shared fixture decoder without its default-fixture convenience
  items. It is confined to that support import, not a global warning waiver or
  disabled functional test. Strict all-target Clippy passes. The private test
  lane has no added dead-code suppression.
- jsonschema 0.17.1 retains its MIT grant and resolves only draft202012, with
  HTTP/file/CLI resolution disabled. The lock adds one root development edge;
  all 138 package/version/source/license entries remain identical. The
  structural mutation walk is iterative over bounded fixed test fixtures and
  tests missing, extra and wrong-type members at every encountered object.
  Real invariant/local-binding/pre/post/result packages also exercise the
  schema. A false model-artifact claim remains structurally valid by design;
  the future reader must establish correspondence through real reconstruction.
- New code has no unsafe, global state, async/locking boundary, unchecked wire
  casts, test mock or unfinished production placeholder. I/O is confined to
  explicit fixture authoring. Discovery found no new unstated requirement:
  remaining evidence is already scoped by Plan-007, with no shared repository
  or global-skill edit needed.

## Actual validation

All final gates below completed with exit 0 at the evaluated source. The full
commands, retained failures and limits are in
[vector-schema-verification.txt](data/native-packages/vector-schema-verification.txt).
Cargo phases ran serially at nice 10, one job and one test thread, locked and
offline. No hosted CI was dispatched. Saved logs normalize only final blank
lines for git diff --check.

| Gate | Result | Evidence |
| --- | --- | --- |
| Formatting / strict Clippy | Passed, all targets and no default features | vector-schema-format.txt; vector-schema-clippy.txt |
| Regression | 225 ordinary tests plus one compile-fail doctest passed | vector-schema-regression.txt |
| Private audit lane | Three selected tests passed against adopted e897f81 immutable archive | vector-schema-private-audits.txt |
| Cached build / strict rustdoc | Passed; no clean-cache claim | vector-schema-build.txt; vector-schema-rustdoc.txt |
| Audit self-test / model bytes | Six controls plus duplicate refusal and five historical digests passed | vector-schema-audit-self-test.txt; vector-schema-audit-model-bytes.txt |
| Fixture author | All 14 candidate files match; existing/missing/extra-argument calls each refuse with exit 1 | vector-author-final.txt; vector-author-*-argument.txt; vector-author-existing-directory.txt |
| Structural schema | Two focused tests pass; compiler setup failures retained separately | structural-schema-final.txt; structural-schema-tests.txt; structural-schema-result-setup.txt |
| Trace census | 228/228 Rust candidates bound; 221/249 matrix rows backed | vector-schema-coverage.json |
| Scoped document validation before this report | 234/234 grammar-clean | vector-schema-validation.txt |

The trace census retains 22 catalog diagnostics, six registry diagnostics,
three historical unmatched tags and the 20 metric references above. Package
matrix rows remain Planned; no status lie is reported, but that does not
establish full acceptance. No fuzz campaign, Loom result, benchmark, mutation
adequacy or independent consumer adoption is claimed.

Task-016 remains in_progress, Task-017/018 remain not_started and PR #12 stays
draft. Independent FS05/domain acceptance, executable IR lowering, qualified
backend/compiled ConfigVersion and Quire integration remain part of the original
assignment. The remaining ordered tasks are recorded in Plan-007.
