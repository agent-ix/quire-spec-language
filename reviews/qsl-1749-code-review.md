---
id: SR-20261105101031
title: "Code review — QSL-1749 checked successor and v3 deletion gate (PR #2417)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; PR #2417 c2ef896c25372b0cffa56f00599636c44a8e2904 on 7a410035e27d7a0c64fe0f059b56fd2e1d0f99e6; docs/deletion-table.md, qsl-package/src/emit/tests/protocol_requirements/fixture_w.rs, src/protocol_artifact/native/{mod.rs,temporal_v3.rs}, tests/it/compiled_protocol_v2.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-054
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-513
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-449
    type: references
---

## Summary

Ticket: QSL-1749. Reviewed the committed PR head `c2ef896c25372b0cffa56f00599636c44a8e2904` against base `7a410035e27d7a0c64fe0f059b56fd2e1d0f99e6`. The Rust lane is folded into this code review. The change deletes the native v3 admission API, retains the strict v3 reader test through `v3::encode_candidate`, and adds the FR-054-AC-1 tag to the checked-package emission/read-back test.

The focused checked-package test passed. The checked-in v3 candidate fixture helper reproduces the deleted `admit_v3` conversion in the integration-test tree. This leaves a second owner for the conversion from admitted v2 package plus expected activation rows to the v3 wire package, and can drift from the retained v3 reader contract. The newly edited trace list also binds `FR-054-AC-3` to a checked-package test that never exercises the compiled-protocol cross-version byte/readers.

## Verdict

**FAIL**. FND-001 is high severity under the repository's no-duplication review rule; FND-002 is a false trace binding. Both should be corrected before merge.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `v3_package_from_v2` reproduces the deleted `admit_v3` conversion by copying the v2 inherited package, rewriting its version headers, copying temporal bindings, and mapping expected activations into v3 wire rows. This leaves a second implementation of the v3 package-construction behavior in test code, where it can drift from the retained producer/reader contract. | tests/it/compiled_protocol_v2.rs:1295-1315 |
| FND-002 | medium | `complete_w_activation_map_matches_read_back` is tagged with `FR-054-AC-3`, but it only emits and reads a checked package; it does not check `/1` and `/2` byte identity or cross-version `/3` reader refusals. `v3_control_temporal_activation_mapping_is_strict_and_non_inferential` remains the test that covers this criterion. Remove the `FR-054-AC-3` binding from the checked-package test. | qsl-package/src/emit/tests/protocol_requirements/fixture_w.rs:609-623 |

## Coverage

| Unit | Role | Result |
| --- | --- | --- |
| FR-054-AC-1 | examined | Checked-package relation is compared before emission and after the Contract IR byte reader; legacy `/3` reader round-trip remains exercised. |
| FR-054-AC-2 | examined | Missing, duplicate, foreign, out-of-range, non-event, reordered, and substituted mapping cases are checked across the retained strict-reader test and checked-package tests. |
| FR-054-AC-3 | examined | TC-143 covers the version rejection paths; the added checked-package tag is false because that test does not cover them. |
| FR-054-AC-4 | examined | Independently expected rows and mutations prevent profile/clock or other incidental data from supplying the mapping. |
| FR-513-AC-11 | context_only | Checked-package activation relation is the named successor; legacy `/3` reader remains covered. |
| FR-449-AC-1..5 | examined | Checked-package emission and I2 read-back compare the protocol activation relation and its trigger and temporal nodes. |
| TC-143 | examined | Strict v3 reader behavior remains tagged and exercised using an encoded candidate. |
| `docs/deletion-table.md` A19 | examined | The retired native v3 admission row is removed after its successor coverage is present. |

## Verification

- Passed: `/home/peter/dev/worktrees/locked-build.sh /home/peter/dev/worktrees/qsl-1749-code/target cargo test --locked -p qsl-package complete_w_activation_map_matches_read_back` (1 passed).
- `cargo test --locked -p quire-spec-language --test it v3_control_temporal_activation_mapping_is_strict_and_non_inferential` was started through `locked-build.sh` and passed (1 passed).
- Repository conventions, Rust review guidance, changed test behavior, and the current FR-054 / FR-513 / FR-449 contracts were inspected.

## Scope excerpts

- FR-054-AC-1: “A valid authored mapping round-trips through canonical `/3` production and strict reading with each control and temporal declaration unchanged.”
- FR-054-AC-2: “Missing, duplicate, foreign, out-of-range, non-event-triggered, reordered and substituted mappings refuse before a view is admitted.”
- FR-054-AC-3: “`/1` and `/2` remain byte-identical and reject `/3`; `/3` rejects `/1` and `/2` rather than upgrading or inferring a mapping.”
- FR-054-AC-4: “A profile/clock match, display name, source position, timestamp or observation cannot substitute for an independently expected mapping row.”
- FR-513-AC-11 excerpt: “For each checked protocol clause, the spine derives its activation relation only from that clause's validated structural node identity, activation trigger and required temporal node identities; it returns one row per unique requirement, in structural clause and temporal-key order.”
- FR-449-AC-1: “Over fixture **W** (TC-1061: `activation on each (opened: M::OrderStarted)`, `requires temporal OrderClosureBounded`), the map has exactly one row: the `OrderFlow` clause key, `Each` with `opened`'s node key, and `OrderClosureBounded`'s node key.”
- FR-449-AC-2: “**W** with `activation on origin` and the `order` capture rebound from `opened.order` to `deref(view).order` (the origin activation introduces no trigger binder) gives one row carrying `Origin` and no trigger; **W** with no `requires temporal` gives an empty map; **W** with a second `requires temporal OrderClosureBounded` gives AC-1's map.”
- FR-449-AC-3: “**W** with a second temporal clause `OrderPaidBounded` and `requires temporal OrderPaidBounded` gives two rows in ascending node id order of the two temporal clause nodes; **W** with a temporal clause `RefundBounded` on the clock `refund-clock` that the protocol does not require gives AC-1's map.”
- FR-449-AC-4: “The map of **W** derived from the in-process package equals the map derived from the package read back through the I2 reader; compiling **W** twice, and compiling **W** reformatted with a comment added, give equal maps.”
- FR-449-AC-5: “**W**'s emitted package edited so that the `requirements` entry, or separately the `each` trigger, names an existing node absent from the clause's `dependencies` is refused by the I2 reader with `invalid_semantic_graph` at the clause's `/dependencies`. Separately, each reference retargeted to a key present in the clause's `dependencies` but naming no node is refused with `missing_declaration`/`missing-name` at the target. Neither refusal returns a map.”
- TC-143 description: “Verify canonical compiled-protocol `/3` mappings between eligible linked controls and their exact selected temporal declarations without changing `/1` or `/2`.”
- A19 excerpt: “| A19 | `native::{admit_v3, ActivationSelection, AdmissionV3}` native/temporal_v3.rs:13-89 | GREEN |”

- The checked-package test's `FR-054-AC-3` trace binding is not supported by its body; the legacy-reader test is the correct binding.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The conversion helper was removed. The reader test consumes an authored serialized fixture via `include_bytes!`; mutation cases still use the authoritative `v3::encode_candidate`. |
| FND-002 | fixed | `FR-054-AC-3` was removed from the checked-package test's trace list. The retained TC-143 reader test still carries AC-3 and asserts cross-version refusals. |

### Round 1 after excerpts

- FND-001:

  ```rust
  let bytes = include_bytes!("../fixtures/compiled-protocol-v3-control-map.json");
  let package: v3::wire::Package =
      serde_json::from_slice(bytes).expect("authored canonical v3 wire fixture");
  ```

- FND-002:

  ```rust
  #[trace(
      "TC-1194",
      "FR-449-AC-1",
      "FR-449-AC-2",
      "FR-449-AC-3",
      "FR-449-AC-4",
      "FR-449-AC-5",
      "FR-054-AC-1",
      "FR-054-AC-2",
      "FR-054-AC-4",
      "FR-513-AC-11"
  )]
  ```
