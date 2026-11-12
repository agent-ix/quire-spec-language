---
id: SR-2026111205102300000001
title: "code-review and rust-review of IR-1493 Text size lowering and evaluation (quire-spec-language PR 2687)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; qsl-semantics/src/check/check/typing.rs; qsl-semantics/src/check/claims.rs; qsl-semantics/src/check/lowering.rs; qsl-eval/src/value/expression/evaluate.rs; qsl-eval/tests/it/text_size.rs; tests/it/text_size_v2.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-1409
    type: references
---

# Code review of IR-1493 Text size (quire-spec-language PR 2687)

## Summary

Ticket: IR-1493. Head e53f04012, diff `origin/main...HEAD` (one commit, 8 files). Rust lane
(rust-review) folded into this file. Gates run via locked-build.sh with a dedicated target dir:
`cargo fmt --check` and `cargo clippy -p qsl-semantics -p qsl-eval --all-targets --all-features
-D warnings` clean; `cargo test -p qsl-eval --test it text_size` 4 passed; root `--test it
text_size` 1 passed; `cargo test -p qsl-semantics --lib -- claims size` 23 passed.

claims.rs: before this PR `NodeKind::Size(_)` was in the `false` arm, so collection Size was
never a claim site; after it, only a Size whose operand is `ValueType::Text(_)` returns true,
and every collection Size still returns false, so no collection Size claim is added or removed.
The change is required, not optional: `key_claims` classifies lowered nodes by identity prefix,
`quire.op.text.size` matches `quire.op.text.` in `SCALAR_FAMILIES`, and the final loop of
`key_claims` returns `unkeyable` for any body-origin scalar application with no claim record.
Without the claims.rs arm, every Text size body would fault at keying. The predicate matches
the lowering guard (`text_profile(operand type).is_some()` is `ValueType::Text(_)`), so
`claims_of` and `key_claims` agree on every site. The site bound is `Own(Int[min, max])`,
the typed result.

evaluate.rs: the new `size_of` matches every `Value` variant without a wildcard; Text returns
`Text::length()`, which in quire-exact counts scalars of the retained (normalized) sequence for
`nfc` and bytes only for `binary_utf8`, matching QSpec FR-141's profile length.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The Text Size lowering arm computes `text_profile(operand type)` twice (once in the match guard, once in the body) and maps the unreachable `None` to `KeyFault::UnbuiltLiteral`, a fault reason unrelated to a Size operand. One `NodeKind::Size(operand)` arm matching on `text_profile(..)` (Some: text.size call, None: collection.size) removes both the duplicate computation and the dead fault path. | qsl-semantics/src/check/lowering.rs:5291-5293 |
| FND-002 | low | The definedness interval walk gives `size(t)` over a `Text[min, max]` operand `Interval::unbounded()` (only `ValueType::Collection` gets `cardinality`), although the checker now types the node `Int[min, max]`. Sound, but a guard or arithmetic over `size(t)` cannot use the text bounds the type already carries. | qsl-semantics/src/check/facts.rs:1654-1660 |

## Verdict

No high or medium code defects. The claims.rs change is correct and necessary, and it does not
regress collection Size (still false, unchanged lowering to `quire.op.collection.size` with a
member). The typing, lowering and evaluation arms implement FR-093-AC-63. The two low findings
are idiom and precision nits. The FR-090 invariant-identifier gap is recorded in the
gap-analysis artifact for this ticket.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Round 1, b5064481f: lowering.rs now has one `NodeKind::Size(operand) => match text_profile(..)` arm (Some: `quire.op.text.size` call; None: `quire.op.collection.size` with member); the duplicate computation and the `KeyFault::UnbuiltLiteral` path are gone. text_size_v2 green; clippy -D warnings clean. |
| FND-002 | fixed | Round 1, b5064481f: facts.rs gives a `ValueType::Text(text)` Size operand the interval `[text.min(), text.max()]`. New test `size_over_text_carries_its_length_bounds_into_arithmetic` accepts `size(t) - 1` as `Int[0, 7]` and refuses `Int[0, 6]` for `Text[1, 8]`; reverting only the facts.rs hunk locally makes that test fail, so it pins the fix. |
