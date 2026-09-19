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

Round 2. Reviewed commit: 432e615 (branch `task/206-observed-architecture`),
file `spec/decisions/ADR-010-observed-architecture-baseline.md` and its index
row in `spec/spec.md`. Round 1 reviewed faa1731. This round checks each round-1
finding against 432e615 and reviews the revised parts for new scope-boundary
problems: §1.2, the §7 owner rule and remaps, the §7.5 rule and §8.

Sources checked: the bodies of #205, #209, #210, #211 and #229; the ARCH-01
comment on #207 (issuecomment-5743530928); the bodies of all 68 open QSL
issues (`gh issue list --state open`), scanned for downstream issue
references; and the code at the ADR's Context revisions (QSL de627b5, IR
553b6d1, RT d97bc0b, QSpec 3a79dce).

Coordinator constraint applied: each §9 item has one owner among #209, #210 and
#211, and #229 may only be a secondary input.

What holds:

- §1.2 checks all seven #205 "Ownership boundaries" statements against the
  code. OBS-038 and OBS-039 record the replay-ownership disagreements. The
  cited facts hold: `runtime::execute` at `QSL:runtime/execution.rs:97`,
  `CheckedPackage::call` at `QSL:value/expression/mod.rs:635`, IR
  `replay_with_native_runtime` calling `runtime::execute` at
  `IR:src/kani/replay.rs:88`, no `replay` anywhere in RT `src/`, and AD-016
  arrow 7 naming the QSL executor at line 236–238.
- §8 now gives one merge order, #228 → #204 → #200, and it matches the ARCH-01
  rulings and §3. The deferred items gated on Layer 1 (timed-refund on #211,
  QSpec PR #59 on #210, QI PR #2 on #209, QSpec PR #76 feeding #211) and the IR
  keep PRs that change OBS facts are all listed.
- The §7 owner rule is stated, and the §7.2 and §7.3 remaps follow it and
  agree with #210 "Families in scope".
- Decision 3 gives #229 a secondary role on OBS-012, OBS-013 and DA-11. That
  fits the coordinator constraint and #229's scope (the six FR-290 kinds, and
  which backend advertises support).

What remains: §7.5 now states a rule the program can cite, but the listed set
does not match that rule. The summary count claims "every" downstream issue,
which is false. Some §7.1 and §7.2 cells file OBS or DA items under a ticket
that §9 does not name as their owner.

Verdict: ACCEPT WITH FINDINGS. No finding is blocking (high). All 11 round-1
findings are resolved or partly resolved.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The §7.5 downstream set does not match its own rule, and the Summary count "Downstream issues mapped 28: every downstream issue cited by a mapped QSL issue body, §8 or §9" is false. The rule reaches issues the table leaves out: QSpec #63 (cited by #1), QSpec #13 (cited by #42), QSpec #104 (cited by #155, whose FR-208 work §7.3 maps to #210), spec-objects-business #8 (cited by #133) and quire-wasm #6 (cited by #207). The table also lists two issues as cited in places that do not cite them. IR #137 is "cited by §8, OBS-027", but neither §8 nor OBS-027 names IR #137. RT #51 is "cited by §8 (RT PR #52 on #207)", but §8 has no RT PR #52 row. Both come from ARCH-01, not from ADR-010. A smaller gap: the "Cited by" cell for QSpec #112 leaves out #190. Fix: add the five missing issues, marked "unrelated" where no Layer 1 decision is consumed (quire-wasm #6 is a #205 non-goal). Either widen the rule to "or cited by the ARCH-01 comment", or give IR #137 and RT #51 a real citation in §8 or §9. Update the Summary count. | ADR-010 Summary counts, §7.5, §8; #1, #42, #133, #155, #190, #207 bodies; ARCH-01 §2a |
| FND-002 | low | In some §7.1 and §7.2 "decision consumed" cells, the ticket named disagrees with the §9 owner of the items in the parentheses. #215 lists "#209 (OBS-031, OBS-034)", but OBS-034 belongs to #211. #217 lists "#209 (…, OBS-027, …)", but OBS-027 belongs to #211. #231 lists "#211 (OBS-002, OBS-027, OBS-028, X6)", but OBS-002 and OBS-028 belong to #209. #222 in §7.1 and #189 in §7.2 attribute DA-12 to #210, but §9.3 gives DA-12 to #211. The §7 owner rule adds to the ambiguity: it gives the "unbounded" family to #210 and the "accounting representation" to #211. Both #210 ("finite, bounded, and unbounded execution/proof modes") and #211 ("finite/bounded/unbounded limits") name bounds, and the rule does not say which ticket owns what. Fix: in each cell, group the items under their §9 owner, for example "#209 (OBS-031); #211 (OBS-034)". Add one clause to the owner rule: the bounded or unbounded mode belongs to the #210 family contract, and the limit or budget representation (DA-12) belongs to #211. | ADR-010 §7 owner rule, §7.1 #215/#217/#222/#231, §7.2 #189, §9.2, §9.3 DA-12; #210 Families in scope; #211 Objects requiring decisions |
| FND-003 | low | The §8 exclusion clause ("PRs whose disposition names no Layer 1 ticket") drops QSpec PR #16. ARCH-01 classifies that PR as "revise", with boundary "QSpec vocabulary (capabilities; must align with #185 / FR-290)". It is capability-vocabulary WIP inside #229's scope and DA-11. A #210 or #229 reader of ADR-010 will not see it. Fix: add a §8 row for QSpec PR #16 (revise; FR-290 capability vocabulary; input to DA-11 and #229). Or widen the exclusion clause so that it names the PR and states why it is excluded. | ADR-010 §8, Decision 3, §9.3 DA-11; ARCH-01 §2a QSpec PR #16; #229 Scope |
| FND-004 | low | The negative evidence in the §1.2 Runtime row, "absent: `replay` in `src` (RT, case-insensitive)", does not follow the Evidence convention. The convention defines `absent:` as `git grep -n -F` at the named revision, which is case-sensitive. The claim holds: `git grep -n -i -F replay d97bc0b -- src` in RT returns nothing. But the cell uses an undefined form, and the repository is named in a parenthesis instead of by a prefix. OBS-038 repeats the same form. Fix: add a case-insensitive variant (`-i`) to the Evidence convention, or write the cell as two case-sensitive absences (`replay`, `Replay`) with an `RT:` path. | ADR-010 Evidence convention, §1.2 Runtime row, OBS-038 |

## Round 1 resolution

Round-1 findings are those of SR-464 at faa1731. Tally: 10 resolved, 1 partly
resolved, 0 unresolved.

| Round-1 ID | Severity | Status | Reason |
| --- | --- | --- | --- |
| FND-001 | high | resolved | §1.2 checks all seven #205 ownership statements against code (0 agree, 5 partial, 2 disagree). OBS-038 records that QSL and IR host execution and replay that #205 gives to RT (owner #209). OBS-039 records that AD-016 and #205 name different replay owners (owner #211, secondary #209). The evidence checks out at QSL de627b5, IR 553b6d1, RT d97bc0b and QSpec 3a79dce. |
| FND-002 | high | resolved | §8 table notes and prose agree on #228 → #204 → #200, and cite ARCH-01 "Rulings applied" and §3. That matches issuecomment-5743530928. |
| FND-003 | medium | resolved | Under the coordinator constraint: Context and Consequences name #229 as a Layer 1 ticket. Decision 3, §7.1, §9.2 OBS-012 and OBS-013, and §9.3 DA-11 make #229 the secondary input on its FR-290 scope. |
| FND-004 | medium | resolved | OBS-031 is now owned by #209, with #211 secondary for the pin versus current-head rule, and it cites the ARCH-01 deferral of QI PR #2. |
| FND-005 | medium | resolved | §8 adds every ARCH-01 item whose disposition names a Layer 1 ticket (timed-refund, QSpec #59, QI #2, QSpec #76), the IR keep PRs that change OBS-027 and OBS-036, and FCD PR #200. The out-of-scope remainder is sent to #207. The QSpec PR #16 edge case is new finding FND-003. |
| FND-006 | medium | resolved | The §7 owner rule is stated. #121, #188 and #198 are remapped to #210, and so are #176, #174 and #147. #188 names #222 for boundedness. |
| FND-007 | medium | resolved | OBS-033 and OBS-035 now say the string sites and the reader conformance belong to IR, CG and RT, and name the decision the Layer 1 ticket makes. OBS-027 and OBS-036 name IR PR #139, IR #140 and IR PR #138. |
| FND-008 | medium | partly resolved | §7.5 now uses a rule the program can cite, and it adds QSpec #112–#116, QSpec #124 and FCD #199. The listed set still does not match that rule (new finding FND-001). |
| FND-009 | low | resolved | The §2.6 column is now "Reference that names it". Absent stages carry X1–X7 labels, not invented lane slots. Lane C is numbered C1–C7 with no gap. |
| FND-010 | low | resolved | OBS-005 is owned by #211, with #209 secondary for crate existence and direction. |
| FND-011 | low | resolved | The Context names all four Layer 1 tickets. The `spec/spec.md` row lists no owners and is still accurate, so it needs no change. |

## Round 2 resolution (author)

Recorded by the authoring agent; the round-2 verdict stands.

- FND-001 resolved: the §7.5 rule covers open and closed issues and PRs,
  including the ARCH-01 comment; QSpec #104, #63, #13, quire-research #28,
  spec-objects-business PR #8 and quire-wasm #6 added; IR #137 and RT #51 cite
  the ARCH-01 comment; #190 added to QSpec #112. Count 34.
- FND-002 resolved: §7.1 cells follow §9 owners; OBS-039 moved to #209 so all
  replay items share one owner; the owner rule splits bounds (mode #210,
  representation DA-12 #211).
- FND-003 resolved: QSpec PR #16@199adc3 added to §8 as revise, feeding #229
  and DA-11.
- FND-004 resolved: the convention defines `absent-i:`.
