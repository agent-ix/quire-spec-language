---
id: SR-2452
title: "Spec integrity review of the QSL certificate-locus wire conformance (FR-314, FR-338) at 84fba37c8"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@84fba37c8a342218e99aa0f2cb66b89204e3e879; diff against origin/main b24dbda01d56843cd14d8cab9233ec3cff67535f: spec/functional/FR-314-check-an-smt-proof-certificate.md, spec/functional/FR-338-check-an-en-1-closure-certificate.md; context: spec/functional/FR-127-settle-a-model-check-verdict-as-a-terminal-record.md, spec/functional/FR-339-check-an-en-1-component-certificate.md, spec/functional/FR-149-check-a-refinement-s-simulation-certificate.md, spec/functional/FR-163-check-a-hyper-item-s-product-closure-certificate.md, spec/functional/FR-169-settle-a-state-graph-verdict-with-its-settlement-method.md, spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md, spec/decisions/ADR-020-refinement-mappings-between-qsl-models.md, spec/decisions/ADR-022-possible-properties-and-state-graph-queries.md, spec/test-cases/TC-525-an-en-1-closure-certificate-is-checked.md, spec/test-cases/TC-893-an-smt-proof-certificate-is-checked.md; counterpart agent-ix/quire-specification@cff80d04fa88ecae0831be2ef2fb70adabed1c98 FR-331, FR-411, FR-412, FR-418, TC-363, TC-371, TC-431, proposals/backend-provider-v1/schema.json"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-314
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-338
    type: reviews
---
# Spec integrity review of the QSL certificate-locus wire conformance

## Summary

Prepublication review, no PR and no ticket assigned yet. PR: pending. Lane:
E4 CertificateLocus wire. Reviewed commit 84fba37c8 against origin/main
b24dbda01. The diff adds 8 lines: five to FR-314 Inputs (the step-index
meaning and the QSpec spelling of `Query` and `ProofStep`), and three to
FR-338 Inputs (only `Query` and `ProofStep` have a wire spelling, and
`ProductState` has none). The QSpec counterpart cff80d04 closes `at` to those
two shapes and says "a reader SHALL refuse any other `at` before consumption".

Units examined: FR-314 Inputs (lines 69-80), FR-314 Behavior `ProofStepInvalid`
and `NotRefutation` (102-107), FR-314-AC-2, TC-893 step 2; FR-338 Inputs
(72-73), Outputs (77-79), FR-338-AC-2, FR-338-AC-3, TC-525; FR-127 V-6 row,
certificate paragraph and AC-8 (93-94, 117, 159-166, 210); FR-339-AC-2;
FR-149 locus extension and AC-2 (89-91, 134); FR-163 locus extension (101-102);
FR-169 locus extension, V-6 row and AC-10 (105-107, 125, 150, 219); ADR-018
PC-2 (210) and PC-6 (214); ADR-020 CT-3; ADR-022 GC-3; QSpec FR-331
Certification (81-90), FR-411 (190), FR-412 (83-105), FR-418 (88-105),
TC-363 EC-02/03, TC-371 ZC-02 to ZC-04.

## Verdict

FAIL. The SMT half is sound: `Query{part}` keeps the ruled spelling
`unrolling`, `base` and `step` (the only existing positive vector is still
admitted), `ProofStep{part, index}` stays typed with a `u64` index, and the
index travels as a canonical decimal string, so RFC 8785 carries it exactly.
A zero-based index over every command in text order is a sound, total and
deterministic choice for a non-empty proof. It does not depend on Alethe
`:id` names, and it counts `assume`, `anchor` and the commands inside a
subproof the same way the checked-rule list already treats them as commands.

The closure half is not sound. FR-338's new sentence makes every core
checker's rejection except the SMT checker's unwritable on the wire. Yet
FR-127, FR-338, FR-339, FR-149, FR-163, FR-169, ADR-018 PC-2 and QSpec
FR-411, FR-412 and FR-418 still make those outcomes mandatory. The deferral
to E8a appears only in the author's logs. No spec text names it.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-338 now says `ProductState` "has no wire spelling", and QSpec FR-331 (cff80d04) says "a reader SHALL refuse any other `at`". But FR-127 still requires every EN-1 `Holds{Exhaustive}` whose certificate `check_closure` or `check_components` rejects to settle as "exactly one QSpec FR-331 terminal record" with `Inconclusive(CertificateRejected{rule, at})`, "written certificate-rejected{rule, at} (QSpec FR-331)", "a rejection names the rule and the product state at which it failed". FR-338 Outputs, FR-338-AC-2 and AC-3, FR-339-AC-2, FR-127-AC-8 (`CertificateRejected{WitnessFails, ...}` naming that component's first state), TC-525 and ADR-018 PC-2 ("naming the check rule that failed and the product state it failed at") all require such a record. No `at` exists for any of those rejections, so a conforming writer either cannot emit the mandatory V-6 record, or emits one that every conforming reader refuses. Either way a certificate rejection (inconclusive, a finding about the proof) turns into a writer failure or a reader refusal, which erases the rejection/refusal distinction for the main certificate kind. The deferral to E8a is not recorded in any spec text: `grep E8a` and `grep E3c` over spec/ find nothing, and there is no `Remaining work:` reference. Fix: either specify a `ProductState` wire spelling now (for example a closed `{model-state, automaton-state}` object over the existing state-key digest and `AutomatonStateKey`), or amend FR-127, FR-338 and ADR-018 PC-2 to say exactly how a closure or component rejection settles until it has one, and record the owning ticket as `Remaining work: #N`. Do not ship the contradiction. | spec/functional/FR-338-check-an-en-1-closure-certificate.md:72-73; spec/functional/FR-338-check-an-en-1-closure-certificate.md:77-79; spec/functional/FR-338-check-an-en-1-closure-certificate.md:107-108; spec/functional/FR-127-settle-a-model-check-verdict-as-a-terminal-record.md:34; spec/functional/FR-127-settle-a-model-check-verdict-as-a-terminal-record.md:93-94; spec/functional/FR-127-settle-a-model-check-verdict-as-a-terminal-record.md:117; spec/functional/FR-127-settle-a-model-check-verdict-as-a-terminal-record.md:159-166; spec/functional/FR-127-settle-a-model-check-verdict-as-a-terminal-record.md:210; spec/functional/FR-339-check-an-en-1-component-certificate.md:99; spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md:210; QSpec spec/objects/interfaces/FR-331-backend-provider-envelope.md:84-90 |
| FND-002 | high | FR-338's sentence names only `ProductState` as unspelled. FR-149 extends `CertificateLocus` with `SimulationStep{position, edge, verdict}`, FR-163 with `HyperProductState(ProductStateKey)` and FR-169 with `ModelState(DigestRecord)`, and each requires a settled V-6 `CertificateRejected{rule, at}`: FR-149-AC-2 "at that position and edge", FR-163 Settlement, and FR-169-AC-10 "`SuccessorMissing` at the state that reaches it", "`RankBroken` at `(1, 0)`". ADR-020 CT-3 and ADR-022 GC-3 say the same. In QSpec, FR-411 ("naming the failed check and its state"), FR-412-AC-3, FR-418-AC-2 ("its node or component"), TC-363 EC-02/03 and TC-371 ZC-02 to ZC-04 require loci that the closed `at` now refuses. The commit acknowledges none of these, so a reader of FR-338 would conclude that only `ProductState` is affected. The refinement, hyper, state-graph, probabilistic and zone checkers are all left with mandatory outcomes that cannot be written. Fix: cover every `CertificateLocus` variant and every QSpec checker locus in the same resolution as FND-001, listing each one, or narrow QSpec's closure so that it does not refuse them. | spec/functional/FR-338-check-an-en-1-closure-certificate.md:72-73; spec/functional/FR-149-check-a-refinement-s-simulation-certificate.md:89-91; spec/functional/FR-149-check-a-refinement-s-simulation-certificate.md:134; spec/functional/FR-163-check-a-hyper-item-s-product-closure-certificate.md:101-102; spec/functional/FR-169-settle-a-state-graph-verdict-with-its-settlement-method.md:105-107; spec/functional/FR-169-settle-a-state-graph-verdict-with-its-settlement-method.md:219; QSpec spec/functional/analysis/FR-411-settle-probabilistic-claims-exactly.md:190; QSpec spec/objects/interfaces/FR-412-probability-certificate.md:105; QSpec spec/objects/interfaces/FR-418-zone-certificate.md:105 |
| FND-003 | medium | The new index definition ("the zero-based position of its command in that part's proof, in text order") is backed by no acceptance criterion. FR-314-AC-2 and TC-893 step 2 say only "`ProofStepInvalid` at that step", "`ProofStepInvalid` at that `Step` step" and "`Unverifiable` at that step", with no index value, and no vector puts the invalid step after an `assume`, an `anchor` or inside a subproof. A checker that counts from 1, counts only `step` commands, or skips subproof bodies passes every AC, which is exactly the reader/writer disagreement the definition exists to rule out. Fix: give AC-2 and TC-893 an exact `ProofStep{part, index}` for each step-located vector, including one step after at least one `assume` and one inside an `anchor` subproof. | spec/functional/FR-314-check-an-smt-proof-certificate.md:76-77; spec/functional/FR-314-check-an-smt-proof-certificate.md:139; spec/test-cases/TC-893-an-smt-proof-certificate-is-checked.md:38-42 |
| FND-004 | low | FR-314 now uses three words for one locus: Inputs says "command", Behavior says "proof step" and "last step", and the checked list calls `anchor` "the `anchor` command". The text does not say whether an `anchor`, which has no rule and no conclusion, can be the "first proof step" `ProofStepInvalid` names or the "last step" `NotRefutation` names. Nor does it say that `index` is unrelated to an Alethe step's `:id` symbol (`t5` is not index 5), or how a non-inference command in the proof text, if any, is counted. Fix: say "command" throughout Behavior, state that `:id` symbols play no part in `index`, and state which commands can carry `ProofStepInvalid` and `NotRefutation`. | spec/functional/FR-314-check-an-smt-proof-certificate.md:76-77; spec/functional/FR-314-check-an-smt-proof-certificate.md:102-107; spec/functional/FR-314-check-an-smt-proof-certificate.md:131-132 |
| FND-005 | low | The index meaning and the wire spelling are untagged prose in FR-314 Inputs, written as "QSpec FR-331 writes ...", not SHALL statements in Behavior. The canonical-decimal rule restates QSpec FR-331's normative text instead of citing it, so the two repositories now carry two copies of one contract that can drift apart. Fix: make the index meaning a Behavior SHALL, and cite QSpec FR-331 for the spelling instead of restating it. | spec/functional/FR-314-check-an-smt-proof-certificate.md:76-80; spec/functional/FR-338-check-an-en-1-closure-certificate.md:72-73 |
