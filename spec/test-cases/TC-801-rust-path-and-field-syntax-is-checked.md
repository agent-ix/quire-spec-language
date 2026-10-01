---
id: TC-801
title: "RustPath, RustField and RustReceiver segments are checked against Rust syntax"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-304
    type: verifies
---
# TC-801: RustPath, RustField and RustReceiver segments are checked against Rust syntax

## Description

Scope: FR-304-AC-6.

## Test Procedure

Check `ObjectBinding`s whose `rust_type` or field values use the segments
`type`, `9lives`, the tuple index `01`, and, separately, `r#type`,
`config_store` and the tuple index `0`. Check `FrameBinding`s whose
`receiver` is `self`, `store` and `0`.

## Expected Results

`type`, `9lives`, `01` and the receiver `0` each refuse `invalid_model_binding`/
`malformed-declaration`, naming the segment and its span. `r#type`,
`config_store`, `0` and the receivers `self` and `store` check. The expected verdicts are literals in the
test.
