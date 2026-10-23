---
id: FR-031
title: "Run a selected Markdown clause through the spine clause runner"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-004
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-030
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-109
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-100
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-312
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-267
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-004
    type: references
  - target: ix://agent-ix/quire-specification/FR-352
    type: references
---
## Description

When a caller selects one authored clause of an original Markdown document,
QSL shall run the clause's extracted body, the `ExtractedSource` FR-030
yields, as the unit of a spine clause run (FR-109): spine compile (S1 to S4)
under the body's own identity, then the clause run. The report shall name
the original document and the body, and shall carry the extraction record
below. QSL has two entries to this run, and both are present in every build
in which `qsl-replay` is built with its feature `quire-extraction` (for the
`quire-spec` command, `--features qsl-replay/quire-extraction`; ADR-011
§7.3 X-4):

- in process, `qsl_replay::spine::run_clause` with an I3 extracted source;
- the `run` command's extraction mode: a `1-draft` native-run/1 clause-run
  request (FR-312) whose `program` carries an `extraction` record.
  `qsl_replay::spine::read_clause_run_request` reads it, extracts the body
  through FR-030 and calls `run_clause`; the command reaches the I3 adapter
  only through that reader.

## Inputs

In process: the original Markdown `Source`, with its four FR-001 labels and
the raw bytes the caller digest-verified; FR-030's `Selection` (the authored
clause id, the authored package and the body's own `SourceIdentity`);
FR-030's `Limits` for the original document; and the other
`ClauseRunRequest` members (FR-109): FR-056's package input, FR-099's
dependency input, the snapshot and invocation provisions, the selection, an
optional expected `package_id`, and the spine, observation and evaluation
limits.

Through the command, the request is FR-312's clause-run request with these
members beside `clause`:

- `program`: the ordinary source selection, which selects the
  digest-verified original Markdown document. Its identity satisfies Quire's
  `SourceLocus` schema.
- `program.extraction`: a closed record holding `body`:
  `{authority, identity, document, digest}`. `authority` and `identity` with
  `digest` are the body's native identity; `document` with `digest` is its
  formal identity. Each is a document identity plus a content digest, the
  key QSpec FR-202 gives an authored source (STD-150). `digest` is the
  raw-artifact digest of the extracted body's complete bytes (QSpec FR-018).
- `program.clauses`: the authored clause binding, a closed record
  `{clause, package}` naming the authored clause id (the Quire heading id)
  and the authored package. Exactly one binding forms FR-030's `Selection`
  with the body's identity.

The command constructs the clause-only Quire context through `qsl-source`'s
`clause_context` for the binding's package and the original path and
identity, with empty exports and imports and a Markdown target. It does not
load installed archetype schemas or infer model meaning from Markdown. Native
model imports retain their existing authority.

## Outputs

- A `ClauseRunReport` (FR-109) whose source names the body identity, with the
  raw-artifact digest of the extracted body's complete bytes, and the
  original document identity with its digest, and whose `package_id` is the
  one the body compiles to under its body identity (FR-109-AC-6). The report
  carries the extraction record: the original document identity, path and
  digest; the body's native and formal identities with their digest; the
  authored binding; Quire's unchanged `ClausesOutcome` (availability,
  clauses, raw `clause_text` and diagnostics); the verified original-to-body
  map's region and segments; and, for a refusal whose locus names a body
  span, the original-document regions that map gives for that span.
- Through the command, one FR-267 `native-run-result/2` clause-run document
  whose top-level `extraction` member is that record, with the member set
  and types QSpec FR-352 lists (`original`, `body`, `binding`, `outcome`,
  `map` and, for a refusal with a body span, `original_regions`).
- Or FR-030's extraction `Error`, which retains the original source, the
  binding, any completed Quire extraction outcome and the actual preflight,
  availability or correspondence cause. No compile or clause run follows,
  and there is no truth. Through the command it is FR-267's command-error
  envelope with stage `extraction-selection` for a selection, preflight or
  correspondence cause and stage `quire` for an unavailable or failed Quire
  extraction, with the retained values in `details`.
- A clause-only Quire context that Quire's public semantic-block validator
  rejects refuses `invalid-quire-context` at stage `quire-context`, exit 20,
  with Quire's unchanged producer diagnostics in `details`.
- An unsupported combination refuses at stage `request`, exit 20, empty
  stdout, with its own catalogued code: `extraction-requires-run`,
  `extraction-package-conflict`, or `extraction-clause-count` with
  `details` `{"actual": n}`.

## Behavior

The caller shall extract the body through FR-030 before any compile. An
extraction failure shall end the request with FR-030's error and no partial
package.

The clause runner shall compile the body under its body identity. The clause
runner shall report compile, select, admit and evaluate outcomes with FR-109's stages,
categories, truth and exit codes, exactly as for a source given as bytes.
The clause runner shall retain the extraction record beside those outcomes.
Only a completed evaluation
has truth.

The original document and the body shall keep distinct identities. The
original selects the Markdown bytes; the body's identity owns the compiled
declarations and so the `package_id` (FR-001, FR-032-AC-4). The command shall
recompute the body digest over the extracted bytes. If `body.digest`
differs, then the command shall refuse `stale_dependency`/`content-mismatch`,
naming both digests.

The request reader shall decode `program.extraction` and `program.clauses`
under FR-100's closed-envelope rules: a `null` or positional record, a
duplicate member and an unknown member refuse `invalid-request` at stage
`request`, naming the member. The request reader shall check the request's
mode before it reads any dependent file:

- If a request carrying `extraction` is a compile request (FR-027) or a run
  request carrying `call` in place of `clause`, then the command shall refuse
  `extraction-requires-run`.
- If a request carrying `extraction` selects a package artifact through its
  `package` member, then the command shall refuse the request with
  `extraction-package-conflict`.
- If a request carrying `extraction` carries a number of `clauses` bindings
  other than one, then the command shall refuse `extraction-clause-count`,
  with `details` `{"actual": n}` for the number it carries.
- If a request carries `extraction` to a command whose `qsl-replay` is built
  without `quire-extraction`, then the reader shall refuse `invalid-request`
  at stage `request`, naming `extraction`, as an unknown member.

The command shall validate the clause-only context through Quire's public
semantic-block validator before extraction.

A body whose fence declares a language other than QSL native shall refuse at
compile with `unknown_language`, exit 20, with no `package_id`
(FR-109-AC-6). A body that does not parse refuses at compile with FR-109's
`source` refusal. A body that declares more than one clause shall refuse at
compile with `extraction-clause-count`, `details` `{"actual": n}` for the
number of clauses the body declares, with no `package_id`. A selection that
names a clause the body does not declare refuses at `select` with FR-109's
`missing_declaration`. An expected `package_id` that the body does not
compile to shall refuse at compile with `stale_dependency`, naming both
identities (FR-109-AC-2).

### Original-region correspondence

When a refusal at compile, select, admit or evaluate has a locus that names a
span of the body, the clause runner shall keep that body span unchanged and
shall carry in the extraction record the original-document regions the verified `SourceMap` (FR-004, FR-030) gives for
it. The regions shall be exact under multi-byte Unicode and under the layout
FR-030's map deletes (fence indentation and a final LF or CRLF), and a span
the map splits shall keep every region rather than collapse them into one.
A refusal whose locus names no body span, such as a package-encoding path,
shall carry no original region, and none is invented. A mapping request that
the map does not cover refuses with FR-004's `invalid_source_map` and the
refusal keeps its body span. A malformed original-to-body mapping shall
refuse with FR-030's correspondence error before any compile.

The original byte and line limits (FR-030) and the spine and evaluation
limits (FR-109) shall stay independent. When a request exhausts any of them,
including the link, check and package limits of spine compile, the clause
runner shall report incomplete with the limit's typed cause; a fresh request
with the same inputs and sufficient limits then succeeds, with counters that
start at zero.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-031-AC-1 | An LF and a CRLF Markdown document whose one selected fence holds FR-108's healthy-parent unit run through `run_clause` report `success`, exit 0, with the `package_id` of the body compiled alone; violating-parent reports `violation`, exit 10; missing-model reports `compile`, `missing_import`/`missing-selection`, exit 20, with no `package_id`. | Test (TC-109) |
| FR-031-AC-2 | A selected clause the document does not hold, an unavailable extraction and a stale original digest each end with FR-030's error, retaining the original identity, the selection and Quire's actual outcome, with no compile, no `package_id` and no truth; a body in an `ix:formal` fence refuses at compile with `unknown_language`, exit 20. | Test (TC-109) |
| FR-031-AC-3 | An original over its byte or line limit, a body run with `work_units` 0, and a body compiled under `SpineLimits` whose checking limit is one below the body's checking work each report incomplete with that limit's typed cause; the same request with default limits then succeeds. | Test (TC-109) |
| FR-031-AC-4 | The report of AC-1's healthy run names the body identity (`agent-ix`, its body identity, the digest of the extracted body bytes) and the original document identity with its digest, and its extraction record holds Quire's unchanged `ClausesOutcome` and the verified map; an expected `package_id` of another unit refuses at compile with `stale_dependency`, naming both. | Test (TC-109) |
| FR-031-AC-5 | Through the command built with `--features qsl-replay/quire-extraction`, a `1-draft` clause-run request whose `extraction` body record names `authority` `agent-ix`, `identity` `b`, `document` `B` and, as `digest`, the raw-artifact digest of the extracted body's bytes runs, and its `/2` document's `extraction` member renders the native identity (`agent-ix`, `b`, digest) and the formal identity (`B`, digest). The same request without the body's `authority`, or without its `digest`, refuses `invalid-request` at stage `request`, exit 20; with a `digest` of other bytes it refuses `stale_dependency`/`content-mismatch`, naming both digests. The same first request through the command built with no features refuses `invalid-request` at stage `request`, naming `extraction`, exit 20, empty stdout. | Test (TC-430) |
| FR-031-AC-6 | With every model file the request names absent, a request carrying `extraction` and `package` refuses `extraction-package-conflict`; one with no `clauses` binding and one with two refuse `extraction-clause-count` with `details` `{"actual": 0}` and `{"actual": 2}`; one carrying `call` in place of `clause`, and a compile request carrying `extraction`, refuse `extraction-requires-run`; each at stage `request`, exit 20, empty stdout, with no file read beyond the request. A `null` `extraction`, a positional `body` and an unknown member in `body` each refuse `invalid-request` naming the member. | Test (TC-109) |
| FR-031-AC-7 | A clause-only context that Quire's validator rejects refuses `invalid-quire-context` at stage `quire-context`, exit 20, with Quire's producer diagnostics unchanged in `details`; an unavailable extraction through the command writes the command-error envelope at stage `quire` retaining the original identity, the binding and Quire's outcome. | Test (TC-109) |
| FR-031-AC-8 | Over an indented CRLF fence preceded on its lines by the two-byte character `é`, a body whose second line holds an ill-typed expression refuses at compile `ill_typed` with its body span unchanged and the original regions the map gives, each region's byte offsets equal to the body offsets plus the fence's original offset and indentation; a body refusal spanning the final CRLF the map deletes carries every original region of the span; a package-encoding refusal carries no original region; a malformed original-to-body map refuses before compile with FR-030's correspondence error. | Test (TC-109) |
| FR-031-AC-9 | A body declaring two clauses refuses at compile `extraction-clause-count` with `details` `{"actual": 2}` and no `package_id`; a body with a syntax error refuses at compile at FR-109's `source` refusal; a selection naming a clause the body does not declare refuses at `select` `missing_declaration`. | Test (TC-109) |
| FR-031-AC-10 | Through the command, AC-1's healthy request writes one `/2` clause-run document whose `extraction` member equals the in-process extraction record, and the FR-267 strict reader reads it back with every member equal (QSpec FR-352-AC-13); a clause-run request over a unit given as bytes writes no `extraction`. | Test (TC-109) |

## Dependencies

- [FR-030](FR-030-consume-quire-extraction.md): the extraction consumer and
  `ExtractedSource`.
- [FR-004](FR-004-verify-source-maps.md): verified body-to-original
  correspondence.
- [FR-109](FR-109-run-a-state-clause-through-the-spine.md): the spine clause
  runner.
- [FR-312](FR-312-read-a-clause-run-request-and-run-it.md): the clause-run
  request reader the command calls.
- [FR-100](FR-100-run-a-named-function-through-the-spine.md): the native-run/1
  request envelope.
- [FR-267](FR-267-write-run-results-as-native-run-result-2.md): the `/2`
  `extraction` member and the command-error envelope; QSpec FR-352 the wire.
