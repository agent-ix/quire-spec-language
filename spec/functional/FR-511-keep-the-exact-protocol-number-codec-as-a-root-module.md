---
id: FR-511
title: "Keep the exact protocol number codec in qsl-package"
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
# FR-511: Keep the exact protocol number codec in qsl-package

## Description

FR-038 and TC-117 specify the canonical decimal-string numeric codec
`quire.protocol.numeric/1` (`ProtocolNumber`, `ExactInteger`, `ExactRational`,
`NumberError`, `NumberWire`, `NUMERIC_PROFILE`) in
`qsl-package/src/number.rs` (formerly also re-exported at the root path
`src/number.rs`, now deleted). FR-493-AC-4 deletes `protocol_artifact`, and
FR-493 gives FR-038 no disposition. FR-499 gives the codec the row "PORT
QSL-1506; KEEP `number.rs` until then". The planner's ruling for item 9 is to
re-home it to FR-497 or FR-322, or to keep `number.rs` explicitly, chosen with
evidence, under the standing rule that a behavior is kept until its successor
is green. This requirement records the one disposition, which FR-499's row
cites: **move the implementation into qsl-package while keeping the public
root module `number` as a compatibility re-export until consumers are
repointed.** The root module has since been deleted: quire-protocol, its one
external consumer, no longer imports `quire_spec_language::number` (end
condition below), so `qsl_package::number` is the codec's only path.

Evidence:

1. FR-497 owns carried quantities and unit conversion; it has no wire codec
   and states nothing about decimal-string numbers. FR-322 (QSpec) owns the
   checked package's own number encoding, a different wire identified by a
   different profile. Re-homing FR-038 under either would change the bytes of
   an interchange profile that has a live reader, or leave FR-038 stating a
   wire no QSL code writes.
2. The spine's checked-package writer and reader do not use this codec.
3. The consumers are: inside `protocol_artifact`, `wire.rs`, `v2`, `v3`,
   `models.rs`, `validate.rs`, `native/metadata.rs` and the
   native emission modules (all kept until QSL-1501, FR-513); outside it,
   the spine vector suite `qsl-package/tests/protocol_number.rs`.
4. The codec depends on nothing in `protocol_artifact` except its module path.

## Inputs

- Decimal-string numeric wire objects of profile `quire.protocol.numeric/1`.

## Outputs

- `ProtocolNumber`, `ExactInteger`, `ExactRational`, `NumberComponent`,
  `NumberError`, `NumberWire` and `NUMERIC_PROFILE`, implemented in and
  importable from `qsl_package::number::*` only. The root crate has no
  `number` module.

## Behavior

- The codec SHALL keep the behavior FR-038-AC-1 to AC-5 state, unchanged, when
  `protocol_artifact` is deleted.
- The implementation SHALL live at `qsl-package/src/number.rs` with no
  dependency on `protocol_artifact`. `protocol_artifact` is deleted and has no
  `number` path.
- The root crate SHALL re-export no codec item and has no `number` module
  (FR-087-AC-9). The former transitional root re-export `src/number.rs`
  ended under its stated end condition: a search of quire-protocol's source
  finds no `quire_spec_language::number`, so the module, its FR-087-AC-9
  exemption and TC-1306's root-path steps were removed together.
- The codec SHALL NOT be used by the spine's checked-package writers or
  readers.
- FR-038 stays live: the codec remains in qsl-package, and FR-038's integer
  spelling is the one FR-100, FR-106 and FR-286 cite. Retiring FR-038 would
  need those requirements to own their spelling first; removing the root path
  does not.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-511-AC-1 | **RETIRED (root re-export removed).** The criterion asserted that `quire_spec_language::number` re-exports the codec. Ruling: its end condition held (quire-protocol main names no `quire_spec_language::number`), so `src/number.rs` was deleted. The root crate re-exporting nothing is FR-087-AC-9 (TC-256). | Test (TC-1306), retired |
| FR-511-AC-2 | The TC-117 vector groups (FR-038-AC-1 to AC-5) pass from `qsl_package::number` with their fixed expected strings, kinds and refusals unchanged. | Test (TC-1306) |
| FR-511-AC-3 | The qsl-package checked-package v2 writer and reader do not use the legacy numeric codec. | Test (TC-1306) |
| FR-511-AC-4 | FR-038-AC-1 to AC-5 each have a tagged test in qsl-package after the move. | Test (TC-1306) |

## Dependencies

- **Ended exemption:** FR-087-AC-9 and TC-256 once exempted `src/number.rs`
  from the no-layer-crate-re-export rule; the exemption ended with the module.
- **Upstream:** FR-038, TC-117, FR-493-AC-4/AC-5, the QSL-858 and QSL-1501
  consumer rulings.
