---
id: SR-494
title: "Evidence analysis of FR-057 capability-kind admission"
type: SpecReview
analysis: evidence
scope: "spec/functional/FR-057-admit-shared-capability-kinds.md, spec/functional/FR-036-link-composed-native-packages.md (FR-036-AC-6), spec/test-cases/TC-115-preserve-composed-admission-stages.md, spec/test-cases/TC-153-admit-exact-capability-kinds.md, spec/test-cases/TC-154-refuse-unsupported-capability-vocabulary-version.md, spec/test-cases/TC-155-keep-admission-backend-independent.md, spec/model-linking/tests.md"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-057
    type: reviews
---
# SR-494: Evidence analysis of FR-057

## Summary

Reviewed commit cd4f71a on `task/229-capability-spec`, the diff against
`origin/main`, for issue #229. The review checks the verification method of
FR-057-AC-1 to AC-7 and the changed FR-036-AC-6. It also checks that TC-115,
TC-153, TC-154 and TC-155 can produce the evidence their criteria need.

`quoin advise` agrees with the authored method for seven of the eight criteria.
The one mismatch is FR-057-AC-7: it is authored `Inspection`, and the advisor
recommends `Test`. For FR-057-AC-2, AC-3 and AC-6 the advisor also lists
`property-based-testing`, because they are universal claims. The planned tests
use fixed example lists only.

FR-057-AC-1 is a closed six-member set. Enumerating all six is exhaustive, so a
property test adds nothing there. This is reviewer judgement, not advisor
output.

The main evidence problem is in TC-155 and TC-115. Both ask QSL to "settle" an
item as `supported` or `unsupported`. FR-057 puts that settlement in
quire-contract-codegen's `negotiate_*`, and forbids the linker from reading
backend support. So as written, neither test can run inside QSL without a
second negotiation point.

Verdict: ACCEPT WITH FINDINGS. No `high` finding. Three `medium` and three
`low` findings. Each is a wording change in this spec's criteria, test cases or
matrix rows.

## Findings

| ID | Severity | Summary | Refs |
|----|----------|---------|------|
| FND-001 | medium | TC-155 step 3 says "run negotiation", and TC-115 step 3 asks for one pair "that settles `supported`". FR-057 fixes negotiation at CG `negotiate_*` and says the linker SHALL NOT consult backend support. A QSL test that settles items itself is a second negotiation point. FR-057-AC-6 also says "not a hold" and "no item waits", but names nothing a test can observe. Fix: in TC-155 and TC-115, supply settlements as fixture input shaped as `negotiate_*` output (one disposition per `request_index`). Test only that QSL keeps both pairs, applies the aggregate rule and keeps the warning's label. Split FR-057-AC-6. The QSL part covers retention and aggregation. The settlement part ("no registered backend → `unsupported` with a warning naming the label") cites the CG `negotiate_*` test as its evidence, marked planned under #210. State the observable for "not a hold": one negotiation call returns a disposition for every item, and no disposition is pending. | FR-057-AC-6, FR-036-AC-6, TC-155, TC-115 |
| FND-002 | medium | TC-153 step 1 reads "the vendored FR-290 Values table". No copy of FR-290 exists in the QSL tree: `git ls-files` finds none, and `resources/native-v1` vendors only FR-059, FR-060 and FR-061. Without a pinned source, the set-equality check in FR-057-AC-1 compares the code against a copy of the labels in the test, so it cannot catch drift from FR-290. Fix: in TC-153, name the exact source and revision, for example `quire-specification:spec/objects/protocol/FR-290-protocol-claim-kind.md` at a pinned commit. Also name how the test reads it. Do not use `resources/native-v1/` for this. | FR-057-AC-1, TC-153 |
| FND-003 | medium | Two verifiable statements in FR-057 have no criterion and no test. (1) "Routing selects a target backend only for an item that negotiation settled `supported`. Target selection does not depend on registration order, display text or ambient global state." (2) "Timeout is a result of running a `supported` item. It never becomes `unsupported`, a refusal or a hold." #185 and #213 are meant to implement from this spec without inventing policy, so both need evidence. Fix: add FR-057-AC-8 for routing: an item not settled `supported` gets no target, and permuting the registration order gives the same target. Mark it Test, planned under #185. Add FR-057-AC-9 for timeout: a timed-out `supported` item keeps its `supported` disposition and gets an FR-331 run result. Mark it Test, planned under #213. Add both to the tests.md matrix. | FR-057 Stage ownership, FR-057 Absence table |
| FND-004 | low | FR-057-AC-2 ("each unknown label") and FR-057-AC-3 ("any other version string") are universal claims. TC-153 step 3 and TC-154 step 3 test 9 fixed labels and 3 fixed versions. `quoin advise` recommends `property-based-testing` for both (`invariant`, `universal`). Fix: add a property step to each test. Any byte string not byte-equal to one of the six labels refuses as `unknown-kind`, echoing the exact bytes. Any version string other than `quire.capability-kind/v1` refuses as `unsupported-version`. Keep the named cases as fixed regression inputs. | FR-057-AC-2, FR-057-AC-3, TC-153, TC-154 |
| FND-005 | low | FR-057-AC-7 is authored `Inspection`. `quoin advise` recommends `Test` (`unit-testing` on `example`). Its `fuzzing` match on the word "parses" is spurious (reviewer judgement). The tests.md note names no record that will hold the inspection result, unlike FR-017-AC-2, which names SR-083. Fix: make FR-057-AC-7 a Test in TC-153. The test scans `src/` and requires the six labels to appear as literals only in the canonical `Capability` module. If Inspection is kept instead, name in tests.md the SpecReview that records it at the #213 PR. | FR-057-AC-7, spec/model-linking/tests.md |
| FND-006 | low | FR-057-AC-5's second sentence, "The linker reads no backend state", is a structural claim. The three-registry comparison in TC-155 step 2 shows the output does not change. It does not show that no backend state is read. Also, TC-154's matrix row says "Planned; #213", but FR-057 leaves the serialized carrier to #211, so TC-154 cannot name its input until #211 decides. Fix: either drop the sentence from AC-5, or add a check that the admission entry point takes no registry or backend parameter. Change TC-154's row to "Planned; #211/#213", and state that TC-154 reads the carrier #211 assigns. | FR-057-AC-5, FR-057-AC-3, TC-154, TC-155, spec/model-linking/tests.md |

## Method

- Read the full diff with
  `git -C /home/peter/dev/worktrees/qsl-arch13 diff origin/main...HEAD`, and
  read issue #229.
- Ran `quoin advise --repo /home/peter/dev/worktrees/qsl-arch13 --json`, with
  and without `--mismatch-only --inconclusive-only`. Results per criterion:
  - FR-036-AC-6: Test; no mismatch.
  - FR-057-AC-1: Test; golden, metamorphic and property-based
    (`serialization`, `round-trip`); no mismatch.
  - FR-057-AC-2: Test; property-based, compile-time check, design by contract
    (`invariant`); no mismatch.
  - FR-057-AC-3: Test; property-based (`universal`); no mismatch.
  - FR-057-AC-4 and AC-5: Test; unit and BDD (`example`); no mismatch.
  - FR-057-AC-6: Test; property-based (`universal`); no mismatch.
  - FR-057-AC-7: Inspection; mismatch; recommends Test (unit, BDD, fuzzing).
- Read QSpec FR-290, AD-010 and AD-016 at `origin/main` of
  `/home/peter/dev/quire-specification`. These are the settled authority for the
  vocabulary, the single registration contract, the single negotiation point and
  the four dispositions.
- Checked the QSL tree:
  - `src/linking/composed/requests.rs:36-44` holds the current four-member
    `Capability`. It has no serde or label form.
  - `git ls-files` finds no vendored FR-290.
  - FR-290 labels appear in `src/` only as the FR-059 file path in
    `definition_source.rs:213`.
- Not flagged, because they are owner rulings:
  - the six FR-290 kinds;
  - explicit refusal of the four-kind vocabulary, with no mapping;
  - backend absence settles as warned `unsupported`;
  - CG `negotiate_*` as the single negotiation point;
  - the #213, #185, #222, #210 and #211 split.

## Round 2 dispositions

Checked against the current tree: cd4f71a plus the uncommitted edits. FR-057
pins quire-specification `046d1bd`.

| Finding | Disposition | Evidence |
| --- | --- | --- |
| FND-001 | resolved | TC-155 supplies settlements as fixture records shaped like `negotiate_*` output and does not run negotiation (TC-155:16-18, 37-39). TC-115 step 3 does the same (TC-115:25-31). FR-057-AC-6 is now a routing property over settled data (FR-057:311). "Not a hold" is observable: "nothing is pending after routing returns" (TC-155:58-60). |
| FND-002 | resolved | TC-153 step 1 holds FR-057's v1 table, pinned to the FR-290 revision FR-057 names, as a test constant (TC-153:21-23; FR-057:38-40). |
| FND-003 | resolved | Routing and registration order are covered by FR-057-AC-8 (FR-057:313), and timeout by FR-057-AC-9 (FR-057:314). Both are planned under TC-155 (`tests.md:307-308`). |
| FND-004 | resolved | Property steps were added: TC-153 step 4 (TC-153:28-30) and TC-154 step 4 (TC-154:25-26). |
| FND-005 | resolved | FR-057-AC-7 is Test (TC-153) (FR-057:312), with the source scan in TC-153 step 6 (TC-153:34). |
| FND-006 | resolved | FR-057-AC-5 now requires that the entry point takes no registry or backend parameter (FR-057:310), checked in TC-155 step 1 (TC-155:23-26, 47). The TC-154 row is "#211/#213" (`tests.md:222,302`), and TC-154 reads the carrier #211 assigns (TC-154:15). |
| FND-007 (new) | resolved (SR-490), medium | FR-057-AC-9 is verified by a QSL test, but the behavior it describes belongs to other repositories. The adapter's tool probe and its FR-331 `unsupported`/`tool-unavailable` record belong to quire-contract-codegen, with FR-290-AC-8 as the owner (FR-057:269, 281-284). The timeout mapping belongs to the IR outcome map (FR-057:270). TC-155 step 7 can only supply those results as fixtures (TC-155:41-43). So "records the FR-331 result" and "is reported as a run result" (TC-155:62-67) observe the fixture, not QSL behavior. The one QSL-observable part is that routing sends the item to no other candidate or mode, and FR-057 states even that without a `SHALL` (SR-497 FND-011). Fix: restate AC-9 as a routing property. Given a fixture FR-331 result that is a timeout or `tool-unavailable` for a routed `supported` item, routing keeps its `supported` disposition, routes it to no other candidate or mode, and does not change the result. Cite FR-290-AC-8 and CG's tests as the evidence for recording the result. |
| FND-008 (new) | resolved (SR-490), medium | FR-057-AC-10's evidence depends on constructs QSL does not have, and the matrix does not say so. TC-153 step 7 needs a contract refinement gate, an operation bound by an abstraction relation, and a function-application clause (TC-153:35-42). The first two are open issues #191 and #192 and quire-specification FR-353; no QSL requirement or source covers them (a `grep` of `src/` and `spec/functional/` finds them only in FR-057). Function application is open issue #217. The AC-10 row says only "Planned; #213" (`tests.md:309`). FR-057 Dependencies lists #222 and #210 but not these (FR-057:331-338). Fix: mark the FR-057-AC-10 row "🚧 Planned; #213; refinement-gate forms #191/#192, abstraction relation per FR-353, function application #217". In FR-057 Dependencies, state that each claim-form row of AC-10 is tested when its construct lands. |

Round 2 verdict: ACCEPT WITH FINDINGS
