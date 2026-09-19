---
id: SR-464
title: "Scope-boundary review of ADR-010 observed architecture baseline"
type: SpecReview
analysis: scope-boundary
scope: "spec/decisions/ADR-010-observed-architecture-baseline.md"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-010
    type: reviews
---
# Scope-boundary review: ADR-010 observed architecture baseline

## Summary

Reviewed commit: faa1731 (branch `task/206-observed-architecture`), file
`spec/decisions/ADR-010-observed-architecture-baseline.md` and its index row in
`spec/spec.md`. The review checked five things against issues #205, #206, #207
(ARCH-01 comment), #209, #210, #211 and #229:

- whether the #205 repository ownership boundaries (QSL, QSpec, IR, RT, CG) are
  represented;
- whether each OBS, DA and L1-D1 item goes to the Layer 1 ticket whose scope
  owns it;
- whether the record stays inside the #206 non-goals;
- whether the ticket-map scope statement is justified;
- whether the §8 WIP dispositions match ARCH-01.

What holds: the record describes code, not target design. It authorizes no
refactor. The 68-issue QSL map matches `gh issue list --state open` (68 open
issues, same numbers). Leaving downstream tickets with their repositories is
backed by #205 "Existing consumers".

What fails:

- The findings are compared with AD-016 and the #205 program flow, but never
  with the #205 ownership boundaries. The biggest resulting gap is that QSL
  hosts native execution and replay, which #205 gives to Runtime. No finding
  records this.
- #229 is a Layer 1 ticket, but the record leaves it out of the owner set,
  although it owns FR-290 capability vocabulary alignment.
- §8 contradicts itself on merge order.
- Several ticket-map and OBS owner assignments disagree with the ticket scopes
  or with ARCH-01.

Verdict: REJECT. FND-001 and FND-002 are blocking. The other findings are
fixable in the same revision.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The #205 ownership boundaries are never used as a reference, so a boundary disagreement between intended and observed design goes unrecorded, against the #206 Method. The record compares code only with AD-016 and the #205 program flow (Context; §1; §2.5). The #205 "Ownership boundaries" say Runtime owns executable semantic behavior and native replay surfaces, and Codegen owns backend emission and proof harnesses. Observed: QSL owns native execution (A8 `runtime::execute`), value-lane evaluation (C5 `CheckedPackage::call`) and the only replay path (A11). IR performs native replay (OBS-028). AD-016 names QSL `value::expression::CheckedPackage::call` as the replay executor (OBS-036). No finding states that QSL and IR hold responsibilities #205 gives to RT, or that AD-016 and #205 disagree on who owns replay. DA-16 records only the duplicated kernel. Fix: add a subsection, §1b, that checks the five #205 ownership statements (QSL, QSpec, IR, RT, CG) against the code, one row each with a verdict and evidence. Add a finding for each disagreement, at minimum: (a) QSL and IR host native execution and replay that #205 gives to RT; (b) AD-016 arrow 7 and #205 name different replay owners. Assign (a) to #209 and (b) to #211, with #209 as secondary. | ADR-010 Context, §1, §2.1 A8/A11, §2.3 C5, OBS-028, OBS-036, DA-16; #205 Ownership boundaries; #206 Method |
| FND-002 | high | §8 contradicts itself on merge order. The table notes say #228 "merges first" and #204 "merges second". The prose says "#204 → #228 → #200 as set by the #205 coordinator". The ARCH-01 ruling on #207 says #228 → #204 → #200. The coordinator ruling that §8 says governs has no evidence path, which breaks the record's own rule that every assertion carries evidence (#206 Deliverables). A reader cannot tell which order is in force. Fix: keep one order. Either cite the coordinator ruling (comment URL) and update the table notes to match it, or use the ARCH-01 order. Merge order is #207's process detail, so the simplest fix is to drop the order from ADR-010 and point to #207. | ADR-010 §8; #207 ARCH-01 comment (Rulings applied, §3) |
| FND-003 | medium | #229 is a Layer 1 ticket in #205 (Layer 1 list and dependency graph `#208 → #209/#210/#211/#229`), but ADR-010 treats Layer 1 as #209, #210 and #211 only (Context; Decision 1 and 3; §9 owner tally). OBS-012's core claim ("FR-290 states QSL's enum aligns to its 6; the enum has 4 different variants") and OBS-013 (FR-290 names a QSL Kani backend as a registrant) are #229's stated scope: adopt the six FR-290 kinds, and define which backend advertises support. §7.1 lists #229 only as "secondary input to #210", which reverses #229's role. Fix: add #229 to the Layer 1 owner set in Context and Decision. Make #229 the primary owner of OBS-012's vocabulary claim and of OBS-013, with #210 as secondary. Keep DA-11 (capability authority across stages) with #210, with #229 as secondary. Update the §9.2 tally and the §7.1 #229 row. | ADR-010 Context, Decision 1/3, §7.1, §9.2 OBS-012/OBS-013, §9.3 DA-11; #205 Layer 1; #229 Scope |
| FND-004 | medium | The OBS-031 owner disagrees with ARCH-01. ADR-010 gives OBS-031 (no current-head integration lane in QI) to #211. The ARCH-01 comment defers QI PR #2 because it "waits on #209 (stage DAG and cross-repo dependency direction), which decides whether quire-integration owns the current-head integration lane". Decision 4 says ARCH-01 is the authority for WIP dispositions, so its stated Layer 1 dependency should match. Fix: make #209 the primary owner of OBS-031 (which repo hosts the lane) and #211 secondary (the rule that separates exact release pins from current-head). Or record the split explicitly and cite ARCH-01. | ADR-010 §9.2 OBS-031, Decision 4; #207 ARCH-01 §2a (QI PR #2); #211 Acceptance |
| FND-005 | medium | §8 is incomplete against ARCH-01, yet Decision 4 presents it as the WIP disposition record. §8 lists only the three QSL keep PRs. ARCH-01 also records WIP that is explicitly gated on a Layer 1 decision: the timed-refund spec work (defer, waits on #211), QSpec PR #59 (defer, waits on #210), QI PR #2 (defer, waits on #209) and QSpec PR #76 (revise, feeds #211 and #217). It also records downstream keep PRs that change observed facts: IR PR #139 replaces the `witness: String` in OBS-027, and IR PR #138 corrects FR-031-AC-3 behind OBS-036. A Layer 1 owner reading ADR-010 alone will not see WIP waiting on its decision, which works against the #206 acceptance "sufficient input for #205 Layer 1 decisions". Fix: add rows for every ARCH-01 item whose disposition names a Layer 1 ticket or changes a cited OBS fact, with the owning ticket. State that all other ARCH-01 items (stale-merged, supersede, and the #28 matrix work) are outside Layer 1 and live on #207. | ADR-010 §8, Decision 4, OBS-027, OBS-036; #207 ARCH-01 §2a |
| FND-006 | medium | Several §7.2 and §7.3 ticket-map owners conflict with #210's "Families in scope". #210 lists state and model population, lookup, inheritance and dispatch; temporal and trace semantics; and refinement and model-to-implementation relations as families it owns. ADR-010 maps #121 (state, model population), #188 (temporal and trace) to #209, and #198 (model-to-implementation relation) to #211. It maps #176 (dispatch through inherited operations), #174 (query-only dispatch restriction) and #147 (dispatch preconditions and navigation) to #211, but these consume the dispatch and inheritance family contract. It maps #191 and #192 (refinement) to #210, which is consistent, so the treatment of refinement and #198 differs for no stated reason. Fix: remap #121, #188 and #198 to #210. #188 can name #222 as its boundedness consumer. Remap #176, #174 and #147 to #210, or mark them "unrelated", meaning local normalize fixes that consume no Layer 1 decision. The #205 program says feature tickets stay with their epics, which supports that option. State the rule used: the family contract goes to #210, the stage edge to #209, and the object or representation to #211. | ADR-010 §7.2, §7.3; #210 Families in scope; #205 Existing consumers |
| FND-007 | medium | OBS-035, and in part OBS-033, record IR-, CG- and RT-internal conformance defects but give them to QSL Layer 1 tickets with no downstream owner named. OBS-035 (IR `CheckedPackageRefusalCode` has 13 variants against the 16 in QSpec FR-322) is an IR reader lagging a QSpec contract. Under #205, IR owns its representation and QSpec owns the contract, so no #211 decision resolves it. OBS-033 lists string dispatch inside IR, CG and RT. #210 scopes QSL semantic families, and #205 leaves downstream tickets with their repositories. Fix: add a "downstream owner" column to §9.2, or a note, for findings whose resolution sits in another repository (OBS-027 IR #137 / PR #139, OBS-035 IR, OBS-033 IR, CG and RT, OBS-036 IR #140). Keep the Layer 1 ticket only for the cross-repo decision it actually makes. For OBS-035, say that #211 decides the refusal-outcome contract and IR owns reader conformance. | ADR-010 §9.2 OBS-033, OBS-035; #205 Ownership boundaries, Existing consumers; #211 Required design |
| FND-008 | medium | The §7.5 selection rule is not evidenced, and it leaves out downstream tickets the record depends on. §7.5 maps "downstream tickets named by the program", but #205 names no IR, RT, CG or QSpec ticket numbers. The list appears to come from the non-durable program draft that ARCH-01 cites (`/tmp/qsl-architecture-program.md`). §9.1 L1-D1 and §7.1 rely on QSpec #112–#116 and #124, including QSpec #116, the prerequisite of #185 and #229. ARCH-01 relies on FCD #199 and RT #51. Of these, only RT #51 appears in §7.5. Excluding downstream issues in general is justified by #205, but the named subset has no citable basis and is not closed under the record's own references. Fix: base §7.5 on a durable rule, for example "every downstream issue cited in §8, §9 or a Layer 1 ticket body", and list every issue that rule matches, adding QSpec #112–#116 and #124 and FCD #199. Or cite the durable source of the named-by-program list. | ADR-010 §7 scope statement, §7.5, §9.1 L1-D1, Consequences bullet 4; #205 Existing consumers; #229 header |
| FND-009 | low | §2.6 "Where it would sit" places absent stages in the target design by inventing positions B11, B12 and C8 ("after B7 (B11)", "after C4 (C8)"). Lane C also skips C7 and C8 with no explanation. Placing absent stages in a DAG is #209's decision (legal stages and edges). A descriptive record should say what the reference design names, not assign stage slots. Fix: rename the column to "Reference that names it" and cite AD-016 or the #205 program flow for each row. Drop the invented B11, B12 and C8 labels. Explain the C7 and C8 gap in lane C numbering, or renumber C9 to C7. | ADR-010 §2.3, §2.6; #209 Required design |
| FND-010 | low | OBS-005 (the `quire-exact` crate is absent) is given only to #211. Whether a shared kernel crate exists, and in what dependency direction, is #209's "target crate/module DAG and extraction criteria". #211 owns the canonical owner of the value kernel, which DA-16 already covers. Fix: name #209 as secondary on OBS-005. Or split it into crate existence and direction (#209) and kernel ownership (#211, through DA-16). | ADR-010 §9.2 OBS-005, §9.3 DA-16; #209 Required design |
| FND-011 | low | The `spec/spec.md` index row says "Proposed; observed QSL and backend architecture baseline and Layer 1 decision items (#206)". This is accurate. The index gives no hint that #229 is also a Layer 1 owner (FND-003), and neither does the ADR Context line naming "#209, #210, #211". Fix: when FND-003 is applied, make the Context sentence and any owner list in the index row name all four Layer 1 design tickets. | spec/spec.md ADR-010 row; ADR-010 Context |
