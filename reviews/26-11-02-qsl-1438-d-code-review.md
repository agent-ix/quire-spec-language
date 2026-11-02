---
id: SR-202611020537
title: "Code review — QSL-1438 direct RFC 8785 key writer and dev opt-level"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; Cargo.toml; qsl-eval/src/simulation/model/encode.rs; qsl-eval/src/simulation/key.rs; qsl-eval/src/simulation/model.rs; qsl-eval/src/simulation/model/application.rs; qsl-eval/src/simulation/protocol.rs; qsl-eval/src/simulation/protocol/memory.rs; qsl-eval/src/simulation/protocol/ra.rs; qsl-eval/src/simulation/protocol/tso.rs; qsl-analyze/tests/it/rc11_oracle.rs"
review_set: subset
---

## Summary

Ticket: QSL-1438. Code review with the Rust lane (rust-review and rust-style) of PR #2069 (branch `qsl-1438-D`, `git diff origin/main...HEAD`, three commits). The PR adds a second RFC 8785 encoder, `Canonicalizer`. It writes `Enc` trees straight to bytes, with `Enc::Map`, `Enc::Declaration` and `Enc::Shared` variants. It caches node, effective and declaration identity bytes per system behind `RwLock`s. It adds an overridable `MemoryModel::key_bytes` for RA and TSO. Shared RA location and message values are encoded once. The memo-miss walk keeps its synthesized observation. The dev profile builds dependencies and five workspace crates at `opt-level = 1`.

## Verdict

**PASS with medium findings.** For every value that encodes, the bytes match `quire-canonical` at rev 35987f1. Each claim was traced by hand against `writer.rs`, `escape.rs` and `order.rs`:

- **String fast path.** Raw bytes are written only when no byte is `"`, `\` or below 0x20. That is exactly the set `escape_fragment` escapes. 0x7F and non-ASCII text are literal in both encoders. Every other string goes through `quire_canonical::to_vec`.
- **Member order.** Names are sorted by UTF-16 code units (`encode_utf16`). This matches `cmp_member_names`. A held order that is already strictly ascending cannot contain a duplicate, so the duplicate check on the sorted path alone is complete. The error is the same `DuplicateMemberName{name}`. A nested object is reordered inside its own span before its parent sorts, so the parent's spans hold canonical bytes.
- **Map.** The literals `{"entries":[`, `{"key":`, `,"value":` and `],"type":"map"}` are the canonical member order (`entries` < `type`, `key` < `value`). The empty map gives `{"entries":[],"type":"map"}`. Keys are sorted by key bytes with a stable sort, as in `sorted_map_enc`. A key that does not encode refuses with the same error as before: the old path sorted it first as empty bytes, and the new path refuses it in held order.
- **Identities and `Shared`.** These are encoded by `quire_canonical::to_vec` of the same leaf, so the bytes cannot differ. An `Enc::Shared` with no bytes writes its value, so it refuses as the value does. Depth and recursion are unchanged: one level per declared nesting level, as in the `Encode` impl. The writer has no depth limit.
- **Byte bound.** `Out::push` refuses `required > bound` with `LimitKind::CanonicalBytes`. Both encoders count the same total, so either both refuse or both pass.
- **Memo and budget are unchanged.** `walk_observed` rebuilds `rows_of` and reuses only the observation, a pure function of the state. `walk_of` charges no budget. `before = budget.used()` is still taken before the walk. `keep_observation` keeps the observation even when the apply fails, which is sound because the observation does not depend on the outcome. The memo identity bytes come from `canonical().to_vec`. They are equal to the old bytes when the key encodes, and `None` when it does not.
- **Locks.** Each `read()` guard is a temporary that is dropped at the end of its statement. The leaf is encoded with no lock held, and through `quire_canonical`, not the `Canonicalizer`, so there is no re-entry and no nested lock. A racing miss inserts identical bytes. Poison recovery follows the repository's `into_inner` convention. `Canonicalizer` is `Sync`. Each cache grows with the number of distinct identities, which the model bounds.
- **Cargo profile.** `[profile.dev.package."*"]` covers non-member packages only. A named override takes precedence over `"*"`, so the existing opt-level 3 entries still apply, as the comment says. The five member overrides are valid syntax. `debug-assertions` and `overflow-checks` stay on under the dev profile, so test semantics do not change.
- **No ceremony.** The diff adds no `#[allow]`, no caps, no pins or digests and no compatibility layer. `expect` appears only in the test.

The defects are in the refusal paths, which the PR's own docs promise to match but nothing tests, and in the case for having a second encoder at all.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Canonicalizer refusals diverge from the writer near the byte bound: commas are counted early, map keys are pre-written under a fresh bound, and `memory` is encoded before the other members. This changes `LimitExceeded.required` and can turn a NoEncoding or DuplicateMemberName refusal into a Limit refusal, contrary to the "refusals included, in the same order" doc | qsl-eval/src/simulation/model/encode.rs:136-138; qsl-eval/src/simulation/model/encode.rs:333-381; qsl-eval/src/simulation/model/encode.rs:384-391; qsl-eval/src/simulation/protocol.rs:1806-1818; qsl-eval/src/simulation/key.rs:127 |
| FND-002 | medium | The equivalence test checks only successful ProtocolSystem keys at LT-12 under ra and tso. No test compares Canonicalizer with `quire_canonical::to_vec` for refusals, escaped or non-ASCII strings, duplicate names, unencodable map keys or values, `Shared` with no bytes, `object_with` at a bound, or the `sc` memory default | qsl-analyze/tests/it/rc11_oracle.rs:507-544; qsl-eval/src/simulation/model/encode.rs:1438 |
| FND-003 | medium | The second canonical encoder is justified by quire-canonical's writer monomorphizing at O0 downstream, but the same PR builds qsl-eval at opt-level 1, and release builds are optimized anyway. Nothing measures the profile change alone against the profile change plus the Canonicalizer, so the main risk (two encoders) is taken without evidence that it is needed | qsl-eval/src/simulation/model/encode.rs:131-149; Cargo.toml:199-215 |
| FND-004 | low | The RA and TSO `key_bytes`, ra `canonical()`, `keyed_name` and `Enc::shared` each build a fresh `Canonicalizer::default()`. The identity caches are dropped after every call, so the memory member, the bulk of a protocol key, gets no per-system reuse, contrary to "encoded once per system" | qsl-eval/src/simulation/protocol/ra.rs:277-281; qsl-eval/src/simulation/protocol/ra.rs:1611-1613; qsl-eval/src/simulation/protocol/tso.rs:576-578; qsl-eval/src/simulation/model/encode.rs:740-747 |
| FND-005 | low | The protocol key envelope is now built in two places, `ProtocolKey::encode_into` and `ProtocolSystem::key_bytes`. On the public `MemoryModel` trait, `key_bytes` can be overridden to return bytes that differ from `Encode`, an invariant nothing enforces | qsl-eval/src/simulation/protocol.rs:585-595; qsl-eval/src/simulation/protocol.rs:1806-1818; qsl-eval/src/simulation/protocol/memory.rs:250-258 |
| FND-006 | low | The new test has no `#[trace]` tag. Every other test in the file is tagged, and the test asserts FR-101-AC-2 (the state key is the JCS encoding) | qsl-analyze/tests/it/rc11_oracle.rs:507-511 |
| FND-007 | low | The opt-level 1 overrides on the workspace members (qsl-foundation, qsl-semantics, qsl-package, qsl-eval, qsl-replay) slow every edit-rebuild of the five core crates and hurt debugger stepping. The PR gives no measurement for that half, separate from the `"*"` dependency half. The comment also embeds a stale-prone count ("LT-16 expands 7,607 states") | Cargo.toml:199-215 |

## Review detail

FND-001: Take `Object([("b", Str("x")), ("a", Str("y"))])` with bound 15. The writer counts `{"b":"x"` (8) and then `"a":"y"` (15). It charges the comma and the brace at `end_object`, so it refuses with `required = 17`. The Canonicalizer pushes `,` immediately, so it refuses at `"y"` with `required = 16`. Now let member `b` be `Unencodable` with the counted prefix at the bound. The writer reaches `b` and reports NoEncoding. The Canonicalizer has already pushed the comma, so it reports Limit. That is a different `EncodingRefusal` variant, and a different stop cause. In `map`, each key is written under `Out::new(out.bound)`. A key larger than the bound refuses before any value, and reports the key's own length as `required`. In `ProtocolSystem::key_bytes`, `memory` is encoded first, so a memory refusal wins over a refusal in an earlier-held member, which `ProtocolKey::encode_into` would report. Each divergence appears only near the identity bound or when two refusals coexist, so the impact is low. The defect is that the module docs (encode.rs:136, key.rs:127) promise the same refusals in the same order. Either count separators the way the writer does (charge `n` at close), or narrow the doc to "the same bytes, and a refusal whenever the writer refuses". Add the tests from FND-002 either way.

FND-002: A differential unit test in encode.rs should compare `Canonicalizer::to_vec(e, L)` with `quire_canonical::to_vec(e, L)`, for `Ok` and `Err`, over hand-built trees. Cover strings with `"`, `\`, `\n` and `\u001f`, non-ASCII text, and supplementary-plane text. Cover held member orders that are reversed or duplicated, maps with an unencodable key and with an unencodable value, `Shared` of an `Unencodable`, `Declaration`, `Node` and `Effective`, and small bounds. Add an `sc` case to the rc11 test. Member names are `&'static str` and all ASCII in practice, so the UTF-16 path is correct but never exercised.

FND-003: The PR body states the motive: "quire-canonical's generic writer, which monomorphizes at O0 downstream". After commit 81a257a33, qsl-eval itself is built at opt-level 1, so the monomorphized writer is optimized too. Release builds never had the problem. Some of the gain is independent of the optimization level: `Shared` and the identity byte caches inject precomputed bytes, which the event writer cannot. The trade still needs a number. Measure LT-16 with (a) the profile alone and (b) the profile plus the Canonicalizer. Keep the second encoder only if (b) is materially faster, and record that number on QSL-1438.

FND-005: `ProtocolKey::encode_into` writes `members` and then `memory`. `key_bytes` passes the same parts to `object_with`. A member later added to only one side would drift. The rc11 test would catch that for ra and tso only. One way to fix it is a `ProtocolKey::bytes(&Canonicalizer, Limits)` that owns the layout. The trait hook could be sealed or kept `pub(crate)` rather than left publicly overridable.

FND-007: The dependency half (`"*"`) costs one rebuild and follows the existing precedent (`quire-canonical` at opt-level 3), so it is a real feature. The member half differs: it costs every iteration on the most-edited crates.

## Coverage

- Code review and Rust review (rust-style baseline) ran together in this artifact.
- Independently run, under `locked-build.sh` with `CARGO_PROFILE_DEV_DEBUG=0`:
  - `cargo fmt --check -p qsl-eval -p qsl-analyze` is clean.
  - `cargo test -p qsl-analyze --test it -- rc11_oracle`: 8 passed, including `the_written_state_key_is_the_encoded_key_envelope`.
  - `cargo clippy -p qsl-eval --all-targets --all-features -- -D warnings` fails only on the known `clippy::type_complexity` at qsl-eval/src/simulation/protocol/steps.rs:126, which is pre-existing on main.
- Not run, per the brief: the full gates.
- Existing tests that cover the new path for successful encodes: TC-471 (FR-120-AC-4) pins a model state-key envelope whose text carries escaped quotes. `rc11_oracle` covers ra and tso protocol keys.

## Findings data

```yaml
clean: false
findings: [FND-001, FND-002, FND-003, FND-004, FND-005, FND-006, FND-007]
```
