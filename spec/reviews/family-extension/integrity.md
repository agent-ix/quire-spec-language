---
id: SR-476
title: "Integrity review of ADR-012 semantic-family extension contracts"
type: SpecReview
analysis: integrity
scope: "spec/decisions/ADR-012-semantic-family-extension-contracts.md, spec/spec.md"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: reviews
---
# SR-476: Integrity review of ADR-012 semantic-family extension contracts

## Summary

Reviewed ADR-012 on `task/210-family-extension` at commit 048deb3, plus the
uncommitted one-sentence edit that adds the `ADR-012 S<n>` citation form. Also
reviewed its index row and `contains` edge in `spec/spec.md`. The review checked
four things:

- completeness against the #210 required design and acceptance bullets;
- that every ADR-010 item routed to #210 is decided;
- consistency with accepted QSpec AD-016, merged QSpec PR #133 and the fixed
  sibling ownership (#229, #213, #185, #222, #209, #211);
- atomicity, unique ids and cross-references.

What holds:

- Every routed item is decided. OBS-003, 004, 012, 013, 014 and 033 and DA-11
  are in §10, and L1-D1 is in §11 for all seven ladder tickets.
- Each #210 required-design bullet has a section: the contract (§2), family
  ownership (§3), subnodes and builders (§4), exhaustiveness (§5), four
  admissions (§6), selection (§7) and stage coverage (§8).
- CG `negotiate_*` is kept as the single settling point.
- The registry is an explicit value, and ambiguity never falls back to
  first-wins.
- Vocabulary, `Capability` type, registry and boundedness are cited to their
  owners, not designed here.
- The id is unique: ADR-011 is #209's record on `qsl-arch10`, ADR-013 is #211's
  on `qsl-arch12`, and SR-476 is unused.
- The index row and `contains` edge are present.

The blocking defect: the mode rule in §1.1 makes `requires-bound` unreachable.
§1.1 says a backend that advertises a capability only in bounded mode is not a
candidate for an unbounded requirement. §7.2 settles an empty candidate set as
`unsupported`. So `negotiate_*` can never settle `requires-bound` for an
unbounded construct that has a finite bound available. That contradicts AD-016's
"Frames and unbounded constructs" row, its change scenario 4, and the four
dispositions restated by PR #133.

The other findings are medium or low:
- the per-backend negotiator wording;
- the redefinition of AD-016's QSL `Capability` row;
- capability kinds that do not exist yet for the `Value`, sum/case and frame
  claims;
- unanswered or conflicting sibling questions from ADR-011 and ADR-013;
- incomplete §12 touch sets;
- the trait shape;
- work handed to #213 outside its scope;
- stale or unprefixed citations.

Verdict: ACCEPT WITH FINDINGS (round 2, commit 8fb238b). Round 1 was REJECT
because FND-001 contradicted AD-016 and PR #133. The revision resolves
FND-001. FND-005 is partly open (the mode vocabulary owner), and three new low
findings remain; see "Round 2".

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `requires-bound` is unreachable for unbounded requirements. §1.1 says a bounded-only backend "is not a candidate" for an unbounded requirement, yet "when a finite bound is available, `negotiate_*` settles `requires-bound`". §7.2 settles an empty candidate set as `unsupported`, and `requires-bound` comes only from the "exactly one" row. Kani advertises only bounded mode, so an unbounded construct that has a finite bound always settles `unsupported`. AD-016 ("Frames and unbounded constructs", scenario 4) and FR-290/AD-010 as amended by PR #133 need CG negotiation to map IR `requires-bound` to `requires-bound` when a finite bound is available, and to `unsupported` otherwise. Fix: match candidates on capability kind only and pass the mode to `negotiate_*` with the candidate set. Then `negotiate_*` settles `requires-bound` for a bounded-only candidate when a finite bound is available, `unsupported` (warned) when none is, and never `supported`. Update §1.1, §5.2 row 3, the §7.2 table and §7.3 to match. | ADR-012 §1.1, §5.2, §7.2, §7.3 · AD-016 "Frames and unbounded constructs", scenario 4 · QSpec PR #133 (FR-290, AD-010) |
| FND-002 | medium | Two readings of "single negotiation point". §7.2 "exactly one" says "that backend's own settlement of the IR form", and Consequences says a new backend costs "one descriptor plus its own negotiator". One reading puts a negotiator in each backend repository, which would be a second negotiation point beside CG `negotiate_*` (AD-016, PR #133). The ADR also does not say where settlement happens for a backend outside CG, such as the quire-analyze #40 implication backend or the tl-mltl liveness backend #188 needs. The backend set is open, but CG `negotiate_*` arms are closed. #212 scenario 7 (add a backend without syntax or checking authority) depends on this. Fix: say that every settlement is an arm of CG `negotiate_*`, keyed on the typed `BackendId` or backend kind. Say what a non-CG backend contributes (a descriptor, plus a CG arm written by hand). Change the Consequences sentence to "one descriptor plus one CG `negotiate_*` arm". | ADR-012 §7.2, Consequences · AD-016 arrow 4 · FR-290 "Backend absence" · #212 scenario 7 |
| FND-003 | medium | OBS-012 changes AD-016's meaning of QSL `Capability`, although the ADR claims to adopt AD-016 unchanged. AD-016's shared-type row reads "QSL `Capability` = language admission (FR-290, six kinds)", and its arrow 1 capability point reads "Language admission only (QSL `Capability` …)". ADR-012 §10 OBS-012 says "Language admission … is not a capability" and gives `Capability` one meaning: the kind a claim requires and a backend advertises. Both may intend "recorded at admission, negotiates nothing", but as written they conflict. Fix: add one sentence to OBS-012 stating how it reads AD-016's row: the QSL `Capability` values are recorded during language admission as `capability_report` data, and they are not an admission decision. If that reading does not hold, route an AD-016 wording note to QSpec with the FR-290 edit in §13.4. | ADR-012 Context, §6, §10 OBS-012 · AD-016 arrow 1, Shared-type strategy |
| FND-004 | medium | The selection mechanism has no capability kinds for the families it is meant to serve. FR-290's six kinds are protocol claim kinds. §13.4 Q2 says the value, state and temporal claims have no confirmed kind, and §13.3 Q3 says the same for frame and sum/case. But §12.1 asserts "Requirements: none beyond the `Value` family's … S7 (explicit arm)", and §12.2 assumes "the frame obligation's capability kind (from #229)". Adding a sum/case form adds no capability kind, so S7 is not forced. Until Q2 and Q3 are answered, §7 cannot select Kani for the #217 function exemplar. #212 forbids passing by deferring an ambiguity to implementation. Fix: make §12.1 and §12.2 conditional on #229's answer, and drop the S7 entry from §12.1. List §13.3 Q3 and §13.4 Q2 as inputs that must be resolved before #212, and name the owner of each. | ADR-012 §7, §12.1, §12.2, §13.3 Q3, §13.4 Q2 · FR-290 · #212 pass conditions · #217 |
| FND-005 | medium | Questions that sibling Layer 1 records send to #210 are unanswered or conflict with ADR-012. (a) Mode. ADR-013 O-20 says the mode "is settled by CG negotiation … not a QSL-emitted value", and that the vocabulary is "decided in #210 with #222". ADR-012 §1.1 and §2 put the mode in family-emitted `Requirements` and send the mode type to #222 alone. (b) Capability wire spelling and version. ADR-013 Q210-1 sends them to #210; ADR-012 sends them to #229. (c) ADR-013 Q210-3 (family result → O-16 outcome categories) and Q210-4 (FR-351 witness used unchanged) have no answer. (d) ADR-011 asks #210 which v2 family forms replace IR's predicate and temporal admission of QSL types; ADR-012 has no answer. (e) The owner of the bound representation is #222 in ADR-012 but DA-12 → #211 in ADR-010 §7. ADR-012 does not cite ADR-011 or ADR-013. Fix: add a section "Answers to sibling questions" covering ADR-011's three questions and Q210-1 to Q210-4. Reconcile the mode and wire-spelling owners with ADR-013 (edit whichever record is wrong). State that #222 selects the bound representation that #213 implements under #211's DA-12 ownership. Cite ADR-011 and ADR-013 once they are on main. | ADR-012 §1.1, §2, Context table · ADR-011 "Questions handed to sibling tickets" (qsl-arch10 944a1c8) · ADR-013 O-19, O-20, §8 Q210-1 to Q210-4 (qsl-arch12 660aa25) · ADR-010 §7 |
| FND-006 | medium | The §12 touch sets are not complete, although §12 states "A row outside the table is a defect". (a) The new `SumCase` causes (non-exhaustive, unreachable arm, wrong variant) and the FR-340 frame and anchor causes need codes in the vendored QSpec `native-diagnostics.md` catalog, plus a revendor (§3 Diagnostics). Neither table has a QSpec catalog row. (b) §12.1 names QSpec #115 as the wire vocabulary but has no QSpec row. (c) Neither table has a Witness or Replay row. #210 requires contracts for both stages, and #218 requires frame counterexample replay. (d) §12.2 checks the scoped anchor "by its own function", but the §4.2 builder has no anchor transition. Fix: add QSpec catalog and vendor rows, and a QSpec wire row for §12.1. Add Witness and Replay rows that say "none while the item settles `unsupported`; on IR #109 / CG #49 landing: the family witness binding schema and the `evaluate` replay arm". Add the anchor transition to the §12.2 Check row and to §4.2. | ADR-012 §3, §4.2, §8, §12.1, §12.2 · #210 Acceptance 1 and 5 · #218 |
| FND-007 | medium | The trait shape contradicts the prose. In §2, `FamilyContract` declares `evaluate` with no default. The prose then says a family with no reference evaluation (`Relation`) "implements no `evaluate` hook", which the trait does not allow. S1 lists "reference evaluation dispatch" as a closed seam over `FamilyKind`, and the arm it needs for `Relation` is not stated. The context parameter is `&CheckContext` in the §2 table but `&mut CheckContext` in the trait. Fix: move `evaluate` into a separate `Evaluates` trait implemented only by evaluating families. State that the S1 evaluation seam covers only those families; `Relation` gets no arm, or gets an explicit arm that returns a typed refusal. Use one borrow (`&mut`, because the meter is mutable) in both places. | ADR-012 §2, §5.1 S1, §8 Replay |
| FND-008 | medium | §14 hands work to the wrong ticket and leaves one family without an owner. It gives "QSL negotiate copies removed in favour of RT's" (OBS-004) to #213. #213's scope is identities, typestate, `Capability` and outcome primitives and bounds, and its non-goals exclude backend-specific behavior. `negotiate_ieee` and `negotiate_integer_division` live in `value::ieee` and `value::division`, the `Value` family that #214 migrates. §14 also lists migrations for `StateModel`, `ProtocolClause`, `SumCase` and `Relation` only, so `TemporalTrace` has none. §4.3 implies #222, but #222 is a design ticket. Fix: move the QSL negotiate removal to #214, or to a named follow-up. Add a `TemporalTrace` migration row with its owner (#188 under #222's design, or a bounded child). | ADR-012 §4.3, §10 OBS-004, §14 · #213 Scope/Non-goals · #214 · #222 · `src/value/ieee.rs:830,882`, `src/value/division.rs:223` |
| FND-009 | low | §7.2 step 3 says the registry "routes each `supported` item to the lowering target of its one candidate". It does not define "lowering target" once AD-016 applies. The current QSL targets (`boolean-oracle/v1`, `integer-ir/v1`, `state-scalar-ir/v1`, `src/lowering/target.rs:39-46`) come before the IR, but negotiation happens after target-neutral IR lowering (arrow 4). So the step reads as QSL → CG → back to QSL. §13.1 Q1–Q2 leave the stage to #209, which is the right owner. But §7.2 should have only one reading. Fix: define "lowering target" as the backend-specific generation that follows target-neutral IR, and state the order: candidates, then IR, then `negotiate_*`, then routed generation. | ADR-012 §7.2, §8, §13.1 · AD-016 arrows 2–5 · #185 |
| FND-010 | low | A request that names an unregistered `BackendId` gets the wrong disposition. §7.2 step 1 gives it an empty candidate set, and §7.3 then settles `unsupported` naming the required capability. That misreports a malformed request as a missing capability. Fix: settle it as `invalid-request`, naming the unknown `BackendId`. Keep `unsupported` for a named, registered backend that lacks the required pair. | ADR-012 §7.2, §7.3 |
| FND-011 | low | The §4.2 builder is not fully specified. The diagram has no `Header → Post` edge, no `Framed → [*]` edge and no anchor state. It does not say whether an operation with only postconditions, or only a frame, is admitted. "A refused clause does not stop sibling clauses" is not reconciled with a typestate transition that returns a refusal: the state the builder is in after a refused transition is unstated. Fix: state the admitted clause sequences, including anchors, or mark the diagram as illustrative and cite the grammar. Say that a refused transition advances to the next state carrying the refusal, so sibling checks still run. | ADR-012 §4.2, §12.2 |
| FND-012 | low | The string rule's edge list leaves out the parser. §9 allows strings to select semantics only "at a serialization or command-line edge". The §3 parser entry table dispatches on source keywords, which is neither. Fix: add source lexing to the §9 edge list, where keywords become closed token kinds. State that the entry table is keyed on token kinds, not text. | ADR-012 §3, §9 |
| FND-013 | low | §9 and §4.3 citations have no `QSL:` prefix or revision. §9 inherits ADR-010's de627b5 pins, but the reader sees them at the branch head. At 048deb3, `value/expression/mod.rs:635` is inside a doc comment, and `CheckedPackage::call(function: &str …)` is at `:650-652`. At de627b5 it was `:635`. Other §9 lines were verified at both revisions: `model/systems.rs:269`, `protocol_artifact/validate.rs:287,291`, `state/evaluation.rs:2478-2484,2781-2783`, `temporal.rs:50`, `linking/composed/requests.rs:80`, `lowering/target.rs:39-46` and `complete/package.rs:690`. `infer_form` spans `check.rs:683-966` (about 284 lines), which matches "about 280". Fix: use ADR-010's `QSL:<path>:<line>` form with the de627b5 pin stated once, or cite 048deb3 lines (`mod.rs:650`). | ADR-012 §4.3, §9 · ADR-010 Evidence convention |
| FND-014 | low | Three wording and placement defects. (a) §7.4 "The ticket fixes only that solver absence is never a hold" does not say which ticket. (b) §10 OBS-003 "are removed when #185 lands" is transitional, where the target state is wanted. (c) §13.4 Q4 asks the owner about compatibility for four-kind artifacts, but #229's scope owns "compatibility behavior for older four-kind QSL artifacts", and OBS-003's removal already presumes the answer. Fix: (a) name #229. (b) State the target state: "The composed linker has no `Backend` parameter and no capability dispositions; #185 implements the removal." (c) Move Q4 to §13.3, addressed to #229, and keep OBS-003's removal conditional on #229's answer. | ADR-012 §7.4, §10 OBS-003, §13.3, §13.4 Q4 · #229 Scope |
| FND-015 | low | Nothing in ADR-012 or the index records how the #210 acceptance bullet "`/specify` and `/spec-review all` complete with findings resolved" is met. The review set under `spec/reviews/family-extension/` is not referenced from the ADR's Status. Fix: when the review set is complete, add one Status line naming ADR-012 as the `/specify` output and linking `spec/reviews/family-extension/`. | ADR-012 Status · #210 Acceptance 6 |

## Method

- **Acceptance coverage.** Each #210 Required-design and Acceptance bullet was
  mapped to a section:
  - shared contract §2; family ownership §3; typed subnodes and builders §4;
    exhaustiveness §5;
  - selection including unsupported and solver absence §7; four admissions §6;
  - bounded change sets §12; no whole-grammar routine §4.1 and §4.3;
    dispatch independence §7.1 and §9; absent capability §7.3; stage coverage
    §8.
  - Gaps are FND-004, FND-006 and FND-015.
- **ADR-010 routing.** ADR-010 §9 was read at the branch. It routes OBS-003,
  004, 012, 013, 014 and 033, DA-11 and L1-D1 to #210. Each has a decision in
  ADR-012 §10 or §11. L1-D1 covers #186, #187, #188, #189, #191, #192 and #198,
  and each verdict was checked against the issue's exit criterion from
  `gh issue view`. The two "kept" verdicts (#188, #189) are the only ones whose
  exit criteria name the no-registrant `unsupported` outcome.
- **AD-016 and PR #133.** AD-016 was read at `origin/main`. PR #133 is merged
  (818f555, 2026-09-19T17:28Z), and FR-290 and AD-010 were read on main. Checks
  made:
  - the single `negotiate_*` point, the four dispositions and the terminal rule;
  - `requires-bound` for unbounded constructs;
  - the RT placement of the `negotiate_*` predicates;
  - QSL as the only minter, and the arrow 7 executor.
  - Conflicts are FND-001, FND-002 and FND-003.
- **Fixed ownership.** The Context table and every "decided in #NNN" were
  compared with the bodies of #229, #213, #185, #222, #209, #211, #214 and
  #220–#223. The sibling records ADR-011 (qsl-arch10 944a1c8) and ADR-013
  (qsl-arch12 660aa25) were read for the questions they send to #210 (FND-005).
  ADR-012 designs no vocabulary, `Capability` type or registry internals beyond
  the selection contract #210 asks for. The `BTreeMap` example and the rule
  against ambient registries implement #212's "no ambient registry" condition.
- **Code citations.** Every §9 and §4.3 site was read at 048deb3 and, where a
  line differed, at de627b5. The composed `Disposition::UnsupportedCapability`
  and `UnsupportedFamily` are at `requests.rs:131,133`, and `report` is at
  `:282`. `negotiate_ieee` and `negotiate_integer_division` are at
  `value/ieee.rs:882` and `value/division.rs:223` (FND-013).
- **Ids and cross-references.** The ADR-012 id is unique across local
  worktrees and remote branches. The gap at ADR-011 belongs to #209's record.
  SR-476 is unused; SR-474 is the base review in the same set. §1–§14 and seams
  S1–S8 have no gaps or duplicates. The `spec/spec.md` index row and `contains`
  edge are present. The frontmatter edges (ADR-010, AD-016, FR-290, FR-340)
  resolve to the documents cited.
- **Rules.** The two mermaid diagrams contain no `;`. No compatibility layer,
  shim or fallback is designed; the fallback is explicitly rejected, and the
  compatibility question is placed with the wrong owner (FND-014). Current-state
  wording holds except for FND-014(b). String dispatch is covered except for
  the parser edge (FND-012). A monolithic routine is forbidden by §4.1, with
  `infer_form` named for conversion.
- **Artifact kind.** ADR-012 is a design decision record, not an
  FR/StR/US set, so the US→FR→StR traceability matrix does not apply. The
  hidden-assumption probes were applied as follows:
  - multi-source lookup tie-break: `invalid-request`, FND-010;
  - dependency not yet implemented: explicit `unsupported` arms;
  - external tool absence: §7.4.
- **Failure-domain check.** Extension failures: §5.1 closed seams, §5.2 open
  seams. Identity keys: typed `BackendId`, duplicate refusal, order-independent
  registry equality. Evaluation purity: `requirements` is a pure function and
  the checker never reads the registry. Topological robustness: the family DAG
  is acyclic, and placement is left to #209.

## Round 2

Reviewed ADR-012 at 8fb238b (diff from 048deb3), with ADR-013 at the
`qsl-arch12` worktree head for the sibling questions. Round-1 verdict: REJECT.

| ID | Round-1 severity | Status | Note |
| --- | --- | --- | --- |
| FND-001 | high | resolved | Candidates match on capability kind alone (§7.2 step 1). §1.1 gives `negotiate_*` the extent rules: `requires-bound` with an available finite bound, `unsupported` (warned) without one, never `supported`. §5.2, §7.3 and Alternatives agree with AD-016 and PR #133. |
| FND-002 | medium | resolved | §7.2: "Every settlement is an arm of CG `negotiate_*`, whichever repository implements the backend". A non-CG backend contributes a descriptor, a CG arm and its runner. Consequences reads "one CG `negotiate_*` arm". |
| FND-003 | medium | resolved | §10 OBS-012 states how it reads AD-016's row: values are recorded during admission and decide nothing. |
| FND-004 | medium | resolved | §12.1 needs no capability kind, and S7 is dropped. §12.2 Requirements is conditional on §13.3 Q3. §13.3 Q3 and §13.4 Q1 must be settled before #212, each with a named owner. |
| FND-005 | medium | partial | §13.5 answers ADR-011's three questions and Q210-1 to Q210-4. The Context table places the bound representation (#222 selects, #213 implements, #211 DA-12 owns). §1.1 now agrees with ADR-013 O-20 that CG settles the mode. Still open: ADR-013 O-20's Owner row says "The mode vocabulary and its family rules are decided in #210 with #222". ADR-012 decides no mode vocabulary. It sends the extent vocabulary to #222 and the mode advertisement question to #229 (§13.3 Q2), and §13.5 says only "This agrees with O-20". The (capability kind, mode) pairs of §1.1 and §7.1 therefore have no deciding owner in either record. Fix: in §13.5's Q210-2 row, state that the mode vocabulary (bounded, unbounded) and its rules are decided in #222, and ask #211 to change ADR-013 O-20's Owner row to #222. Or decide the two-value mode set in §1.1. |
| FND-006 | medium | resolved | §12.1 and §12.2 gain QSpec catalog and re-vendor rows, a QSpec wire row, and Witness and replay rows. §4.2 has the anchor transition. |
| FND-007 | medium | resolved | `ReferenceEvaluation` is a separate trait. S1 has an explicit `Relation` arm returning a typed `unsupported` refusal. `&mut CheckContext` is used in both places. |
| FND-008 | medium | resolved | §14 gives the QSL `negotiate_*` removal to #214 and adds a `TemporalTrace` row (#188, #189 via #222). |
| FND-009 | low | resolved | §7.2 defines backend-specific generation after target-neutral IR, in the order candidates, IR, `negotiate_*`, routing. |
| FND-010 | low | resolved | Unknown `BackendId` settles `invalid-request` (§5.2, §7.2). See new FND-017. |
| FND-011 | low | resolved | §4.2 diagram is marked illustrative and cites the grammar. It adds `Header → Post`, `Framed → [*]` and the anchor state, and defines the state after a refusal. |
| FND-012 | low | resolved | §9 lists source lexing as an edge. §3 keys the entry table on token kinds. |
| FND-013 | low | resolved | Context states ADR-010's `QSL:<path>:<line>` form at de627b5. §4.3 and §9 use it. |
| FND-014 | low | resolved | (a) §7.4 names #229. (b) OBS-003 states the target state. (c) Q4 moved to §13.3 for #229. |
| FND-015 | low | resolved | Status names ADR-012 as the `/specify` output and links the review set. |

New findings:

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-016 | low | §2 contradicts the terminal-disposition rule for items with no capability. §2 says such a claim "yields no `Requirements` value and is not negotiated", while §7.3 applies AD-016's completeness test (one accounting record per `request_index`) and §12.1 gives sum/case a CG `negotiate_*` arm. A requested item without `Requirements` then has no disposition. Fix: see SR-475 FND-012. Drop "and is not negotiated" and define its candidate set in §7.2. | ADR-012 §2, §7.2, §7.3, §12.1 · AD-016 terminal-disposition rule |
| FND-017 | low | §9 and §7.2 disagree on where an unknown `BackendId` fails. §9 says an open-set identity "is resolved by a lookup in the registry value, and an unknown name is refused" at the edge. §7.2 marks it unknown-backend and `negotiate_*` settles `invalid-request`. §5.2 joins them with "unless the CLI edge refused it first". Fix: let the edge check only syntax, and leave membership to §7.2. | ADR-012 §5.2, §7.2, §9 |
| FND-018 | low | ADR-011 and ADR-013 are cited by item (ADR-013 O-16, O-19, O-20, Q210-n) but neither is on this branch, and the frontmatter has no edge to them. The citations cannot be resolved from the repository at 8fb238b. Fix: add `relates_to` edges and links once ADR-011 and ADR-013 are on main, or name their branches and revisions in Context. | ADR-012 Context, §1.1, §8, §13.2, §13.5 |

Round 2 verdict: ACCEPT WITH FINDINGS
