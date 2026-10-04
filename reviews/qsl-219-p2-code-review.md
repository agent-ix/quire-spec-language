---
id: SR-1284
title: "Code review of PR #625: out-of-range number refuses noncanonical_wire from its lexeme (QSL-219 part 2)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@25610e30b8e443aa7b665d3880145b1b9c2d858a; PR #625 diff against origin/main: Cargo.lock, qsl-semantics/src/library/package_identity.rs, qsl-semantics/src/model/intake.rs, qsl-semantics/src/model/observation.rs"
review_set: subset
---
# Code review of PR #625

## Summary

Ticket: QSL-219 (part 2). PR: quire-spec-language#625. The PR bumps
quire-canonical to 5dc4e12d and maps `ReadError::NumberOutOfRange { offset,
pointer, lexeme }` to `noncanonical_wire` in intake (`read_refusal`) and in
observation admission (`digest_read_refusal`). The cause comes from
`out_of_range_number`, which reuses the #617 `Decimal` reader. The Rust lane
(rust-review) is folded into this file.

What the brief asked to check:

- **Exact classification, no float.** Confirmed. `out_of_range_number` calls
  only `Decimal::of(lexeme).is_whole_beyond_2_53()`. That is digit and
  exponent arithmetic in `i128`, with the exponent saturating at `u64::MAX`.
  No `f64` is involved. Edge lexemes, traced by hand:
  - `-0e999` and `0e400`: Rust's `f64` parse gives `-0.0` and `0.0`, both
    finite. The reader sends `NumberOutOfRange` only when `!value.is_finite()`
    (quire-canonical src/read.rs:1055-1057), so neither reaches this code.
    They go to the tree, where `inexactness` admits them: `same_value` treats
    every zero as equal. If one did arrive, `significant == 0` would return
    `Inexact::Number`, but that cannot happen.
  - `1.0e400`: trailing = 1, significant = 1, shift = 400 - 1 + 1 = 400, so
    whole beyond 2^53, which gives `inexact-integer`. Correct.
  - `10e-1` forms: an overflow with a negative exponent needs a mantissa of
    more than 309 digits. `1` + 400 zeros + `e-1` gives shift 399, so
    `inexact-integer`. `1` + 400 digits + `5e-1` gives shift -1, so
    `inexact-number`. Both are correct, but neither is tested (FND-002).
  - Huge negative exponent with a huge mantissa: an exponent that saturates
    the `u64` with a negative sign makes shift negative, because no mantissa
    has `u64::MAX` digits. The value is not whole, so it can never overflow
    anyway.
  - Very many digits: `Decimal::of` makes a constant number of linear passes
    (`take_while` over the leading digits, a reverse `take_while` over the
    trailing ones, one exponent fold) and allocates nothing. The digit-by-
    digit compare runs only when `significant + shift == 16`, so the
    `repeat_n` zeros are at most 16. On the reader's side, the lexeme copy
    (`owned`) and the pointer are allocated fallibly, once, on the error path
    only. Nothing is quadratic or unbounded.
- **I2 claim.** Confirmed. `qsl-package/src/checked_v2.rs` calls
  `quire_canonical::to_vec` only (line 600), never `read`. Every production
  `quire_canonical::read` call site:
  - `model/intake.rs:331`: changed here.
  - `model/observation.rs:548`: changed here.
  - `library/package_identity.rs:431`: an identity preimage. It now refuses
    as `PreimageDefect::ReaderRefused`, which becomes
    `LibraryRefusal::InvalidPreimage`. That is not a digest refusal. The
    `PackageIdMismatch` check before it hashes raw bytes and is unaffected.
  - `check/node_key/shape.rs:39`: reads bytes QSL itself encoded, so it can
    hold no out-of-range number.
  - `qsl-replay/src/result/wire.rs:108`: replay result wire, which never
    maps to `stale_dependency`.

  `src/package/reading.rs:380` checks `ByteDigest::of(bytes)` for the
  checked-package artifact before IR decodes it. That is a selection-identity
  check on raw bytes, not a read. No remaining path maps `NumberOutOfRange`
  to a digest refusal.
- **Unreachable fallbacks.** If the reader's pointer were not RFC 6901,
  intake would fall back to `malformed-declaration` and admission would fall
  back to a raw digest (`None`). The two fallbacks differ, but the reader
  never produces such a pointer, so this has no effect. Not a finding.
- **Test oracles.** `assert_noncanonical` compares the whole refusal against
  `noncanonical_number(..)`, a production helper, for the detail text. It
  then asserts the code, the catalog code and the cause with literal values,
  so the oracle for code, cause and pointer is independent. The observation
  test uses literal `AdmissionRecord`s under both the raw digest and a zero
  digest. `refuses_a_number_serde_json_cannot_represent_at_the_one_parse`
  checks the pointer only with `ends_with("/weight")`. That is weak, but the
  sibling test pins exact pointers.
- **Gate.** `make ci` log at ~/dev/worktrees/logs/qsl-219-p2-make-ci.log
  (written after the head commit) shows all three new tests passing on every
  test lane. I ran no extra build. The worktree has no `target/`.

## Verdict

Request changes. One high finding in the spec (SR-1286 FND-001) is mirrored
here as FND-001 because two code doc comments repeat the "first in document
order" contract that the code breaks. FND-002 is a real test gap: the
`inexact-number` arm of the new function is never exercised. The
classification logic itself is exact and correct.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The doc comments still say the refusal names "the first such number in document order" / "the first such number". But `{"x":[1e-400,1e400]}` names `/x/1` (the PR's own test asserts this), because the reader refuses before the tree walk runs. The rustdoc states a contract the code does not keep. Fix: carve out the out-of-range case in both comments, in step with the FR-056 fix (SR-1286 FND-001). | qsl-semantics/src/model/intake.rs:260-262, qsl-semantics/src/model/observation.rs:539-545 |
| FND-002 | medium | No test reaches `out_of_range_number`'s `Inexact::Number` arm. Every `inexact-number` case in the new tests is `1e-400`/`-1e-400`, which the reader accepts as 0.0, so those cases go through the #617 tree path, never through `read_refusal`/`digest_read_refusal`. Every overflow lexeme tested is whole. The ruling's "a non-whole one is inexact-number" is unproven through the new code, and so is a negative-exponent overflow. Fix: in `refuses_a_number_with_no_finite_double_by_its_lexeme` and the observation test, add a non-whole overflow (`format!("1{}.5", "0".repeat(400))`, which should give `inexact-number`) and a negative-exponent whole overflow (`format!("1{}e-1", "0".repeat(400))`, which should give `inexact-integer`). | qsl-semantics/src/model/intake.rs:438-446, qsl-semantics/src/model/intake.rs:5737-5766, qsl-semantics/src/model/observation.rs:671-691 |
