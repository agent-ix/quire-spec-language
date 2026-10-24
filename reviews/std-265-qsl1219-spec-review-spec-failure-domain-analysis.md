---
id: SR-15507
title: "Failure-domain review of STD-265 FR-290 plugin refusal codes (QSL PR #1219)"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quire-spec-language@cb08ccc279d61b5231d0f8dad90fb8a60569ece2; spec/functional/FR-290-settle-plugin-results-as-typed-terminal-records.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-290
    type: reviews
---
# SR-15507: Failure-domain review of STD-265 FR-290 plugin refusal codes (QSL PR #1219)

## Summary

Ticket: STD-265. PR: quire-spec-language#1219, head cb08ccc27 (live head verified), scoped to `git diff origin/main...cb08ccc27` (one content commit 14620c703 plus a main merge; FR-290 only, +13/-8). The diagnostics table drops the default codes for plugin refusals: UnknownMember -> invalid_result/unknown-member, Malformed -> invalid_result/invalid-value, CertificationNotTrusted -> invalid_result/certification-mismatch with expected/actual; Unsupported stays unknown_required_feature/unsupported-feature; certification steps 3/4 and AC-7 move `vouched` to Malformed. Read against QSpec #366 at b34f4d4, FR-290 AC-5..14 as merged by #1217, FR-289 (AC-11, AC-13, AC-14) and qsl-replay's PluginResultRefusal::diagnostic. Gate: `make check-spec-validation` with QSPEC_DIR at the #366 worktree exits 0 with 17 warnings, none in FR-290. The author's qsl-package checked_v2 (42) and qsl-semantics lib (736) runs against #366 were not re-run: trusted. The QSL diff is spec-only and #366 changes no file those suites read except the two catalog documents, whose changed rows no QSL code parses. No textual conflict with the other open FR-290 PR, #1221 (git merge-tree clean). quoin 0.28.3, quire 0.36.2 (engine 0.50.2). Failure modes examined: certification read order (absent, non-string, outside the three, a core value; disjoint, and `vouched` vs `certified` now distinct), driver order certification-before-basis (AC-7's bounded-proof case still yields CertificationNotTrusted), a non-object assignments entry (`[]` -> invalid-value at the empty pointer, which QSpec allows), and a closed-set value QSL cannot hold (Unsupported, matching QSpec's new sentence). The missing pointers for four refusal kinds are recorded once, in SR-15505 FND-001.

Units examined:

- FR-290 (examined): | `QslOnlyCause`, `CoreOnlyBasis`, `UnknownSpelling` | `invalid_result`/`unknown-cause` (member `cause` or `basis`) | ... | `MissingCounterexample` | `invalid_result`/`missing-member` (member `counterexample`) | `UnknownMember` | `invalid_result`/`unknown-member` | `Malformed` | `invalid_result`/`in
- FR-290 (examined): Each `invalid_result` diagnostic names its pointer as `document_pointer`, the pointer the refusal carries (QSpec FR-272, FR-331-AC-34).
- FR-290 (examined): 1. The member is absent: `MissingMember` at `/certification`. 2. The member is not a string: `Malformed` at `/certification`. 3. The string is not one of FR-331's three values `certified`, `uncertified` and `trusted`: `Malformed` at `/certification`. 4. The string is `certified` or `uncertified`, wh
- FR-290-AC-7 (examined): `read_plugin_certification` accepts `"trusted"`. It refuses an absent member with `MissingMember` at `/certification`, `7` and `"vouched"` each with `Malformed` at `/certification`, and `"certified"` and `"uncertified"` each with `CertificationNotTrusted` at `/certification`. For a `proved` entry wi
- FR-290-AC-11 (context_only): The diagnostic of each refusal kind in AC-6 to AC-10 carries the code and cause the diagnostics table gives. Its detail names the result item index, the member or pointer, and the received spelling or JSON kind. A refused member's item settles `failed`, naming the plugin, with no diagnostic in the r

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

Clean apart from SR-15505 FND-001, which is not repeated here.
