---
id: SR-1299
title: "Gap analysis of PR #632 (IR-582): coverage that left QSL with the two extracted crates"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@f40729a9a9adafaeb233fadcb938245f2861776c; xtask/src/{string_edge,typestate_scan,definition_scan,canonical_types}.rs, tools/arch-lint/{api_surface,canonical_encoder,qualified_core}.rs, tests/it/family_outcome_layering.rs; compared against agent-ix/quire-exact@1098e44e and agent-ix/quire-semantic-value@787d87bb"
review_set: subset
---

## Summary

Ticket: IR-582. For each QSL scan whose roots lost `quire-exact/src` or
`quire-semantic-value/src`, does the property still hold somewhere?

| QSL scan | Property for the leaves | Where it holds now |
| --- | --- | --- |
| no_std targets (Makefile) | builds for thumbv7em-none-eabi | each repo's `make ci` → `build-no-std` |
| `string-edge` (FR-064) | no unmarked string dispatch | No string comparison or string `match` in either crate's src (grep). FR-064's named sites are QSL's. Nothing lost |
| `typestate_scan` `QSL_CRATES` | stage-output types defined once, no NodeKey mint outside `check` | stage types are QSL's; `make ci` passes with the narrower roots, so no positive assertion depended on quire-exact |
| `canonical-encoder` | one RFC 8785 encoder | holds by construction: neither crate depends on `serde_json` or a hasher; SV reaches `quire-canonical` only |
| `qualified-core` ambient input | no env/clock/fs/stdio/process/global state | `#![no_std]` (no `std::env/fs/time/io/process`) plus `unsafe_code = "forbid"` (no `static mut`). The direction check still walks the leaves through the core crates |
| `family_outcome_layering` dependency allowlist (K names no workspace crate; SV exactly 4 deps) | leaves never depend on QSL | each repo's `deny.toml` sets `unknown-git = "deny"` with an `allow-git` list that omits quire-spec-language, so `make deny` refuses a QSL edge |
| `definition_scan` / TC-390 | family outcome types defined once | the test now asserts no QSL definition of the SV-owned names; the owner side is in SV |
| `api-surface` T12-B (FR-060-AC-5) | SV mints no `NodeKey` | moved: SV `FR-060-AC-5`, clippy `disallowed-methods` on `NodeKey::from_digest`/`decode_admitted` |
| `api-surface` T12-C, T12-D | SV mints no `EffectiveId` / `PopulationId` | **not enforced anywhere** (FND-001) |
| `canonical-types` | one definition per canonical type | still scans the leaves from the git checkout (see SR-1298 FND-001) |

The deleted spec items FR-089-AC-6/TC-297 and FR-106-AC-10/TC-904 exist in
the owning repositories under the same ids, with `#[trace]` tags on the tests
(quire-exact `value.rs:1270`, `equality.rs:283`, `key.rs:190`; SV
`object_closure.rs:235,250,265`).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | QSL's T12-C and T12-D rules (only `model` mints `EffectiveId` / `PopulationId`) scanned `quire-semantic-value/src` through `qsl_scan_src_roots`. The successor in SV covers `NodeKey` only (`clippy.toml` disallowed-methods), yet ADR-011 still says SV mints no `EffectiveId` or `PopulationId`. Nothing enforces that now. Fix in agent-ix/quire-semantic-value: add `quire_exact::EffectiveId::from_digest` and `quire_exact::PopulationId::from_digest` to `disallowed-methods`, with an AC. Today's shipped SV code calls neither; the one call is a test at object_closure.rs:193 | tools/arch-lint/api_surface.rs:1245-1272 |

## Verdict

One medium gap, fixable in quire-semantic-value, not in this PR. Every other
scan that dropped the leaves either has a successor in the new repositories or
never applied to them.
