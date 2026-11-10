# Outcome parity: the lifecycle conformance matrix

`qsl-conformance` publishes the cases that both halves of FR-564 run: this
repository through the library, the driver repository through the `quire` CLI.
The crate holds the only copy of a case (FR-565).

## Shape

- `lifecycle_matrix()` returns one `MatrixCell` per (verb, outcome kind) pair
  of the six verbs (`check`, `package`, `execute`, `analyze`, `monitor`,
  `prove`), 48 in all, in FR-564's order: a `Case` (40) where the verb
  reaches the kind, a `NotApplicable` with FR-564's reason (8) where it does
  not. No cell is pending.
- A `MatrixCase` holds its inputs as the crate's own bytes. Each `Input`
  states the file name the driver writes it under; the case states the working
  directory (`WorkingDirectory::NewEmpty`: a new empty directory the driver
  creates). `RequestParameters` hold the rest of what the operation is called
  with: the authored source authority and identity (separate from its file name), the package name, the accounting limits (`work_units` is `execute`'s
  call limit), `monitor`'s `max_window_values`, an EN-4 item's seed, witness
  bound and sampler, and an EN-1 or EN-5 item's `max_states`. What the
  parameters do not carry stays at the engine's default: an EN-4 item uses the
  Okamoto method and `StatisticalLimits::default()`, and an EN-1, EN-5 or
  clause item's limits other than `max_states` are the published defaults. The case also
  holds the cancellation plan (none, or cancelled before the call with
  `Requested` or `Deadline`, so no outcome depends on timing) and the expected
  category and exit code. The driver reproduces a case from the case alone.
- Engine items (`EngineItem`) run through `analyze` and `provider::run` over
  bytes, with the unit's source bytes provisioned under the label the package
  names: an EN-4 claim (the `Steady` quantile claim), an EN-1 item over the
  unit's one authored temporal clause (`test/counter-step`), and an EN-5 exact
  claim (the Link subject). Each is a `NegotiatedItem`. The analyze
  unsupported cell is FR-281-AC-3's item: the clause item with the
  `unsupported` disposition and its `unsupported-requested-capability`
  diagnostic, which `analyze` settles as `Unsupported`. The `prove` unsupported cell is EN-1's
  `RequiresBound` refusal over a model with an unbounded `Integer` field.
- `run_case(case)` runs the library operation with an empty registry and
  `Engines::qsl()`; `run_case_with(case, registry, engines)` takes the
  driver's registry (it links the Kani backend, QSL cannot) and engines. Both
  read no file and return a `CaseOutcome`: the typed FR-286 `OutcomeDocument`
  (for the `prove` row, the `OutcomeItem`), the terminal records an `analyze`
  or `prove` case settled, for an `execute` case the typed
  `qsl_eval::execution::execute` result (the `CallFailure` FR-564-AC-3 names;
  it is a separate call from the document's, because no public function
  derives the FR-286 document from a typed result, and `ExecuteResult` has no
  equality, because `Evaluation` has none),
  and the FR-285 exit code. A case that cannot reach its operation returns
  `CaseError { stage }`; a case's own outcome is never an error, whatever its
  category.
- Every source is read under the case's explicit `source_authority` and
  `source_identity`. A file name locates bytes; it does not supply their authored
  label. The driver must pass the same labels in its request. Published parse
  and format syntax is `quire parse <authority> <identity> <file>` and
  `quire format <authority> <identity> <file>`; compile and run take request
  files carrying their labels. Machine output remains one `quire-outcome/1`
  document. This crate publishes library lifecycle cases; it does not wire
  additional driver verbs.

## Known gaps

- FR-566-AC-1 remains untagged: document and live-plugin controls are
  present, but the package-byte equality control is not asserted.

- FR-564-AC-4 and AC-6 are not tagged: the `refuted` cells do not yet produce
  an `ItemCounterexample` of the case's outcome; `AnalyzeOutcome::counterexamples`
  is not read yet.
- FR-567 qualification integration is outside this library slice. The
  corpus/matrix/parity combined gate, its linked-backend controls and its
  scratch-copy version test remain open; this crate does not establish full
  QSL-12 qualification.

## What the driver companion exposes

The driver companion must compare typed documents: it encodes the
`OutcomeDocument` that `run_case_with` returns and the document its CLI wrote
with its own canonical JSON encoder, each followed by one newline, and
compares those encodings and the exit codes. For each `Case` it writes the
inputs at their file names in a new empty directory, runs the verb there with
each file argument spelled as its file name, and compares. To make that
possible the CLI side needs:

- the verbs `check`, `package`, `analyze` and `monitor` taking the case's
  request parameters and limits, and `prove` with the Kani provider and a
  process plugin for the `prove` row;
- a test-only adapter, behind a Cargo feature the shipped binary does not
  enable, that cancels the operation's `Cancel` handle with `Requested` or
  `Deadline` before the first stage runs (the timeout and cancellation cells);
- for the `execute` row's tool-failure cell, the same adapter linking this
  crate's fault-injecting backend as a supplied backend.

## Documents this crate's operations write

`OutcomeDocument::from_check` and `from_package` write the front-end
documents; `from_run` writes an `execute` call. `monitor` and `analyze` write
theirs through `MonitorOutcome::document`, `MonitorRefusal::document`,
`AnalyzeOutcome::document` and `AnalyzeRefusal::document`, and
`OutcomeDocument::from_stopped` writes a cancelled, limited or faulted stage.
`analyze` takes EN-4, EN-1 and EN-5 work. A record a request's disposition
settles carries its FR-272 diagnostic as the item's `diagnostic` member,
`{code, cause, detail}` with `detail` keyed by FR-272 payload names, and for
`unsupported` no `cause` (FR-286). A `declined` invalid-request item carries
both `cause` and `diagnostic` (QSpec FR-331).

A whole-request refusal, a cancel before the first stage and a fault write
`last_stage` `null`; the front-end operations write the stage they reached.
The driver builds its documents through the same functions.

The analyze timeout and cancellation cells settle on the EN-4 backend id
`en-4`.
