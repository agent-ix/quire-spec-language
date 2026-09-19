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

Verdict: ACCEPT WITH FINDINGS (round 3, delta 924f239..9045cd5). Round 1 was
REJECT because FND-001 contradicted AD-016 and PR #133. Round 2 (8fb238b)
resolved FND-001. Round 3 confirms the Codegen #86, Contract IR #141, QSpec
#134 and ADR-011 alignment edits. It opens three medium findings (FND-019,
FND-020, FND-021) and two low ones; see "Round 3".

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

## Round 3 (delta 924f239..9045cd5)

Reviewed only the ADR-012 changes in 924f239 (coordinator wording fixes: S9
through Codegen #86, S6 through Contract IR #141, QSpec #134 citations) and
9045cd5 (alignment with ADR-011 at QSL PR #235 head e62a39f). The three tickets
were read with `gh issue view` (Codegen #86, Contract IR #141 and QSpec #134,
all open). ADR-011 was read at e62a39f in the `qsl-arch10` worktree. AD-016 was
read at QSpec `main`.

Checks on the four 924f239 fixes:

- S9 is cited to Codegen #86 in §5.1 (table and rule), §7.1, §7.4, §9, §10
  OBS-004 and OBS-033, §11, §14.1 and §14.2. `negotiate_kani_obligations` at
  `src/kani_obligations.rs:454` matches #86's measured state.
- S6 is cited to Contract IR #141 in §5.1, §9, §10 OBS-033, §13.5 and §14.1.
  The "decoded once in the v2 reader" wording is gone from IR and CG text. The
  remaining "decoded once at … intake" wording is in QSL rows of §9 only, where
  it is correct.
- No "to be opened" or "no numbers yet" text remains for CG or IR. §13.4 Q4 is
  removed. Only the RT row of §14.1 keeps "to be opened" (SR-477 FND-019).
- QSpec #134 is cited in the Context table, §7.3, §10 OBS-033 and DA-11, §13.4
  Q1 and Q3, and §13.5. See FND-020 for the places that still omit it.

Checks on the 9045cd5 alignment with ADR-011 at e62a39f:

- The `route` module (layer R), placed after S4 and before E7, matches ADR-011
  §2.1 and its "Answers to ADR-012 §13.1" item 1. The driver being a separate
  crate downstream of CG and placed by #225 matches items 2 and §6.1 "driver".
  "Families are modules in the one QSL crate" matches item 3.
  "`CheckContext` in the `check` core" matches item 4.
- ADR-011 records the L1-D1 edges as §11 states them. #188, #189, #217 and
  #223 keep #185.
- The rewritten §13.5 answers agree with ADR-011's three questions to #210.

Prior open, partial or delta-affected findings:

| ID | Prior status | Status | Note |
| --- | --- | --- | --- |
| FND-002 | resolved | resolved | Strengthened. §5.1 makes S9 closed and puts settlement in its arms by construction, with the #86 test that fails on settlement outside an arm. |
| FND-004 | resolved | resolved, with residue in FND-020 | §13.4 Q1 now names QSpec #134 as the vocabulary widening. §12.2 and §13.3 Q3 still route the frame and sum/case kinds to #229 alone, although #134 scope item 2 owns them. |
| FND-005 | partial | resolved | §13.5 Q210-2 (6b9a603, unchanged by the delta) says #222 decides the mode and extent vocabulary and asks #211 to amend ADR-013 O-20. |
| FND-008 | resolved | resolved | The owner of the QSL `negotiate_*` removal moved from #214 to #185 in 6b9a603, following ADR-010 §7. The delta keeps §10 OBS-004, §14.1 and §14.2 consistent on #185. |
| FND-016 | open (low) | resolved | §2 and §7.2 step 1 (6b9a603): an item with no `Requirements` still reaches `negotiate_*` exactly once. |
| FND-017 | open (low) | resolved | §5.2 (6b9a603) splits the two paths. A CLI argument is refused at the edge and forms no request. A request that names an unknown identity settles `invalid-request`. |
| FND-018 | open (low) | partial | §13.1 now names ADR-011 by QSL PR #235. ADR-013 still has no branch, PR or revision, and the frontmatter has no edge to either record. |

New findings:

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-019 | medium | OBS-004 contradicts the AD-016 row it cites. The new text calls `negotiate_ieee` and `negotiate_integer_division` "RT-internal operation-eligibility predicates (AD-016 arrow 3)". AD-016's arrow 3 row says "RT exposes pure capability predicates (`negotiate_ieee`, `negotiate_integer_division`) … full predicate list consumed by arrow 4" (WP7 open). "Not settlement points" agrees with AD-016 and Codegen #86. "RT-internal" does not: it is a new name for an AD-016 term and denies the arrow 4 consumption that AD-016 leaves open. This repeats the pattern of round-1 FND-003. Codegen #86 says only that CG calls neither today and that they "should not be pulled in as arms without a separate decision". Fix: in §10 OBS-004, write "RT capability predicates (AD-016 arrow 3). A CG `negotiate_*` arm may take them as inputs (arrow 4, WP7), but they are not settlement points and settle no disposition. Today CG calls neither (Codegen #86)." Drop "RT-internal" and "operation-eligibility". | ADR-012 §10 OBS-004 · AD-016 arrow 3 capability point, WP7 · Codegen #86 "Measured state" |
| FND-020 | medium | QSpec #134 is not cited where its scope decides the open question. The delta cites #134 for FR-290's kinds and the FR-331 field. It leaves these open against #229 or "an open QSpec issue". (a) §13.3 Q3 and the §12.2 Requirements row: the kinds for the frame obligation and the sum/case claim are #134 scope item 2. (b) §13.3 Q2: whether a backend advertises a mode is #134 item 3. (c) §13.3 Q5 and §10 OBS-013 say the FR-290 Kani wording "needs an open QSpec issue". #134 item 5 is that issue. (d) §13.4 Q2 asks the owner whether a preference order is wanted. #134 item 6 records `invalid-request` with no preference order as the rule. (e) §13.3 Q4 asks #229 about four-kind compatibility. #134's non-goals record the owner ruling on #229: refused as unsupported. So the record keeps questions open that the numbered ticket answers or owns, and it names two owners for each. Fix: cite QSpec #134 in (a) and (b), and route them "#229 with QSpec #134". In (c), replace "needs an open QSpec issue" with "QSpec #134 item 5". Close (d) and (e) as answered by #134 and the #229 owner ruling, and make OBS-003's removal unconditional. | ADR-012 §10 OBS-003, OBS-013, §12.2, §13.3 Q2–Q5, §13.4 Q2 · QSpec #134 Scope 2, 3, 5, 6, Non-goals |
| FND-021 | medium | The 9045cd5 text mixes ADR-011 stage ids with ADR-012 seam ids, and both use bare `S<n>`. "after S4 and before E7" (§6 table, §7.1, §13.1) means ADR-011 stage S4 (package emission). In this record a bare S4 is seam S4 (family `Cause` enums, §5.1). The §13.5 row on per-stage hooks lists "S2 family form builder, S3 …, S4 `package`, S6a `evaluate`" (stages) and then "every S1 dispatch seam" (seam) in the same cell. The row on `capability_report` says "S3 negotiates nothing", which uses the stage id. ADR-011 requires other records to cite its ids as `ADR-011 S3`, `ADR-011 E5`, and ADR-012 line 70 reserves `ADR-012 S<n>` for its seams. As written, "after S4" names the wrong thing. Fix: prefix every ADR-011 stage and edge id with `ADR-011`: `ADR-011 S4`, `ADR-011 E7`, `ADR-011 S2`, `ADR-011 S3`, `ADR-011 S6a`. Keep bare `S<n>` for this record's seams. | ADR-012 Context (citation rule), §6 table, §7.1, §13.1, §13.5 · ADR-011 Decision item 11 (e62a39f) |
| FND-022 | low | §5.1 introduces "Three rules make the failure certain" and now lists five bullets, after the S9 and S6 rules were added. Fix: "These rules make the failure certain", or move the S9 and S6 bullets under their own lead-in. | ADR-012 §5.1 |
| FND-023 | low | Citation forms are inconsistent. The new text uses "Codegen #86" and "Contract IR #141". Existing text uses "CG #49" and "IR #109" for the same repositories. `CG:src/kani_obligations.rs:454` has no revision, while Context pins `QSL:` citations to de627b5. Fix: use one prefix per repository (`CG #86`, `IR #141`, or define the long forms once in Context). State the CG revision the line was measured at (`origin/main` per Codegen #86, with a SHA). | ADR-012 Context, §5.1, §7.1, §9, §10, §11, §14 |

Round 3 verdict: ACCEPT WITH FINDINGS. All four 924f239 fixes are present and
consistent with Codegen #86, Contract IR #141 and QSpec #134. The 9045cd5
placements match ADR-011 at e62a39f. Three medium findings remain (FND-019
OBS-004 against AD-016 arrow 3, FND-020 open questions that #134 answers, and
FND-021 stage and seam id collision). None of them blocks the design, and each
has a one-line fix.

### Author response (after round 3)

FND-019, FND-020, FND-021 and FND-022 are addressed in the ADR commit that
follows 9045cd5:
- OBS-004 uses AD-016's term (RT capability predicates, possible arm inputs
  under WP7, not settlement points).
- QSpec #134 is cited for the frame and sum/case kinds, mode advertisement and
  the FR-290 Kani wording. The four-kind and preference-order questions are
  closed as answered, and the OBS-003 removal is unconditional.
- ADR-011 stage and edge ids carry the `ADR-011` prefix everywhere.
- §5.1 reads "These rules".

FND-023 (ticket naming style) is left as is.

## Author closure (after the PR review)

Every finding this record left open or partial has one closing line. "Fixed"
names the ADR-012 section in the commit that carries this section. "Routed"
names the owner that holds the remaining work.

| ID | Closure |
| --- | --- |
| FND-018 | Fixed: the body cites ADR-011 by QSL PR #235 and ADR-013 by QSL PR #236, and QSL PR #239 adds `relates_to` frontmatter edges among ADR-011, ADR-012 and ADR-013 after both merged. |
| FND-019 | Fixed: §10 OBS-004 uses AD-016's term, RT capability predicates. The predicate list is decided by AD-016 WP7. |
| FND-020 | Fixed: QSpec #134 is cited for the frame and sum/case kinds, mode advertisement and the FR-290 Kani wording (§12, §13.3, OBS-013). The four-kind and preference-order questions are closed, and OBS-003's removal is unconditional. |
| FND-021 | Fixed: every ADR-011 stage and edge id carries the `ADR-011` prefix. |
| FND-022 | Fixed: §5.1 reads "These rules make the failure certain". |
| FND-023 | Fixed: every IR and CG ticket is cited as `Contract IR #n` or `Codegen #n`, and §5.1 states the CG revision of `kani_obligations.rs:454` (CG `origin/main` on 2026-09-19). |

## PR review (QSL PR #234, delta 43677c9..10664aa)

The PR reviewer checked the author-closure lines above against ADR-012 at
10664aa, and checked cross-record consistency with ADR-011 22fa948 and
ADR-013 4152eb8. FND-019 to FND-023 are confirmed fixed. FND-018 is routed
until PRs #235 and #236 merge; its closure names that trigger but no owner
(SR-474 PR-N5). Three medium cross-record findings are open and recorded in
SR-474 (PR review): PR-N1 (the capability vocabulary owner is #229 here but
QSpec #134 in ADR-011 and ADR-013), PR-N2 (the RT name lookup against ADR-013
R-06) and PR-N3 (ADR-011:801-804 still says `unsupported` for a family that
sits out S6a).

PR review verdict: CHANGES.
