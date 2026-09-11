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

**Updated 2026-09-11 (recheck):** correction `b34ab8c` rechecked against the recorded
findings at `549dd81`, integrated at `485573b`. Both `medium` findings are resolved;
see the disposition table below.

## Verdict

**CONDITIONAL** — recheck of `b34ab8c` against `549dd81`, integrated at `485573b`.
Both `medium` findings are resolved and verified; two `low` residuals remain
(FND-003 and FND-005). FND-006 was resolved by local author verification after
Claude's recheck, as distinguished below. No `high` finding at any point.

## Recheck disposition (b34ab8c vs 549dd81)

Finding recheck only, not a second campaign. PR59's shared recovery provenance was
separately reviewed and is not re-examined here.

| Finding | Disposition | Evidence |
| ------- | ----------- | -------- |
| FND-001 | **Resolved** | Named ignored release test now runs the real `producer::write`; executed 1 passed / 0 failed / 0 ignored |
| FND-002 | **Resolved** | Typed `DeclarationCause` + four typed operation/anchor variants replace the `&'static str` discriminant |
| FND-003 | **Open (low)** | Unchanged by design; no new public model interface was added |
| FND-004 | **Resolved** | Source comments now distinguish domain/activation-implied correspondence conjuncts from the real recovery conditions |
| FND-005 | **Open (low)** | Branches now carry accurate typed context but stay unreachable for the current recipe |
| FND-006 | **Resolved — local author verification** | After Claude's recheck, the bare trace attribute binds the example test; all five ids are backed and absent from `unmatched_tags` |

**FND-001 — resolved.** `examples/native_protocol_handoff.rs:29-135` adds
`stripped_release_producer_keeps_original_owners_and_compensations`: it calls the real
`super::producer::write` into a fresh `tempfile::tempdir()` child, so the producer
selects this actual test executable's ELF bytes through `current_exe()` and runs its
own `artifact::read` before any assertion. It then decodes the output as the existing
typed `w::Package` and asserts four original source identities/paths/formal documents
and revisions, six declaration→source/requirement/clause owners, exactly two
compensations named `Full`/`Partial`, a shared `forward_effect` resolving to control
`Applied`, `Full`'s commit resolving to control `Committed` with
`ControlOperation::Commit`, and `Partial.commit.0.is_none()`. Seam discipline holds
(real producer, real reader, no doubles, no `#[cfg(test)]` branch in production code),
the oracle literals are authored in the test rather than read back from `UNITS`, and
every destructuring guard has a failing `else`. It adds no second parser, runner or B
consumer substitute: `serde_json::from_slice::<w::Package>` is post-hoc inspection of
output the canonical reader already accepted inside `write`. It traces to
`TC-121, FR-042-AC-1/4/6/7` and correctly never claims AC-10.

**FND-002 — resolved.** `DeclarationCause` is a five-variant typed enum
(`Missing`, `Ambiguous { matches }` retaining the candidate count, `MissingSyntax`,
`MissingUnit`, `DifferentSource`) attached with `#[source]`. `Error::Operation` is
split into `OperationCount { count }` and `OperationIdentity { context, name }`, and
the pre/post fallbacks into `PreAnchorMismatch`/`PostAnchorMismatch` carrying
`expected` and `actual` anchors plus the clause name — so no refusal now reports a
cause that did not occur. The `ExecutionPoint` match remains exhaustive with no
catch-all arm.

**FND-004 — resolved.** `state.body.native:2` now reads "A bounded sum and size over
the declared receipt sequence"; `state.body.native:11` and `workflow.body.native:2-3`
name the domain/activation-implied conjuncts as correspondence exercises and point at
the receipt/target comparisons as the real recovery conditions.

## Evidence verified

Re-verified against the final integrated fixture
`/tmp/quire-native-ecosystem-fixture-integrated-20260911`; the corrected and earlier
reviewed fixtures are retained historical output and were not re-read. All 67
dependency digests re-checked with `sha256sum --check`; `dependencies/0.bin`
(`c58585d3…`) is byte-identical to the live main executable
`/tmp/formalization-a-language-target/release/examples/native_protocol_handoff`; the
output seal equals SHA-256 of `compiled-protocol.json` (`424e4942…`). Four isolated
source identities/formal documents, six declarations with `pre`/`post` on
`before_apply`/`after_apply`, `Full` commit → control `Committed` and `Partial`
`commit: null` all reproduce. Everything below was established on the pre-correction
tree and still holds.

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
| FND-001 | medium   | RESOLVED in b34ab8c — named ignored release test now executes the real producer recipe          | examples/native_protocol_handoff.rs:29                | correct-requirement-no-evidence |
| FND-002 | medium   | RESOLVED in b34ab8c — typed `DeclarationCause` and typed operation/anchor variants replace message discrimination | examples/protocol-handoff/producer.rs:165, 175, 519 | correct-requirement-no-evidence |
| FND-003 | low      | OPEN — wire export-ordering rule restated as arithmetic in the example; no single source of truth | examples/protocol-handoff/producer.rs:580-599         | correct-requirement-no-evidence |
| FND-004 | low      | RESOLVED in b34ab8c — source comments now separate correspondence conjuncts from recovery conditions | examples/protocol-handoff/workflow.body.native:2; state.body.native:2, 11 | missing-requirement |
| FND-005 | low      | OPEN — anchor-mismatch and `Initialization` arms carry accurate typed context but stay unreachable for the current recipe | examples/protocol-handoff/producer.rs:505, 522        | correct-requirement-no-evidence |
| FND-006 | low      | RESOLVED — local author verification confirms the bare trace attribute binds the example test | examples/native_protocol_handoff.rs:33                | correct-requirement-no-evidence |

### FND-001 — nothing runs the example (as recorded at 549dd81; resolved in b34ab8c)

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

### FND-002 — refusals discriminated by message (as recorded at 549dd81; resolved in b34ab8c)

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

### FND-004 — conjuncts that cannot be false (as recorded at 549dd81; resolved in b34ab8c)

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

### FND-006 — trace ids did not bind (low; resolved by local author verification)

Claude's recheck found all five ids (`TC-121`, `FR-042-AC-1/4/6/7`) in
`unmatched_tags`: the fully qualified attribute did not match the configured
`rust-trace-attribute` marker. The criteria already had backing from other tests.

Local author verification after that recheck confirms the correction to
`use ix_trace_rs::trace;` and bare `#[trace(...)]`. Completed `quire coverage`
output identifies the example's actual
`tests::stripped_release_producer_keeps_original_owners_and_compensations` symbol
under `TC-121`; all five ids are backed and none is unmatched. Only the three
inherited `IT-004` unmatched tags remain. The unchanged 367/376 rollup reflects
additional evidence for already-backed criteria, not a new acceptance claim.
The root's named stripped release rerun also passed: 1 passed, 0 failed, 0 ignored.
This disposition is local author verification, not an additional Claude recheck.

### FND-005 — unreachable arms, now accurately typed

`AuthoredExecution` has only `Handler`/`Pre`/`Post`, and `UnitInput::new` always
builds `Pre`/`Post` from the single selected anchor. The
`ExecutionPoint::Initialization` arm and the anchor-mismatch fallback are therefore
unreachable for every input the recipe can construct. `b34ab8c` improves them — the
mismatch arms now report `expected`/`actual` anchors instead of a misattributed cause —
but they remain untested branches. Retained as a recorded low residual; no new public
model interface was added for them, and none is being asked for.

## Merge disposition

**May merge.** No `high` finding was ever open and both `medium` findings are resolved
and independently verified. The two residuals are `low` and deliberately
retained; FND-006 has the separate local verification above. FR-042-AC-10 / B's IT-001
and the inherited matrix debt stay open and are disclosed in FR-042, TC-121 and both
READMEs; per the owner directive of 2026-09-09
incomplete assurance is tracked separately from implementation delivery, so this
review does not convert that deferred acceptance into a prototype engineering gate.

## Gates

Executed by the root at the integrated head `485573b`, inspected here, not re-run
(no green heavy-check repeats):
`/tmp/quire-native-ecosystem-integrated-{fmt,clippy-minimal,clippy-all,release-test,run,spec}.log`.
`fmt` clean; both Clippy lanes finish clean; 398/398 specs grammar-clean. The named
release test log records `Running unittests examples/native_protocol_handoff.rs
(/tmp/formalization-a-language-target/release/examples/native_protocol_handoff-6f7d642293608482)`
and `1 passed; 0 failed; 0 ignored`, confirming it ran against actual ELF test-executable
bytes. The release main example run emits the integrated fixture. As before, the logs
record outcomes but not the invoked flag sets, so `--all-targets -D warnings` with
respective `--no-default-features` / `--all-features` rests on the root's statement,
not on the log text.

**Default versus named execution.** `cargo test` does not reach this test by two
independent mechanisms: the example has no `[[example]] test = true` in `Cargo.toml`,
so examples are not test-built by default, and the test additionally carries
`#[ignore]`. It runs only under the command documented at
`examples/protocol-handoff/README.md:28-31`. That is the intended design, and the
example README states the `#[ignore]` reason (the 16 MiB producer binary limit) though
not the example-target mechanism. Library suites at parent PR59: 558/574, not re-run
for an example-only change. `cargo deny` — no `deny.toml` in this repository. Re-run
here: `quire coverage --scope <worktree> --json` (see SR-352) and `quire validate`.
