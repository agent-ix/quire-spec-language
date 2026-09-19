---
id: SR-480
title: "Scope-boundary review of ADR-012 semantic-family extension contracts"
type: SpecReview
analysis: scope-boundary
scope: "spec/decisions/ADR-012-semantic-family-extension-contracts.md; spec/spec.md ADR-012 row"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: reviews
---
# Scope-boundary review: ADR-012 semantic-family extension contracts

## Summary

Round 1. Reviewed commit: 048deb3 (branch `task/210-family-extension`), file
`spec/decisions/ADR-012-semantic-family-extension-contracts.md` and its
`contains` edge and index row in `spec/spec.md`.

The question: does ADR-012 stay inside #210, leave sibling decisions to their
owners, give each responsibility one owner, and respect the QSL, IR, RT, CG and
QSpec repository boundaries? The fixed ownership chains were taken as given:
#229 owns the capability specification, #213 the Rust `Capability` and outcome
types, #185 the registry and routing, #222 the boundedness design, #209 the
stages and crate DAG, and #211 the canonical types and conversions. AD-016 and
QSpec PR #133 are accepted and not reopened.

What holds:

- Every #210 "Required design" bullet has a section: the shared contract (§2),
  family-owned work (§3), typed subnodes and builders (§4), exhaustive
  extension (§5), capability selection with absence behaviour (§7) and the four
  admissions (§6). Each acceptance bullet has an anchor: change sets (§12), no
  monolithic routine (§4.1), dispatch independent of text, order and ambient
  state (§7.1, §9), the absent-capability outcome (§7.3), and stage coverage
  (§8).
- The sibling table in Context matches the fixed chains for #209, #211, #213,
  #185 and #222. The body cites them as "decided in #NNN" and does not design
  the vocabulary, the `Capability` type, the registry implementation, the mode
  type or the bound representation.
- The four admissions keep AD-016's split: QSL admission is language-only, and
  the checker never reads the registry. The registry computes candidates and
  settles nothing. CG `negotiate_*` settles every disposition from the PR #133
  set.
- All six ADR-010 findings routed to #210 (OBS-003, OBS-004, OBS-012, OBS-013,
  OBS-014, OBS-033), DA-11 and L1-D1 are decided. L1-D1 covers all seven ladder
  tickets that ADR-010 names, and the ADR edits no issue.
- The ADR assigns the IR, CG and RT string-dispatch sites to their own
  repositories and states the contract only.
- The `spec/spec.md` row and the `contains` edge are accurate.

What fails: one sentence contradicts the single negotiation point that PR #133
states. Several work-hand-off rows name tickets whose scope excludes that work.
In two places the ADR decides a #229 question and then asks #229 the same
question. Cross-repository interface changes have no named QSpec contract owner.

Verdict: REJECT. FND-001 is blocking (high) because it contradicts AD-016 and
PR #133. Its fix is one sentence. After FND-001 is fixed, the remaining
findings would support ACCEPT WITH FINDINGS.

## Method

Applied `quoin:spec-scope-boundary-analysis`. For each ADR-012 section, the
review recorded the responsibility decided, its owner and repository. It then
checked that owner against the ticket bodies and the fixed chains. Sources:

- issue bodies of #210, #205, #209, #211, #212, #213, #214, #185, #222, #229,
  #218, #220, #221, #223, #188 and #189 (`gh issue view`), and QSpec #116
  (state only);
- ADR-010 at 8170101 (§7 map, §9 items OBS-003/004/012/013/014/033, DA-11,
  L1-D1);
- QSpec AD-016 at `origin/main`: arrows 1, 4, 5 and 7, the Shared-type
  strategy, "Frames and unbounded constructs", and Decisions;
- QSpec PR #133 diff (FR-290 and AD-010 negotiation-point text, SR-604,
  SR-605).

Boundary allocation used:

| Responsibility (ADR-012 §) | Owner in ADR-012 | Consistent with sibling scope? |
| --- | --- | --- |
| Family catalogue, shared contract, builders, closed seams (§1–§5) | #210 design; #214 implements S1–S4 | yes, except the migration owners (FND-003) |
| Mode type, bound representation (§1.1) | #222 | yes; the mode-carrier question overlaps #229 (FND-005) |
| Vocabulary, unknown-kind rule, absence outcome (§5.2, §7.3, §7.4) | #229 | partly (FND-005, FND-008) |
| Role allocation: who declares, who advertises, who selects (§6, §7.1, §10 DA-11) | #210 | also named in #229 scope (FND-006) |
| Registry value, candidates, routing (§7.1, §7.2) | #185 | yes |
| Per-item disposition (§7.2) | CG `negotiate_*` | yes, except the Consequences wording (FND-001) |
| `negotiate_*` input, replay executor key (§7.2, §9, §13.2) | CG ticket not yet opened; #211 | no QSpec contract owner (FND-002) |
| QSL `negotiate_*` copy removal (§10 OBS-004, §14) | #213 | ADR-010 maps OBS-004 to #185 (FND-004) |
| FR-290 "Kani backend" wording (§10 OBS-013, §13.4) | QSpec via #116 | #116 is closed (FND-009) |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Consequences says "A new backend costs one descriptor plus its own negotiator". The §7.2 table says a single candidate gets "that backend's own settlement of the IR form". Both read as one negotiator per backend. That contradicts AD-016 Decisions ("CG `negotiate_*` is the single capability negotiation point") and the PR #133 text in FR-290 and AD-010 ("Capability negotiation happens at one point"). It matters now: registrants outside CG are already planned (IR SMT/runtime #136, quire-analyze #40 from SR-605). Under this wording each of them would bring its own settlement point. Fix: in Consequences, write "plus its arm in CG `negotiate_*`". In the §7.2 table, write "the settlement `negotiate_*` computes for that backend over the IR form". Add one sentence: a backend implemented outside CG still settles through CG `negotiate_*`, and moving settlement elsewhere would need a QSpec amendment that this record does not make. | ADR-012 Consequences, §7.2; AD-016 Decisions, arrow 4; QSpec PR #133 (FR-290, AD-010); SR-605 |
| FND-002 | medium | The ADR requires two changes to AD-016-fixed cross-repository interfaces but names no QSpec contract owner for either. (a) §7.2 and §14 give CG `negotiate_*` a new input, the candidate set. §13.1 Q2 asks #209 how that input crosses the repository boundary, and §14 leaves the work to a "CG ticket, to be opened by the CG owner". (b) §9 and §8 require the arrow-7 executor entry, which AD-016 fixes as `CheckedPackage::call(&self, function: &str, …)`, to be keyed by a typed identity, and they route the question only to #211. #211's body says "Normative cross-repository wire/API changes are authored in QSpec". #212 fails a scenario that has no "named QSpec contract owner" or no bounded implementation ticket. The IR, CG and RT rows in §14 ("to be opened by the owner") also have no ticket. Fix: add a §13.4 owner question, or a §14 row, naming the QSpec issue that amends AD-016 arrow 4 (the candidate-set input to `negotiate_*`) and arrow 7 (the typed executor key). Alternatively, state that (a) fits AD-016 plus PR #133 as written ("over the capabilities registered backends advertise") and only (b) needs an amendment. Name existing IR, CG and RT tickets, or state that #212 needs them opened before it passes. | ADR-012 §7.2, §8, §9, §13.1 Q2, §13.2 Q1, §14; AD-016 arrows 4 and 7; #211 Required design; #212 Pass conditions |
| FND-003 | medium | §4.3 and §14 give the family code migrations to tickets that exclude implementation, and the `TemporalTrace` migration has no owner. #220, #221 and #223 are mapping and coordination tickets that "own no feature implementation" (#220, #223) or have "No sum/case production code" (#221). Their implementation owners are #120/#121/#164, #187, #191/#192/#198, and #218 for frames. §4.3 says the other migrations "stay with #220 to #223". That range includes #222, which has the non-goal "no … unbounded feature implementation". §14 lists no owner for `TemporalTrace`. §14 also gives #214 "the `Value` migration", and §4.3 has #214 turn all of `infer_form` into a thin seam. #214's scope is function application "as the sole representative family", and it "does not modify unrelated family internals". Fix: in §14, give each family's migration to its implementation owner, with the mapping ticket as the design input: `StateModel` → #120/#121/#164 via #220; `SumCase` → #187 via #221; `ProtocolClause` → #218 plus the QSpec, IR and CG frame tickets via #223; `Relation` → #191/#192/#198 via #223; `TemporalTrace` → #188 via #222. Limit #214 to making the check seam thin for function application. Name the owner of the remaining `Value` forms (#120/#164/#170/#175, or a named child). | ADR-012 §4.3, §14; #214 Scope and Non-goals; #220, #221, #222, #223 Non-goals |
| FND-004 | medium | §10 OBS-004 and §14 ("QSL removal in #213") give the removal of QSL's `negotiate_integer_division`, `negotiate_ieee` and `IeeeBackendCapabilities` to #213. ADR-010's §7 map gives OBS-004 to #185 (row "#185 … #210 (DA-11, OBS-003, OBS-004, OBS-012)"). #213 is limited to shared types and first consumers, and it has the non-goal "introduce backend-specific behavior into shared types". The capability predicates are backend-negotiation code, and #185 already removes the other QSL negotiation path (OBS-003). Fix: give the QSL copy removal to #185 in §10 OBS-004 and §14, next to the composed-linker removal. If #213 is meant, first amend ADR-010 §7 so that both records name one owner. | ADR-012 §10 OBS-004, §14; ADR-010 §7 (#185 row), OBS-004; #213 Scope, Non-goals; #185 |
| FND-005 | medium | Twice the ADR decides a question that it also sends to #229. (a) The §5.2 row "A registration advertising a capability kind outside the #229 vocabulary" decides "refused at registration". §13.3 Q4 then asks #229 for "Unknown-kind behaviour at registration". #229 acceptance requires "unknown-kind behavior are explicit". (b) §1.1 fixes "The mode is a field of the requirement", and §2 fixes `Requirements { capabilities, mode, bound }`. §13.3 Q2 asks #229 whether the requirement carries a mode, and says §7 "does not care which record carries the mode". Once #229 answers, one of the two texts will be wrong. Fix: (a) change the §5.2 cell to "follows #229's unknown-kind rule; never accepted silently", and keep Q4. (b) Change §1.1 to "selection keys on the pair (capability, mode); which record carries the mode is decided in #229 and #222", and describe `Requirements` in §2 as carrying that pair without fixing the field layout. Or drop Q2 and cite this decision as input to #229. | ADR-012 §1.1, §2, §5.2, §13.3 Q2 and Q4; #229 Scope, Acceptance |
| FND-006 | medium | #229's scope includes "Define which stage declares a requirement, which backend advertises support, and which QSL component selects a target". ADR-012 decides exactly that in §6, §7.1 and §10 DA-11. The Context table sums up #229 as vocabulary, identity and version rules, and absence policy, and leaves this bullet out. The allocation follows ADR-010 (DA-11 is owned by #210, with #229 secondary). But as written, two Layer 1 tickets claim the same allocation, and #212 requires that no "unexplained duplicate authority" remains. Fix: add the bullet to the #229 row in Context, noting that ADR-010 DA-11 gives it to #210 with #229 secondary. Add a §13.3 item asking #229 to cite §10 DA-11 for these roles instead of restating them. | ADR-012 Context, §6, §7.1, §10 DA-11, §13.3; #229 Scope; ADR-010 DA-11, §7 (#229 row); #212 |
| FND-007 | medium | §10 OBS-012 says "Language admission is called semantic admission (§6) and is not a capability". AD-016's Shared-type strategy row reads "QSL `Capability` = language admission (FR-290, six kinds)". Its arrow 1 "Capability point" is "Language admission only … Records `capability_report`; negotiates nothing". SR-604 in PR #133 restates "QSL `Capability` stays language admission". The meaning ADR-012 intends matches AD-016: QSL records the requirement kinds as data and negotiates nothing. But the sentence reads as rejecting AD-016's classification. Fix: reword OBS-012 as "QSL's `Capability` is the requirement kind that semantic admission records as `capability_report` data (AD-016 arrow 1). Recording it negotiates nothing." Remove "is not a capability". | ADR-012 §6, §10 OBS-012; AD-016 arrow 1, Shared-type strategy; QSpec PR #133 SR-604 |
| FND-008 | low | §7.4 sends the solver-absence outcome only to #229. #222's "Required decisions" also list "Structured outcomes for unsupported backend, solver absence, bound exhaustion, timeout, cancellation" and "Capability negotiation for liveness and quantifier-capable backends", and the Context row for #222 leaves both out. §7.4 also says "The ticket fixes only that solver absence is never a hold", which does not say which ticket. Fix: say "#229 fixes only …". Add a §13.3 or §13.4 question that asks #229 and #222 to confirm one owner for the solver-absence outcome (#229 for policy, #222 only for bound-specific outcomes such as bound exhaustion). | ADR-012 Context (#222 row), §7.4, §13.3 Q1; #222 Required decisions; #229 Scope |
| FND-009 | low | §10 OBS-013 and §13.4 Q1 route the FR-290 wording fix ("quire-spec-language's Kani backend") "via QSpec #116". QSpec #116 is closed. PR #133 closed it and kept that phrase in FR-290. Fix: route the edit to a named open QSpec issue, or have #229 raise it as part of its FR-290 alignment, and cite that issue in both places. | ADR-012 §10 OBS-013, §13.4 Q1; QSpec #116 (closed); QSpec PR #133 |
| FND-010 | low | The §2 typing-context row says "context type decided in #211". `CheckContext` is a QSL-internal part of the family contract, and #211's object list does not include it. The row already fixes its contents (resolved declarations, type environment, scope stack, limits, meter), and §13.1 Q4 correctly asks #209 where it lives. Fix: set the representation owner to "this record (contents); placement in #209; the identity, limit and meter types it holds are decided in #211 and #222". | ADR-012 §2, §13.1 Q4; #211 Objects requiring decisions |
| FND-011 | low | §13.4 Q4 asks the owner whether "any compatibility" for four-kind QSL artifacts is wanted. That question is in #229's scope ("compatibility behavior for older four-kind QSL artifacts"), and PR #133 left it "to #229". Fix: move Q4 to §13.3 as a question for #229, and keep this record's current statement that it designs no reader. | ADR-012 §13.3, §13.4 Q4; #229 Scope; QSpec PR #133 |
