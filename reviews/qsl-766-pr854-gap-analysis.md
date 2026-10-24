---
id: SR-11088
title: "QSL-766 quire-spec-language PR 854 gap analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@00aefc8ba60d0dd4e612eee38c387caab9ce1b7f; qsl-package/src/emit.rs; qsl-package/src/emit/tests/envelope_adoption.rs"
review_set: subset
---

## Summary

Ticket: QSL-766. The ticket's four acceptance criteria were checked against
the diff and the locked IR rev. Plan completion: not assessed. The ticket
names no FR ids. The touched ids are QSpec FR-322 (cited by id only),
FR-092..094, and TC-390 layering.

- AC1 (built through the IR encoder; local type and closure deleted; no
  wrapper): met. `encode_checked_package_v2` is called directly inside the
  `EmittedPackage::new` closure. `WireV2`, its `Encode` impl and
  `encode_array` are deleted.
- AC2 (emitted bytes equal before and after; emit tests pass): met. The
  bytes are equal by construction (see SR-11087). The existing emit,
  golden and admission-corpus tests pass unedited, and no golden was
  re-recorded.
- AC3 (a test fails if qsl-package defines its own envelope wire type): met.
  `qsl_package_defines_no_envelope_wire_type` would fail on `origin/main`,
  because the old emit.rs had both `name("contract_version")` and
  `impl Encode for WireV2`. It asserts the scan found files, so it cannot
  pass vacuously on an empty directory.
- AC4 (lock bump if needed): not needed. The IR rev in Cargo.lock,
  f7dd6130, is an ancestor of IR `origin/main`, and it defines
  `pub fn encode_checked_package_v2` at
  `crates/quire-contract-model/src/checked_package/v2/mod.rs:498`.

## Verdict

PASS. All four ACs are met. Two non-blocking findings are recorded.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The AC3 scan matches only `name("contract_version")` or `Encode for `. A local envelope built as a `serde_json::json!` value or a serde-derived struct with a `contract_version` field would pass it. The deleted shape is caught, so the test is not vacuous, but it is narrow. | qsl-package/src/emit/tests/envelope_adoption.rs:38-41 |
| FND-002 | low | No test pins the envelope bytes. AC2 byte equality rests on the code-equivalence argument plus IR-reader round trips. The only byte golden pins the identity preimage digest, not the envelope. | qsl-package/src/checked.rs:637 |
