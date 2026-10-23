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
---
## Description

When a caller selects one authored clause of an original Markdown document,
the spine clause runner `qsl_replay::spine::run_clause` (FR-109), built with
`qsl-replay`'s feature `quire-extraction`, shall run the clause's extracted
body, the `ExtractedSource` FR-030 yields, as its unit: spine compile (S1 to
S4) under the body's own identity, then the clause run. The report shall name
both the original document and the body. This is the one way QSL runs an
extracted Markdown clause. The `quire-spec` command's native-run/1 request
has no `extraction` member in any build (FR-100; ADR-011 §7.3 X-4).

## Inputs

- The original Markdown `Source`, with its four FR-001 labels and the raw
  bytes the caller digest-verified, and FR-030's `Selection`: the authored
  clause id, the authored package and the body's own `SourceIdentity`.
- FR-030's `Limits` for the original document.
- The other `ClauseRunRequest` members: FR-056's package input, FR-099's
  dependency input, the snapshot and invocation provisions, the selection, an
  optional expected `package_id`, and the spine, observation and evaluation
  limits (FR-109).

## Outputs

- A `ClauseRunReport` (FR-109). Its source names the body identity with the
  raw-artifact digest of the extracted body's complete bytes (QSpec FR-018)
  and the original document identity with its digest. Its `package_id` is
  the one the body compiles to under its body identity (FR-109-AC-6).
- Or FR-030's extraction `Error`, which retains the original source, the
  selection, any completed Quire extraction outcome and the actual preflight,
  availability or correspondence cause. No compile or clause run follows,
  and there is no truth.

## Behavior

The caller shall extract the body through FR-030 before any compile. An
extraction failure shall end the request with FR-030's error and no partial
package.

The clause runner shall compile the body under its body identity. It shall
report compile, select, admit and evaluate outcomes with FR-109's stages,
categories, truth and exit codes, exactly as for a source given as bytes. A
body whose fence declares a language other than QSL native shall refuse at
compile with `unknown_language`, exit 20, with no `package_id` (FR-109-AC-6).

The original document and the body shall keep distinct identities. The
original selects the Markdown bytes; the body's identity owns the compiled
declarations and so the `package_id` (FR-001, FR-032-AC-4).

`Selection` names exactly one clause, so a request cannot select a second
clause. An expected `package_id` that the body does not compile to shall
refuse at compile with `stale_dependency`, naming both identities
(FR-109-AC-2).

The original byte and line limits (FR-030) and the spine and evaluation
limits (FR-109) shall stay independent. When a request exhausts any of them,
the clause runner shall report incomplete; a fresh request with the same
inputs and sufficient limits then succeeds.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-031-AC-1 | An LF and a CRLF Markdown document whose one selected fence holds FR-108's healthy-parent unit run through `run_clause` report `success`, exit 0, with the `package_id` of the body compiled alone; violating-parent reports `violation`, exit 10; missing-model reports `compile`, `missing_import`/`missing-selection`, exit 20, with no `package_id`. | Test (TC-109) |
| FR-031-AC-2 | A selected clause the document does not hold, an unavailable extraction and a stale original digest each end with FR-030's error, retaining the original identity, the selection and Quire's actual outcome, with no compile, no `package_id` and no truth; a body in an `ix:formal` fence refuses at compile with `unknown_language`, exit 20. | Test (TC-109) |
| FR-031-AC-3 | An original over its byte or line limit, and a body run with `work_units` 0, each report incomplete; the same request with default limits then succeeds. | Test (TC-109) |
| FR-031-AC-4 | The report of AC-1's healthy run names the body identity (`agent-ix`, its body identity, the digest of the extracted body bytes) and the original document identity with its digest; an expected `package_id` of another unit refuses at compile with `stale_dependency`, naming both. | Test (TC-109) |
| FR-031-AC-5 | A `1-draft` native-run/1 request carrying a program `extraction` body record refuses `invalid-request` at stage `request`, exit 20, through the command built with no features and with `--all-features` (FR-100-AC-3). | Test (TC-430) |

## Dependencies

- [FR-030](FR-030-consume-quire-extraction.md): the extraction consumer and
  `ExtractedSource`.
- [FR-109](FR-109-run-a-state-clause-through-the-spine.md): the spine clause
  runner.
- [FR-100](FR-100-run-a-named-function-through-the-spine.md): the native-run/1
  request, which has no `extraction` member.
