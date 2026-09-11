---
id: SR-340
title: "Code and Rust review of the native protocol producer example"
type: SpecReview
analysis: code-review
scope: "examples/native_protocol_handoff.rs; examples/protocol-handoff/producer.rs; examples/protocol-handoff/model.json; examples/protocol-handoff/workflow.body.native; examples/protocol-handoff/README.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-042
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-121
    type: references
---

## Summary

Review of the five new example files at `6243ef9` against base
`agent-a/native-population-exports`; no library or spec file changed. Skills
applied: `/home/peter/dev/agent-skills/code-review/SKILL.md`, which dispatches
the Rust lane to `/home/peter/dev/agent-skills/rust-review/SKILL.md`, with
`agent-skills/rust-style` as the portable idiom default because this repository
documents no Rust idiom skill of its own. The recipe is real end to end: located
rule model, authored native body, registry-selected Edition/definition/rule
bytes, the embedded compiled-protocol contract, the actual stripped `current_exe`
ELF, then `native::admit` → `native::emit` → public `artifact::read`. Nothing is
synthetic and no expected field is copied back from the offered package. The
findings are error-surface, fidelity and structure issues in the example itself.

## Verdict

**CONDITIONAL** — no `high` finding. The compilation chain, the input selections
and the reader expectations are genuine; four `medium` items concern the operator
error surface, one hardcoded semantic revision and a 362-line producer body.

## What was verified as real, not asserted

- Every dependency in the produced package is byte-identical to its recorded
  digest: 67/67 files under `/tmp/quire-native-producer-fixture-20260911`
  recomputed clean, and the package digest matches `compiled-protocol.ref.json`
  (`sha256:baa1d368…f27c4`).
- `dependencies/0.bin` is the actual example executable — `file` reports a
  stripped ELF 64-bit PIE, and its SHA-256 equals the built
  `release/examples/native_protocol_handoff`. No source file, string or synthetic
  producer stands in for the binary.
- The baseline is the registered `RegisteredDefinition::Edition` selection
  (`ix:native`, semantic revision `1-draft.2`) whose bytes come from
  `include_bytes!` over `resources/native-v1/`, and the definition/rule closure is
  computed from `requirements()`/`rules()`, not enumerated by hand
  (`producer.rs:170-187`, `src/linking/composed/definition_source.rs:151-200`).
- Declaration names, spans, requirement owners and clauses are read from the
  admitted `SyntaxNamespace` and the authored `CheckBindings` before emission
  (`producer.rs:269-306`); the emitted package is never consulted for them. Spans
  in `expected.json` (742..825, 827..932, 934..1051, 1053..1353) index the
  original 1354-byte source.
- `artifact::read` is load-bearing for everything except the output seal: it
  re-decodes, re-checks headers, sources, definitions, models and re-encodes for
  canonical byte equality (`src/protocol_artifact/intake.rs:489-511`).
- Bounds: the 16 MiB `current_exe` ceiling is the example's own
  (`producer.rs:33,119-141`), enforced by a metadata check *and* a
  `take(limit + 1)` re-check, so a growing file cannot slip past. No
  `artifact::Limits`, `ModelLimits`, `WorkLimits` or `ProofLimits` default is
  altered anywhere in the change.
- Output discipline: `fs::create_dir` refuses an existing directory, and every
  compiler and reader stage completes before any file is created
  (`producer.rs:716-745`). Confirmed by running the built binary against the
  existing fixture path (`Io { … AlreadyExists }`, exit 1) and with no arguments
  (`Arguments`, exit 1).
- Panic surface is clean: no `unwrap`, `expect`, `panic!`, `todo!`, `dbg!`,
  `TODO`/`FIXME` or new `#[allow]` in any of the five files. The only `as` casts
  are `usize → u64` widenings; `usize → u32` goes through `try_from`.
- Licensing: the new sources carry `AGPL-3.0-only`, and the README defers the
  embedded standard documents to `resources/native-v1/README.md`, matching that
  file's deferred-license statement. The compiler header relicenses nothing.

## Findings

| ID      | Severity | Summary                                                                     | Refs                                        |
| ------- | -------- | --------------------------------------------------------------------------- | ------------------------------------------- |
| FND-001 | medium   | `main` returns `Result`, so every authored `#[error]` message is dead; the operator sees a `Debug` dump | examples/native_protocol_handoff.rs:7        |
| FND-002 | medium   | Model reference claims the requirement-revision namespace with a literal `"1"` instead of the admitted owner revision | examples/protocol-handoff/producer.rs:447    |
| FND-003 | medium   | 362-line `write` holds four order-dependent invariants and an index↔filename coupling in one scope | examples/protocol-handoff/producer.rs:319    |
| FND-004 | medium   | `Error::Stage` Debug-stringifies a whole namespace/proof report into an unbounded `String` | examples/protocol-handoff/producer.rs:384    |
| FND-005 | low      | `Error::Inventory` conflates eight distinct causes into one opaque message   | examples/protocol-handoff/producer.rs:277    |
| FND-006 | low      | The reader's seal axis cannot fail here; README implies all selections are input-derived | examples/protocol-handoff/producer.rs:597    |
| FND-007 | low      | Output publication is not atomic: a mid-write IO error leaves a partial directory that then blocks retry | examples/protocol-handoff/producer.rs:727    |

## Finding detail

**FND-001.** `fn main() -> Result<(), producer::Error>` uses the `Termination`
impl for `Result<T, E: Debug>`, which prints `Error: {:?}`. Verified by running
the built binary: no arguments prints `Error: Arguments`, never the authored
`usage: native_protocol_handoff <new-output-directory>`; an existing directory
prints a raw `Error: Io { path: …, source: Os { code: 17, … } }`. The
consequence that matters is `Error::BinaryLimit`, whose entire purpose is the
remediation hint "build this example in release mode with debug information
stripped" (`producer.rs:52`) — an operator who hits it on a debug build sees
`Error: BinaryLimit { maximum: 16777216 }` and no hint. The sibling example
`examples/config_version_fixtures.rs:7-13` already does this correctly with
`eprintln!` plus `ExitCode`. Fix: print `{error}` (walking `source()`) and return
`ExitCode`.

**FND-002.** `producer.rs:447` builds the model reference with
`revision(REQUIREMENT_NAMESPACE, "1")`. That namespace,
`quire-contract-ir/requirement-revision`, asserts a *semantic* owner revision,
not an opaque source-artifact label, and the actual value is already in hand —
`producer.rs:203-206` writes `model.environment().owner().revision().get()` into
the source header. Scenario: change `"revision": 1` to `2` in `model.json`. The
source header and the model digest track the change, the package still emits, and
the fixture ships a model package labelled requirement-revision 1 for a
revision-2 model. `src/protocol_artifact/models.rs:62-107` does not bind that
label, so nothing refuses it. The two `revision("native-source-revision", "1")`
literals (`producer.rs:415,456`) are *not* the same defect — FR-042 makes
source-artifact labels deliberately separate from derived semantic revisions —
but they would read better derived from `sources[0].identity().revision`.

**FND-003.** `write` runs lines 319-680. Inside it: `baseline` must be captured
before rule dependencies are appended (`producer.rs:481-500`); `requires` must be
wired before `sort_by` (`producer.rs:519-536`); `dependencies/{index}.bin` names
are produced by two independent `enumerate()` walks, one in `Selection`
(`producer.rs:654-659`) and one in `write_files` (`producer.rs:728-733`), over
what is today the same slice. Scenario: a later edit that filters or re-sorts the
dependency list in only one of those walks mislabels every dependency file while
all digests still verify, because each file is self-consistent. Extracting
selection assembly, dependency assembly and output assembly — and carrying the
filename inside `Dependency` instead of deriving it twice — removes the whole
class. This is the "unnecessary monolithic helper" smell, not a line-count
complaint.

**FND-004.** `producer.rs:382-409` embeds `format!("{namespace:?}")` and
`format!("{proofs:?}")` in `Error::Stage.details`. rust-review §0b asks that
errors carry context rather than being stringified; both reports expose structured
accessors (`declarations()`, `disposition()`) that the code already uses for the
predicate immediately above. For a large source the `Debug` render of a
`ProofReport` is megabytes of `String` built on the failure path, and combined
with FND-001 it reaches the terminal as `Debug`-of-`Debug`.

**FND-005.** `Error::Inventory` is returned for: a namespace lookup miss, an
ambiguous lookup, a missing syntax entry, `u32` span overflow, an unexpected
execution point, a missing baseline dependency, a missing dependency reference and
an ambiguous dependency reference (`producer.rs:277,279,283,284,301,484,532,691`).
All print "an authored declaration or supplied dependency is missing or
ambiguous". A span-overflow diagnosis and a missing-rule diagnosis are then
indistinguishable without a debugger. Splitting into two or three variants that
name the subject costs nothing here.

**FND-006.** `Expected.artifact` is the `output` reference built at
`producer.rs:597-605` from `emitted.bytes()`, so `intake.rs:493-496` compares a
digest with itself, and `read.digest() != emitted.digest()` at `producer.rs:644`
cannot fail. This is correct for a producer — the code comment at
`producer.rs:596` says so, and FR-042-AC-7's tamper controls belong to
TC-121's tests, not here. The README is the looser statement: "checks the
unchanged bytes … against selections derived from the original inputs"
(`README.md:5-7`) is true of the contract, baseline, producer, language, sources,
declarations, dependencies and model, but not of the output seal. One clause
naming that exception would keep the README exactly as honest as the code
comment.

**FND-007.** `write_files` creates the directory and then writes seventy-two files with no
rollback (`producer.rs:716-745`). A disk-full or permission error on file 40
leaves a partial package, and because `create_dir` correctly refuses an existing
directory, the same command cannot be retried against that path. Writing into a
sibling temporary directory and renaming at the end preserves the refusal
semantics and gains atomicity.

## Style notes (no failure scenario)

- `fn reference` (`producer.rs:96-118`) takes seven positional parameters, five of
  them `&str`. Every call site is a literal in this file, so nothing is wrong
  today, but `wire` and `version` are adjacent, same-typed and silently swappable.
- `pub fn write` (`producer.rs:319`) carries no `///`, in a crate whose `src/`
  documents every public item. The module `//!` header covers the intent.
- The ELF check (`producer.rs:137`) makes the example Linux-only by construction;
  a Mach-O or PE host refuses with `BinaryFormat`. The README describes the ELF
  identification but never states the platform constraint outright.
- `#[path]`-included example modules match existing repo practice
  (`examples/config_version_fixtures.rs`, `examples/author_native_package_vectors.rs`),
  so that is not a seam finding.

## Gates

| Gate | Result |
| --- | --- |
| `cargo check --locked --no-default-features --example native_protocol_handoff` | pass (`/tmp/quire-native-handoff-check.log`) |
| `cargo fmt --check` | pass, no output (`/tmp/quire-native-handoff-fmt.log`) |
| `cargo clippy` all-targets, minimal features, `-D warnings` | pass (`/tmp/quire-native-handoff-clippy-minimal.log`) |
| `cargo clippy` all-targets, all features, `-D warnings` | pass (`/tmp/quire-native-handoff-clippy-all.log`) |
| `cargo run --release --example native_protocol_handoff -- <new dir>` | pass, 65,819-byte package + 67 dependencies (`/tmp/quire-native-handoff-run.log`) |
| Refusal runs: existing directory, no arguments | typed `Io`/`AlreadyExists` and `Arguments`, exit 1 (re-executed in this review) |
| `quire validate` / spec grammar | 398/398 docs grammar-clean (`/tmp/quire-native-handoff-spec.log`) |
| `cargo deny` | not applicable, no `deny.toml` in this repository |
| Parent library/spec suites | 533/549 pass, 0 fail, 4 inherited ignored (`/tmp/quire-native-populations-test-{minimal,all}.log`); unchanged here, not rerun |
