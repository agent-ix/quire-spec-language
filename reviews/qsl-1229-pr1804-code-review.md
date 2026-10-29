---
id: SR-1804001
title: "code review (with rust-review) of PR #1804 (QSL-1229): fixture digest purge"
type: SpecReview
analysis: code-review
date: 2026-10-29
scope: "agent-ix/quire-spec-language; PR #1804 git diff origin/main...d4a0e4f28 (43 files)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-136
    type: reviews
---
# SR-1804001: code review of PR #1804 (QSL-1229)

## Summary

Head d4a0e4f28, base main (a6f9b417e). Judged by the global standard: a frozen digest that is recorded, compared against or gated on is ceremony; the only exceptions are a canonical identity digest bound to the content it proves and a pure known-answer vector.

**Removed correctly.**
- The 12 native-package vector files and their writer example are gone. Each vector is now composed at test time by `vectors::expected_inner`. That recipe hashes the canonical text with the explicit `quire-spec-language\0quire.native.bound-package/v1\0linked-package\0` domain through `sha2` directly, and the source digest with `Sha256` directly. Neither reads the producer, so the test still compares the producer with an independent oracle.
- `controls.native` was byte-identical to `minimal.native` on main, so composing both from `vectors::source` loses nothing.
- `spine-model.native` now carries `{model_digest}`, filled from `package_input` at test time. `model-artifact.json` carries `{source_digest}`, filled with the SHA-256 of `model-source.json`.

**The residue, judged item by item.**
- **Placeholders are never compared.** I mutated qsl-analyze `counter.rs`'s `PLACEHOLDER` (used as `manifestDigest` and `source.digest`) from zeros to `sha256:12345678…`. All 59 `counter` tests pass, so nothing compares or gates on it.
- **Syntax and refusal inputs:** the `aaa…` grammar inputs, tc465 row 13, the profile tails, and `111…`/`222…` as preimage inputs. All are inputs, not records. Accepted.
- **`package_id_is_the_hash_of_its_hand_written_preimage` is a real known-answer test.** The expected value is SHA-256 of the hand-written RFC 8785 text, computed at test time. The test checks that `EmittedPackage::new` mints that identity and that the text round-trips byte for byte. The embedded type digest is an input to the hash. Accepted.
- **NIST "abc" and empty-input vectors.** Accepted.

**Missed hits.** None in code or fixtures. A repo-wide grep for 64-hex literals (outside `reviews/`, `plan/`, `spec/`, `artifacts/`, lockfiles and the two residue directories) finds only repeated placeholders, the two NIST vectors, the 2^256 decimals and the long decimal in `intake.rs`.

**FND-001 (medium).** `cargo fmt --check` fails on six hunks in five files this PR touches: `qsl-replay/src/spine/call/tests.rs`, `qsl-replay/src/spine/lifecycle/tests.rs` (two hunks), `src/package/tests.rs`, `tests/it/compile_command.rs` and `tests/it/formatter_identity.rs`. Main is fmt-clean.

**FND-002 (medium).** `tests/fixtures/refinement/versioning/config-version-revision/` keeps a frozen `sha256-jcs` digest of `examples/config-version/model.semantic-ir.json`, in four files.
- Any edit to the example model breaks `make refinement-versioning` until the digest is re-pinned. That is the frozen-sibling pattern this ticket removes.
- The PR body's reason is real: FR-340-AC-6, FR-344 and TC-864 run the shipped CLI over the on-disk corpus, and FR-056 needs the digest spelled.
- So this is a decision, not a fix. Under the global rule it must be surfaced and tracked, not left as residue. File a ticket for the spec change (for example, the corpus states its documents and the gate supplies the digests) and cite it here.

**FND-003 (medium).** The compiled-protocol handoff ceremony is kept and deferred to the unpushed 580w rewrite with no ticket. That covers the `Producer.binary` "informational" digest, the frozen-copy byte comparison in `compiled_protocol_v2.rs`, and `examples/protocol-handoff/*-frozen.*`.
- The global rule forbids "informational" values and requires an expiry-tracked ticket for any deferral.
- File one, naming the 580w rewrite as the expiry, or remove them here.

**FND-004 (low).** `qsl-replay/src/spine.rs` reaches out of its crate with `#[path = "../../tests/support/spine_model.rs"]`. It is `cfg(test)` only, so it does not affect the library build. But a qsl-replay unit test now depends on the root crate's test tree. A small helper inside qsl-replay's own test support (twenty lines, no copied data) would keep the crate self-contained.

**Gates** (own target, `nice -n 10`, `RUST_TEST_THREADS=4`, `CARGO_BUILD_JOBS=4`, QSPEC_DIR = qspec-63ff21f1):

| Suite | Result |
| --- | --- |
| qsl-package (full) | 429 passed, 11 failed |
| qsl-replay (full) | all pass (lib 910) |
| qsl-eval (full) | lib 145; `it` 824 passed, 1 failed |
| root `quire-spec-language` (full) | lib 64, `it` 1003, plus the rest; 0 failed |
| clippy (4 crates) | exit 0 |
| fmt | **fails** (FND-001) |

**The failures predate this PR.**
- The 11 qsl-package failures are exactly the 9 timed/clock tests plus `conformance_c14` and `conformance_i2`. The same 11 fail on main a6f9b417e with the same QSPEC_DIR.
- `qsl-eval timed_subject::the_clock_presence_frame_vectors_step_as_published` is outside the coordinator's known set. It fails identically on main (`StateModel(IllTyped(TypeMismatch))` at clause `P` index 6), so it is not caused by this PR.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `cargo fmt --check` fails on six hunks in five PR-touched files; main is clean | repo gate |
| FND-002 | medium | The refinement-versioning corpus keeps a frozen sibling digest; needs a tracked spec decision (ticket), not silent residue | FR-340-AC-6, FR-056 |
| FND-003 | medium | The compiled-protocol `Producer.binary` "informational" digest and frozen-copy comparisons are deferred to unpushed 580w with no ticket | FR-042, FR-050 |
| FND-004 | low | `qsl-replay/src/spine.rs` includes the root crate's `tests/support/spine_model.rs` by path | - |

## Verdict

Changes requested (FND-001 to FND-003).
