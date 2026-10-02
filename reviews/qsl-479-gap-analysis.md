---
id: SR-1200
title: "QSL-479 gap analysis of PR #590 (decode_admitted, T12-F)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@def95b6896e0ff83303cb9e0329cb443d00823f7; PR #590 diff against origin/main: quire-exact/src/node.rs, tools/arch-lint/api_surface.rs, qsl-package/src/checked_v2/tests.rs, spec/functional/FR-060-check-qsl-api-surface-boundary.md, spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md; context: spec/functional/FR-087-typestate-and-cross-package-node-key.md, xtask/src/typestate_scan.rs, qsl-package/src/checked_v2.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-060
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-087
    type: reviews
---
## Summary

Ticket: QSL-479. PR: quire-spec-language#590.

Trace:
- FR-060 T12-F row (FR-060:91) and FR-060-AC-2/AC-3:
  `tc_arch_lint_api_surface_027` (api_surface.rs:2694, AC-3) and
  `tc_arch_lint_api_surface_028` (api_surface.rs:2759, AC-2, live tree). Both
  are correct bindings with real oracles.
- ADR-011 T-12 (1): `NodeKey::decode_admitted` (node.rs:108), tested by
  `a_decoded_admitted_key_is_the_key_check_minted`. This is a unit test of the
  kernel type. It is untagged because no FR AC owns the constructor.
- ADR-011 T-12 (3): no production code. QSL has no decode site today. The two
  new `checked_v2` tests exercise only a test-local helper (FND-002).
- FR-087-AC-6 / AC-11 (context): checked against the T12-F allow-list
  (FND-001).

Author-raised gap (read_import_view): confirmed real. `read_import_view`
(checked_v2.rs:825) is in `qsl-package/src/checked_v2.rs`, so T12-F's
`checked_v2` module allow-list admits a `decode_admitted` call there. TC-255's
FR-087-AC-6 scan (`tc_255_no_node_key_is_minted_outside_check_or_from_a_wire_id`,
xtask/src/typestate_scan.rs:983) forbids mints in `qsl-package/src/`. Its mint
detector (`is_mint_path`, typestate_scan.rs:290-355) recognises only
`from_digest` and `node_key_of`, so it does not see that call either.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | T12-F allow-lists all of `qsl-package`'s `checked_v2` for `NodeKey::decode_admitted`. That contradicts FR-087-AC-6, which says no `NodeKey` constructor in the crate is fed by a wire-read value and `package`'s E4 path makes zero constructor calls, and FR-087-AC-11, which says no `NodeKey`-constructing call inside `package`. `decode_admitted` is a `NodeKey` constructor fed by wire bytes by design. FR-087 is not amended. Neither gate catches a breach: a `decode_admitted` call in `read_import_view` (checked_v2.rs:825) passes T12-F, and TC-255's scan only matches `from_digest`/`node_key_of`. Fix it one of two ways. (a) QSL has no decode site, so give T12-F no QSL allowed caller. Every QSL call then fails, which keeps FR-087-AC-6/11 true, and the FR-060 row and ADR-011 T-12 (2) say a backend names its own reader. (b) Amend FR-087-AC-6/AC-11 to name the one exception and narrow it to a function that runs only after `read_checked_package_v2` admits, excluding `read_import_view`'s E4 path, then teach TC-255's scan that `decode_admitted` is a constructor. | spec/functional/FR-060-check-qsl-api-surface-boundary.md:91; tools/arch-lint/api_surface.rs:374-402; spec/functional/FR-087-typestate-and-cross-package-node-key.md:597; xtask/src/typestate_scan.rs:290-355 |
| FND-002 | low | Ceremony (value test). `decodes_admitted_keys_after_the_identity_check` and `a_forged_package_is_refused_before_any_key_is_decoded` exercise `decode_admitted_keys`, a helper defined in the test file (tests.rs:283), not production code. No production code calls `decode_admitted`. The `None` result for a forged package is guaranteed by the helper's own `outcome.ok()?`. The forged-package refusal itself duplicates `refuses_package_id_that_does_not_recompute` (tests.rs:710). The tests prove the test helper obeys ADR-011 T-12 (3), which nothing in production does yet. Delete both tests and the helper, and keep the quire-exact unit test. | qsl-package/src/checked_v2/tests.rs:279-345 |

## Verdict

Changes needed: FND-001 (high) blocks. FND-002 is a low cleanup.

## Dispositions

Round 1, reviewed at c185ebdcdbfbf5e6ee2616010292644ac7b540d0.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | c185ebdc: fix option (a). T12-F `allowed_callers: &[]`, so every QSL call fails, `read_import_view` included. `tc_arch_lint_api_surface_027` now asserts the allow-list is empty and that a call in `checked_v2::read_import_view` is a violation (traced to FR-087-AC-6). The FR-060 T12-F row and ADR-011 T-12 (2) say QSL has no allowed caller and a backend names its own reader. FR-087-AC-6/AC-11 now hold without amendment. |
| FND-002 | fixed | c185ebdc: deleted both helper tests, the `decode_admitted_keys` helper and their imports. qsl-package now has no diff against origin/main. The quire-exact unit test is kept. |
