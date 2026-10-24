---
id: SR-15505
title: "Integrity review of STD-265 FR-290 plugin refusal codes (QSL PR #1219)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@cb08ccc279d61b5231d0f8dad90fb8a60569ece2; spec/functional/FR-290-settle-plugin-results-as-typed-terminal-records.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-290
    type: reviews
---
# SR-15505: Integrity review of STD-265 FR-290 plugin refusal codes (QSL PR #1219)

## Summary

Ticket: STD-265. PR: quire-spec-language#1219, head cb08ccc27 (live head verified), scoped to `git diff origin/main...cb08ccc27` (one content commit 14620c703 plus a main merge; FR-290 only, +13/-8). The diagnostics table drops the default codes for plugin refusals: UnknownMember -> invalid_result/unknown-member, Malformed -> invalid_result/invalid-value, CertificationNotTrusted -> invalid_result/certification-mismatch with expected/actual; Unsupported stays unknown_required_feature/unsupported-feature; certification steps 3/4 and AC-7 move `vouched` to Malformed. Read against QSpec #366 at b34f4d4, FR-290 AC-5..14 as merged by #1217, FR-289 (AC-11, AC-13, AC-14) and qsl-replay's PluginResultRefusal::diagnostic. Gate: `make check-spec-validation` with QSPEC_DIR at the #366 worktree exits 0 with 17 warnings, none in FR-290. The author's qsl-package checked_v2 (42) and qsl-semantics lib (736) runs against #366 were not re-run: trusted. The QSL diff is spec-only and #366 changes no file those suites read except the two catalog documents, whose changed rows no QSL code parses. No textual conflict with the other open FR-290 PR, #1221 (git merge-tree clean). quoin 0.28.3, quire 0.36.2 (engine 0.50.2). Cross-checks: the six causes against the vectors. BP-10's six certification adverse vectors (vectors.json adverse 90-95) patch temporal-outcome results/1 (partial-order `reduced`, `uncertified`) and certification-outcome results/4 (`bounded-solver`, `uncertified`) and results/6 (copy-swap+symmetry `reduced`, `uncertified`). Each FR-331-AC-34 assignment matches the patch and the Certification table: missing -> missing-member, refuted+certification -> unknown-member, `checked` -> invalid-value, and the three `certified` patches -> certification-mismatch expected `uncertified`. No other adverse vector patches `certification`. QSL FR-290 certification steps 1-4 match the QSpec causes, including `vouched` -> invalid-value and `certified`/`uncertified` -> certification-mismatch expected `trusted`. QSL's Unsupported mapping matches QSpec's new unknown_required_feature sentence. Claim 'no QSpec or QSL test parses the invalid_result row': confirmed on both mains. QSpec tests reference NATIVE_DIAGNOSTICS/FR-272 only through code-specific row parsers and never for `invalid_result`; FR-272-AC-15's tests (TC-350/358/367) do not read the row. QSL code reads no catalog text (qsl-semantics diagnostics_catalog holds the identity only). qsl-replay's PluginResultRefusal::diagnostic emits `unknown-cause` and `missing-member`, which stay in the set. IDs: FR-272-AC-23, FR-331-AC-34 and BP-25 are unused on QSpec main and on the four other open QSpec PRs (#365, #267, #207, #59). #1219 adds no ids. Merge order #366 -> #1219 is required: #1219 cites FR-331-AC-34, 'Result entries' and three causes that exist only after #366. Language: behaviour-only. Neither diff has status or changelog wording.

Units examined:

- FR-290 (examined): | `QslOnlyCause`, `CoreOnlyBasis`, `UnknownSpelling` | `invalid_result`/`unknown-cause` (member `cause` or `basis`) | ... | `MissingCounterexample` | `invalid_result`/`missing-member` (member `counterexample`) | `UnknownMember` | `invalid_result`/`unknown-member` | `Malformed` | `invalid_result`/`in
- FR-290 (examined): Each `invalid_result` diagnostic names its pointer as `document_pointer`, the pointer the refusal carries (QSpec FR-272, FR-331-AC-34).
- FR-290 (examined): 1. The member is absent: `MissingMember` at `/certification`. 2. The member is not a string: `Malformed` at `/certification`. 3. The string is not one of FR-331's three values `certified`, `uncertified` and `trusted`: `Malformed` at `/certification`. 4. The string is `certified` or `uncertified`, wh
- FR-290-AC-7 (examined): `read_plugin_certification` accepts `"trusted"`. It refuses an absent member with `MissingMember` at `/certification`, `7` and `"vouched"` each with `Malformed` at `/certification`, and `"certified"` and `"uncertified"` each with `CertificationNotTrusted` at `/certification`. For a `proved` entry wi
- FR-290-AC-11 (context_only): The diagnostic of each refusal kind in AC-6 to AC-10 carries the code and cause the diagnostics table gives. Its detail names the result item index, the member or pointer, and the received spelling or JSON kind. A refused member's item settles `failed`, naming the plugin, with no diagnostic in the r

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | major: FR-290 now has every invalid_result diagnostic carry 'the pointer the refusal carries' as `document_pointer`. QslOnlyCause, CoreOnlyBasis, UnknownSpelling and MissingCounterexample carry none: 'Basis and cause' steps 3 and 4, 'Assignments' step 2 and AC-6, AC-9 and AC-10 give no pointer, and the table says only 'member `cause` or `basis`' / 'member `counterexample`'. FR-289-AC-14 and qsl-replay's PluginResultRefusal::diagnostic also name only the member. QSpec #366's 'Result entries' gives `unknown-cause` the pointer of the `type` member (`/basis/type`, `/cause/type`). An implementer must guess between `/basis` and `/basis/type`. MissingCounterexample also covers an entry that names a `transcript` and one whose `counterexample` is present but not `assignments`, where QSpec's `missing-member` ('a member ... is absent') does not hold and no pointer is defined. Fix: give each of these refusals its pointer (`/basis/type` or `/cause/type` for the three spelling refusals, `/counterexample` for an absent counterexample), and map a present non-assignments counterexample or a `transcript` to the QSpec cause and pointer that fit it (for example `invalid-value` at `/counterexample/type`), or get QSpec to define one. | spec/functional/FR-290-settle-plugin-results-as-typed-terminal-records.md:252-261 |

## Verdict

Not mergeable: one major (FND-001, undefined document_pointer for four refusal kinds, which disagrees with QSpec #366's `type`-member pointer). Merge only after #366.

## Dispositions

Round 1, reviewed QSL #1219 at c62980557 (fix commit d84e35855).

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | The pointers match QSpec #366's `type`-member pointer. The assignments reader now follows QSpec's reading order (entry members, then counterexample type, missing, unknown, then values; duplicate parameter before value). A `transcript` beside a counterexample is UnknownMember. FR-289-AC-14 is narrowed consistently. Whether an in-set but plugin-forbidden spelling is `unknown-cause`/`invalid-value` is recorded as SR-15501 FND-005 (QSpec side). |
