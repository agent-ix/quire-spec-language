---
id: SR-1380
title: "Code review of quire-spec-language PR #660: composite witness values encode once (QSL-647)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@2219a6a2b88b5a93156d70bb2af10377da2074a0; PR #660 diff against origin/main: qsl-replay/src/witness/value_text.rs, qsl-replay/src/witness/value_text/tests.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-070
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-263
    type: reviews
---
# Code review of quire-spec-language PR #660

## Summary

Ticket: QSL-647. Code review with the rust-review lane folded in.

The encoder now builds each value bottom-up: `encode_shell` writes a value's own JCS with each child as a bare-number placeholder, and `splice` replaces each placeholder with the child's finished bytes. Set and bag elements are sorted by those finished bytes. The decoder drops the per-collection `to_vec` re-encode and instead runs `check_collections`, one explicit-stack scan of the verified canonical bytes that checks each `set`, `bag` and `ordered-set` by the byte spans of its elements.

Checked and sound:
- Span check soundness. `from_value_text` compares `to_vec(read(bytes))` with `bytes` and refuses `NotJcs` before `build` and `check_collections` run, so only canonical JCS reaches the scan. In canonical JCS an element's own encoding is exactly its span of the parent text, so span comparison equals the old `to_vec` comparison. Only tag members are named `type` (record field names are carried as `"name"` values, never as keys), and `build` has already accepted the whole shape, so the scan cannot mistake a non-collection for a collection or miss one.
- Byte identity. A scratch probe at the head compared `encode_bottom_up` with the old single-writer encoder (main's `sort_orders` + `encode_into`, re-hosted on the PR's `value_steps`) over 20,000 seeded random values of depth up to 5 covering every `WitnessValue` variant, all three slot states, and text with quotes, backslashes, digits, control characters, `%;<>`, non-BMP and private-use characters, plus a 1,234-element sequence and set (multi-digit placeholders). All 20,002 encodings were byte-identical.
- Placeholder safety. Every scalar a form writes is a string, boolean or null, so a bare number in a shell is always a placeholder; `splice` skips string contents including escapes.
- Recursion. Encode uses a pre-order `Vec` and a loop; `splice`, `check_collections` and `string_end` are loops over an explicit stack. No native recursion grows with depth. The 512 KiB-stack tests pass.
- Fault order. A shape fault is now reported before an element-order fault. FR-070's Decode list gives no precedence among faults and `DecodeRefusal::Malformed` carries one; FR-263, FR-358 and TC-736 set none for decode. Allowed.

Gates run at the head (release, through locked-build.sh): `cargo test -p qsl-replay --lib value_text` passes, 11 tests. Timings below.

| Test (release) | base dac0d90fc | head 2219a6a2b |
| --- | --- | --- |
| witness::value_text::tests::tc_736_a_100000_long_list_decodes_and_walks_on_a_small_stack | 1.17 s | 207.09 s |
| execute::tests::composite::tc_736_a_100000_long_list_replays_under_the_raised_input_bound | 1.83 s | 393.70 s |
| execute::tests::composite::tc_736_a_100000_term_source_and_list_replay_to_the_proving_verdict | 46.36 s | 333.72 s |
| witness::value_text::tests::tc_736_nested_sets_encode_each_value_once | n/a | 3.42 s |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `encode_bottom_up` materialises every value's full bytes and `splice` copies each child's bytes into a new parent buffer, so the bytes copied total the sum of every subtree's size: O(depth x output) for any deep nesting. This applies to every composite (record, option, sequence, tuple), not only sets, where main streamed them through one `Writer` in linear work. The FR-263-AC-1/FR-070-AC-10 100,000-long recursive list now takes 207 s to 394 s per test in release, against 1.2 s to 1.8 s on main (table above). A scratch probe timed `to_value_text` over nested `Option`s, release: depth 5,000 in 11.5 ms, 10,000 in 70.8 ms, 20,000 in 299.4 ms, about 4x per doubling, so quadratic. It also makes `value_text_len` (FND-003) and every replay of a deep value quadratic. The ticket wanted linear work; this moves the depth multiplier from sets to all composites. One fix: keep main's single streaming `Writer` pass for everything and compute each set/bag element's bytes once only where a set or bag needs to sort them, or build a rope of shell segments and child ids and write the output once at the end. | qsl-replay/src/witness/value_text.rs:161-200, qsl-replay/src/witness/value_text.rs:207-256 |
| FND-002 | medium | The `encoded <= 2 * bytes.len()` oracle cannot fail for this encoder. `encoded` sums shell bytes only, and each shell is its value's output with each child replaced by a shorter placeholder, so the sum of shells is below the output length by construction. It counts neither the splice copies nor the sort comparisons, which is where the depth-multiplied work now is (FND-001), and the 20,000-deep set test passes while the 100,000-deep list takes 200x longer than main. A real oracle counts every byte the encoder writes or copies (shell plus splice), or compares that count at depth d and 2d and requires a ratio near 2, not 4, over sets and over non-set composites both. | qsl-replay/src/witness/value_text/tests.rs:476-493 |
| FND-003 | low | `value_text_len` now builds the whole encoding to measure it (`encode_bottom_up` then a count), where main counted through the `EscapedLength` sink without building it. It feeds the `replay.input_bytes` measure (witness.rs:412) and the composite claim size (composite.rs:221), so each measurement allocates the full output and, with FND-001, pays the quadratic copy before the bound can refuse. Measure with a counting sink again once FND-001 is fixed, or reuse the bytes when the caller encodes next. | qsl-replay/src/witness/value_text.rs:475-483 |

## Verdict

One high, one medium and one low finding. The decode side (span-based order check) is correct and is the part of the ticket that is done. The encode side replaces depth-multiplied encoding for nested sets with depth-multiplied copying for every nested composite, a 100x to 200x regression on the TC-736 tests. Not mergeable until FND-001 and FND-002 are fixed; FND-003 should be fixed with them.
