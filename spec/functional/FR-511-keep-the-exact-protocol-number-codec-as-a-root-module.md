---
id: FR-511
title: "Keep the root number path as a compatibility re-export"
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
# FR-511: Keep the root number path as a compatibility re-export

## Description

FR-038 and TC-117 specify the canonical decimal-string numeric codec
`quire.protocol.numeric/1` (`ProtocolNumber`, `ExactInteger`, `ExactRational`,
`NumberError`, `NumberWire`, `NUMERIC_PROFILE`) in
`qsl-package/src/number.rs`, with the established root compatibility path at
`src/number.rs`. FR-493-AC-4 deletes `protocol_artifact`, and
FR-493 gives FR-038 no disposition. FR-499 gives the codec the row "PORT
QSL-1506; KEEP `number.rs` until then". The planner's ruling for item 9 is to
re-home it to FR-497 or FR-322, or to keep `number.rs` explicitly, chosen with
evidence, under the standing rule that a behavior is kept until its successor
is green. This requirement records the one disposition, which FR-499's row
cites: **move the implementation into qsl-package while keeping the public
root module `number` as a compatibility re-export until consumers are
repointed; then retire FR-038.** There is no permanent keep.

Evidence:

1. FR-497 owns carried quantities and unit conversion; it has no wire codec
   and states nothing about decimal-string numbers. FR-322 (QSpec) owns the
   checked package's own number encoding, a different wire identified by a
   different profile. Re-homing FR-038 under either would change the bytes of
   an interchange profile that has a live reader, or leave FR-038 stating a
   wire no QSL code writes.
2. The spine's checked-package writer and reader do not use this codec. Root
   protocol-artifact consumers still import the compatibility path until
   QSL-1501 repoints the external consumer.
3. The consumers are: inside `protocol_artifact`, `wire.rs`, `v2`, `v3`,
   `models.rs`, `validate.rs`, `occurrence.rs`, `native/metadata.rs` and the
   native emission modules (all kept until QSL-1501, FR-513); outside it,
   `src/temporal/formula.rs:802` (kept by the QSL-858 ruling until luna
   repoints IR-358), the spine vector suite
   `qsl-package/tests/protocol_number.rs` and root compatibility test
   `tests/it/protocol_number_root_module.rs`,
   `tests/support/protocol_artifact/mod.rs`,
   and quire-protocol, which imports
   `quire_spec_language::protocol_artifact::ProtocolNumber` (its
   `src/assessment.rs:22`) until QSL-1501 repoints it.
4. The codec depends on nothing in `protocol_artifact` except its module path.

## Inputs

- Decimal-string numeric wire objects of profile `quire.protocol.numeric/1`.

## Outputs

- `ProtocolNumber`, `ExactInteger`, `ExactRational`, `NumberComponent`,
  `NumberError`, `NumberWire` and `NUMERIC_PROFILE`, implemented in and
  importable from `qsl_package::number::*`; the established
  `quire_spec_language::number::*` path re-exports those exact items.

## Behavior

- The codec SHALL keep the behavior FR-038-AC-1 to AC-5 state, unchanged, when
  `protocol_artifact` is deleted.
- The implementation SHALL live at `qsl-package/src/number.rs` with no
  dependency on `protocol_artifact`. `src/number.rs` SHALL only re-export the
  spine items while legacy root consumers remain. `protocol_artifact`
  re-exports nothing of the codec and has no `number` path; quire-protocol
  imports `quire_spec_language::number` until it next bumps its pinned revision
  (QSL-1501).
- The one re-export module `src/number.rs` is a transitional exemption from
  FR-087-AC-9's rule that the root crate re-exports no layer-crate item. It
  exists while the external quire-protocol consumer imports
  `quire_spec_language::number::*`, and ends when quire-protocol no longer imports
  it (the quire-protocol checked-package repoint). The observable end check is a
  search of the external quire-protocol source for the path
  `quire_spec_language::number` that finds no use; at that point the module, the
  exemption and TC-1306's root-path steps are removed together. The exemption
  covers exactly the path `src/number.rs` and exactly the FR-511 item set,
  `ProtocolNumber`, `ExactInteger`, `ExactRational`, `NumberComponent`,
  `NumberError`, `NumberWire` and `NUMERIC_PROFILE` (the Outputs list, the one
  owner of this set); it covers no other module and no other item.
- The codec SHALL NOT be used by the spine's checked-package writers or
  readers.
- The change that removes the module SHALL wait until the successor QSL-1506 ports is
  green and every consumer above is repointed, and SHALL retire FR-038
  explicitly.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-511-AC-1 | `qsl_package::number::{ProtocolNumber, ExactInteger, ExactRational, NumberComponent, NumberError, NumberWire, NUMERIC_PROFILE}` is the implementation surface; `quire_spec_language::number` re-exports those same items and profile `quire.protocol.numeric/1`; the path `protocol_artifact::number` does not resolve. `src/number.rs` re-exports exactly those items and no other. | Test (TC-1306) |
| FR-511-AC-2 | The TC-117 vector groups (FR-038-AC-1 to AC-5) pass from `qsl_package::number` with their fixed expected strings, kinds and refusals unchanged. | Test (TC-1306) |
| FR-511-AC-3 | The qsl-package checked-package v2 writer and reader do not use the legacy numeric codec; root compatibility consumers may import the `crate::number` re-export until QSL-1501 repoints them. | Test (TC-1306) |
| FR-511-AC-4 | FR-038-AC-1 to AC-5 each have a tagged test in qsl-package after the move; a root integration test verifies the compatibility path resolves to the same spine types. | Test (TC-1306) |

## Dependencies

- **Exemption:** FR-087-AC-9 and TC-256 name `src/number.rs` as the one path
  exempt from the no-layer-crate-re-export rule, for as long as this requirement's
  external consumer exists.
- **Upstream:** FR-038, TC-117, FR-493-AC-4/AC-5, the QSL-858 and QSL-1501
  consumer rulings.
