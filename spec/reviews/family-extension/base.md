---
id: SR-474
title: "Base checklist review of ADR-012 semantic-family extension contracts"
type: SpecReview
analysis: base
scope: "spec/decisions/ADR-012-semantic-family-extension-contracts.md, spec/spec.md"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: reviews
---
# SR-474: Base checklist review of ADR-012

## Summary

Reviewed commit 048deb3 on `task/210-family-extension` against the quoin
spec-review checklist. ADR-012 is a design decision record with no US, FR, AC,
TC, option or constraint rows. The user-story, functional-requirement and six
test-coverage rules therefore have no subject. The applicable gates are ID
format and uniqueness, cross-references, link validity, terminology, and the
#210 acceptance bullets. The authoring agent ran this base pass. Independent
reviewers ran the seven analyses (SR-475 to SR-481).

Verdict: ACCEPT WITH FINDINGS (no blocking findings).

## Method

- ID format: `ADR-012` matches `^[A-Z]{2,4}-[0-9]+$`. It was reserved for #210
  by the program coordinator. No `origin/*` branch or open PR in
  agent-ix/quire-spec-language used it at authoring time.
- Local item ids: seams `S1` to `S9` (§5.1) are sequential and each is defined
  once. Section numbers run §1 to §14 with no gaps.
- Relationship targets: `ADR-010` resolves to
  `spec/decisions/ADR-010-observed-architecture-baseline.md`. QSpec `AD-016`,
  `FR-290` and `FR-340` resolve on quire-specification `origin/main`.
- `spec/spec.md` gains one `contains` relationship and one index row, and both
  resolve to the new file.
- Mermaid blocks contain no `;`.
- #210 acceptance bullets, each traced:
  - bounded change set for a sum/case form and a scoped frame clause: §12.1
    and §12.2;
  - no routine consumes a complete grammar: §4.1 and §4.3;
  - no dispatch on display text, string tags, registration order or ambient
    state: §7.1, §7.2 and §9;
  - absent capability gives a specified structured outcome and diagnostic:
    §7.3 and §5.2;
  - contracts cover check, package, lower, execute or prove, witness and
    replay: §8;
  - `/specify` and `/spec-review all`: this set.
- `quire validate --scope <repo> <ADR-012> spec/spec.md --strict --summary`:
  2/2 grammar-clean.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The checklist rules for US, FR and TC quality and the six test-coverage rules do not apply, because the ADR carries no acceptance criteria. They are recorded as not applicable, not as passed. The ADR's obligations become testable through #213, #214 and #185, which own the FRs. | ADR-012 §14 |
| FND-002 | low | Seam ids `S1` to `S9` are local to ADR-012. Other records must cite them as `ADR-012 S<n>` to stay unambiguous. Fix: state the citation form. | ADR-012 §5.1 |

## Round 2

The seven analyses were rerun against commit 8fb238b, which resolved every
round-1 high finding (`requires-bound` reachability and the single CG
`negotiate_*` settlement point). All seven round-2 verdicts are ACCEPT WITH
FINDINGS, with no open high finding. The author then addressed the remaining
medium items in the ADR: every requested item reaches `negotiate_*` once; the
mode vocabulary is decided in #222; the OBS-004 copy removal belongs to #185;
the solver-absence fault-injection test and the `xtask string-edge` scan have
owners; the AD-016 arrow-7 key change needs a QSpec issue; the unnumbered CG
and IR tickets are an owner question. Remaining low findings are recorded in
each analysis file for #212.

Round 2 verdict: ACCEPT WITH FINDINGS

## Author closure (after the PR review)

Every finding this record left open or partial has one closing line. "Fixed"
names the ADR-012 section in the commit that carries this section. "Routed"
names the owner that holds the remaining work.

| ID | Closure |
| --- | --- |
| FND-001 | Accepted as stated: the ADR carries no acceptance criteria, and #213, #214 and #185 own the FRs that make its obligations testable (§14.1). |
| FND-002 | Fixed: the Context citation rule reserves `ADR-012 S<n>` for this record's seams S1–S9 and `ADR-011 S<n>` for ADR-011 stages. |
| Round 2 text | Superseded: the CG and IR tickets are numbered (Codegen #86, Contract IR #141), and the arrow-7 key is a typed `QualifiedName` with AD-016 arrow 7 kept, so no QSpec issue is needed (§8, §13.2 Q1). |

## PR review (QSL PR #234)

Single PR reviewer. Round 1 reviewed 43677c9, and round 2 reviewed the delta
43677c9..10664aa. Sibling records were read at ADR-011 22fa948 (PR #235) and
ADR-013 4152eb8 (PR #236). AD-016 was read at QSpec `origin/main`.
`quire validate --strict --summary` on the ten changed files: 10/10
grammar-clean, 0 findings.

Round 1 findings (at 43677c9) and their status at 10664aa:

| ID | Severity | Summary | Status at 10664aa |
| --- | --- | --- | --- |
| PR-M1 | medium | The §1 family DAG was carried by module imports, which contradicts ADR-011 §6.1 ("family modules never depend on each other"). | resolved: §1 moves shared checked types into the layer-3 `check` core, which matches ADR-011 22fa948 §6.1. |
| PR-M2 | medium | The S5 clause kind sat in layer-5 `value::expression`, above the layer-4 emitter. | resolved: it is in the `check` core in §5.1 S5, §12.2 and §13.2 Q2, matching ADR-013 O-10 at 4152eb8. |
| PR-M3 | medium | §12.2 changed a `Value` module but said no `Value` module changes, and it had no QSpec wire row. | resolved: a clause-kind row for `TemporalTrace` and `Relation`, a QSpec wire row, and the closing sentence are corrected. |
| PR-M4 | medium | The §12 tables named no modules, and the kernel `ValueType` consumers were missing. | resolved: every row names a module or path. The RT, CG, IR and QSpec paths exist at `origin/main`. |
| PR-M5 | medium | The `Relation` evaluate arm mapped to `unsupported`, which is not an O-16 evaluation outcome. | resolved: `Refused(FamilyNotNativelyEvaluable)`, category `refusal` (§2, §8, §13.5), matching ADR-013 O-16 and Q210-3. |
| PR-M6 | medium | Review records had open findings, and nobody had reviewed 43677c9. | resolved: each record has an author-closure table, and this section records the review of 43677c9 and 10664aa. |
| PR-L1 | low | Two owners were named for the solver-absence FR-331 result. | resolved: ADR-013 QC-9 (TK-06); #229 cites it. |
| PR-L2 | low | §1.1 omitted AD-016's single IR `requires-bound` predicate. | resolved |
| PR-L3 | low | The ADR-013 pin at bedfab9 was stale. | resolved: the pin is removed and the citation is by PR. |
| PR-L4 | low | The replay result was cited as O-26. | resolved: O-27. |
| PR-L5 | low | The §9 RT row said "as for QSL" after the QSL row changed. | resolved as text, but see PR-N2. |
| PR-L6 | low | The OBS-004 predicate list had no decider. | resolved: AD-016 WP7, matching ADR-011 22fa948. |
| PR-L7 | low | §13.4 Q2 asks the owner to confirm what QSpec #134 scope item 4 already says. | open: ADR-012:836-837. |
| PR-L8 | low | ADR-013 cited §13.2 Q1–Q4, which had lost their labels. | resolved: the labels are restored and match ADR-013 4152eb8. |
| PR-L9 | low | #187 did not wait on #213 or QSpec #115. | resolved |
| PR-L10 | low | Unsupported alternatives were named outside Alternatives Considered. | partial: ADR-012:234 ("Neither is an object-safe plug-in interface"), :254 ("does not try productions in order") and :487 ("It is not a `static`, …") remain. |
| PR-L11 | low | The SR-480 heading had no id, and SR-474 said S1–S8. | resolved |

New findings from the delta or from a cross-record contradiction (round 2):

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| PR-N1 | medium | Capability vocabulary owner. ADR-012 names #229 as the owner of the vocabulary and wire spelling. ADR-013 4152eb8 (lines 30, 425, 441, 764, 854) and ADR-011 22fa948 (line 45) name QSpec #134 (FR-290) as owner, with #229 aligning QSL to it. Consequence: the DA-11 authority table assigns the vocabulary to two owners. Fix: vocabulary and wire spelling are QSpec #134's (FR-290); #229 specifies QSL's capability and aligns to it. | ADR-012:58, :64, :380, :413, :668, :672, :848 |
| PR-N2 | medium | The §9 RT row keys RT's function lookup by a `QualifiedName`. ADR-013 R-06 allows exactly one name lookup after the check stage, the replay executor's (OQ-5). Consequence: a second post-check name resolution, which R-06 forbids. Fix: key the RT lookup by the checked declaration node id, or have ADR-013 amend R-06. | ADR-012:657; ADR-013 4152eb8:85 |
| PR-N3 | medium | ADR-011 still says a family that sits out a stage returns "a typed `unsupported` refusal". That contradicts ADR-012 §2 and §13.5 and ADR-013 O-16 (`Refused(FamilyNotNativelyEvaluable)`, category `refusal`, at S6a). Consequence: #214 gets two rules for the `Relation` arm. The fix belongs in ADR-011. | ADR-011 22fa948:801-804 |
| PR-N4 | low | §13.5 still says "every S1 dispatch seam has one arm per family". The delta's S1 row moved the hook calls to S2 and S3 arms. ADR-011:802 repeats the old text. | ADR-012:847 |
| PR-N5 | low | The SR-476 FND-018 closure names its trigger (the merge of #235 and #236) but no owner. SR-477 FND-019 names both an owner (the RT owner) and a trigger (the owner files the ticket). | integrity.md:267 |

PR review verdict at 10664aa: CHANGES (PR-N1, PR-N2, PR-N3).

### Author response (after PR review round 2)

Fixed in the commit that carries this section, under Agent A's rulings:

| ID | Response |
| --- | --- |
| PR-N1 | Fixed: agent-ix/quire-specification#134 (FR-290) owns the capability vocabulary and wire spelling. #229 (QSL PR #237) aligns QSL's specification to it and holds the claim form → kind table (FR-057). The Context table, the consumption sentence, the §2 Requirements row, S7, §5.2, OBS-012, DA-11, §12.1, §12.2, §13.3, §13.4 and §13.5 all say so. |
| PR-N2 | Fixed: the §9 RT row keys the function lookup by the checked declaration node id (`NodeKey`, ADR-013 O-04), and the checker resolves the name (R-06). The QSL row names the replay executor entry as the one lookup by `QualifiedName` that R-06 allows. |
| PR-N2 (update) | superseded: RT keys by `WireNodeId` (0d4a15e) |
| PR-N3 | Routed to ADR-011 (#209), per Agent A. |
| PR-N4 | Fixed: §13.5 says the S2 and S3 hook matches have one arm per family, and the S1 stage-participation table has one entry per family. |
| PR-N5 | Fixed: SR-476 FND-018 names Agent A (the coordinator) as owner, triggered by the merge of #235 and #236. |
| PR-L7 | Fixed: §13.4 Q2 is deleted. agent-ix/quire-specification#134 scope item 4 settles it, and FR-331 there carries the candidate set. |
| PR-L10 | Fixed: §2, §3 and §7.1 state what is (closed-enum dispatch, the leading-token selection, the registry passed as an argument). |

Also applied, from the #229 author's questions:
- Candidate matching is on kind alone, and CG `negotiate_*` compares the mode (§1.1, §7.2).
- The `Relation` refinement gates (#191, #192) emit `operation-contract` claims, one per clause implication. `refinement` is for refinement between protocols (§3).
- Each family records the Requirements for its own claim forms. The ADR cites FR-057 for the table (§2).

## Round 3 (PR review, delta 12468e3..eecf825)

Checked against ADR-011 at 1666d02 (QSL PR #235) and ADR-013 at 02a504f (QSL PR #236).

| ID | Status at eecf825 |
| --- | --- |
| PR-N1 | Closed. QSpec #134 (FR-290) owns the vocabulary in every place ADR-012 cites it. #229 appears only as QSL's alignment and the FR-057 claim form → kind table. ADR-013 Context and T-7 say the same. |
| PR-N2 | Closed. RT keys its lookup by `NodeKey` (ADR-013 O-04). The replay executor entry is the one `QualifiedName` lookup, which matches ADR-013 R-06 and OQ-5. |
| PR-N3 | Closed in ADR-011 1666d02 (lines 827-830): the S6a arm returns `FamilyOutcome::Refused(FamilyRefusal::FamilyNotNativelyEvaluable)`. |
| PR-N4, PR-N5, PR-L7, PR-L10 | Closed. |

New content checked:
- §12.3 treats the descriptor as the FR-331 provider manifest, converted by QSL `route`. This matches ADR-013 T-7 and C-28, and ADR-011 E7.
- `FamilyOutcome { Evaluated(kernel::Outcome), Refused(FamilyRefusal) }` is in the layer-3 `check` core, and F `diagnostic` maps it to `refusal`. This matches ADR-011 layer 3 and S6a, and ADR-013 O-16 and T-6.
- The family `check` return is renamed `CheckOutcome`, and no sibling names that type differently.
- The §13.5 row says RT gets `NodeKey`s only in process, through S6a, which matches ADR-013 O-04.

The round found nothing new. `quire validate --strict --summary` passes on the ADR and the changed records.

PR review verdict at eecf825: PASS.

## Round 4 (PR review, delta fdcdc41..e71986a)

Checked against ADR-013 at 5d08cd7 (QSL PR #236) and FR-057 at the head of QSL PR #237 (`task/229-capability-spec`).

- RT lookup by `WireNodeId`, with RT holding no `NodeKey` (§9, §13.5): matches ADR-013 O-04 (line 166) and R-06.
- `FamilyRefusal::catalog_code()` yields the code, and F maps the code to a category (§2, §13.5): matches ADR-013 O-16 and O-17 (lines 366-370 and 404-405).
- Backend absence against capability absence (§7.3, §8), and exactly one kind per item, where kind none requests no backend (§1.1, §2, §7.2): matches FR-057's claim-form table, its absence table and AC-10.

New findings (low, not blocking):

| ID | Sev | Finding | Location |
| --- | --- | --- | --- |
| PR-N6 | low | §12.2 is still conditional ("if FR-057 assigns one; otherwise none", "S7 only if a kind is added", "when a kind is assigned"). FR-057 now assigns `operation-contract` to frame obligations, an existing kind, so §12.2 can state that and drop the S7 condition. | ADR-012:770, 780 |
| PR-N7 | low | The §7.2 empty-set row and the §7.3 warning name only the kind. FR-057 and FR-290-AC-4 also name any backend the request named. | ADR-012:535, 562-563 |

`quire validate --strict --summary` passes.

PR review verdict at e71986a: PASS (PR-N6 and PR-N7 are optional wording fixes).

### Author closure (after PR review round 4)

| ID | Closure |
| --- | --- |
| PR-N6 | Fixed: §12.2 states that FR-057 (QSL PR #237, merged) assigns frame obligations `operation-contract`, an existing kind, so S7 is unchanged. The conditional wording is removed. |
| PR-N7 | Fixed: the §7.2 empty-candidate-set row and the §7.3 warning name the item's kind and any backend the request named (FR-057, FR-290-AC-4). |
