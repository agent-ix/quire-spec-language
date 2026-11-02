---
id: FR-509
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
# FR-509: Keep the exact protocol number codec as a root module

## Description

FR-038 and TC-117 specify the canonical decimal-string numeric codec
`quire.protocol.numeric/1` (`ProtocolNumber`, `ExactInteger`, `ExactRational`,
`NumberError`, `NumberWire`, `NUMERIC_PROFILE`) in
`src/protocol_artifact/number.rs`. FR-493-AC-4 deletes `protocol_artifact`, and
neither FR-493 nor FR-499 dispositions FR-038. The planner's ruling for item 9
is to re-home it to FR-497 or FR-322, or to keep `number.rs` explicitly, chosen
with evidence. This requirement records the choice: **keep it, as a root
module `number`.**

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
3. Two consumers remain: `src/temporal/formula.rs:802` (kept by the QSL-858
   ruling until luna repoints IR-358) and quire-protocol `link/mod.rs:125,1675`
   (until QSL-1501 repoints it).
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
  `protocol_artifact`, and the old path `protocol_artifact::number` and its
  re-exports SHALL be deleted with `protocol_artifact`; no alias at the old
  path is kept (no compatibility shim).
- The codec SHALL NOT be used by the spine's checked-package writers or
  readers.
- The module SHALL be removed only by a requirement that names the last
  consumer's repointing and retires FR-038 explicitly, with the owner's
  approval.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-509-AC-1 | After `protocol_artifact` is deleted, `quire_spec_language::number::{ProtocolNumber, ExactInteger, ExactRational, NumberComponent, NumberError, NumberWire, NUMERIC_PROFILE}` resolve; `NUMERIC_PROFILE` equals `quire.protocol.numeric/1`; the path `protocol_artifact::number` does not resolve. | Test (TC-1304) |
| FR-509-AC-2 | The nine TC-117 vectors groups (FR-038-AC-1 to AC-5) pass at the new path with their expected strings, kinds and refusals unchanged. | Test (TC-1304) |
| FR-509-AC-3 | No `qsl-*` crate's source and no checked-package v2 writer or reader refers to the module or to `quire.protocol.numeric/1`. | Test (TC-1304) |
| FR-509-AC-4 | FR-038-AC-1 to AC-5 each have a tagged test after the deletion (`quire matrix` lists none untagged). | Test (TC-1304) |

## Dependencies

- **Upstream:** FR-038, TC-117, FR-493-AC-4/AC-5, the QSL-858 and QSL-1501
  consumer rulings.
