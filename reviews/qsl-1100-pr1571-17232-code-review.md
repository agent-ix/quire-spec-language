---
id: SR-17232
title: "review of QSL-1100 main reds fix (quire-spec-language PR 1571)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@c70f92eea; qsl-semantics/src/check/time_units.rs; Makefile; qsl-route/tests/it/route_registry.rs; qsl-replay/src/execute.rs; qsl-replay/src/hybrid/process_request.rs"
review_set: subset
---

# Review of QSL-1100 main reds (PR 1571)

## Summary

Verdict PASS. No blocking findings.

Preimage bytes (the one risk). I built a throwaway in-crate test (not committed) that builds the old hand-written `serde_json::json!` preimage (copied from main) and calls the new typed `preimage_bytes(identity)` for the same inputs, over 5,625 dimension and unit preimages: 15 owner identities (ASCII, accents, CJK, an astral emoji, quotes, backslash, tab, U+0001, U+001F, U+007F, U+2028/2029, `</script>`, U+10000, combining accent), 5 node strings, 7 declaration names, 8 scales (integers, negative fractions, a 39-digit numerator, zero, a 25-digit/3 ratio), 3 offsets, and unit chains with and without a target unit. Result: 0 differences. Every old and new byte string is identical, and the SHA-256 of the typed bytes equals `nominal_key` for all 5,625. So no node key, package id, golden or emitted fixture changes. The stored preimage did not mismatch its key on these inputs, so the coder's claim that the old bytes disagreed with the key is not supported by this probe; the change is a pure refactor with identical output (serde_json's map is ordered and its string escaping matches RFC 8785 for these inputs, and no JSON numbers are involved because rationals are strings). The typed path is still the right one: it removes the dependence on that coincidence.

Make and cargo-deny. `cargo deny --workspace check bans` reads `./deny.toml` by default, and the target runs from the workspace root, so dropping `--config` loses nothing. Checked with cargo-deny 0.20.2 (installed) and 0.19.8 (built into a scratch root): `make cargo-deny-bans` prints `bans ok` on both. Placement facts verified: 0.20.2 accepts `--config` before `check` and rejects it after; 0.19.8 does the reverse; the test's comment is accurate. The route test `cargo_deny_refuses_a_graph_that_depends_on_a_denied_crate` passes on both versions and is not weakened: it still copies the real `deny.toml`, runs `check bans` from the scratch directory, asserts a failing status, rejects a resolution failure as proof, and requires a per-crate `error[banned] ... is explicitly banned` line for each of the three denied crates.

String edges. `ReplayRefusal::pointer` returns RFC 6901 pointers keyed by member names; `HybridProcessRequest::read` and `read_no_initial_state_result` compare `serde_json::Value` members and wire kind, role, form and requirement values. These are wire comparisons, the intended case of the lint, not strings that should be typed.

Checks run (via the shared cargo lock): `make arch-lint-canonical-encoder` PASS; `make cargo-deny-bans` PASS; `make string-edge` PASS; `cargo test -p qsl-semantics -p qsl-route -p qsl-replay` all pass (877 in the qsl-semantics lib, 0 failures anywhere); `cargo clippy -p qsl-semantics -p qsl-route -p qsl-replay --all-targets -D warnings` clean.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | `.github/workflows/ci.yml:43` still runs `cargo deny --workspace check bans --config deny.toml`. It works with the pinned 0.19.8 but breaks as soon as the pin moves to 0.20 or later. Needs a follow-up by someone with workflow scope: drop `--config deny.toml` (the default file is the same). | .github/workflows/ci.yml:43 | correct-requirement-no-evidence |
| FND-002 | low | `ReplayRefusal::pointer` matches `ObservedTemporalMismatch { member: &'static str }` against six string literals with a `_ => "/replay/source"` fallback, so an unknown member silently maps to the source pointer. The marker is legitimate (these are wire member names), but a closed enum for `member` would make the mapping total. Optional follow-up. | qsl-replay/src/execute.rs:626 | implementation-bug-despite-evidence |
| FND-003 | low | The doc on `HybridProcessRequest::read` says the member and kind names are the wire's; the function also compares wire digest strings (`claim_node`, `wire_node_digest`). Same edge, doc undersells it. | qsl-replay/src/hybrid/process_request.rs:237 | implementation-bug-despite-evidence |

## Verdict

PASS at c70f92eea. FND-001 is the only follow-up with consequence (CI breaks on a future cargo-deny bump).

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
