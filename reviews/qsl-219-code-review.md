---
id: SR-1267
title: "Code review of quire-spec-language PR #617: intake refuses a number with no exact RFC 8785 spelling (QSL-219)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@91a1607feb9a65dfa70e01dd66b53c6baa24ea26; PR #617 diff 9e035e7fa...91a1607f: qsl-semantics/src/model/intake.rs, qsl-semantics/src/model/intake/unit.rs, qsl-semantics/src/model/observation.rs, qsl-semantics/src/model/refusal.rs, qsl-foundation/src/diagnostic.rs, qsl-package/src/checked_v2.rs, qsl-package/src/checked_v2/tests.rs, qsl-replay/src/request.rs, qsl-semantics/tests/it/identity_golden_vectors.rs, qsl-semantics/tests/it/state_clauses.rs, tests/it/native_boundaries.rs, docs/native-error-codes.md; context: quire-canonical 59fe4f06 src/number.rs and src/read.rs, spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md section 2"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: reviews
---
# Code review of quire-spec-language PR #617

## Summary

Ticket: QSL-219. PR: quire-spec-language#617, head 91a1607f, one commit on
main 9e035e7fa. The Rust lane (rust-review) is folded into this file. Spec
traceability is in SR-1268.

The rule is built the right way: the integer cause is decided on the text, the
pointer and document order are right, admission refuses before check 3, and the
cause reaches replay and observation admission. One defect matters.
`inexactness` gets "the text RFC 8785 writes" from Rust's `{:e}`, not from
quire-canonical. Rust's formatter and RFC 8785's differ on ties, so intake
refuses some numbers that are already in RFC 8785's own spelling, and admits
other numbers whose digest spells a different value (FND-001).

**The exact decimal comparison is sound.** I checked `Decimal::of`,
`is_whole_beyond_2_53` and `same_value` against each edge in the brief:

- `total - leading - trailing` cannot underflow. `trailing` is counted only
  when some digit is non-zero, so `leading + trailing < total`.
- Zero and negative zero both give `significant == 0`, and `same_value` treats
  them as equal whatever the sign or exponent. `-0`, `-0.0` and `-0.0e-999`
  admit, and `1e-400` (which underflows to zero) refuses.
- Exponent saturation cannot change a result. A saturated positive exponent
  only shows up on a text the reader has already refused as out of range, or
  on zero digits. A saturated negative exponent leaves `shift` hugely
  negative, so the number is not whole, and its double is zero, so
  `same_value` is false. The other side of `same_value` is always formatter
  output, whose exponent is within ±324.
- Leading zeros, long fractions and long exponents are handled. The Equal
  branch of `is_whole_beyond_2_53` compares two 16-digit strings, so
  lexicographic order is numeric order. `repeat_n` is bounded because
  `shift <= 15` there, and `Iterator::cmp` stops at 16 elements anyway.
- Every integer within ±2^53 has a shortest text equal to its exact value:
  any shorter candidate is an integer at least 1 away, and the rounding
  interval there is at most ±1. So no whole number within ±2^53 can refuse
  `inexact-number`, as FR-056 requires.

**Shortest round-trip text (FND-001).** Rust's `{:e}` and ECMAScript both pick
the closest of the shortest candidates, and both include the interval
endpoints when the mantissa is even. They differ only on an exact tie.
`core::num::flt2dec::strategy::dragon::format_shortest` rounds a tie up ("tie
breaking prefers rounding up"). ECMAScript `Number::toString` picks the even
digit, and so does ryu-js, which quire-canonical's `number.rs` uses for the
digest. I measured this with a scratch probe (ryu-js 1.0.3 and Rust 1.98.1,
the locked versions), not in the repo:

- The double `0x4310000000000001` (1125899906842624.25): ryu-js writes
  `1125899906842624.2`, and Rust writes `1.1258999068426243e15`.
- With the PR's `Decimal` copied into the probe, `1125899906842624.2`,
  `-1125899906842624.2` and `1500000000000000.2` refuse `inexact-number`,
  although each is exactly the RFC 8785 text of its double.
  `1125899906842624.3` admits, although RFC 8785 writes its double as `.2`.
- A sample of 20M random doubles, plus 4000 doubles in each binade, finds
  such ties in every binade from 2^31 to 2^50 (3506 in 2^50 alone) and at
  2^-25. There, `2.9802322387695312e-8` refuses and `2.9802322387695313e-8`
  admits, while RFC 8785 writes `...312e-8`.

**Heap-stack walk.** It does not recurse. The stack holds one entry per open
container, so its memory is bounded by the input. Intake caps depth with
`too_deep` before the read. Observation admission caps it at check 1.2, and
when serde_json cannot parse the document there, `document_bytes` still bounds
it. The pointer is built only on refusal. Allocation happens per number, not
per container: `format!("{:e}")` builds a `String` for every number (see
FND-001).

**Float re-parse fix: the oracle is strong.** serde_json 1.0.151 is built here
without `float_roundtrip` or `arbitrary_precision` (`cargo tree -e features -i
serde_json`). Its default parser reads `1.5e-300` as `1.4999999999999998e-300`
(bits `...682`), while `str::parse::<f64>` and quire-canonical give `1.5e-300`
(bits `...683`). Without the fix, the tree would hold serde_json's double, so
`a_non_whole_number_in_the_tree_is_the_digests_double` fails on its first
assertion for `1.5e-300`, and on the digest assertion too. The other three
inputs (`0.1`, `2.5`, `-3e-7`) parse the same both ways. They add regression
cover but do not tell the two apart. The `parsed.is_f64()` split keeps
integers exact, and `from_f64` cannot fail on a finite double.

**Wiring.**

- `admit` returns `NoncanonicalNumber` with the limit and allocation causes,
  after check 2 and before check 3. `package_input` keys a refused document by
  its raw digest, so a selection of it reaches that refusal.
- `package_document_refusal` gives the cause its own arm and keeps the cause
  and pointer. The `_` arm that was already there now catches only the
  malformed-read cause.
- `check_document_digest` refuses before the digest comparison, over the whole
  document. The four FR-106-AC-11 tests fail if that check is removed, and the
  exact replacements prove the refusal is about the number.
- The I2 reader maps IR's `NoncanonicalWire` with an exact arm. The `Code`,
  `as_str`, `all()` and `CATALOG_CATEGORIES` rows agree.
  `catalog_fields` stays exhaustive under `deny(wildcard_enum_match_arm)`.
- `Rfc8785Numbers` is already gone: #613 removed it, and nothing in the tree
  names it.

**Deleting `seen.len() == 47` is right.** The loop already checks every code
in `Code::all()` for uniqueness, round-trip and category. The count could not
catch a variant missing from `Code::all()`, because the count stays the same,
and it failed on every correct addition. Its comment also said
`NoncanonicalWire` collapses onto `InvalidPackage`, which this PR makes false.
It was a hand-bumped count with no behaviour behind it.

**Rust lane.** There is no new `unwrap`, `expect`, `panic!`, indexing or
`unsafe` on a production path. `count` and the `shift` conversion use
`try_from` with fallbacks that cannot fire. No catch-all arm was added over a
closed enum. `Decimal::of` carries `#[qsl_attrs::string_edge]`. The new tests
carry `#[trace]` ids that resolve (see SR-1268 for one binding that is wrong).
No new limit, pin, compatibility layer or ceremony. One nit, not a finding:
`inexactness` is `pub(super)` but only `first_inexact_number` calls it.

**Gate.** The coder's `make ci` passed at this head
(`~/dev/worktrees/logs/qsl-219-make-ci.log`, exit=0): fmt, clippy
`-D warnings` with default and all features, the workspace tests including
every new test, the clean build, xtask string-edge, and arch-lint
canonical-encoder. I did not re-run it. arch-lint canonical-encoder passes
because FND-001 builds no RFC 8785 document. It only spells one number.

**Not in scope, noted once.** Observation admission still builds its tree with
`serde_json::from_slice` (`observation/document.rs`), the same float re-parse
this PR fixed in intake. This diff does not touch it, so it is not reviewed
here.

## Verdict

**FAIL**: one high finding. FND-001 is a second RFC 8785 number formatter
beside quire-canonical, against ADR-013 section 2, and it gives wrong results
on ties, against FR-056's rule. Fix it with tie tests, fix the two low
findings, and the PR is mergeable.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `inexactness` gets "the text RFC 8785 writes" from Rust's `{:e}`, which is a second number formatter beside quire-canonical (ADR-013 section 2 allows one RFC 8785 implementation). The two differ when a double has two equally close shortest spellings: Rust rounds the last digit up, while ECMAScript and quire-canonical's ryu-js encoder pick the even digit. Measured: `1500000000000000.2`, `1125899906842624.2` and `2.9802322387695312e-8` are each the RFC 8785 text of their double, and each refuses `inexact-number`. `1125899906842624.3` and `1500000000000000.3` admit, while the digest spells `.2`. Sampling finds such doubles in every binade from 2^31 to 2^50. Fix: spell the double through quire-canonical's own encoder (its public `Writer::number`), and add these tie cases as tests. That also removes the per-number `String` allocation, which the test doc at intake.rs:6077 says does not happen. | qsl-semantics/src/model/intake.rs:587-589 |
| FND-002 | low | The `PackageDocument` doc says the derived view "converts it with `serde_json`'s own number parser, so it equals what `serde_json::from_slice` would have read". Since this PR's `value_of` change, a non-whole number takes quire-canonical's double, which is not serde_json's for `1.5e-300`: serde_json reads `1.4999999999999998e-300`. The doc on `the_one_parse_reads_what_serde_json_reads` ("for numbers at every edge") says the same. Reword both: integers keep serde_json's exact read, and non-whole numbers take the digest's correctly rounded double. | qsl-semantics/src/model/intake.rs:268-271, qsl-semantics/src/model/intake.rs:5789-5791 |
| FND-003 | low | `big_integers_admit_under_neither_their_double_nor_their_digits` is tagged FR-056-AC-13 but only asserts `!admit_under(..)`, that is, `admit(..).is_ok()` is false. Any refusal passes, including `stale_dependency`/`byte-digest-mismatch`, which AC-13 says must never happen. Its exact-digits half passed before this rule too. Assert the refusal itself: `noncanonical_wire`/`inexact-integer` at `/n`. | qsl-semantics/tests/it/identity_golden_vectors.rs:385-391 |

## New findings (disposition pass 1)

Found at 7d2d36551295eb197932458c24407c464afbc320 (fix commit 7d2d36551 on
7fe95cce9, which is 91a1607f rebased unchanged onto main a7017bea2; `git
range-diff` shows `=`).

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | low | The fix round added `OrderedJson::take_digest_doubles`, a `tree` parameter on `check_document_digest`, FR-106-AC-12, TC-465 row 46 and the test `a_non_whole_number_takes_the_digests_double`. Together they correct floats in a tree that observation admission never reads a number from. `OrderedJson` has no number accessor (only `as_object`, `as_array`, `as_str`, `as_bool`). Version numbers and `integer` payloads are strings, `read_raw_value` has no float form, and no `OrderedJson` or `Value` outlives `read_document`. So no admission result and no admitted value can change. The test calls the helper directly and never `read_document`, so it stays green with the wiring removed. It would fail only on a no-op helper (serde_json reads `1.5e-300` one unit off). Nothing breaks without this code, so delete the helper, the `tree` parameter, FR-106-AC-12, TC-465 row 46 and the test. My review-pass note that observation admission has "the same float re-parse" as intake was wrong on this point. Intake's tree is public API (`PackageDocument::tree`), but observation's is private and no number is read from it. | qsl-semantics/src/model/observation/ordered_json.rs:53-72, qsl-semantics/src/model/observation/ordered_json.rs:296, qsl-semantics/src/model/observation.rs:555, qsl-semantics/src/model/observation.rs:568, qsl-semantics/src/model/observation/document.rs:232, spec/functional/FR-106-admit-snapshots-and-invocations.md:350, spec/test-cases/TC-465-admission-refuses-each-input-defect.md:73 |
| FND-005 | low | `cargo fmt --all -- --check` fails at this head. `Spelled`'s `write_bytes` is not formatted, and in `identity_golden_vectors.rs` the new `normalize` import is out of order and the `assert_eq!` in `assert_inexact_integer_at_n` overruns the line width. `make ci` runs fmt first, so the pre-merge gate stops there. Run `cargo fmt --all`. The fix-round logs show tests only, with no fmt or clippy run. | qsl-semantics/src/model/intake.rs:617-624, qsl-semantics/tests/it/identity_golden_vectors.rs:30, qsl-semantics/tests/it/identity_golden_vectors.rs:309 |

## Dispositions

Round 1, reviewed at 7d2d36551295eb197932458c24407c464afbc320.

- **FND-001.** `inexactness` writes the double through
  `quire_canonical::Writer::number` into `Spelled`, a 32-byte stack sink.
  `Writer::new` allocates nothing (empty `Vec`s), and a top-level scalar goes
  straight to the sink, so the per-number allocation is gone and the "allocates
  nothing" doc is now true. `{:e}` appears nowhere in `intake.rs`.
  `a_tie_between_two_shortest_texts_admits_only_the_even_digit` covers the
  three even-digit spellings (admitted, with the tree's double and the digest
  checked) and the three odd ones (refused `inexact-number`). The coder's red
  log (`qsl-219-red.log`) shows it failing on the old code at
  `1500000000000000.2`.
- **FND-002.** Both docs are reworded and now match `value_of`.
- **FND-003.** Both halves now assert `noncanonical_wire`, `inexact-integer`
  and `/n` through `assert_inexact_integer_at_n`.
- **Tests run by the coder:** `qsl-219-fix2.log` and
  `qsl-219-fix2-rebased.log` show the touched crates green, including every
  new test.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 7d2d36551 |
| FND-002 | fixed | 7d2d36551 |
| FND-003 | fixed | 7d2d36551 |
| FND-004 | still-open | Found this round. The observation float correction has no reader. Delete `take_digest_doubles`, the `tree` parameter, FR-106-AC-12, TC-465 row 46 and the test. |
| FND-005 | still-open | Found this round. `cargo fmt --all -- --check` fails, so `make ci` stops at its first step. |

Round 2, reviewed at 946367d92d1c42cdfbe14fc94053547c1aa25d1f (fix commit 946367d92 on cbe927948, rebased onto main
6f314877f; `git range-diff` shows the first two commits unchanged).

- **FND-004.** The observation float correction is deleted completely:
  `take_digest_doubles` and its test (`ordered_json.rs`, 44 lines, nothing
  added), the `tree` parameter and the `OrderedJson` import in
  `observation.rs`, `parsed.as_mut()` in `document.rs`, FR-106-AC-12,
  TC-465 row 46 and TC-465's AC entry in `spec/tests.md`. Nothing in the
  tree names any of them now.
- **FND-005.** `cargo fmt --all -- --check` passes. The fmt changes in that
  commit are layout only.
- **Tests run by the coder:** `qsl-219-fix3.log` shows the focused tests and
  a clippy check pass (exit=0).

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-004 | fixed | 946367d92 |
| FND-005 | fixed | 946367d92 |
