---
id: SR-472
title: "Scope-boundary review of ADR-011 stage DAG and dependency architecture"
type: SpecReview
analysis: scope-boundary
scope: "spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: reviews
---
# Scope-boundary review: ADR-011 stage DAG and dependency architecture

## Summary

Round 1. Reviewed commit: 944a1c8 (branch `task/209-stage-dag`), file
`spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md` and its index
row in `spec/spec.md`.

Sources checked: the bodies of #205, #209, #210, #211, #212, #213, #215, #216,
#185, #225 and #229; ADR-010 at the same commit as the evidence baseline; and
QSpec AD-016 (`status: accepted`) at `quire-specification` `origin/main`. AD-016
is treated as binding and is not reopened. QSL is prerelease, so no
compatibility layer is expected.

The question: does ADR-011 stay inside #209 (stages, edge contracts, bypasses,
CLI orchestration constraints, module and crate DAG, extraction criteria)? Does
it leave the sibling tickets' decisions to them: #210 (family contracts,
dispatch, capability selection), #211 (canonical types, identities,
conversions, versions), #229 (capability vocabulary) and #225 (lifecycle and
CLI design)? And do its repository allocations match #205 and AD-016?

What holds:

- The per-stage owners match AD-016 exactly. S3 and S4 are QSL (arrows 1 and
  2). S5 is IR `quire-contract-model` (`CheckedPackageV2::read`/`lower`). S6b
  is CG negotiation, oracle and harness, plus RT ops and ABI, plus the IR
  outcome (arrows 3 to 6). S7 is the IR packet and witness. S8 is reconstruction
  and comparator in the CG replay adapter, with QSL as the executor (arrow 7,
  Owner decision 3). I1 is the only FCD ↔ QSL point, `model::intake`. QI owns
  `heads/`. FCD is a producer only.
- The FB-05 exception, a normal CG → QSL edge to the S6a entry, is AD-016 Owner
  decision 5. X-1 `quire-exact` is Owner decision 2. The RT `qsl-agreement`
  retarget matches the Shared-type strategy. The §7.1 crate graph extends
  AD-016's graph, removing the dashed edges that AD-016 marks as outside the
  pipeline or retiring in WP9.
- Most sibling questions are handed off rather than decided. #210 gets the
  S2/S3/S4/S6a family hooks, the content of `capability_report`, and the v2
  family forms. #211 gets typestate encoding, outcome and refusal types, the
  command outcome vocabulary, and the `diagnostic` locus (DA-13). #225 gets the
  lifecycle surfaces. #209's own acceptance items are all addressed: the module
  map (§6.2), no reparsing (FB-01, FB-02), checking before lowering (§4), CLI
  orchestration (§5), and extractions with direction, API, order and
  compatibility disposition "none" (§7.3).
- The `spec/spec.md` row (line 388) is accurate and names #209. It needs no
  change.

What does not hold:

- Capability routing. ADR-011 labels #185 "the only capability registry and
  router (E7 selection)". E7 is CG's single negotiation point (AD-016). #185 is
  a QSL-repo ticket that replaces `src/lowering/target.rs`, and ADR-011 retires
  that module as part of SEAM-1. No stage or module in §6 hosts the registry.
  This decides, without saying so, #229's "which QSL component selects a target"
  and #210's capability selection (FND-001).
- SEAM-1 retirement is owned by a gate (#216), not by a change. The Consequences
  pull CLI migration into Layer 2, which re-sequences #205's lifecycle chain
  (FND-002).
- Three items that AD-016 already settled are handed to #211 as if they were
  open (FND-003).
- OBS-039 is closed by reading #205 in a way its text does not support, instead
  of routing an amendment to the epic (FND-004).

Verdict: REVISE. One blocking (high) finding, FND-001. It would fail #212
scenario 7 ("add a backend") and the #212 pass condition that #229 leave "no
implementation decision for #185". The other findings are medium or low and
can be fixed in the same revision.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Capability routing is placed without an owner or a module, which pre-empts #229 and #210. Three places say #185 is "the only capability registry and router (E7 selection)": the Implementing-tickets row, FB-12, and scenario 7 ("registers with #185"). But E7 is CG's stage, and AD-016 Decisions make "CG `negotiate_*` the single capability negotiation point". #185 is a QSL ticket: "Replace the fixed lowering target catalog at `src/lowering/target.rs`". §6.2 and SEAM-1 retire `lowering` and "its three targets" wholesale, and §6.1 has no layer or module that hosts a registry. So the ADR decides where target selection sits and how it relates to CG negotiation, and #229's scope names that decision ("which QSL component selects a target"). At the same time it deletes the module #185 rebuilds. The Context also promises that "this record names the question it hands to each one", but there is no "To #229" list. Fix: drop "(E7 selection)" and the #185 wording from scenario 7. Say only that a backend is admitted through the capability registry that #229 specifies and #185 implements. Add a "To #229 / #210" question: which stage and QSL module hosts the target registry, and how registry routing relates to CG per-item negotiation at E7. In §6.2, keep `lowering::target` out of SEAM-1 as a temporary seam that retires into that answer, or name it explicitly as the #185 successor. | ADR-011 Implementing tickets, §3 FB-12, §6.1, §6.2 SEAM-1, §10 scenario 7, Questions handed to sibling tickets; #185 body; #229 Scope; #212 pass conditions; AD-016 Decisions, arrow 4 |
| FND-002 | medium | A gate owns SEAM-1's retirement, and the ADR re-sequences #205. The SEAM-1 "Owning change" is "#216 gate precondition; lowering-target removal with #217". #216 is a gate that checks "Old producer/bypass paths are unreachable". It removes nothing. So the native `runtime`, `mapped`, `formal_source`, `native_model`, `model_source`, `NativePackage` and "the native half of `command`" have no implementing ticket. That breaks the ADR's own seam definition ("a named retirement change"). The Consequences then say "The CLI therefore moves to the spine within Layer 2, not in Layer 5". That changes #205's dependency graph (`#224 + #216 → #225 → #232 → #230`), which is epic authority, not #209's. #216 also asks for no unreachable *producer* paths, not a migrated CLI. Fix: name one implementing change (existing or to be filed) that removes SEAM-1, and make #216 its gate only. Either limit the pre-#216 removal to the producer and bypass paths #216 names, or record the Layer 2 CLI move as a proposed amendment to #205's dependency graph for the epic owner to rule on. Do not state it as a consequence. | ADR-011 §6.2 SEAM-1, Consequences bullet 2; #216 Required evidence; #205 Dependency graph, Layered delivery 6 |
| FND-003 | medium | Three items that accepted AD-016 already settled are handed to #211 as open. (a) §2.2 E3: "The scheme is decided in #211" for the checked node id and declaration identity. AD-016 arrow 1 fixes `quire.checked-semantic-node/v1` (FR-201) and `model::key::DeclarationKey{package,node}` (`sha256-jcs`). (b) §6.2 `value` row and "To #211": "#211 decides the exact `quire-exact` type list". The AD-016 Shared-type row fixes it as "exactly the types listed in this row plus `Undefined`". (c) §4: "#211 decides the canonical name" for the one checked-package type. AD-016 Owner decision 6 defers `CheckedPackage` renames "until the owner asks", and AD-016 keeps "the four `CheckedPackage` types" distinct. Fix: cite AD-016 as the decision for (a) and (b), and give #211 only the typestate and verification rule around them. For (c), keep the rule "exactly one type per stage output" by deleting the native-v1 type with SEAM-1, and say that no rename happens without an owner request (OD-6). | ADR-011 §2.2 E3, §4 bullet 3, §6.2 `value` row, Questions to #211; AD-016 arrow 1, Shared-type strategy, Owner decision 6 |
| FND-004 | medium | OBS-039 is closed by re-reading #205 instead of routing an amendment. #205's Ownership boundaries say "Runtime owns executable semantic behavior and native replay surfaces". The OBS-039 row reads this as "Consistent once read as above: #205 gives reference semantics to QSL". The Alternatives reject "reading #205 literally". The allocation itself is right, because AD-016 is binding (executor in QSL, reconstruction in CG, RT with no replay surface). But a #209 ADR cannot reinterpret the epic's ownership text, and #212 checks scenario 6 ("replay it natively") against that text. Fix: state that #205's RT bullet conflicts with AD-016 arrow 7 and Owner decision 3. Record a required amendment of the #205 Ownership boundaries (RT: runtime ops and host ABI; QSL: reference execution and replay executor; CG: replay reconstruction) as a handoff to the epic owner. | ADR-011 §9 OBS-038, OBS-039, Alternatives bullet 3; #205 Ownership boundaries; AD-016 Replay ownership, Owner decision 3; ADR-010 OBS-039 |
| FND-005 | medium | §2.3 "Proof-stage acceptance" and Decision 8 bind other repositories' gates. They name the RT `make kani` gate (RT #53), CG #58, #59, #60 and #73, and the #217 and #219 gates, and require a discharged check location and a mutation control per claimed module. A rule that binds IR, RT and CG evidence is a normative cross-repository contract. #205 gives those to QSpec ("QSpec owns normative cross-repository contracts"), and #211 restates it. RT #53 is not among the ADR-010 findings routed to #209. Fix: keep the rule for QSL-owned gates (#217, #219 evidence as QSL cites it). For RT and CG, record it as a proposed QSpec evidence contract, or hand it to #226 (drift and compatibility gates), naming the QSpec artifact that will carry it. Reword FB-10 and the Consequences bullet on RT's gate to match. | ADR-011 Decision 8, §2.3 Proof-stage acceptance, FB-10, Consequences bullet 4; #205 Ownership boundaries; #211 Required design |
| FND-006 | low | §5 settles parts of #225's required decisions. #225 lists "CLI adaptation, stable exit-code mapping, and structured output rules". §5 fixes the command outcome fields ("last stage reached, terminal outcome kind, diagnostics …, identities") and the exit-code mechanism ("one total, exhaustive map … no `_` arm"). These are reasonable, but they are design content rather than stage-boundary constraints. Fix: phrase them as constraints #225 must meet: outcomes are structured and total, rendering is output-only, and no command logic reads rendered text. Move the specific field list and exit-map shape into "Questions handed to #225", or mark them as proposed inputs. | ADR-011 §5; #225 Required decisions |
| FND-007 | low | Two wire questions are routed without naming QSpec as the authoring home. SEAM-3 says "Whether these formats remain as separate wires is decided in #211". Compiled-protocol /1 to /3 and the checked handoffs cross into IR, and #211 states that "Normative cross-repository wire/API changes are authored in QSpec". The §7.1 IR root → QSL removal names QSpec correctly. SEAM-3 and the matching "To #211" bullet do not. Fix: add "authored in QSpec" to SEAM-3 and to the #211 question. | ADR-011 §6.2 SEAM-3, Questions to #211; #211 Required design |
| FND-008 | low | The frontmatter relationships do not record the binding input. ADR-011 declares AD-016 binding ("Its seven arrows … are binding"), but lists only `depends_on` ADR-010 and `relates_to` IT-010. The traceability graph therefore cannot show that ADR-011 depends on AD-016. Fix: add `target: ix://agent-ix/quire-specification/AD-016`, `type: depends_on` (or `references`, if cross-repository `depends_on` is not allowed). | ADR-011 frontmatter, Context bullet 2 |
