---
id: SR-11686
title: "Code review — QSL-798 Text admission, witness String row and String replay (PR 887)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@178ecdca85e971037417e5559f7fe4dce9500234; qsl-semantics/src/model/observation/document.rs, qsl-semantics/src/check/mod.rs, qsl-semantics/src/model/observation.rs, qsl-semantics/tests/it/native_snapshot_values.rs, qsl-replay/src/execute/argument.rs, qsl-replay/src/execute/tests/native_scalar.rs, qsl-replay/src/witness/value.rs, qsl-replay/src/witness/value_text.rs, qsl-replay/src/witness/value_text/tests.rs"
review_set: subset
---

## Summary

Ticket: QSL-798 (code half, partial). One pass of code-review with the rust-review lane over `git diff origin/main...HEAD`. No build was run. The changes were checked against FR-106-AC-33..36, FR-070-AC-14/15 and FR-098-AC-13 on origin/main, with TC-1038 steps 2, 4, 5 and 6, TC-905 steps 7 and 8, and TC-1028 steps 6 and 7.

## Verdict

**PASS: no blocking findings.** `admit_text` runs the envelope, then type fit (the match arm), then the profile length on the retained sequence. The refusal names `text`, `expected` `Text[min, max; profile]` and the measured `actual`. The `UNBOUNDED_SCALAR_LIMITS` alias re-exports the ungated `pub(crate)` `family::SCALAR_LIMITS_UNLIMITED`. That is the same constant in every build, so the non-test build does not depend on the feature gate. Only the existing `test-support` re-export of the fixture name is gated. `WitnessValue::String` is routed in every match that routes `Timestamp`. The replay arm admits a String only into a String slot, and every other pairing falls to `WrongValueKind`. The added lines contain no non-ASCII bytes. The escapes `é` and `\ud800` are literal ASCII inside raw strings. Every witness vector is in JCS member order except the deliberate member-order row.

Mutation check of the Text admission tests (no red run):
- Length counted in bytes instead of the profile unit: caught by step 6 (nine U+0065 U+0301 pairs under `nfc` and three pairs under `unicode-scalars` give a different `actual`) and by step 2 (eight pairs would refuse).
- Length checked before type fit: caught by step 5 (`aaaaaaaaa` in the String slot must refuse `wrong-value-kind`).
- A String object admitted into a Text slot: caught by step 5's first assertion.
- `nfc` not applied: caught by step 2 (`e\u{301}` must retain `\u{e9}`, and eight pairs must admit).
- Profile ignored: caught by step 6's `unicode-scalars` and `binary-utf8` rows and by the profile name in `expected`.
- A Text object counted as two values: not caught here. Counting belongs to the document walk (TC-1038 step 8), which this PR cannot build.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Recorded. If the kernel's `admit_text` refuses after the local length check passes, the refusal is a bare `invalid-value` with no `member`. This cannot happen while the local length and the kernel length agree. | qsl-semantics/src/model/observation/document.rs:1365-1374 |
| FND-002 | low | Recorded. In `tc_905_string_and_text_entries_decode_exactly`, the two `assert_ne!` checks compare values that the test constructs, so they test the derived `PartialEq`. The decode rows carry the claim. | qsl-replay/src/witness/value_text/tests.rs:700-708 |
