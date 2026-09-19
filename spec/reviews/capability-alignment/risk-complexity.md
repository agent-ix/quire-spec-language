---
id: SR-495
title: "Risk and complexity review of FR-057 capability-kind admission"
type: SpecReview
analysis: risk-complexity
scope: "spec/functional/FR-057-admit-shared-capability-kinds.md, spec/functional/FR-036-link-composed-native-packages.md, spec/test-cases/TC-115-preserve-composed-admission-stages.md, spec/test-cases/TC-153-admit-exact-capability-kinds.md, spec/test-cases/TC-154-refuse-unsupported-capability-vocabulary-version.md, spec/test-cases/TC-155-keep-admission-backend-independent.md, spec/model-linking/tests.md, spec/spec.md"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-057
    type: reviews
---
# SR-495: Risk and complexity review of FR-057

## Summary

Reviewed commit cd4f71a (branch `task/229-capability-spec`), the diff
`origin/main...HEAD`. Checked against the #229 acceptance bullets, QSpec
`origin/main` FR-290 and AD-016, `src/linking/composed/requests.rs` and
`tests/composed_admission_stages.rs`.

The owner rulings hold. FR-057 admits exactly the six FR-290 labels. It refuses
any other kind or version with no mapping. Backend absence settles warned
`unsupported`, never a hold. CG `negotiate_*` is the single negotiation point.
No finding asks for a compatibility path.

The main risk is what the change removes. The current `Capability::FamilyCheck`
is how the linker decides which declaration bodies a checker may receive
(`admitted_bodies`). FR-057 drops it and forbids the linker from reading
backend families. It puts nothing in its place, yet FR-036-AC-6 still promises
that unsupported family bodies are never shown as checked. #213 would have to
invent that rule (FND-001).

The second risk is test placement. Three planned checks need a negotiation
result, and negotiation lives in CG at a placement #210 has not decided. The
spec plans all three as QSL tests under #213 and #185 (FND-002). TC-153 also
reads a vendored FR-290 file that does not exist in this repository (FND-003).

Verdict: REVISE (1 high, 2 medium, 3 low). FND-001 blocks. Every fix is a
sentence or a test step inside this spec.

## Risk register

| Req | Tech risk | Volatility | Drivers | Mitigation |
| --- | --- | --- | --- | --- |
| FR-057 vocabulary and spelling (AC-1, AC-2, AC-4) | Low | Medium | Closed six-label set, exact bytes, no order. FR-290 is still "Proposed shared ruling" upstream, so its table can change. | Fixed v1 table plus a new version on any FR-290 change (FR-057 §Dependencies). TC-153 binds to a source that does not exist: FND-003. Revision pinning: SR-491 FND-010. |
| FR-057 carrier version (AC-3) | Medium | Medium | QSL mints `quire.capability-kind/v1`. No QSL carrier exists today (no `capability_report` in `src/`). Carrier ownership is #211's. | TC-154. Carrier refusal vs FR-036 retention: SR-491 FND-003. |
| FR-057 stage split (AC-5, AC-6) | High | High | Moves backend support out of the linker (`requests.rs` `report` reads `Backend` today). Settlement depends on CG negotiation, whose placement #210 decides. Five tickets touch it (#213, #185, #222, #210, #211). | TC-155 planned under #185. Negotiation-side verification is placed where it cannot run yet: FND-002. Order of work unstated: FND-006. |
| FR-057 one capability type (AC-7) | Medium | Low | Deletes the four-kind enum and its `families()` table. `admitted_bodies` keys on `FamilyCheck`. | Inspection at the #213 PR. Family-body gate has no replacement: FND-001. |
| FR-036 amended AC-6 | High | High | "Settled `supported`" is a negotiation result, yet TC-115 is a linker control. The unsupported-family guarantee lost its mechanism. | Matrix row set to 🚧. FND-001, FND-002. |
| FR-036 AC-5 | Low | Medium | The ✅ control constructs a four-kind `Backend` (`tests/composed_admission_stages.rs:202-218`), so #213 rewrites it. | None stated: FND-004. |

## Top hazards

1. FR-036-AC-6 / FR-057-AC-7: removing `FamilyCheck` strands `admitted_bodies`
   and the "never represented as checked" guarantee (FND-001).
2. FR-057-AC-6 and TC-115 step 3: negotiation-side checks are planned as QSL
   tests before #210 places negotiation (FND-002).
3. FR-057 stage split: five-ticket coordination with no stated order
   (FND-006).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Removing `FamilyCheck` leaves the family-body gate undefined. Today `Report::admitted_bodies` returns exactly the admitted `Capability::FamilyCheck` requests (`src/linking/composed/requests.rs`). `UnsupportedFamily` comes from the caller's `Backend.families`. That is how FR-036 keeps an unsupported family body from being shown as checked (FR-036 Status, "`admitted_bodies` exposes only admitted family-check subjects"). The TC-115 test `an_unsupported_family_leaves_the_body_inspectable_and_never_checked` (`tests/composed_admission_stages.rs:570`) exercises it. FR-057 admits only the six FR-290 protocol-claim kinds, so no kind requests a family check. It also forbids the linker from reading backend support. FR-036-AC-6 and TC-115 step 4 still require the guarantee. Nothing says how #213 decides which bodies a checker may receive. #229's acceptance says #213 and #185 must not invent policy. Fix: in FR-057 §One capability type (or FR-036 Behavior), state that family-body checking is not a capability request. State what decides a body's handoff after the change, for example "a body is handed to a family checker only after static family admission accepts it; an unsupported family result from that checker is retained per declaration and never counted as checked." Reword FR-036-AC-6 and TC-115 step 4 to test that rule without a capability kind. Leave per-family applicability of the six kinds to #210. | FR-057 §One capability type, FR-057-AC-7 · FR-036-AC-6, FR-036 Status · TC-115 step 4 · `requests.rs` `admitted_bodies` |
| FND-002 | medium | Negotiation-side checks are planned as QSL tests before negotiation has a place. FR-036-AC-6 now depends on a pair being "settled `supported`". TC-115 step 3 needs one pair that settles `supported` and one that settles `unsupported`. TC-155 step 3 says "run negotiation". FR-057 puts settlement in CG `negotiate_*`, and #210 decides where that negotiation runs over the registry (FR-057 §Dependencies). Yet `tests.md` plans TC-155 under #185 alone and keeps TC-115 in `tests/composed_admission_stages.rs`, which is a linker control. As written, #185 or #213 must either stub negotiation in QSL or wait on #210 with no recorded block. Fix: split each check at the stage line. Keep in QSL: both pairs retained in caller order, admission output unchanged, pairs handed on unchanged (FR-036-AC-6 first clause, FR-057-AC-5, TC-155 step 2). Move the settlement parts (FR-036-AC-6 "not settled `supported`" clause, FR-057-AC-6, TC-115 step 3 dispositions, TC-155 step 3) into steps that consume a negotiation result as input. Mark those matrix rows "🚧 Planned; #185, blocked by #210". | FR-036-AC-6 · FR-057-AC-6 · TC-115 step 3 · TC-155 step 3 · `spec/model-linking/tests.md` TC-155 and FR-057-AC-6 rows |
| FND-003 | medium | TC-153 step 1 reads "the vendored FR-290 Values table", but no FR-290 is vendored in this repository. `resources/native-v1/` has FR-060 and FR-061 and no FR-290. That tree is vendored and is not edited here. The test cannot be written as specified. If a copy is added later and refreshed from upstream, the test would follow FR-290 silently and never force the new vocabulary version FR-057 §Dependencies requires. Fix: in TC-153 step 1, compare the admitted set with FR-057's own v1 table, held as a test constant. State that any change to that constant needs a new capability vocabulary version. The FR-290 revision it matches is SR-491 FND-010. | TC-153 step 1 · FR-057-AC-1 · FR-057 §Dependencies |
| FND-004 | low | Two FR-036 passages go stale, and the matrix does not show it. FR-036 Status still says "TC-115 keeps an explicit unsupported temporal projection beside an admitted state request" (FR-036 lines 210-211). That contradicts the amended TC-115 step 3. The FR-036-AC-5 row stays ✅. Its control builds a four-kind `Backend` (`tests/composed_admission_stages.rs:202-218`), which #213 must rewrite. Fix: replace the FR-036 sentence with "TC-115 keeps two required pairs over different FR-057 kinds". Add to the `tests.md` L2 note that the FR-036-AC-5 control is rewritten under #213. | FR-036 Status · FR-036-AC-5 · `spec/model-linking/tests.md` L2 note |
| FND-005 | low | FR-057 §Absence cites "Its FR-331 result" with no repository. QSL has its own FR numbers, so the bare ID reads as a QSL requirement. FR-331 is quire-specification's (AD-016 references it). Fix: write "quire-specification FR-331" (`ix://agent-ix/quire-specification/FR-331`) and add it as a `references` relationship. | FR-057 §Absence, unsupported, refusal, timeout and hold |
| FND-006 | low | The implementation order across tickets is unstated. FR-057 Status names #213 (type and outcomes) and #185 (registry and routing). #185 consumes #213's type, and #185's routing consumes a negotiation result whose placement #210 decides. The Status and matrix rows list tickets without order, so #185 could start first and define a second vocabulary. Fix: add one sentence to FR-057 Status: "#213 lands the canonical `Capability` first. #185 builds on it, and its routing waits for #210's negotiation placement." | FR-057 Status · `spec/model-linking/tests.md` FR-057 rows |

## Failure-domain gaps

`spec/reviews/capability-alignment/failure-domain.md` (SR-491) is current for
cd4f71a. It overlaps here in two places, and this review does not repeat them.
SR-491 FND-003 covers the retention of refused pairs and a version-refused
carrier against FR-036. SR-491 FND-010 covers which FR-290 revision v1 denotes.
FND-003 here depends on that fix.

## Round 2 dispositions

Checked against the current tree: cd4f71a plus the uncommitted edits.

| Finding | Disposition | Evidence |
| --- | --- | --- |
| FND-001 | resolved | The new "Family-body admission" section states that body checking is language admission, not a capability kind. Every resolved declaration goes to its family checker, and a refused form is never shown as checked (FR-057:184-194). FR-057-AC-5 tests the handoff (FR-057:310). FR-036-AC-6 and TC-115 step 4 test the refusal without a kind (FR-036:139; TC-115:32-36). #213 owns the family-body handoff (FR-057:347-348). |
| FND-002 | resolved | Settlement parts are consumed as fixture input (TC-155:16-18, 37-39; TC-115:25-31). Negotiation is not a QSL stage (FR-057:232). The routing checks need no #210 placement. |
| FND-003 | resolved | TC-153 step 1 compares against FR-057's own v1 table as a test constant, pinned to FR-290 at `046d1bd` (TC-153:21-23; FR-057:38-40). |
| FND-004 | resolved | FR-036:210-211 now names the FR-057-kind pairs. `tests.md:244-247` records that the TC-115 controls use the four-member vocabulary that #213 replaces. The FR-036-AC-5 criterion names no kind and its control passes today, so ✅ is accurate until #213 rewrites the control. |
| FND-005 | resolved | FR-331 is qualified and linked as quire-specification's (FR-057:20-21, 223, 269, 287-289). |
| FND-006 | resolved | "#213 lands the canonical `Capability`, admission and family-body handoff first; #185 builds registration, candidate sets and routing on it" (FR-057:347-349). |

No new risk defects. The #191/#192/#217 prerequisites of the widened
claim-form table are a new coordination risk. They are recorded as SR-494
FND-008 rather than repeated here.

Round 2 verdict: ACCEPT
