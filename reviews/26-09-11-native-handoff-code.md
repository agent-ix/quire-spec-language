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

Recheck of the example at correction source `56c1621`, which merges PR-54's
separately reviewed `84aec59` into the handoff branch; the example correction
itself is `fa73912..915f479`. Skills applied: the actual
`/home/peter/dev/agent-skills/code-review/SKILL.md`, which dispatches the Rust
lane to `/home/peter/dev/agent-skills/rust-review/SKILL.md`, with
`/home/peter/dev/agent-skills/rust-style/SKILL.md` as the portable idiom default
because this repository documents no Rust idiom skill of its own. Library
production source is unchanged from the tested PR-54 correction, so its suites
were not rerun. The five earlier `medium` items are resolved in the code, the
two deferred `low` items are now documented rather than fixed, and three new
`low` items remain.

## Verdict

**CONDITIONAL** — no `high` and no `medium` finding. The compilation chain, the
input selections and the reader expectations remain genuine at this source; what
is left is one documentation inconsistency, one dead error-message surface and
one construction-order sentinel.

## Disposition of the prior findings

| Prior | Disposition | Evidence |
| --- | --- | --- |
| FND-001 `main` returns `Result`, `#[error]` messages dead | resolved | `native_protocol_handoff.rs:7-17` returns `ExitCode` and prints `{error}`; controls re-executed here print `usage: native_protocol_handoff <new-output-directory>` and `cannot access /tmp/quire-native-producer-fixture-reviewed-20260911: File exists (os error 17)`, both exit 1, no `Debug` variant |
| FND-002 model reference asserts the semantic requirement-revision namespace with a literal `"1"` | resolved | `producer.rs:510-521` derives it from `model.environment().owner().revision().get()`; `producer.rs:478-486` likewise derives the source-artifact label from `sources[0].identity().revision`. Remaining literals (`compiled-protocol-contract`, `selected-rule-source`, `example-output`, `crate-version`) are producer-owned source-artifact labels, not owner-derived semantic revisions |
| FND-003 362-line `write` with a doubly derived index↔filename coupling | resolved | Split into `Inputs`, `DefinitionInputs`, `SelectedInputs`, `compile`, `emit_and_read`, `write_files`; `dependencies/{index}.bin` is assigned once at `producer.rs:663-666`, carried on `Dependency.file`, and consumed by both the sidecar (`producer.rs:565-573`) and publication (`producer.rs:1026-1028`). Verified on the fixture: all 67 dependency files hash to the digest recorded beside their carried filename |
| FND-004 `Error::Stage` Debug-stringified whole namespace/proof reports | resolved | `producer.rs:62-70,90-123` now carry `stage`, `completed/expected`, issue count, an `incomplete` flag and one `Copy` `StageIssue` holding first kind/supplied/span/code or declaration/disposition/site/cause. No report, source body or diagnostic list is retained |
| FND-005 `Error::Inventory` conflated eight causes | resolved | Replaced by distinct `Declaration`, `Dependency` (with match count) and `Span` variants (`producer.rs:82-88`), each naming its subject |
| FND-006 README implied all selections are input-derived | resolved | `README.md:8-9` now states the producer selects the output seal after emission and that the local reader check neither authenticates it nor exercises tamper controls, matching the code comment at `producer.rs:901-902` |
| FND-007 publication is not atomic | open, deliberately deferred and disclosed | `README.md:26-29` names the partial-directory outcome and the retry procedure; no temporary-directory-and-rename machinery was added. Carried below as FND-003 |

## Re-verified, not asserted

- `dependencies/0.bin` in `/tmp/quire-native-producer-fixture-reviewed-20260911`
  is the actual stripped ELF 64-bit PIE example binary: SHA-256
  `b0fb9184…c91d`, byte-identical to
  `/tmp/formalization-a-language-target/release/examples/native_protocol_handoff`.
  No synthetic producer or hand-authored package.
- All 67 dependency files recompute to the digests recorded in `expected.json`
  against their carried `file` names, and `compiled-protocol.ref.json` carries
  `sha256:382a642b…ddc8`, the digest of `compiled-protocol.json`.
- The baseline is the registered Edition selection (`ix:native`,
  `quire/native-definition-revision=1-draft.2`), present as `dependencies/3.bin`
  with 12 direct prerequisites derived from `requirements()`/`rules()`.
- `workflow.native` (1354 bytes) and `model-source.json` hash to the source
  digests in `expected.json`, and the four declaration spans (742..825,
  827..932, 934..1051, 1053..1353) index that original source.
- Panic surface is still clean: no `unwrap`, `expect`, `panic!`, `todo!`,
  `dbg!`, `TODO`/`FIXME` or `#[allow]` in either file. The wildcard arm in
  `proof_cause` (`producer.rs:857`) is required — `proofs::CauseKind` is
  `#[non_exhaustive]` — and maps into a closed local enum with an explicit
  `Unrecognized`, so it is not a silently absorbing catch-all.

## Findings

| ID      | Severity | Summary                                                                     | Refs                                        |
| ------- | -------- | --------------------------------------------------------------------------- | ------------------------------------------- |
| FND-001 | low      | README tells B to "independently accept the expected sidecar", which FR-042 now declines as an accepted-inventory authority | examples/protocol-handoff/README.md:50      |
| FND-002 | low      | `StageIssue`/`ProofCause` authored `#[error]` messages are dead; `Error::Stage` renders them with `{first:?}` | examples/protocol-handoff/producer.rs:62    |
| FND-003 | low      | Publication is still not atomic; a mid-write I/O error leaves a partial directory that blocks retry on that path | examples/protocol-handoff/producer.rs:1014  |
| FND-004 | low      | `Dependency.file` is constructed as an empty-string sentinel and filled in a later pass | examples/protocol-handoff/producer.rs:604   |

## Finding detail

**FND-001.** The corrected FR-042 Inputs says B's integration caller constructs
the accepted inventory as Rust values from independently selected original
inputs, and that `expected.json` "is an inspection aid for the fixture, not an
accepted-inventory interchange format or an authority B may accept by default"
(`spec/functional/FR-042-publish-compiled-protocol-artifacts.md:57-61`).
`README.md:50-52` still opens with "B must independently accept the expected
sidecar and referenced bytes before examining an offered payload". The rest of
the paragraph is consistent with the specification — it confines the sidecar to
this recipe and describes reconstructing the model through `Source::read` /
`FormalSource::new` / `model_source::read` — but the lead sentence reads as
sidecar-as-input. Scenario: a B implementer follows the README's first sentence
and parses `expected.json` as its selection source, which is exactly what the new
specification sentence exists to prevent. Naming the original sources, models,
registry/contract and producer bytes as the accepted inputs in that sentence
aligns the two.

**FND-002.** `StageIssue` and `ProofCause` derive `thiserror::Error` with
authored `#[error]` messages (`producer.rs:91-123`), but the only consumer is
`Error::Stage`'s `{first:?}` (`producer.rs:62`), so the operator sees
`Some(Proof { declaration: 3, disposition: Unfinished, … })` rather than the
authored sentence. This is the FND-001 class in miniature, one level down. It is
`low` rather than `medium` because the `Debug` render is now `Copy`, fixed-size
and genuinely informative — the unbounded-report defect is gone. Same note for
`Error::Identifier`'s `{0:?}` (`producer.rs:56`) where `ir::Diagnostic`'s own
rendering was available.

**FND-003.** Unchanged from the earlier FND-007: `write_files` creates the
directory and writes seventy-two files with no rollback
(`producer.rs:1014-1043`). The owner has deliberately deferred the
temporary-directory-and-rename machinery for this example and the README now
discloses both the partial-directory outcome and the retry procedure
(`README.md:26-29`), so this is recorded as an accepted, documented limitation
rather than an unresolved defect.

**FND-004.** Every `Dependency` is built with `file: String::new()`
(`producer.rs:604,619,627,633,639`) and the real name is assigned in a second
pass after sorting (`producer.rs:663-666`). The single-assignment fix for the old
index↔filename coupling is correct as written and verified on the fixture; the
residue is that the type permits a state — an empty filename — that no correct
output has. Scenario: a future edit that returns dependencies from an earlier
path, or inserts one after the naming loop, publishes `directory.join("")` and
writes the payload over the output directory entry instead of a dependency file.
Building the name at sort time (or an `Option`/newtype that the writer must
unwrap) states the invariant in the type.

## Style notes (no failure scenario)

- `declaration_selection` calls `namespace.lookup(&clause.name)` twice — once in
  the `let [id] = … else` pattern and once inside the `else` to choose the
  message (`producer.rs:320-323`).
- `fn reference` (`producer.rs:139-160`) still takes seven positional
  parameters, five of them `&str`; `wire` and `version` remain adjacent and
  swappable. All call sites are literals in this file.
- `pub fn write` now carries a `///` (`producer.rs:959`); `pub enum ProofCause`
  does not.

## Gates

| Gate | Result |
| --- | --- |
| `cargo fmt --check` | pass, no output (`/tmp/quire-native-handoff-corrections-fmt.log`) |
| `cargo clippy` all-targets, minimal features, `-D warnings` | pass (`/tmp/quire-native-handoff-corrections-clippy-minimal.log`) |
| `cargo clippy` all-targets, all features, `-D warnings` | pass (`/tmp/quire-native-handoff-corrections-clippy-all.log`) |
| `cargo run --release --example native_protocol_handoff -- <new dir>` | pass, package + 67 dependencies in `/tmp/quire-native-producer-fixture-reviewed-20260911` (`/tmp/quire-native-handoff-corrections-run.log`) |
| Refusal controls: no arguments, existing directory | `usage: …` and `cannot access …: File exists (os error 17)`, exit 1, re-executed in this recheck |
| Fixture re-verification | 67/67 dependency digests, package↔reference digest, binary↔`release/examples/native_protocol_handoff` identity, source/model digests and declaration spans recomputed in this recheck |
| `quire validate --scope <worktree> "spec/**/*.md"` / grammar | 398/398 docs grammar-clean, only module-registry first-wins notices (`/tmp/quire-native-handoff-corrections-spec.log`) |
| `cargo deny` | not applicable, no `deny.toml` in this repository |
| Parent library/spec suites | 533/549 pass, 0 fail, 4 inherited ignored, 52 suites including doctests (`/tmp/quire-native-populations-corrections-test-{minimal,all}.log`); production source identical to `84aec59`, not rerun for example-only fixes |
