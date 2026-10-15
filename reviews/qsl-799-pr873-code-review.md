---
id: SR-11270
title: "Code review of quire-spec-language PR #873: TC-453 population variants"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@7b0e0f7eeb288260e338f6e693b1bee18bea00e6; src/population_fixtures.rs, qsl-package/src/emit.rs"
review_set: subset
---
# Code review of quire-spec-language PR #873

## Summary

Ticket: QSL-799. Code review with the rust-review lane folded in, scoped to
`git diff origin/main...HEAD` (2 files). Read in place, no builds run. Oracle:
Contract IR FR-053 (AC-1, AC-2) and TC-453 steps 1, 3 and 5, read from
quire-contract-ir `origin/main`.

Examined:
- `Variant`, `VARIANTS`, `Variant::file_name`, `variant_fixture`,
  `VariantFixture`, `RetargetRefusal`, `retarget`, `rekey`, `derived_keys`,
  `rename`, and `write()` writing the three variants beside the four cases
  (examined)
- Step 1 variant: `Gadget[0]`, `Gadget[5]`, `Widget[3]`, one parameter each,
  through the real emitter and QSL's I2 read (examined, matches)
- Step 3 variant: only `p1`'s maximum differs (5 to 6), regenerated through
  the emitter (examined, matches)
- Step 5 variant: one parameter's `semantic_type` moved to the other
  same-type population node, keys rederived with IR's public
  `derive_structural_candidates` and `derive_application_node_id`, encoded
  with `encode_checked_package_v2`, read by `CheckedPackageV2::read`;
  population node ids and source rows kept (examined, matches)
- `fixture(bounds)` generalised over `(object, bound)` bindings with
  `M::Gadget` for every old case: same `TypeForm`, so the four cases' bytes
  are unchanged; the existing typed tests still assert kinds, roles and
  maxima per case (examined)
- `Emission::evidence()`: a read-only `&CheckedPackageEvidence` accessor; it
  cannot construct or mutate an `Emission`, so the emitter stays the sole
  constructor of the bytes (examined, clean)
- `M::Gadget`/`M::Widget` in place of TC-453's `M::Order`: TC-453 names an
  object type, not this model's (examined, clean)
- Gate coverage: `ci-all-features` runs the workspace tests with
  `--all-features`, so the new tests run in `make ci` (examined, clean)
- No consumer in this repo globs the output directory (examined, clean)
- No vendoring: IR is used through its public API; no copied bytes, schema
  or golden hashes (examined, clean)
- `make ci`'s `arch-lint-canonical-encoder` lane against the new shipped code
  (examined, finding)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `make ci` goes red on `arch-lint-canonical-encoder`. In shipped (non-test) code, `src/population_fixtures.rs` now both names `serde_json` (`rename`, lines 551 and 561) and names the `of_preimage(` hasher (`retarget`, line 450). The module has no `EXEMPT` entry, so the file is a violation. The `package_id` mint (`quire_canonical::to_vec` then `PackageId::of_preimage(..).hex()`) also restates `qsl-package` `checked_v2.rs`'s preimage-then-`of_preimage` derivation. Fix: compute the digest with the one encoder, `quire_canonical::sha256(&preimage, limits)`, as `qsl-replay/src/call_site.rs:925` does. That removes the `HASHERS` site. Do not add an exemption or split the file to dodge the lint. | src/population_fixtures.rs:445-452, src/population_fixtures.rs:551-561, tools/arch-lint/canonical_encoder.rs:108 |

## Verdict

One blocking finding (a broken gate). The three variants match TC-453 steps
1, 3 and 5. The four existing cases are unchanged. The `evidence()` accessor
is minimal and safe. Recorded, not blocking: every population node's `Type`
occurrence is at ordinal 0 over the same whole-unit region. So IR's FR-053-AC-2
"own provenance" check and step 5's second-node/first-occurrence pairing
cannot tell the two same-type nodes apart from these files (medium). There is
no byte-level regression test for the four cases. The existing typed tests
cover their content, and golden bytes are not wanted (low).
