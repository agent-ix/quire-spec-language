---
id: SR-351
title: "Code review — native multi-unit producer handoff example"
type: SpecReview
analysis: code-review
scope: "examples/protocol-handoff/, examples/native_protocol_handoff.rs, README.md, spec/functional/FR-042-publish-compiled-protocol-artifacts.md, spec/test-cases/TC-121-publish-compiled-protocol-artifacts.md"
review_set: subset
---

## Summary

Reviewed `agent-a/native-ecosystem-handoff` against `origin/main` (`9c145bd`, PR57
integrated) — the producer recipe's growth from one source unit to four
(`predicates`/`state`/`temporal`/`workflow`), the `/2` model gaining
`Amount`/`Total`/`Tally` scalars, a `Notice` record, three values and the
`Workflow::apply` operation, plus scoped FR-042/TC-121 and README edits. Rust lane
per `/home/peter/dev/agent-skills/skills/rust-review/SKILL.md` with portable
`rust-style` defaults (the repo documents no Rust idiom skill of its own). The
composition is sound and the claimed behaviors are genuinely present in the emitted
bytes; the findings are one coverage gap and a cluster of expressiveness/idiom
issues. No `AssuranceProfile` document exists anywhere in `spec/` — none applied,
consistent with the previous producer recipe review.

## Verdict

**CONDITIONAL** — no `high` finding; one `medium` coverage gap (nothing executes the
example) and one `medium` error-discrimination smell, plus three `low` items.

## Evidence verified

Read only `/tmp/quire-native-ecosystem-fixture-reviewed-20260911`, not the earlier
initial-run directory.

- **Identity isolation is real.** Four distinct source identities, `native` refs,
  formal documents (`ProtocolHandoffPredicates`/`State`/`Temporal`/`Workflow`) and
  requirement ids (`HandoffPredicates`/`HandoffState`/`HandoffTemporal`/`HandoffWorkflow`),
  with four distinct source digests. `declaration_selection` now re-checks that each
  authored clause resolved in its own unit by identity, path and text
  (`examples/protocol-handoff/producer.rs:424-440`).
- **Source/span correspondence holds.** Every emitted declaration span slices exactly
  its authored declaration out of the retained source text; `intake.rs:319-352`
  locates declarations by `(source, span)`, compares name/requirement/clause/execution
  and requires every emitted declaration to be claimed.
- **Cross-unit calls are in the bytes.** `Healthy`/`BeforeApply`/`AfterApply`/`Due`
  each `requires` `Allowed` (source 0); `Flow` requires all four, plus
  `temporal_requirements = [Due]`.
- **Pre/post operation contracts are real.** `BeforeApply`/`AfterApply` emit
  `pre`/`post` execution against `ExportRef{model:0, export:9}`, and the model export
  table confirms index 9 is `operation ["Workflow","apply"]`. The producer derives
  that handle by counting admitted inputs while the emitter resolves it from the
  anchor — a genuine two-sided check, not a copy of the emitted table.
- **Model `/2` ownership.** All 18 exports (scalar/record/reference/object/population/
  field/operation) carry a locus into `model-source.json`, which is byte-identical to
  `examples/protocol-handoff/model.json`.
- **Query/population/capture/Full/Partial are exercised, not asserted.** `Healthy`
  emits a `sum` query and a `size`; `Flow` emits 9 `population` bindings, a `let`, two
  `query` values, a `deref` unary, 5 capture initializers, and two `compensations`
  records with distinct clocks (`full-recovery`/`partial-recovery`), registration and
  activation anchors, retry/recovery handles. `Full` carries `commit` → a handle;
  `Partial` carries `commit: null` — the `Nullable` wire type
  (`src/protocol_artifact/wire.rs:523`) distinguishes `commit never` from absence, so
  the README's Full/Partial claim is backed by the bytes.
- **Bytes are real.** All 67 dependency digests recomputed and matched;
  `dependencies/0.bin` is byte-identical to the live executable
  `/tmp/formalization-a-language-target/release/examples/native_protocol_handoff`;
  contract bytes equal `docs/compiled-protocol-v1.md`; the output seal equals
  SHA-256 of `compiled-protocol.json`.
- **No second parser, no shell path.** The reader path is `artifact::read` only;
  `expected.json` is written and never read back; the sole stdout/stderr use is the
  error print in `examples/native_protocol_handoff.rs`.
- **Retained limitation, unchanged.** The output seal is computed from the emitted
  bytes and then handed to `read` as `Expected.artifact`, so the round trip proves
  encoding stability, not source equivalence. Disclosed in the example README and in
  FR-042-AC-7; the previous review's distinction survives.

## Findings

| ID      | Severity | Summary                                                                                      | Refs                                                  | Escape Cause                   |
| ------- | -------- | -------------------------------------------------------------------------------------------- | ----------------------------------------------------- | ------------------------------ |
| FND-001 | medium   | No automated gate executes the producer recipe; a library regression breaks it silently        | examples/native_protocol_handoff.rs:1                 | correct-requirement-no-evidence |
| FND-002 | medium   | `Error::Declaration`/`Error::Operation` discriminate refusals by message, and `Operation`'s message names only one of its two causes | examples/protocol-handoff/producer.rs:165, 171, 479   | correct-requirement-no-evidence |
| FND-003 | low      | Wire export-ordering rule restated as arithmetic in the example; no single source of truth      | examples/protocol-handoff/producer.rs:526-545         | correct-requirement-no-evidence |
| FND-004 | low      | New authored conjuncts are implied by their own guards or scalar domains, so they constrain nothing | examples/protocol-handoff/workflow.body.native:14, 34; state.body.native:11 | missing-requirement |
| FND-005 | low      | Unreachable `Initialization` and anchor-mismatch arms added with no caller and no test          | examples/protocol-handoff/producer.rs:462, 479        | correct-requirement-no-evidence |

### FND-001 — nothing runs the example

`grep` for `native_protocol_handoff` / `protocol-handoff` across `tests/` and `src/`
returns nothing. `cargo test` never invokes the recipe; `cargo clippy --all-targets`
only compiles it. Scenario: a change to `linking::admit_namespace` unit ordering, to
model export ordering, or to `proofs::discharge` dispositions leaves every suite and
both strict Clippy lanes green while the delivered source-to-artifact path stops
producing a package — discovered only when someone re-runs the example by hand. The
gap is inherited, but this increment widens what rests on it: TC-121 step 1 now
asserts the recipe "exercises four original predicate/state/temporal/workflow units
with cross-unit references, ordered queries, population/reference roles, actual model
operations and distinct full/partial recovery requirements", and no gate holds that
sentence true. An `#[ignore]`d integration test in a named lane that runs `write()`
into a temp dir and asserts the four sources / six declarations / two compensations
would close it without adding cost to the default run.

### FND-002 — refusals discriminated by message

`Error::Declaration { name, problem: &'static str }` now stands for four distinct
refusals ("not present in the original namespace", "ambiguous in the original
namespace", "original syntax is unavailable", "original unit is unavailable"), two of
them added here, plus a fifth "clause was selected in a different source unit". The
last is the most important refusal in the change — it is what proves per-unit
isolation — and a caller can only distinguish it by string compare. Separately,
`Error::Operation` is returned both when the model does not expose exactly one
`Workflow::apply` (`producer.rs:520-525`) and when an authored pre/post clause names a
different anchor (`producer.rs:479-481`); its message only describes the first, so the
second refusal reports a cause that did not occur. Per `rust-review` §0b, each
condition a caller must distinguish is its own variant with typed fields.

### FND-003 — export ordering in two places

`OperationSelection::new` reconstructs the operation handle as
`objects.len() + Σ(record fields | enum 1)`. That is correct today only because the
emitter sorts exports by kind label and `enum`/`field`/`object` precede `operation`
while `population`/`record`/`reference`/`scalar` follow it — a fact that lives in the
library encoder and is restated here as a comment plus arithmetic, with the compiler
checking neither. It fails closed (the reader refuses a wrong handle) rather than
silently, which is why this is `low` and not higher; a `pub` helper on the model or
emitter that names an export's handle would remove the second edit site.

### FND-004 — conjuncts that cannot be false

`amount >= 1` (state.body.native:11) restates `Amount`'s declared minimum of 1.
`targetFull >= 0` / `targetPartial >= 0` restate `Total`'s minimum of 0.
`not activatedFull` / `not activatedPartial` are implied by their own activation
guard: the guard selects a trigger with `not trigger.ready` and the capture is
`activated = trigger.ready`, so the capture is always false wherever those retry and
recover predicates are evaluated. As a structural fixture this is legitimate — it
proves the capture binder is in scope and typed from registration through recovery —
but nothing in the source says so, and `state.body.native:2` ("Receipt order and
duplicates survive the bounded aggregate") claims an ordering property that neither
`size` nor `sum` can distinguish. One line of comment per conjunct stating "structural
coverage, not a constraint" would keep a later reader from treating these as
behavioral evidence. The same shape in the inherited `self.peer = self.peer`
(state.body.native:7) and in `Allowed`'s domain-implied `ratio <= rational(1,1)`
predates this change and is out of scope here.

### FND-005 — dead arms

`AuthoredExecution` has only `Handler`/`Pre`/`Post`, and `UnitInput::new` always
builds `Pre`/`Post` from the single selected anchor. The
`ExecutionPoint::Initialization` arm and the anchor-mismatch fallback are therefore
unreachable for every input the recipe can construct. They are cheap insurance rather
than a defect, but they are untested branches in a file whose whole job is to be
audited.

## Gates

Inspected, not re-run (root-completed at `336e3ec`; green heavy checks not repeated):
`/tmp/quire-native-ecosystem-final-{fmt,clippy-minimal,clippy-all,run,spec}.log`.
Both Clippy lanes and `fmt` finish clean and the release `--no-default-features`
example run succeeds into the reviewed fixture; the logs record the outcomes but not
the invoked flag sets, so the "both strict all-target configurations" characterisation
rests on the root's statement, not on the log text. Library suites at the unchanged
parent `b7aafe1`: `/tmp/quire-native-compensation-corrections-test-{minimal,all}.log`
(557/573, 0 fail, 4 inherited ignored). `cargo deny` — no `deny.toml` in this
repository. Re-run here: `quire coverage --scope <worktree> --json` (see SR-352).
