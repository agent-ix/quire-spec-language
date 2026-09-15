---
id: SR-439
title: "Code and Rust review of the published compiled-protocol v1 handoff"
type: SpecReview
analysis: code-review
scope: "quire-spec-language#112; FR-042; TC-121; protocol_artifact::handoff; native_protocol_handoff; artifacts/compiled-protocol-v1"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-042
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-121
    type: reviews
---

## Summary

`/rust-review` examined the complete QSL #112 delta at PR readiness. The existing
Rust producer now emits a deterministic checksum inventory after native
admission, canonical emission and its independent strict read have succeeded.
The public handoff module adds version-explicit `/1` directory and member
addresses without changing the existing `/2` names or reader APIs. The committed
four-source corpus reconstructs its selected model and admits through the public
strict `/1` reader.

## Verdict

**CONDITIONAL PASS** — all review findings are resolved and both focused controls
pass. Merge remains conditioned only on the repository's routine serial local
gate at the exact committed head; there is no actionable code-review finding.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Resolved: the blocker ticket initially conflated the synthetic frozen `/1`↔`/2` compatibility digest with Protocol's richer four-source producer corpus. #112 and repository tracking now follow the actual producer/test code; the frozen fixture remains unchanged and the real Protocol input is published. | #112; `tests/compiled_protocol_v2.rs`; `artifacts/compiled-protocol-v1` |
| FND-002 | medium | Resolved: the first checksum test proved the on-disk inventory but not that every `Selection` path was normalized and covered. The final control checks every selected source, dependency and model-source path and rejects symbolic/non-file entries. | `src/protocol_artifact/handoff.rs`; TC-121 |

## Rust review

- New production code is Rust and adds no `unsafe`, panic, unchecked numeric
  conversion, callback, parser, network path, process launch or shared mutable
  state. I/O and path failures return the producer's typed `Error`.
- The checksum pass runs only after the established bounded compiler, emitter
  and strict-reader stages succeed. It traverses the freshly created output,
  rejects symbolic/non-file entries, sorts normalized relative paths through a
  `BTreeSet`, hashes exact bytes and omits only the necessarily self-referential
  checksum file.
- The public additions are constants and the already published closed
  `Selection` type. Decoding selection data grants no admission; the test must
  reconstruct exact model/dependency/source inputs and invoke `read`.
- The checked-in dependency binary is the actual stripped release ELF selected
  by the producer and its exact digest remains bound through the package,
  selection and checksum inventory. No synthetic producer string or copied
  schema substitutes for it.
- The existing `/2` directory has no diff, the historical frozen compatibility
  digest is unchanged, `publish = false` remains set, and no tag or native-v1
  resource is modified.

## Focused evidence

| Control | Result |
| --- | --- |
| red control before publication | failed exactly because `PUBLISHED_V1_HANDOFF` did not exist |
| published address, normalized paths and complete checksums | pass |
| selected model reconstruction and public strict `/1` read | pass |
| `quire validate` for changed plan/spec tracking | pass; inherited duplicate-module warnings only |
| `quire coverage --scope .` | FR-042 10/10 tagged; repository 491/512 with inherited unrelated gaps |

The exact committed-head Cargo gate is the remaining merge condition and will
be recorded on #112 and the pull request rather than asserted in advance here.
