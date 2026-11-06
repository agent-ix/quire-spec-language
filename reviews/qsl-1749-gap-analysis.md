---
id: SR-20261105101032
title: "Gap analysis — QSL-1749 checked activation successor (PR #2417)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; PR #2417 c2ef896c25372b0cffa56f00599636c44a8e2904 on 7a410035e27d7a0c64fe0f059b56fd2e1d0f99e6; scoped to FR-054-AC-1..4, FR-513-AC-11, FR-449-AC-1..5 and the changed code"
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

Ticket: QSL-1749. Planless gap analysis for the changed v3 admission surface and its named checked-package successor. Plan completion: not assessed. The required repository matrix command, `quoin matrix --repo . --json`, ran for more than eight minutes without producing output and was interrupted at the lead's direction (exit 130). The full repository-wide matrix, reverse-gap inventory, and stubs audit therefore remain unverified. Manual review found a false `FR-054-AC-3` trace on the checked-package test; the legacy TC-143 test still correctly covers the criterion. No unowned production behavior was found in the changed paths.

Optional semantic review was skipped. This report makes no repository-wide PASS claim.

## Verdict

**CONDITIONAL**. The legacy TC-143 test covers the criterion, but one checked-package trace binding is false; the repository-wide matrix was unavailable during this review.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `complete_w_activation_map_matches_read_back` is tagged with `FR-054-AC-3`, but it never checks `/1` and `/2` byte identity or cross-version `/3` reader refusals. The legacy TC-143 test correctly covers this criterion; remove the false binding from the checked-package test. | qsl-package/src/emit/tests/protocol_requirements/fixture_w.rs:609-623 |

## Coverage

| Unit | Role | Result |
| --- | --- | --- |
| FR-054-AC-1 | examined | `complete_w_activation_map_matches_read_back` now binds AC-1 to checked-package emission/read-back; TC-143 continues to exercise canonical `/3` encoding and strict reading. |
| FR-054-AC-2 | examined | TC-143 retains mutation/refusal cases; checked-package relation tests cover dependency, deduplication, and target refusals. |
| FR-054-AC-3 | examined | TC-143 exercises cross-version refusals; the checked-package test also carries an unsupported AC-3 tag. |
| FR-054-AC-4 | examined | TC-143 supplies independently expected rows to strict reading and tests substitutions; checked-package tests ensure the relation is derived from checked structure. |
| FR-513-AC-11 | examined | `complete_w_activation_map_matches_read_back` compares the derived relation before emission with the checked-package I2 read-back; the retained legacy reader rejects incompatible envelopes. |
| FR-449-AC-1..5 | examined | The checked-package test checks relation values, structural trigger and temporal node identities, empty and duplicate requirements, and equivalent source forms. |
| A19 deletion gate | examined | The PR removes the native v3 admission row after adding the checked-package successor tag and retaining v3 strict-reader coverage. |
| Changed production paths | examined | The only production behavior deleted is the FR-054 native v3 admission API; the v3 strict reader and canonical encoder remain. No changed production stub or behavior without a named requirement was found. |

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
- A19 excerpt: “| A19 | `native::{admit_v3, ActivationSelection, AdmissionV3}` native/temporal_v3.rs:13-89 | GREEN |”

## Manual bindings

| Test | Criterion | Binding |
| --- | --- | --- |
| `complete_w_activation_map_matches_read_back` | FR-054-AC-1 | correct |
| `complete_w_activation_map_matches_read_back` | FR-054-AC-2 | correct |
| `complete_w_activation_map_matches_read_back` | FR-054-AC-3 | wrong: no compiled-protocol version behavior asserted |
| `complete_w_activation_map_matches_read_back` | FR-054-AC-4 | correct |
| `complete_w_activation_map_matches_read_back` | FR-513-AC-11 | correct |
| `complete_w_activation_map_matches_read_back` | FR-449-AC-1..5 | correct |
| `v3_control_temporal_activation_mapping_is_strict_and_non_inferential` | FR-054-AC-1..4 | correct |

## Verification

- Passed: `/home/peter/dev/worktrees/locked-build.sh /home/peter/dev/worktrees/qsl-1749-code/target cargo test --locked -p qsl-package complete_w_activation_map_matches_read_back` (1 passed).
- Passed: `/home/peter/dev/worktrees/locked-build.sh /home/peter/dev/worktrees/qsl-1749-code/target cargo test --locked -p quire-spec-language --test it v3_control_temporal_activation_mapping_is_strict_and_non_inferential` (1 passed).
- `quoin matrix --repo . --json` was stopped after >8 minutes without output; no computed matrix results are claimed.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Removed the unsupported FR-054-AC-3 trace from the checked-package test. TC-143 continues to carry the tag and exercises cross-version refusals. |

### Round 1 after excerpt

- FND-001:

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
