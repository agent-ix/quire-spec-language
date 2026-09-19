---
id: SR-488
title: "Scope-boundary review of ADR-013 canonical type, package and conversion ownership"
type: SpecReview
analysis: scope-boundary
scope: "spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: reviews
---
# Scope-boundary review: ADR-013 canonical type, package and conversion ownership

## Summary

Round 1. Reviewed commit: 660aa25 (branch `task/211-type-ownership`), file
`spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md` and its
index row in `spec/spec.md`.

Question asked: does ADR-013 stay inside #211? It must not decide #209, #210,
#229 or #222 items. Does it assign each cross-repository item to the right
repository, and to QSpec where the item is normative? Does every object name a
code owner and an implementing ticket, with no overlap or gap between QSL, IR,
RT, CG, FCD, QI and QSpec?

What holds:

- Coverage of the ADR-010 routing is complete. §9 decides all 16 findings whose
  primary owner is #211 (OBS-005, 006, 017–027, 032, 034, 035) and all 17 DA
  items routed to #211 (DA-01 to DA-10 and DA-12 to DA-18). DA-11 is correctly
  left to #210.
- The ADR respects AD-016. It reopens no Owner decision and proposes no rename
  (Owner decision 6). The three places where it departs from AD-016 are sent to
  QSpec as OQ-3: the `Witness` stored fields, selection by node id, and the
  kernel contents.
- It defers stage and DAG items to #209: Q209-1 to Q209-7 cover lane deletion,
  the linked form, module paths, the kernel's direction, replay retirement,
  where `Diagnostic` sits, and the QI heads workspace.
- The capability ownership chain in O-19 matches the coordinator ruling: #229
  owns the vocabulary, #213 builds the Rust value type, #185 alone builds the
  registry and routing, and #210 owns selection.
- The witness and replay chain matches the ruling: #231 builds the common
  carrier and #186 adds only its state payload (O-25, O-27).
- R-02 and R-03 restate #211's cross-repository rule. §4 names a test for each
  layer-owned conversion. The `spec/spec.md` row and `contains` relationship
  are correct.

What fails: four boundary problems.

1. O-24 and O-25 give IR-owned types (`KaniOutcome`, `CounterexamplePacket`)
   #231 as their implementing ticket. #231 is a QSL ticket. The ADR also gives
   #231 a "counterexample envelope" that has no place in the §4 flow.
2. O-21 decides the bound taxonomy and its derivation rule. Both are #222's
   design, and the taxonomy does not match #222's list.
3. Q210-1 sends the capability wire spelling and version to #210. They belong to
   #229, with QSpec FR-290 as the normative authority.
4. Work the ADR assigns to RT, CG, IR, FCD and QSpec has no implementing ticket
   in §7. §7 also omits most of the downstream tickets that ADR-010 §7.5
   already routes to #211.

Verdict: REVISE. FND-001 and FND-002 are high (blocking), and each has a
concrete fix.

## Method

- Read ADR-013 at 660aa25 in full, and the `spec/spec.md` diff.
- Read the bodies of #211, #209, #210, #213, #229, #231, #185, #222, #186,
  #212, #215, #226, #131, #217 and #188 (`gh issue view`).
- Read accepted AD-016 at `quire-specification` `origin/main`: the System
  Boundary, arrows 1–7, Replay ownership, the Shared-type strategy, the Crate
  graph, the Domain-package intake boundary and the Owner decisions.
- Read ADR-010 §9.2 and §9.3 for the routing of each item, and §7.5 for the
  downstream tickets already routed to #211.
- Checked every O-01 to O-27 object in four ways. Does it have exactly one code
  owner, and is that owner in the right repository? Is its implementing ticket
  in the owner's repository? Is its normative authority in QSpec when it crosses
  a repository? Is any of its decisions reserved to a sibling ticket?
- Applied the coordinator ownership chains:
  - capability: #229 spec and vocabulary; #213 the Rust `Capability` value type
    and the shared outcome and refusal types; #185 registry and routing only.
  - witness and replay: #231 the common envelopes; #186 only the state payload.
  - bounds: #222 design; #213 implementation.
- Applied the program rules: no compatibility layer, current-state wording, no
  renames.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Ownership of proof results and counterexamples is split between IR and QSL #231 and is never settled. O-24 makes IR `KaniOutcome` the owner and names #231 as the implementing ticket. O-25 makes IR `CounterexamplePacket` the carrier, but says "#231's counterexample envelope carries it unchanged". It also says that adding a missing packet member "is IR work under #231 and IR #137". #231 is a QSL-repository ticket, and its non-goals exclude backend invocation. The AD-016 Replay-ownership table puts the packet in IR. The §4 flow goes from the IR packet through C-12 (CG) to the FR-323 request to the QSL executor, so no QSL counterexample envelope appears anywhere on the path. Three consequences follow. #231 cannot tell whether it builds a QSL type, edits IR, or both. The IR packet crosses from IR to CG as a cross-repository API, yet O-25 names no QSpec authority for it, against R-02. And O-25 has no Serialized-authority or Validation row for the backend witness, although §3 requires both for every object. Fix: (a) Make IR the code owner of `KaniOutcome` and `CounterexamplePacket`, and name an IR implementing ticket (IR #137 and the AD-016 WP9 item). (b) Limit #231 to QSL-side types: the FR-331 result reader, the FR-323 replay request and result, and the FR-351 record carrier. (c) If #231 really owns a counterexample envelope, name its repository, place it in C-11 or C-12, and author it in QSpec. Otherwise state that the packet is an IR crate API that AD-016 exempts from R-02. (d) Add the missing O-25 rows. | ADR-013 O-24, O-25, §4 C-10–C-12 and diagram, §7 #231 row, R-02; AD-016 Replay ownership, arrow 6, arrow 7; #231 Scope and Non-goals |
| FND-002 | high | O-21 decides design that #222 owns. #222's "Required decisions" include the "Separation of authored semantic bounds, execution resource bounds, proof bounds, and profile ceilings". O-21 fixes a different set of four kinds: it adds "tool budget" and leaves out "profile ceilings". It also fixes the derivation rule ("The only derivation is authored bound → proof bound, by CG negotiation"). O-20 gives the proof-mode vocabulary to "#210 with #222", but O-21 then fixes the conversion path and the no-narrowing rule itself. #222 cannot now reach a different taxonomy without reopening ADR-013. And #213, which "implements the QSL bound types to that design", has two designs to follow. ADR-010 routes DA-12 to #211 for the duplicate representations only: `value::accounting` against `model::accounting`, and the budget formats. The ADR has no "Questions for #222" section, although it defers to #222 in O-20 and O-21. Fix: limit O-21 to DA-12 and OBS-025. That covers which repository and type owns each existing representation, the fold of `model::accounting` into the kernel meter, and the lane-private budgets. State that the set of bound kinds, including profile ceilings, and any derivation between them are decided in #222. Add a §8 "Questions for #222" table: bound taxonomy, derivations, meaning of an absent bound, trace position, interval and horizon. | ADR-013 O-20, O-21, §7 #222 row, §8; #222 Required decisions; #213 Scope; ADR-010 §9.3 DA-12 |
| FND-003 | medium | Capability questions are routed to the wrong owner, and capability conversions are missing from §4. Q210-1 sends "wire spelling and version of capability values" to #210. #229's scope is "canonical spelling, identity, ordering/non-ordering semantics, serialization authority, and version behavior", and the normative vocabulary is QSpec FR-290 (QSpec #116). O-19 never names FR-290 as the serialized authority, although R-02 requires the cross-repository spelling to live in QSpec. AD-016's Capability row makes the IR "closed obligation-capability enum parsed at the wire edge" a layer-owned representation. Under R-03 that enum needs a named conversion and test, but §4 has no capability row. The O-19 row "Counterexample → replay" is about backend identity and a tool pin, which are not capability values. Fix: send wire spelling, version and unknown-kind behavior to #229, with FR-290 as the authority, and keep only the backend-identity and selection question in Q210-1. Add a §4 row for the wire ↔ QSL `Capability` and wire ↔ IR obligation-capability conversions, each with its owner and test. Move the backend-identity row out of the capability-crossings table. | ADR-013 O-19, §4, §8 Q210-1; #229 Scope; #185 body (FR-290); AD-016 Shared-type strategy Capability row, arrow 1 |
| FND-004 | medium | O-20 (proof modes) has no code owner, and `requires-bound` has two owners. The O-20 Owner row only defers ("decided in #210 with #222"). The public type is "CG's disposition plus the declared finite domain", but the implementing ticket is #213, a QSL ticket that cannot implement a CG type. `requires-bound` is also owned three ways: an IR predicate (O-20 Validation, O-21 "Proof bound: IR `requires-bound` forms; CG declared per-argument domain") and a CG disposition (O-16 Negotiation row). Under R-01 each concept needs one owner. Fix: split O-20. The QSL typed proof request and mode (built by #213 to the #210 and #222 design) is one object. The CG disposition, CG code owner with a CG ticket, is another. The IR `requires-bound` predicate, IR code owner, is a third. Link them with a C-row (IR predicate → CG disposition), as AD-016 "Frames and unbounded constructs" states. | ADR-013 O-16, O-20, O-21, R-01; AD-016 arrow 5, "Frames and unbounded constructs" |
| FND-005 | medium | Work assigned to downstream repositories has no implementing ticket, although the brief requires one for every object. §7 names QSL tickets, IR #137 and IR PR #139, and nothing for RT, CG, FCD or QSpec. Four groups of work are unassigned. **RT:** move to `quire-exact` and delete RT `exact` copies (O-13, DA-16, OBS-032); C-06. **CG:** C-11, C-12, the O-26 replay adapter, O-27 parity, removing the OBS-034 literals, and the negotiation disposition. **FCD:** the O-01 producer and C-01 fixtures. **QSpec:** the OQ-2 and OQ-3 amendments, and FR-351/FR-352. ADR-010 §7.5 already routes these downstream tickets to #211: QSpec #81, QSpec #114, IR #137, CG #50, FCD #172, FCD #173, FCD #199 and spec-objects-business PR #8. ADR-013 cites only IR #137. §7 also files "FR-322 code completeness (OBS-035)" under IR #137, which is about typed counterexamples, not the checked-package reader. Fix: add §7 rows by repository. Name CG #50 for C-11, C-12 and O-26 parity; QSpec #114 for FR-351/FR-352 and OQ-2; FCD #172, #173 and #199 for O-01 and C-01; and QSpec #81 where it applies. Where no ticket exists (RT kernel consumption, CG OBS-034 literals, IR OBS-035 reader conformance, the QSpec amendments for OQ-3), write "ticket to be filed by #211" rather than leaving the cell empty. Move OBS-035 off IR #137. | ADR-013 §3 O-01, O-13, O-17, O-23, O-26, O-27, §4, §7, §8 OQ-2, OQ-3; ADR-010 §7.5; #217 Cross-repository coordination |
| FND-006 | medium | O-16 adds an object with no owner row, and it conflicts with the three-family rule. The implementing-ticket paragraph says #213 builds "the QSL-side categories" and #231 carries them "unchanged in the proof-result, witness and replay envelopes". But the O-16 family table has only three families. Proof results are IR `KaniOutcomeKind` mapped to FR-331, and families are "never converted into one another except by the total maps". A category type carried in a proof-result envelope is a fourth representation. It has no owner, public type, serialized authority or C-row. C-08 covers only kernel `Outcome` → FR-323. Fix: add the category type to the O-16 table with its owner (QSL, #213), public type and serialized authority (none, or the FR it maps to). Add C-rows for FR-331 result → category and for disposition → category, and state that a proof-result envelope carries the FR-331 result, not a converted QSL category. | ADR-013 O-16, O-24, §4 C-08, C-09; #213 Scope; #231 Scope |
| FND-007 | low | §9 lists only the items whose primary owner is #211. ADR-010 also names #211 as the secondary owner of five items: OBS-001, OBS-031, OBS-037, OBS-039 and OBS-041. O-23 addresses OBS-031, and R-10 and O-15 address OBS-037, but §9 does not say so. OBS-041 (a second quire-rs revision in the lock through PR #200) has no rule: O-23 covers Cargo pins against literals and vendored QSpec copies, but not two revisions of one dependency. O-26 and O-27 settle the replay executor that OBS-039 contests (AD-016 against the #205 Runtime statement), and OBS-039 belongs to #209. The ADR should cite AD-016 Owner decision 3 as its basis and leave the #205 conflict to #209. Q209-5 covers only retirement. Fix: add a "secondary" block to §9 for the five items, add an O-23 rule of one revision per dependency (AD-016 heads check 6), and add the OBS-039 disagreement to Q209-5. | ADR-013 §9, O-23, O-26, O-27, §8 Q209-5; ADR-010 §9.2 OBS-001, 031, 037, 039, 041 |
| FND-008 | low | O-01 and C-01 do not name the authority of the FCD → QSL wire. The Serialized-authority row names FR-321 and the v2 lock, which are QSL outputs. It does not name the semantic IR 2.0.0 schema that FCD produces, or the QSpec FR-154 admission table that AD-016's intake typestate cites. The FCD edge is the one cross-repository input where R-02 (authored in QSpec) needs an explicit statement. Fix: in O-01, name the semantic IR schema and its owning repository, name FR-154 as the admission authority, and say whether R-02 applies to it. | ADR-013 O-01, C-01, R-02; AD-016 Domain-package intake boundary, Shared-type "FCD identities" row |
| FND-009 | low | O-04 places `NodeKey` in the `quire-exact` kernel and cites the AD-016 kernel row. But AD-016 also has a row "Semantic identities: `DeclarationKey`, checked node id — Layer-owned — QSL `model::key`". AD-016 therefore names two owners for the checked node id, and ADR-013 picks one without saying so. OQ-3 asks QSpec to amend AD-016 for the kernel contents (c) but not for this row. Fix: add an OQ-3 item asking QSpec to remove the checked node id from the AD-016 "Semantic identities" row, so the kernel row is its only owner. | ADR-013 O-04, §8 OQ-3; AD-016 Shared-type strategy kernel row and Semantic identities row |

## Round 2 (commit 0042691)

Reviewed commit: 0042691 (branch `task/211-type-ownership`). Each round-1
finding was re-checked against the revised ADR-013, and the new text was read
for scope-boundary problems. The review applies the coordinator ruling that
#231 owns the common typed proof-result, witness and replay envelopes on the
QSL side, and that IR stays the code owner of `KaniOutcome` and
`CounterexamplePacket`. FR-323 was re-read at QSpec `origin/main`.

Tally: 8 resolved, 1 resolved differently, 0 unresolved. One new finding.

| Round-1 ID | Severity | Status | Reason |
| --- | --- | --- | --- |
| FND-001 | high | resolved | O-24 and O-25 make IR the code owner of `KaniOutcome`, `Witness` and `CounterexamplePacket`. §7 and OQ-4 list the IR work (packet members, WP9 map) as owner-assigned work that has no ticket yet. #231 is limited to QSL-side envelopes. The proof-result envelope is reached through the FR-331 reader (C-23). R-02 now states that a linked Rust API belongs to the repository that defines it, and that its serialized form is QSpec's. O-25 adds the missing Admission row and a serialized Carrier row (FR-331 `counterexamples` plus an `artifacts` transcript, QC-6). |
| FND-002 | high | resolved | O-21 now records only the existing representations and their owners. The taxonomy, derivations and absent-bound meaning go to #222 through Q222-1 and Q222-2, and Q222-1 names profile ceilings. The row "never convert … except where #222 names a derivation" leaves the derivation rule to #222. |
| FND-003 | medium | resolved | Wire spelling and version move to #229 (Q229-1, aligned with FR-290). Q210-1 keeps only the selection question. C-24 adds the per-layer wire ↔ enum conversion. Backend identity and tool pin leave the capability table and move to O-24. |
| FND-004 | medium | resolved | O-20 names CG as code owner of the disposition and IR as owner of the `requires-bound` predicate, which CG reports. #213 S-6 is limited to the QSL request representation. C-25 links authored bound → disposition, and the CG work is in OQ-4. |
| FND-005 | medium | resolved differently | The revision does not write "ticket to be filed by #211" in each cell. §7 instead adds rows for CG #50, QSpec #114 and the §7.5 downstream tickets. It lists the work no ticket owns (executor entry, v2 emitter, RT and CG kernel adoption, IR packet members and WP9 map, IR reader codes, CG obligation conformance), and OQ-4 makes assigning those tickets an owner action. IR #137 is narrowed to the FR-031-AC-3 witness test. Every object now names a ticket or an OQ-4 entry. |
| FND-006 | medium | resolved | O-16 adds the eight-value QSL category type (#213 S-1) as the target of every map. The proof-result envelope carries both the category and the FR-331 terminal record, related by C-23 (category-preserving). |
| FND-007 | low | resolved | §9 adds a secondary-owner table for OBS-001, 031, 037, 039 and 041. O-23 adds "one lock holds one revision of each git dependency" (OBS-041). Q209-5 now carries the OBS-039 conflict with the #205 Runtime text. |
| FND-008 | low | resolved | The O-01 serialized authority names the Semantic IR 2.0.0 schema, admitted under the QSpec FR-154 table, as the FCD → QSL authority. The FR-321 and v2 lock authority is kept for QSL → IR. |
| FND-009 | low | resolved | O-04 records the AD-016 row conflict, and OQ-3 (e) asks QSpec to list node ids in the kernel row only. |

New finding introduced by the revision:

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-010 | medium | O-25 and O-26 add replay-request members that accepted FR-323 does not have, and no QC row covers them. FR-323 at QSpec `origin/main` has `package`, `selection` ("ordered clause/function identifiers"), `state_environment`, `limits`, `results` and `replay` ("optional counterexample identity and expected originating verdict"). The O-26 public type adds `witness_backed`, arguments keyed by parameter node id, and source digests. The O-25 member list, which the #231 envelope also carries, adds semantic profile selections, occurrence key, proof bounds and declared domains, and trace position. QC-1 covers only the byte provision and the `RawSourceRef` digests, and QC-6 covers only the transcript `artifacts` entry. R-02 says "A missing wire member is a QSpec change (§8 QC table), not a local field", so #231 would have to invent these members locally or stall. The `witness_backed = false` rule (a corpus counterexample is replayed but "never counts as backend evidence") is also an evidence policy that no QSpec contract states. Fix: add a QC row (or extend QC-1 and QC-6) that lists each new FR-323 `replay` or request member (`witness_backed`, parameter-keyed arguments) and each FR-331 `counterexamples` member the envelope requires. Mark #231 and CG #50 as blocked on it in §7. | ADR-013 O-25, O-26, R-02, §7 #231 row, §8 QC-1 and QC-6; QSpec FR-323 Properties |

Round-2 verdict: ACCEPT WITH FINDINGS. No high finding remains. FND-010
(medium) is new.

## Round 3 (commit 4152eb8)

PR #236 re-review of the delta 5609e3a..4152eb8, against ADR-011 at 22fa948
and ADR-012 at 10664aa. The full finding table is in
[base.md](base.md) Round 3. ADR-013 line numbers are at 4152eb8.

- FND-010 stays resolved. QC-8 now lists the `ReplaySource` variant, the
  `QualifiedName`, the #231 envelope members and the executor pin.
- The capability vocabulary and wire spelling belong to QSpec #134, and #229
  aligns to it. This matches the ruling chain and ADR-011 E3.
- QC-18 and QC-19 are QSpec changes and are routed to TK-08.

New findings:

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| PR2-M2 | medium | The accepted AD-016 Packet row changes (`witness: Option<Witness>` → `source: ReplaySource`) with no AD-016 amendment in OQ-3, QC-7 or TK-10. | ADR-013 568, 806, 863, 885 · AD-016 Arrow 6, Replay ownership |
| PR2-L4 | low | QC-15's widened list is beyond the text the owner accepted in OQ-3. | ADR-013 814, 863 |

Round-3 verdict: CHANGES (see base.md).
