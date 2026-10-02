---
id: SR-492
title: "Integrity review of FR-057 shared capability kinds"
type: SpecReview
analysis: integrity
scope: "spec/functional/FR-057-admit-shared-capability-kinds.md, spec/functional/FR-036-link-composed-native-packages.md, spec/test-cases/TC-115-preserve-composed-admission-stages.md, spec/test-cases/TC-153-admit-exact-capability-kinds.md, spec/test-cases/TC-154-refuse-unsupported-capability-vocabulary-version.md, spec/test-cases/TC-155-keep-admission-backend-independent.md, spec/model-linking/tests.md, spec/spec.md"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-057
    type: reviews
---
# SR-492: Integrity review of FR-057 shared capability kinds

## Summary

Reviewed commit cd4f71a on `task/229-capability-spec`
(agent-ix/quire-spec-language), diff against `origin/main`: new FR-057, TC-153,
TC-154 and TC-155, and amendments to FR-036, TC-115, `spec/model-linking/tests.md`
and `spec/spec.md`. The ticket is #229, including the 2026-09-19 owner ruling
that there is no compatibility path for the four-kind vocabulary.

The core of FR-057 is sound. The six labels, their spelling and their meanings
match QSpec FR-290 at `818f555` exactly. Matching is byte-exact with no
normalization. Identity is the label, and the kinds carry no order. Absent,
unknown and four-kind labels are refused, never mapped. Backend absence settles
`unsupported` with a warning naming the kind, never a refusal or hold.
Negotiation stays at CG `negotiate_*`. QSL `Capability` is admission only. Every
FR-057 criterion has a verification method, and every TC traces to its criteria.

One finding is blocking. FR-057 mints a refusal code, `invalid_capability`, and
three causes that the closed native diagnostic catalog does not contain. AD-016
requires arrow-1 refusals to come from that catalog, so #213 would have to invent
vocabulary. The other findings are ambiguities an implementer would have to
resolve alone, and matrix rows that disagree with each other.

Ticket #229 acceptance:
- One-to-one vocabulary with no local aliases: met.
- #185 can implement without inventing vocabulary or absence policy: not met
  until FND-001 and FND-004 are fixed.
- #213 consumes one capability type, no competing type: met ("One capability
  type" section, FR-057-AC-7).
- Version and unknown-kind behavior explicit: met, with FND-003 and FND-004 open.
- `validate --strict`: the eight changed documents pass. The repository as a
  whole does not (see FND-010).
- `/spec-review all`: this review is one part of it.

Verdict: REVISE (FND-001 is high and blocking; FND-002 to FND-006 are medium).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The refusal code is outside the closed catalog. FR-057 refuses with `invalid_capability` and causes `absent-kind`, `unknown-kind` and `unsupported-version`. None of these is in `quire.native.diagnostics/v1` (`proposals/quire-v1/definitions/native-diagnostics.md` at QSpec `818f555`) or in `src/diagnostic.rs`. That catalog is closed: "A new required code or cause variant needs a new catalog revision". AD-016 arrow 1 requires QSL refusals to be `Diagnostic{Code}` from the closed catalog, and its gate 7 checks every `catalog_code()` is inside it. So #213 cannot implement FR-057 without inventing a code. FR-057 also does not name the diagnostic stage. Fix: in FR-057, restate each refusal as an existing catalog code/cause pair and name its stage. Candidates: absent label as `invalid_package`/`missing-member`; unknown label as `invalid_package`/`invalid-value`; unsupported version as `unknown_wire`/`unsupported-wire`. If a new code is needed instead, add a `depends_on` to the catalog revision that adds it, and mark FR-057 and TC-153/TC-154 blocked on it. Update FR-057-AC-2, AC-3, TC-153 and TC-154 to match. | FR-057 Outputs, Refusal of labels, Absence table, AC-2, AC-3 · TC-153 · TC-154 · QSpec AD-016 arrow 1 |
| FND-002 | medium | "Hold" is never defined. #229 asks FR-057 to define the distinction among absence, backend absence, unsupported, refusal, timeout and hold. The case table has no hold row. The text says several times that a case is "never a hold", but never says what a hold is or which stage can produce one. So "not a hold" in FR-057-AC-6 and TC-155 has no checkable meaning beyond "no item waits for a registration". Fix: add a hold row to the case table that states what a hold is (a settlement deferred until some input arrives) and where it can occur, then state the testable property TC-155 checks. | FR-057 Absence table, AC-6 · TC-155 · #229 Scope |
| FND-003 | medium | Absent versus unknown is ambiguous. FR-057 says "carries no capability label" is `absent-kind`, and any label not byte-equal to an admitted one is `unknown-kind`. An empty string and a JSON `null` fit both readings. The catalog also says missing and explicit null "may not be collapsed". Fix: state that a missing member is `absent-kind`, and that an empty string and `null` are each one named cause. Add both inputs to FR-057-AC-2 and TC-153 step 3. | FR-057 Refusal of labels, AC-2 · TC-153 |
| FND-004 | medium | The version rule is not implementable as written. (a) FR-057 requires "any serialized artifact that carries capability labels" to declare `quire.capability-kind/v1`, but does not name the member that holds it or where it sits. TC-154 step 2 ("remove the version declaration") cannot be written without that. Carrier ownership is #211's, yet the matrix plans TC-154 under #213 alone. (b) FR-057 says `v1` means FR-290's six labels, but does not pin the FR-290 revision `v1` denotes. FR-290 itself is marked "Proposed". A later FR-290 edit could change what `v1` admits without a new version. (c) The canonical form of one value is a JSON string, but no canonical form is given for a serialized set of kinds, such as a backend's advertised kinds. Fix: in FR-057, name the version member and its JSON type, or state that #211 names it and mark the TC-154 row "Planned; #211/#213". Pin `quire.capability-kind/v1` to FR-290 at a named QSpec commit. State the canonical form of a serialized set (for example, a JSON array in ascending byte order of labels, no duplicates, and a duplicate is refused). | FR-057 Serialization and version, Dependencies · TC-154 · `spec/model-linking/tests.md` TC-154 row |
| FND-005 | medium | A QSL requirement obliges a component outside QSL. "When no registered backend advertises a requested kind, negotiation SHALL settle that item `unsupported`" makes CG `negotiate_*` the subject of a QSL SHALL. FR-290-AC-4 already owns that rule. TC-155 step 3 and TC-115 step 3 then run negotiation inside QSL tests, although where negotiation runs over the registry is #210's decision (FR-057 Dependencies). Fix: restate the QSL obligation as what QSL does, for example that the QSL report retains each negotiated disposition and its warning unchanged, and cite FR-290-AC-4 for the settlement rule. Mark the TC-155 row and the FR-036-AC-6 row as also waiting on #210. | FR-057 Absence section, AC-6 · TC-155 · TC-115 step 3 · `spec/model-linking/tests.md` |
| FND-006 | medium | The matrix contradicts itself on TC-115. The FR-036-AC-6 coverage row is now 🚧, but the TC-115 test-case row still says "✅ Passed locally" for FR-036-AC-6. Four tests in `tests/composed_admission_stages.rs` (lines 465, 524, 569, 653) still carry `#[trace("TC-115", "FR-036-AC-6")]` over the four-kind vocabulary, so `quire coverage` will report AC-6 backed. The matrix Overview warns that a backed row is not a delivered criterion. The same paragraph also says "FR-036 8 of 8 backed", but FR-036 has nine criteria. Fix: set the TC-115 row to 🚧 with the same note as the AC-6 row. State in the model-linking paragraph that the four traced tests exercise the retired vocabulary, and that AC-6 is not delivered until #213/#185 retarget them. Correct the FR-036 count. | `spec/model-linking/tests.md` TC-115 row, FR-036-AC-6 row, composed admission paragraph |
| FND-007 | low | FR-036 Status is stale. Its last paragraph says "TC-115 keeps an explicit unsupported temporal projection beside an admitted state request". TC-115 step 3 now requests two FR-057 kinds. The model-linking paragraph also narrates the change ("now names FR-057 capability kinds"). Fix: reword both to state the current test and remaining work only. | FR-036 Status · `spec/model-linking/tests.md` |
| FND-008 | low | A carrier refusal has no request index. FR-057 Outputs gives every refusal "the request index". An `unsupported-version` refusal applies to a whole carrier, which has no request index. Fix: say that label refusals carry the request index and a carrier refusal carries the carrier's location instead. | FR-057 Outputs |
| FND-009 | low | Terms and trace links drift. FR-036 speaks of a "clause/capability pair", while FR-057 Inputs speaks of a "requested pair" of a declaration, label and `required` flag. US-002 lists the FRs it exercises but not FR-057, although FR-057 implements US-002. FR-331 is cited in FR-057 without a link or relationship. Fix: use one term for the pair in both FRs. Add FR-057 to US-002's `exercises` list. Link FR-331 as `ix://agent-ix/quire-specification/FR-331`. | FR-036 Behavior · FR-057 Inputs, Absence section · US-002 |
| FND-010 | low | The #229 bullet "`validate --strict` passes" is met only for the changed documents. `quire validate --strict` over `spec/**/*.md` fails on seven TestMatrix files that still use the `Coverage Status` column (`spec/tests.md` and six module `tests.md` files). The diff does not touch them; they fail on `origin/main` as well. Fix: record in the #229 PR that the full-repository failure is not in this diff, or rename those columns to `Status` in a separate change before #229 closes. | #229 Acceptance · `spec/tests.md`, `spec/native-*/tests.md` |

## Method

- **Vocabulary.** Compared FR-057's table with FR-290 at QSpec `origin/main`
  `818f555`, label by label and description by description. They are identical.
  FR-290's identity, absence and backend-absence rules match FR-057.
- **Settled rulings.** Checked against the owner rulings given for this review:
  FR-290 is the vocabulary authority, backend absence is `unsupported` with a
  warning, CG `negotiate_*` is the one negotiation point, QSL `Capability` is
  admission only, and there is no compatibility path for the four-kind
  vocabulary. FR-057 follows each one. No finding recommends a mapping, reader or
  migration.
- **Catalog.** Read `native-diagnostics.md` at QSpec `818f555` and
  `src/diagnostic.rs` at cd4f71a. Neither has `invalid_capability` or its causes.
  AD-016 arrow 1 and gate 7 require catalog codes.
- **Code state.** `src/linking/composed/requests.rs:36` still has the four-kind
  `Capability`. `src/checking/composed.rs:320` has a private `Capability`
  (`Queries`, `Graph`) that carries no FR-290 label, as FR-057 "One capability
  type" says.
- **Traceability.** FR-057 → US-002 → StR-001. FR-057-AC-1..AC-6 each map to
  one TC with a matrix row. AC-7 uses Inspection and has a note in place of a row,
  as FR-017-AC-2 does. TC-153, TC-154 and TC-155 each `verify` FR-057.
- **Ids.** FR-057, TC-153..TC-155 and SR-492 are not used on any branch
  (`git log --all`), and SR-492 is above the highest id on `origin/main`
  (SR-465). The `FR-057` targets in FR-042 and FR-048 are QSpec's, not this one.
- **Validation.** `quire validate --strict --summary` over the eight changed
  documents: 8/8 clean. Over `spec/**/*.md`: 7 documents fail, none in the diff.

## Round 2 dispositions

Checked against the current tree: cd4f71a plus the uncommitted edits. The
upstream is quire-specification `046d1bd`.

| Finding | Disposition | Evidence |
| --- | --- | --- |
| FND-001 | resolved | `invalid_capability` and all nine causes are catalogued in `quire.native.diagnostics/v1` revision `1-draft.5` (FR-057:87-91; FR-057 Dependencies at FR-057:327-329; upstream `native-diagnostics.md` `invalid_capability` row, FR-271, FR-272). QSL refreshes its copy to that revision before #213 (FR-057:328-329). |
| FND-002 | resolved | A hold row and a definition were added (FR-057:271, 273-274). TC-155 checks that "nothing is pending after routing returns" (TC-155:58-60). |
| FND-003 | resolved | Missing or `null` is `absent-kind`; `""` and non-string values are `unknown-kind` (FR-057:141-150). AC-2 covers both (FR-057:307), and so does TC-153 (steps 2 and 3, TC-153:24-27). |
| FND-004 | resolved | (a) The member belongs to each format's owner: FR-331's `capability_vocabulary`, and QSL's per #211 (FR-057:133-137); the TC-154 row reads "#211/#213" (`tests.md:222,302`). (b) `v1` is pinned to FR-290 at `046d1bd` (FR-057:38-40). (c) A serialized set of kinds is carrier format. Under FR-290 the owner assigns it, and kinds compare as sets (FR-057:105-107). It is deferred to #211 with the member. |
| FND-005 | resolved | Negotiation is described in the indicative and attributed to CG (FR-057:232-247). FR-290-AC-4 is cited as the owner (FR-057:322-323). TC-155 and TC-115 use fixtures (TC-155:16-18; TC-115:25-31). |
| FND-006 | resolved | The TC-115 summary row is 🚧 (`tests.md:196`) and agrees with the FR-036-AC-6 row (`tests.md:296`). `tests.md:241-247` states that the traced AC-6 tests run over the four-member vocabulary. "8 of 8" is gone. Its replacement sentence is wrong for AC-9, which is recorded as SR-490 FND-002. |
| FND-007 | resolved | FR-036:210-211 now describes the FR-057-kind pairs. `tests.md:244` has no "now". |
| FND-008 | resolved | A carrier refusal carries the received identity or its absence, not a request index (FR-057:75-77). |
| FND-009 | resolved (SR-490), low | The term is unified: "requested clause/capability pair" (FR-057:27, 60). FR-331 is linked (FR-057:20-21, 223). US-002 still does not list FR-057 among the requirements it `exercises` (`spec/usecase/US-002-link-exact-models.md:5-25`), although FR-057 `implements` US-002 (FR-057:6-7). Fix: add `ix://agent-ix/quire-spec-language/FR-057` with type `exercises` to US-002. |
| FND-010 | accepted-as-ruled | The seven baseline TestMatrix `Coverage Status` failures are outside this diff and were excluded by the review brief. Round-2 validation shows only those seven. |
| FND-011 (new) | resolved (SR-490), medium | The Diagnostic code statement is broader than the requirement. It says "The QSL composed linker SHALL report every refusal under this requirement with code `invalid_capability` and exactly one cause from the closed set `absent-kind`, `unknown-kind` and `unsupported-version`" (FR-057:83-85). But the same requirement has other refusals. The registry refuses with `unknown-mode` and `duplicate-backend` (FR-057:215-219). The family checker refuses with `unsupported_construct`/`declaration-form` (FR-057:192-193). `case` exhaustiveness refuses with `undefined_expression`/`unproved-exhaustiveness` (FR-057:170). The carrier refusal is made by "QSL", not by the linker (FR-057:127-129). Read literally, these statements contradict each other. Fix: scope the statement: "The QSL composed linker SHALL report every refusal of a requested pair's label with code `invalid_capability` and cause `absent-kind` or `unknown-kind`; a carrier refused for its version carries `unsupported-version`; registration refusals carry the causes the registry rule names." |
| FND-012 (new) | resolved (SR-490), low | "the `SumCase` checker" (FR-057:170) names a component that does not exist. No QSL source, no QSL spec and no quire-specification document at `046d1bd` defines `SumCase`. FR-290 says only "discharged during language admission" (quire-specification FR-146). Fix: write "none; language admission discharges it (quire-specification FR-146), and an unproved obligation refuses as …". |
| FND-013 (new) | resolved (SR-490), low | FR-057's paraphrase of FR-290 at `046d1bd` drops five details that the review brief asks it to match. (1) `inconsistent-candidates` also covers "a candidate that does not advertise the item's kind" (FR-290:177). FR-057:241 says only "inconsistent with the manifest or the named backend". (2) `unknown-backend` names the backend, and `inconsistent-candidates` names each offending candidate (FR-290:176-177). FR-057:240-241 omits both payloads. (3) Registration refusals are reported ordered bytewise by backend identity, then digest (FR-290:193-195). FR-057:215-219 does not say so. (4) A tool that changes after a passing probe records the FR-331 result `failed` (FR-290:222-224). The FR-057 case table has no row for it (FR-057:269-270). (5) The FR-290 claim-form qualifiers "over a sum type" and "under a finite-trace or infinite-trace profile" are dropped (FR-057:169, 176). Fix: carry these five points into FR-057:215-219, 240-241 and 269-270, and restore the two qualifiers. Or state once that where FR-057's tables abbreviate FR-290, FR-290 at `046d1bd` governs. |

Round 2 verdict: ACCEPT WITH FINDINGS
