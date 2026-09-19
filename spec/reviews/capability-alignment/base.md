---
id: SR-490
title: "Base review of FR-057 capability-kind admission"
type: SpecReview
analysis: base
scope: "spec/functional/FR-057-admit-shared-capability-kinds.md, spec/functional/FR-036-link-composed-native-packages.md, spec/test-cases/TC-115-preserve-composed-admission-stages.md, spec/test-cases/TC-153-admit-exact-capability-kinds.md, spec/test-cases/TC-154-refuse-unsupported-capability-vocabulary-version.md, spec/test-cases/TC-155-keep-admission-backend-independent.md, spec/model-linking/tests.md, spec/spec.md"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-057
    type: reviews
---
# SR-490: Base review of FR-057

## Summary

Round 2. Reviewed the #229 change on `task/229-capability-spec`: commit cd4f71a
plus the uncommitted edits, diffed against `origin/main`. The change rewrites
FR-057, amends FR-036, TC-115, `spec/model-linking/tests.md` and
`spec/spec.md`, and adds TC-153, TC-154 and TC-155. FR-057 now pins
quire-specification revision `046d1bd` (quire-specification#135), which
replaces the placeholders it carried earlier (FR-057:38-40, 319-320).

The upstream authority is quire-specification at commit `046d1bd`, the #134
head in the `qspec-134` worktree. It includes
FR-290, FR-331, FR-271, FR-272 and `native-diagnostics.md` revision
`1-draft.5`. FR-057 matches it on each point the #229 brief lists:

- the ten labels and their families (FR-057:45-56);
- the claim-form table, one kind per item (FR-057:161-178);
- (kind, mode) advertisement (FR-057:206-219);
- the registration causes, including `duplicate-backend` (FR-057:215-219);
- candidate ordering by (identity, manifest digest) (FR-057:221-225);
- the candidate-table causes, including `inconsistent-candidates` and
  `absent-extent` (FR-057:232-247);
- tool absence as the FR-331 result `unsupported`/`tool-unavailable`
  (FR-057:269, 281-284);
- the `capability_vocabulary` carrier member (FR-057:133-137);
- catalog revision `1-draft.5` (FR-057:87-88, 327-329).

Where FR-057 paraphrases FR-290 and loses detail, the gap is recorded in
SR-492 (FND-013).

Base checklist:

- **ID formats.** FR-057, TC-153, TC-154, TC-155 and SR-490..SR-497 are well
  formed. No `spec/` document on `origin/main` or any other branch uses them.
  `resources/native-v1/spec/functional/FR-057-enforce-commit-recovery.md` is
  quire-specification's vendored FR-057, in a different namespace, and is
  outside the validation scope. The AC IDs FR-057-AC-1..AC-10 are sequential
  and unique.
- **FR quality.** Every FR-057 criterion is verified by Test and names its TC.
  Round-2 statement-level defects are recorded in SR-492 FND-011 and SR-497
  FND-011.
- **AC→TC.** Each of FR-057-AC-1..AC-10 names one TC (FR-057:306-315). The TC
  scopes cover exactly these criteria: TC-153 covers AC-1, 2, 4, 7 and 10;
  TC-154 covers AC-3; TC-155 covers AC-5, 6, 8 and 9. FR-036-AC-5, AC-6 and
  AC-8 remain on TC-115.
- **TC→AC.** Each TC step maps to a criterion in its scope. No TC tests a
  criterion outside its scope.
- **Matrix.** `tests.md` has one coverage row per FR-057 criterion
  (`tests.md:300-309`) and one summary row per TC (`tests.md:221-223`). Each
  row names the same TC as the FR. All are 🚧 Planned, which agrees with
  FR-057 Status (FR-057:340-349). The TC-115 summary row (`tests.md:196`) and
  the FR-036-AC-6 row (`tests.md:296`) are both 🚧 and agree with each other.
  The owner ticket on two rows disagrees with FR-057 (FND-001).

Verdict: ACCEPT WITH FINDINGS. There is one medium finding and four low
findings. Each one is a matrix or TC wording fix.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The matrix gives admission work to the wrong ticket. FR-057 Dependencies and Status give #213 the admission rules, the removal of backend reading and the family-body handoff (FR-057:331-333, 347-348). FR-057-AC-5 tests exactly those: the entry point takes no backend parameter, admission is backend-independent, and every resolved declaration is handed to its checker. The FR-057-AC-5 row still says "Planned; #185" (`tests.md:304`), and the TC-155 row says "#185" only (`tests.md:223`). The same issue is SR-496 FND-005, which is still open. Fix: set the FR-057-AC-5 row to "🚧 Planned; #213". Set the TC-155 row to "🚧 Planned; #213 (AC-5), #185 (AC-6, AC-8, AC-9)". | `tests.md:223,304` · FR-057:310,331-333,347-348 · SR-496 FND-005 |
| FND-002 | low | `tests.md:241` says "Every FR-036 criterion has a tagged test". FR-036-AC-9 has no `#[trace]` tag in `tests/` or `src/`, and its row is "🚧 Planned; #131" (TC-148). Fix: write "Every FR-036 criterion except AC-9 (TC-148, #131) has a tagged test". | `spec/model-linking/tests.md:241` · FR-036:142 |
| FND-003 | low | TC-153's step 4 and its expected results disagree. Step 4 generates arbitrary JSON values, including `null`, and says each refuses with "`unknown-kind` or `absent-kind` as step 2 and 3 define" (TC-153:28-30). The expected results say "Steps 3 and 4: each pair refuses with `invalid_capability`/`unknown-kind`" (TC-153:49). A generated `null` must give `absent-kind` (FR-057:141-143). Fix: change the expected bullet to "Step 3, and step 4 for every generated value other than `null`: … `unknown-kind`", and add "a generated `null` refuses as `absent-kind`". | TC-153:28-30,49 · FR-057:141-150 |
| FND-004 | low | TC-155 does not test the named-backend case of the candidate-set rule. When the request names a registered backend that does not advertise the item's kind, the candidate set is empty (FR-290 Candidate set). Step 4 names only a backend that advertises the kind, and an unregistered one (TC-155:34-36, 55-57). Fix: in step 4, also name a registered backend that does not advertise `operation-contract`, and expect an empty candidate set. | TC-155:34-36,55-57 · FR-057:221-223 · FR-290 Candidate set and negotiation |
| FND-005 | low | TC-155's file slug `settle-capability-absence-as-unsupported` no longer describes it. The test no longer settles anything, and its title is "Keep admission backend-independent and route only supported items" (TC-155:3). The file is new in this PR, so renaming it is not churn. Fix: rename it to `TC-155-keep-admission-backend-independent.md`, or similar, and update the tests.md link if there is one. | `spec/test-cases/TC-155-keep-admission-backend-independent.md` |

## Method

- Read `git -C /home/peter/dev/worktrees/qsl-arch13 diff origin/main` in full,
  plus the working tree of every changed document, with line numbers.
- Compared FR-057 clause by clause against `qspec-134` at `046d1bd`:
  FR-290 (Values, Families, Vocabulary authority, Claim-form assignment,
  Advertised mode, Candidate set and negotiation, Identity and comparison),
  FR-331 (Properties, Identity and validation), FR-271 and FR-272
  (`invalid_capability` and `unsupported_projection` rows), and
  `native-diagnostics.md` (revision `1-draft.5`).
- Checked AC→TC and TC→AC links, the matrix rows and statuses, and the ID
  collisions, using `git grep` on `origin/main` and `git log --all -S`.
- Checked the `#[trace]` tags for FR-036 in `tests/` and `src/`.
- Settled rulings were not re-opened: no compatibility path for the four old
  kinds; absence settles as warned `unsupported`; CG `negotiate_*` is the
  single negotiation point; there is one registration contract; `Capability`
  is admission only; and the #213/#185/#222/#210/#211 ownership split holds.

## Post-round-2 author fixes

Every finding left open or raised in round 2 across SR-490..SR-497 is fixed in the committed revision:

| Finding | Fix |
| --- | --- |
| SR-490 FND-001, SR-496 FND-005 | FR-057-AC-5 and TC-155 rows cite #213 for admission; FR-036 and tests.md say #213 removes backend reading from admission and #185 builds registration and routing. |
| SR-492 FND-011 | The FR-057 diagnostic-code SHALL is limited to label and carrier refusals. |
| SR-494 FND-007 | FR-057-AC-9 and TC-155 step 7 are routing properties over supplied results; recording tool absence cites FR-290-AC-8. |
| SR-494 FND-008 | FR-057 Dependencies and the AC-10 row name #217, #191, #192 and FR-353. |
| SR-490 FND-002..005 | FR-036-AC-9 exception noted; TC-153 `null` expectation; TC-155 non-advertising named backend; TC-155 file renamed. |
| SR-492 FND-009, FND-012, FND-013 | US-002 exercises FR-057; FR-146 replaces `SumCase`; FR-057 carries the missing qualifiers and states that FR-290 governs where it abbreviates. |
| SR-497 FND-009, FND-011 | FR-036 retention SHALL has the compiler as subject; claim-form and tool-absence routing rules are SHALL statements. |
