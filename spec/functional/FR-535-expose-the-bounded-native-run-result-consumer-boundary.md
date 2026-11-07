---
id: FR-535
title: "Expose the bounded native-run-result/2 consumer boundary"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-267
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-109
    type: references
---
# FR-535: Expose the bounded native-run-result/2 consumer boundary

## Description

`qsl_replay` SHALL export, at its crate root, the producer, the strict reader,
the typed results and the caller-supplied bounds of
[FR-267](FR-267-write-and-read-run-results-as-native-run-result-2.md), so a
consumer reads and writes exact `native-run-result/2` bytes without naming
`qsl_replay::spine`.

## Inputs

Candidate `native-run-result/2` bytes and a `NativeRunLimits` holding
`max_input_bytes` (encoded bytes) and `max_working_bytes` (decoded working
heap); or a `ClauseRunReport` to write.

## Outputs

A `NativeRunResult` that is either a `Clause` record or a `CommandError`
record, retaining the original disposition, identities, basis, witness and
typed command-error fields; or the located `NativeRunReadError`; or the
canonical bytes of `write_clause_run_result`.

## Behavior

`read_native_run_result_bounded` SHALL apply the FR-267 strict reader unchanged.
It SHALL refuse an input longer than `max_input_bytes` as the canonical
reader's input-byte limit before parsing it. It SHALL charge every growth of
the shared reader's tree, the writer heap and canonical copy of each
re-encoded member, and the canonical length of each envelope decoded into
typed records, against one cumulative `max_working_bytes`. It SHALL refuse a
growth past that bound before it happens as `NativeRunReadError::WorkingBytes`
naming the bound and the required total, and return no partial result.

The root API SHALL NOT add a tool-version record, digest, stored receipt, run
token or generated-source authentication claim. A successful read grants no
campaign or backend qualification; the consumer owns any association between
the bytes and its own invocation.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-535-AC-1 | Using only crate-root names, a consumer writes a clause-run `/2` document and reads it back, distinguishing the clause and command-error variants and inspecting stage, category, code, basis and typed command-error fields. | Test |
| FR-535-AC-2 | Over a decisive-witness record, a changed or removed format identifier refuses as `UnsupportedFormat` (FR-267-AC-2, FR-267-AC-8); an unknown basis, a removed `usage` or `basis`, and an unknown top-level, disposition or witness member refuse as `MalformedMember` naming that member (FR-267-AC-2, FR-267-AC-8); a removed witness `index` refuses as a `Witness` refusal; a basis and witness that disagree in either direction, a witness added to a non-witness record, and a `temporal` disposition added to a witness record refuse as `Invalid` on `witness` (FR-267-AC-2); a malformed command-error shape refuses as `Invalid` on `details` (FR-267-AC-10). None normalizes to success. The temporal `trace_position` null mutation is owned by the FR-267 temporal tests. | Test |
| FR-535-AC-3 | An input over `max_input_bytes` refuses as the input-byte limit before decoding; a working bound below the document's decoding need refuses as `WorkingBytes` with no result; an in-budget genuine `/2` document still yields the original typed result. | Test |
| FR-535-AC-4 | `NativeRunLimits` has exactly the two bounds, and a document written by `write_clause_run_result` has only the FR-267 members `basis`, `disposition`, `format` and `usage`; no digest, receipt, token or tool-version member exists. | Test |

## Dependencies

- [FR-267](FR-267-write-and-read-run-results-as-native-run-result-2.md) owns the `/2` wire contract.
- [TC-1340](../test-cases/TC-1340-bounded-native-run-result-consumer-boundary.md) defines verification.
