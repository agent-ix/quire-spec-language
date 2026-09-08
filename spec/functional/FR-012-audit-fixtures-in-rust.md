---
id: FR-012
title: "Audit selected verification fixtures in Rust"
type: FR
relationships:
  - target: "ix://agent-ix/quire-spec-language/US-004"
    type: traces_to
  - target: "ix://agent-ix/quire-spec-language/FR-001"
    type: depends_on
  - target: "ix://agent-ix/quire-spec-language/FR-002"
    type: depends_on
  - target: "ix://agent-ix/quire-spec-language/NFR-005"
    type: references
---
# FR-012: Audit selected verification fixtures in Rust

## Description

When a verification fixture audit is requested, the Rust audit command shall report the checks actually performed over the selected immutable inputs.

## Inputs

An audit mode and an explicit fixture directory, except for the self-contained
negative-control mode. Paths are OS paths, not assumed UTF-8 strings. Modes are
`self-test`, `review`, `roles`, `model-bytes`, `rule-syntax`, and `model-producer`.
The selected model fixture root contains `model-source/` and `model-output/`.
The review and roles roots contain their respective manifest JSON files.
The rule-syntax root contains `profile.md`, `state-semantics.md`, and `fixtures/`.

## Outputs

A deterministic mode-specific count/claim summary on stdout after all checks
complete; otherwise a stable error code with context on stderr. Exit codes are
0 for the selected audit succeeding, 1 for invalid fixture content, 2 for usage
or I/O failure, and 3 for resource incompleteness or an unapproved producer.
Errors implement the standard Rust Display and Error traits and have an owned
stable-code catalog. No evaluator, semantic matcher or fresh producer run is
claimed by a byte/syntax check.

## Behavior

One Rust verification executable in the existing package replaces the four
Python helpers. Private audit modules share exact-byte hashing, bounded reads,
strict JSON intake and contextual errors. The harness is not a production
shared-reference decoder or a second model authority.

The review mode checks all selected invocation, snapshot and frame artifacts
against their SHA-256, profile and identity/revision, then exercises stale-digest
and recomputed-digest immutable-key conflicts for all three roles. Its registry
key retains authority, identity and separate revision namespace/value fields;
it follows the historical review wrapper rather than silently rewriting it as
a newer wire contract.

The roles mode checks every artifact digest, changed-byte controls, original
source spans/coordinates, profile definition, explicit feature sets, model
manifest/lock/closure correspondence, native parse-output provenance and selected
role membership. A stored parse-output record remains historical evidence.

Model-bytes checks all five selected producer checkpoint artifacts and the exact
producer revision `3b75e01c652ba00bb07c352ff5467419401e792b`. Rule-syntax checks
selected profile/rule digests, distinct case IDs, declared anchors and actual
native parse/refusal behavior using this package's parser. It interprets no
typing/evaluation expectation. Library reuse removes the old temporary-source
and child-CLI runner without changing that claim boundary.

All JSON input rejects duplicate decoded keys, including escaped equivalents,
invalid UTF-8/scalars, malformed or trailing JSON, and wrongly typed required
fields. Relative manifest paths must resolve within the selected fixture root;
absolute paths, traversal outside the root and symlink escapes refuse. Reads
are limited to 8 MiB per file, 64 MiB total and 10000 files/records per audit;
JSON depth is bounded by the decoder and native syntax by its existing Limits.
Exceeded budgets return incomplete; no partial success summary is emitted.
Local fixture trees must remain immutable for the duration of an audit; this is
not a hostile concurrent-filesystem service.

No audit launches Python, Node, a shell or another external producer. The
`model-producer` mode returns `producer-language-unapproved` without spawning a
process. Fresh TypeSpec/Node producer qualification awaits the campaign's
explicit owner disposition and a reviewed runner contract. Historical producer
evidence and its exact bytes remain available through model-bytes.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-012-AC-1 | Self-test rejects stale bytes and recomputed-digest identity reuse for snapshot, frame and invocation roles, plus a duplicate JSON key. | Test |
| FR-012-AC-2 | Review mode checks the selected 23 artifact files and seven invocation cases with all six content/digest controls. | Test |
| FR-012-AC-3 | Roles mode verifies the selected 17 artifacts, four source regions and declared model/native-output/role correspondence. | Test |
| FR-012-AC-4 | Model-bytes verifies the five selected checkpoint digests and producer pin, and rejects a changed artifact or revision. | Test |
| FR-012-AC-5 | Rule-syntax observes 50 parsed cases and one unsupported refusal after checking distinct IDs and exact rule/profile digests. | Test |
| FR-012-AC-6 | Malformed JSON, decoded duplicate keys, missing fields and wrong field types return an error without a panic or success summary. | Test |
| FR-012-AC-7 | An absolute or escaping manifest path, including a symlink outside the selected root, is refused. | Test |
| FR-012-AC-8 | Audits leave selected artifact bytes unchanged and identify byte/syntax checks without claiming evaluation or fresh production. | Test |
| FR-012-AC-9 | A file or aggregate budget overflow reports resource incompleteness without accepting partial results. | Test |
| FR-012-AC-10 | Unknown modes and malformed invocations exit 2; OS paths do not panic because of their encoding. | Test |
| FR-012-AC-11 | Model-producer exits 3 with producer-language-unapproved and launches no producer. | Test |

## Dependencies

- [US-004](../usecase/US-004-reuse-existing-toolchain.md) supplies the integration/evidence need.
- [FR-001](FR-001-read-exact-source.md) and [FR-002](FR-002-parse-native-units.md) supply the native parser boundary.
- [NFR-005](../non-functional/NFR-005-rust-verification-paths.md) governs executable verification languages.
- [IT-004](../integration/IT-004-rust-fixture-audits.md) specifies real audit command execution.

## Status

Specified from the owner's Rust-remediation request before implementation.
The existing Python helpers are preserved until the replacement is reviewed and
verified. This scope does not remediate unrelated parser/formatter findings or
accept the remaining LC02–LC05 contracts.
