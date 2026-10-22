---
id: SR-15508
title: "Gap analysis of STD-265 FR-290 plugin refusal codes (QSL PR #1219)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@cb08ccc279d61b5231d0f8dad90fb8a60569ece2; spec/functional/FR-290-settle-plugin-results-as-typed-terminal-records.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-290
    type: reviews
---
# SR-15508: Gap analysis of STD-265 FR-290 plugin refusal codes (QSL PR #1219)

## Summary

Ticket: STD-265. PR: quire-spec-language#1219, head cb08ccc27 (live head verified), scoped to `git diff origin/main...cb08ccc27` (one content commit 14620c703 plus a main merge; FR-290 only, +13/-8). The diagnostics table drops the default codes for plugin refusals: UnknownMember -> invalid_result/unknown-member, Malformed -> invalid_result/invalid-value, CertificationNotTrusted -> invalid_result/certification-mismatch with expected/actual; Unsupported stays unknown_required_feature/unsupported-feature; certification steps 3/4 and AC-7 move `vouched` to Malformed. Read against QSpec #366 at b34f4d4, FR-290 AC-5..14 as merged by #1217, FR-289 (AC-11, AC-13, AC-14) and qsl-replay's PluginResultRefusal::diagnostic. Gate: `make check-spec-validation` with QSPEC_DIR at the #366 worktree exits 0 with 17 warnings, none in FR-290. The author's qsl-package checked_v2 (42) and qsl-semantics lib (736) runs against #366 were not re-run: trusted. The QSL diff is spec-only and #366 changes no file those suites read except the two catalog documents, whose changed rows no QSL code parses. No textual conflict with the other open FR-290 PR, #1221 (git merge-tree clean). quoin 0.28.3, quire 0.36.2 (engine 0.50.2). Planless, scoped to the diff. Plan completion: not assessed. read_plugin_certification and the other FR-290 readers are not on QSL main yet, so this spec change contradicts no code. qsl-replay's PluginResultRefusal::diagnostic on main still emits only the member name, with no document_pointer, which the coming implementation must change.

Units examined:

- FR-290-AC-11 (examined): The diagnostic of each refusal kind in AC-6 to AC-10 carries the code and cause the diagnostics table gives. Its detail names the result item index, the member or pointer, and the received spelling or JSON kind. A refused member's item settles `failed`, naming the plugin, with no diagnostic in the r
- FR-290-AC-7 (examined): `read_plugin_certification` accepts `"trusted"`. It refuses an absent member with `MissingMember` at `/certification`, `7` and `"vouched"` each with `Malformed` at `/certification`, and `"certified"` and `"uncertified"` each with `CertificationNotTrusted` at `/certification`. For a `proved` entry wi
- FR-290 (examined): Each `invalid_result` diagnostic names its pointer as `document_pointer`, the pointer the refusal carries (QSpec FR-272, FR-331-AC-34).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | minor: the new payload rules have no criterion. AC-11 still requires only the code, the cause and a detail naming 'the member or pointer'. Nothing requires `document_pointer` on an invalid_result diagnostic, and the `expected` `trusted` / `actual` payload of `certification-mismatch` is reached only through the table reference. AC-7 checks refusal kinds, not diagnostics. An implementation that omits `document_pointer` or `expected`/`actual` passes TC-774. Fix: extend AC-11 (and TC-774 step 7) to require `document_pointer` with the refusal's pointer, and `expected` `trusted` and `actual` the received value for CertificationNotTrusted. | spec/functional/FR-290-settle-plugin-results-as-typed-terminal-records.md:473 |

## Verdict

Mergeable for gaps apart from one minor finding (FND-001).
