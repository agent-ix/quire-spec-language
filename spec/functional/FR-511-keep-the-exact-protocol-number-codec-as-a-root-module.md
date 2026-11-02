---
id: FR-511
title: "Keep the exact protocol number codec as a root module"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-004
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-038
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-493
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-497
    type: references
  - target: ix://agent-ix/quire-specification/FR-322
    type: references
---
# FR-511: Keep the exact protocol number codec as a root module

## Description

FR-038 and TC-117 specify the canonical decimal-string numeric codec
`quire.protocol.numeric/1` (`ProtocolNumber`, `ExactInteger`, `ExactRational`,
`NumberError`, `NumberWire`, `NUMERIC_PROFILE`) in
`src/protocol_artifact/number.rs`. FR-493-AC-4 deletes `protocol_artifact`, and
FR-493 gives FR-038 no disposition. FR-499 gives the codec the row "PORT
QSL-1506; KEEP `number.rs` until then". The planner's ruling for item 9 is to
re-home it to FR-497 or FR-322, or to keep `number.rs` explicitly, chosen with
evidence, under the standing rule that a behavior is kept until its successor
is green. This requirement records the one disposition, which FR-499's row
cites: **keep the codec as a root module `number` until the successor that
QSL-1506 ports is green; then retire FR-038.** There is no permanent keep.

Evidence:

1. FR-497 owns carried quantities and unit conversion; it has no wire codec
   and states nothing about decimal-string numbers. FR-322 (QSpec) owns the
   checked package's own number encoding, a different wire identified by a
   different profile. Re-homing FR-038 under either would change the bytes of
   an interchange profile that has a live reader, or leave FR-038 stating a
   wire no QSL code writes.
2. No `qsl-*` crate refers to `ProtocolNumber` or `quire.protocol.numeric/1`
   (`git grep` over `qsl-*/src`); the spine's checked package does not use the
   codec.
3. The consumers are: inside `protocol_artifact`, `wire.rs`, `v2`, `v3`,
   `models.rs`, `validate.rs`, `occurrence.rs`, `native/metadata.rs` and the
   native emission modules (all kept until QSL-1501, FR-513); outside it,
   `src/temporal/formula.rs:802` (kept by the QSL-858 ruling until luna
   repoints IR-358), the `tests/it` files `protocol_number.rs`,
   `native_choice_emission.rs`, `native_refund_emission.rs`,
   `native_owned_attempt_progress.rs`, `native_split_shipment_emission.rs`,
   `native_query_emission.rs`, `native_payment_retry_emission.rs`,
   `native_protocol_emission.rs`, `native_compensation_emission.rs` and
   `native_numeric_domain_emission.rs`, `tests/support/protocol_artifact/mod.rs`,
   `examples/native_protocol_handoff.rs`, and quire-protocol, which imports
   `quire_spec_language::protocol_artifact::ProtocolNumber` (its
   `src/assessment.rs:22`) until QSL-1501 repoints it.
4. The codec depends on nothing in `protocol_artifact` except its module path.

## Inputs

- Decimal-string numeric wire objects of profile `quire.protocol.numeric/1`.

## Outputs

- `ProtocolNumber`, `ExactInteger`, `ExactRational`, `NumberComponent`,
  `NumberError`, `NumberWire` and `NUMERIC_PROFILE`, importable as
  `quire_spec_language::number::*`.

## Behavior

- The codec SHALL keep the behavior FR-038-AC-1 to AC-5 state, unchanged, when
  `protocol_artifact` is deleted.
- The module SHALL live at `src/number.rs` with no dependency on
  `protocol_artifact`, and every consumer listed above that stays SHALL import
  `crate::number`. The re-export `protocol_artifact::ProtocolNumber` that
  quire-protocol imports stays only until QSL-1501 repoints quire-protocol, and
  is deleted, with `protocol_artifact::number`, by the change that deletes
  `protocol_artifact`; no other alias is kept.
- The codec SHALL NOT be used by the spine's checked-package writers or
  readers.
- The change that removes the module SHALL wait until the successor QSL-1506 ports is
  green and every consumer above is repointed, and SHALL retire FR-038
  explicitly.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-511-AC-1 | After `protocol_artifact` is deleted, `quire_spec_language::number::{ProtocolNumber, ExactInteger, ExactRational, NumberComponent, NumberError, NumberWire, NUMERIC_PROFILE}` resolve; `NUMERIC_PROFILE` equals `quire.protocol.numeric/1`; the path `protocol_artifact::number` does not resolve, and the kept consumers import `crate::number`. | Test (TC-1306) |
| FR-511-AC-2 | The TC-117 vector groups (FR-038-AC-1 to AC-5) pass at the new path with their expected strings, kinds and refusals unchanged. | Test (TC-1306) |
| FR-511-AC-3 | No `qsl-*` crate's source and no checked-package v2 writer or reader refers to the module or to `quire.protocol.numeric/1`. | Test (TC-1306) |
| FR-511-AC-4 | FR-038-AC-1 to AC-5 each have a tagged test after the deletion (`quire matrix` lists none untagged). | Test (TC-1306) |

## Dependencies

- **Upstream:** FR-038, TC-117, FR-493-AC-4/AC-5, the QSL-858 and QSL-1501
  consumer rulings.
