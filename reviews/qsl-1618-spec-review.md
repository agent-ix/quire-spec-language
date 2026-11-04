---
id: SR-1618001
title: "spec-review of the src/number.rs transitional re-export exemption (quire-spec-language PR 2244)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; spec/functional/FR-087-typestate-and-cross-package-node-key.md; spec/functional/FR-511-keep-the-exact-protocol-number-codec-as-a-root-module.md; spec/test-cases/TC-256-check-package-edge-directions-closed.md; spec/test-cases/TC-1306-keep-the-exact-protocol-number-codec-as-a-root-module.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-087
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-511
    type: references
---

# Spec review of QSL-1618 number re-export exemption (quire-spec-language PR 2244)

## Summary

Ticket: QSL-1618. One spec-review pass over `git diff origin/main...HEAD` at head
88be3798b6a95939907d830fef2dd7b93a59f4a3 against origin/main 079cf0200. Main-red path.

Verified against real code on origin/main: `src/number.rs` has exactly one `pub use`,
`pub use qsl_package::number::{ExactInteger, ExactRational, NumberComponent, NumberError,
NumberWire, ProtocolNumber, NUMERIC_PROFILE};` the seven named items, nothing else. The spec's
list matches. `tests/it/layer_crate_reexports.rs` scans every `src/**/*.rs` with syn, keyed by
path relative to `src` (`number.rs`), and `scan()` computes layer names crate-wide before
per-file exposures, so skipping one file's exposures is implementable and the seven-item check is
a small addition; the fixture test (`the_reexport_scan_catches_every_public_shape`) takes
`(file, source)` pairs, so the three adverse cases of TC-256 step 6 can be planted as a
`number.rs` fixture. `tests/it/protocol_number_root_module.rs` imports exactly the seven names;
TC-1306 step 4 needs a new read of `pub use` in `src/number.rs` (code PR). The ACs can fail: an
eighth item, another root module, or another layer crate in `number.rs` each fails. No contradiction
with FR-087's other ACs (AC-8 concerns `EmittedPackage`). Open PRs 2243, 2242, 2241, 2231, 2224 do
not touch FR-087, FR-511, TC-256 or TC-1306; no new ids are introduced. `quire validate` exit 0 on
all four changed files (only module duplicate-archetype warnings).

No-compat-layer rule: a documented transitional re-export is acceptable here because the rule's
premise (prerelease, no users, ADR-012 16.9, ADR-011 7.3) does not hold for the external
quire-protocol consumer, the exemption is one named path with a closed item list and an end
condition, and the old state (a hidden, unlisted compat path) was worse. It must be recorded where
the rule lives (FND-001).

End condition: checkable (grep quire-protocol for `quire_spec_language::number`), carries no ticket
id in the new text.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | ADR-011 is not amended. Its 7.2 rule, the extraction bullet ("The root crate re-exports no moved item", ~line 1138) and 7.3 "Compatibility disposition: none" still read as absolute, and FR-087-AC-9 still cites "ADR-011 section 7.2 ... forbid" beside the new exemption. Two normative sources now disagree on whether the root crate may re-export a layer item. Add an Amended note in ADR-011 (one named exemption, owned by FR-511) in this PR or the follow-up. | spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md:1138 | wrong-requirement |
| FND-002 | low | "Exactly seven items ... from `qsl_package::number`" does not say how an item is identified. The existing scan records the first path segment and the bound name; a rename (`ProtocolNumber as X`), a glob, or `qsl_package::other::NumberError` are not named. Say: by un-renamed name, path `qsl_package::number`, no glob; and that the skip applies to exposures only while `number.rs` still feeds the crate-wide layer-name set. | spec/test-cases/TC-256-check-package-edge-directions-closed.md:65-71 | missing-requirement |
| FND-003 | low | FR-511-AC-1 is now compound: it asserts the re-export, the exact seven, and "no other root module re-exports a layer-crate item", which is FR-087-AC-9's claim restated. The seven-item list is stated five times (FR-087-AC-9, FR-511 Behavior, FR-511-AC-1, TC-256, TC-1306), a drift risk. Keep one owner (FR-511) and reference it. | spec/functional/FR-511-keep-the-exact-protocol-number-codec-as-a-root-module.md:AC-1 | missing-requirement |
| FND-004 | low | The end condition depends on a repository outside this one and names no check. State the check (no source of the external consumer imports `quire_spec_language::number`), so the removal trigger is observable by a reader of this repo. | spec/functional/FR-511-keep-the-exact-protocol-number-codec-as-a-root-module.md:Behavior | missing-requirement |

## Verdict

Mergeable. No high finding; FND-001 should be fixed forward (main-red).
