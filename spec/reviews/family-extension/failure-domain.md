---
id: SR-475
title: "failure-domain review of ADR-012 semantic-family extension contracts"
type: SpecReview
analysis: failure-domain
scope: "spec/decisions/ADR-012-semantic-family-extension-contracts.md, spec/spec.md"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: reviews
---
# SR-475: Failure-domain review of ADR-012

## Summary

Reviewed ADR-012 at commit 048deb3 on `task/210-family-extension`, plus the
uncommitted one-sentence citation note in the Context section, which changes no
decision. The review checked ADR-012 against #210's acceptance bullets, the #212
gate scenarios, accepted QSpec AD-016, QSpec PR #133 (CG `negotiate_*` is the
single negotiation point, with dispositions `supported`, `requires-bound`,
`unsupported` (warned) and `invalid-request`), and the fixed ownership split
(#229 vocabulary and absence policy, #213 Rust type and outcomes, #185 registry
and routing, #222 boundedness, #209 stages and DAG, #211 types and identities).

Most of the record holds up. Its failure paths are explicit. Closed seams have no
`_` arms. Registry failures are never first-wins. Solver absence is never a hold
and never falls back to another backend. Refused items emit no substitute
artifact. The checker is independent of the registry. The family DAG is stated
as acyclic. It adds no compatibility layer, and its mermaid labels contain no `;`.

One blocking defect remains. The candidate filter in §1.1 and §7.2 drops every
bounded-only backend from the candidate set of an unbounded requirement. The
candidate set is then empty, and §7.2 settles an empty set as `unsupported`. So
`requires-bound` can never be settled for an unbounded requirement when the only
backend is bounded, as Kani is. AD-016 ("Unbounded construct", arrow 5) requires
CG negotiation to settle `requires-bound` in exactly that case. §1.1 promises it
too, so the record contradicts itself as well as AD-016.

The other findings are medium or low. They cover check-order dependence through
`&mut CheckContext`, the missing budget-exhaustion outcome, the builder's
behaviour after a refused clause, overlapping parser entries, unknown wire tags
under cross-repository version skew, a named but unregistered backend, the line
between absence and run failure, and partial package emission.

Verdict: ACCEPT WITH FINDINGS (round 2, commit 8fb238b). Round 1 was REJECT
because FND-001 made `requires-bound` unreachable. The revision resolves
FND-001. One new medium finding (FND-012) and some low residue remain; see
"Round 2".

## Method

- Checklist, adapted to a design decision record:
  - Extension points and trust boundaries: family hooks (`check`,
    `requirements`, `package`, `evaluate`), the parser entry table, backend
    registration, the v2 reader edge and the runtime-availability probe. For
    each, is the failure behaviour stated, and is it strict or resilient?
  - Entity identity: checked-node identity, `BackendId`, `FamilyKind`, and the
    capability key (capability kind, mode). Is the uniqueness key explicit, and
    is it independent of order?
  - Evaluation purity: `requirements` (stated pure), `check` (takes
    `&mut CheckContext`), and whether a family's side effects can leak into
    another family's check.
  - Topology: the family DAG, the builder state machine, cross-repository enum
    skew at S5 and S6, and the empty, singleton and multiple candidate-set cases
    for every mode.
- Traced each §7.2 candidate-set case against AD-016's terminal-disposition
  rule, AD-016's "Unbounded construct" and "Frame obligation" rows, and the
  arrow-5 bound rules. Checked the dispositions against PR #133's FR-290 and
  AD-010 text.
- Ownership check: each finding's fix stays inside #210's remit or cites the
  owning ticket. No finding asks ADR-012 to decide #229, #213, #185, #222, #209
  or #211 content.
- Sources: `gh issue view` for #210, #212, #222 and #229; quire-specification
  `origin/main:spec/assurance/AD-016-semantic-family-extension-path.md`;
  `gh pr diff 133 -R agent-ix/quire-specification`; and ADR-010 §4.3 and §9 as
  cited by ADR-012.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `requires-bound` cannot be reached for unbounded requirements. §1.1 says a backend that advertises a capability only in bounded mode "is not a candidate" for an unbounded requirement. §7.2 step 1 builds the candidate set from exact (capability, mode) matches, and its table settles an empty set as `unsupported`. With Kani as the only registrant, an unbounded claim therefore always settles `unsupported`, even when a finite bound is available. §1.1 says that case settles `requires-bound`, and AD-016's "Unbounded construct" row and arrow 5 require CG negotiation to settle `requires-bound` there, not `unsupported`. The only path to `requires-bound` in §7.2 is the "exactly one" row, which an unbounded requirement never reaches. This breaks #212 scenario 5 and AD-016. Fix: in §7.2, add a second candidate class. An unbounded requirement's candidate set also records the backends that advertise the same capability kind in bounded mode, as bound-requiring candidates. Add a table row: "no unbounded-mode candidate, one or more bounded-mode candidates" is passed to `negotiate_*`, which settles `requires-bound` when a finite bound is available (the availability rule is decided in #222) and `unsupported` (warned) otherwise. Keep "never narrowed" by stating that a `requires-bound` item runs only after the caller supplies a bound, which turns it into a new bounded request. Update §5.2 row 3 and §7.3 to match. | ADR-012 §1.1, §5.2, §7.2, §7.3 · AD-016 "Unbounded construct", arrow 5, terminal-disposition rule · #212 scenario 5 · #222 |
| FND-002 | medium | Check order can leak through the mutable context. `check(form, cx: &mut CheckContext)` gives every family write access to the resolved declarations, the type environment and the scope stack. Identity is "minted by QSL at check time" (§2), with no rule that the minting is independent of check order. One family's check could then change what a later family reads, or take its identity from a traversal counter. Dispatch and identity would depend on check order, which #210 forbids, and #212 scenario 4 (identity change that keeps provenance) would become order-sensitive. Fix: in §2, split the context. Declarations, the type environment and limits are read-only (`&`). Only the meter, the diagnostics sink and the scope push and pop are mutable, through narrow methods. State that identity is a function of the normalized checked content and its declaration key, never of a counter or of visit order, with the representation decided in #211. Add a test obligation in §3: checking siblings in reversed order yields the same identities and the same diagnostics. | ADR-012 §2 (Typing context, Identity), §3 Normalization, §4.2 · #211 DA-01, DA-02 · #212 scenario 4 |
| FND-003 | medium | The family outcome has no case for budget or limit exhaustion. §2 says a family check returns "either the checked node or a refusal". `CheckContext` carries "limits and the meter", and AD-016's kernel has `Incomplete`, but there is no outcome for a check that runs out of meter or hits a depth limit. Exhaustion would then be reported as a language refusal (a wrong catalog meaning), or it would panic. Fix: in §2 (Structured outcome), state that `FamilyOutcome` has three cases: checked, refused (with a family cause), and incomplete (with a limit cause), where the incomplete case uses the #211 outcome type. State that incomplete is never a refusal of the language form and emits no checked node. List a limit-exhaustion test per family in §3 Tests. | ADR-012 §2, §3 Tests, §4.2 · AD-016 shared kernel (`Incomplete`, `Meter`) · #211 DA-09, DA-10 |
| FND-004 | medium | The builder's behaviour after a refused clause is undefined. §4.2 makes the builder a typestate in which each transition returns "that clause's checked subnode or its refusals". It then says a refused clause does not stop sibling clauses from being checked, and that an out-of-order clause is refused "by the transition it tried to take". A compile-time typestate cannot take a runtime transition with no checked subnode, and it cannot express an out-of-order clause, because source order is runtime data. The record does not say which state the builder is in after a refusal, or whether a driver loop, which is the "sequencer" the section rules out, picks the next transition. Fix: in §4.2, define the builder as a runtime state machine, a closed state enum with one transition function per (state, clause kind) pair. An invalid pair returns a typed out-of-order refusal and leaves the state unchanged. A refused clause advances the state and records that the clause was refused, so later clauses are still checked. Keep the typestate only for the construction of `finish`. Add a transition-table test that covers every (state, clause kind) pair. | ADR-012 §4.1, §4.2 (mermaid and builder rules), §12.2 Check |
| FND-005 | medium | Parser entries can overlap and be resolved by table order. §3 says the parser composes families "through one closed table of entry productions". It does not say how an entry is selected. If the table is tried in order, two families whose productions accept the same leading token are resolved by position in the table. That is dispatch on registration order, which #210 forbids, and adding a family (such as `SumCase`'s `case`) could silently shadow another family. Fix: in §3 (Parsing structure), key the entry table by a closed leading-token or keyword enum, with exactly one family per key, selected by `match` (seam S2). State that a key claimed by two families fails the build or a table-construction test, and that the parser never tries entries in order and backtracks. | ADR-012 §3, §5.1 S2, §12.1 Parse · #210 acceptance "registration order" |
| FND-006 | medium | Unknown wire tags under cross-repository version skew have no specified outcome. S5 and S6 spread closed enums across QSL, IR, CG and RT, which move at separate revisions. §9 says wire strings are "decoded once in the v2 reader into closed tag and form enums". It does not say what the reader does with a tag or form it does not know. For example, QSL emits a new `SumCase` variant before IR's enum has it. A silent map to a nearby variant or to a default would be string-derived dispatch hidden in the decoder. Fix: in §9 and §5.1 (S5, S6), state that each v2-reader conversion is total over the wire vocabulary of the declared package version. An unknown tag or form refuses the package with a typed `CheckedPackageRefusalCode` naming the tag and the package version. Nothing is dropped or defaulted. The version check itself is decided in #211 (DA-14). | ADR-012 §5.1 S5, S6, §9 IR row, §12.1 IR · AD-016 arrow 2 refusal row · ADR-010 OBS-033 · #211 DA-14 |
| FND-007 | medium | A backend that is not registered is confused with a missing capability. §7.2 step 1: when the request names a `BackendId` that does not advertise every required pair, the candidate set is empty, so the item settles `unsupported` "naming the required capability". A request that names a `BackendId` which is not registered at all gets the same outcome. That diagnostic is wrong: the claim is not unsupported, the request is malformed. It also hides typos in `--target`. Fix: in §7.2 step 1 and in the §5.2 table, add a row: a named `BackendId` that is not in the registry settles `invalid-request`, naming the unknown identity (or is refused at the CLI edge conversion, §9 `--target` row). Keep `unsupported` for a registered backend that does not advertise the pair. When a requirement has more than one unmet pair, the warning names every one. | ADR-012 §5.2, §7.2, §7.3, §9 `--target` row · QSpec PR #133 FR-290 dispositions |
| FND-008 | medium | The line between solver absence and a failed run is not drawn. §7.4 maps "tool is absent or unusable" to the #229 absence outcome, "probed ... at the execute or prove stage". It does not say whether a tool that starts and then crashes, times out, or fails its version-pin check counts as absence (§7.4) or as a run outcome through S8 (`KaniOutcomeKind`). #229 must tell absence apart from timeout and hold. Without a boundary, the same failure could be reported both ways. The `BackendDescriptor` in §7.1 also carries no expected tool identity, although §7.4's outcome must name "the tool identity it expected". Fix: in §7.4, define absence as a failure of the pre-run probe: the tool was not found, or its identity does not match the expected pin. Everything after the tool starts maps through S8. Add the expected tool identity (for Kani, AD-016's tool pin) to `BackendDescriptor` in §7.1, or name where the adapter reads it. The outcome codes stay with #229 and #222. | ADR-012 §7.1, §7.4, §5.1 S8 · #229 scope (absence vs timeout vs hold) · #222 (timeout, bound exhaustion) · AD-016 arrow 6 |
| FND-009 | low | The `evaluate` hook and the replay stage do not cover families without evaluation. The §2 trait declares `evaluate` as a required method, returning `Outcome<Value>`. The following paragraph says `Relation` "implements no `evaluate` hook", which a required trait method with no default cannot express without a stub. §8 then routes every family's replay through "the family's `evaluate` hook" without saying what replay does for a `Relation` claim. `ProtocolClause` and `StateModel` evaluation also produce state observations, not a `Value`. Fix: in §2, move `evaluate` into a separate `Evaluate` trait with a family-specific associated output type, implemented only by families that have a reference evaluator. The S1 reference-evaluation seam gets an explicit arm for `Relation` that returns `unsupported` with a catalog code. In §8's Replay row, state that a claim of a family without `Evaluate` produces no replay request and records a typed FR-331 terminal record. | ADR-012 §2 trait and paragraph, §5.1 S1, §8 Replay row · AD-016 arrow 7 |
| FND-010 | low | Package emission is not atomic. `package(checked, out: &mut PackageEmitter) -> Result<(), PackageRefusal>` lets a hook write part of a node to the emitter before it fails. The record does not say whether that partial output is discarded, so a refused node could leave bytes in the checked package. That would break §8's rule of no substitute artifact at later stages. Fix: in §2, state that a node's emission is all-or-nothing. The emitter stages each node and commits it only on `Ok`, or the hook returns the node's value, which the seam appends. Add a test that injects a refusal part-way through emission and checks that the package bytes are unchanged. | ADR-012 §2 trait, §8 Package row and "no substitute artifact" rule · AD-016 terminal-disposition rule |
| FND-011 | low | Adding a backend can change the dispositions of existing requests, and §12 has no change set for it. Under §7.2, registering a second backend that advertises the same (capability, mode) as an existing one turns every request that names no backend from its prior disposition into `invalid-request`. That is correct under the no-preference rule, but it is a failure mode caused by registration alone. #212 scenario 7 (add a backend) will need this recorded. The Consequences section mentions it only in passing. Fix: add §12.3, "Add a backend": the descriptor, its negotiator arm, and the effect on requests that name no backend. List the corpus or CLI requests that must then name a backend, and add a test that registering an overlapping backend produces `invalid-request` for them rather than silently re-routing them. | ADR-012 §7.2, §12, Consequences · #212 scenario 7 |

## Checklist results

| Check | Result |
| --- | --- |
| Extension points: failure behaviour | Registry cases (§5.2), closed seams (§5.1), absent capability (§7.3) and solver absence (§7.4) are stated. Gaps: FND-001 (bounded-only candidates), FND-003 (exhaustion), FND-004 (builder after a refusal), FND-006 (unknown wire tag), FND-007 (unknown backend), FND-008 (absence vs run failure), FND-010 (partial emission). |
| Entity identity | `BackendId` is unique, and duplicates are refused. Registry equality does not depend on order. Checked-node identity is deferred to #211, as ownership requires, but its independence from check order is not stated (FND-002). |
| Evaluation purity | `requirements` is stated pure. `check`'s mutable context is unconstrained (FND-002). The checker does not read the registry (§6). |
| Topology | The family DAG is acyclic and its placement is deferred to #209. The builder state machine is incomplete after a refusal (FND-004). Cross-repository enum skew is not covered (FND-006). The candidate-set cases are incomplete for mode (FND-001). |
| Ownership | No finding asks ADR-012 to decide #229, #213, #185, #222, #209 or #211 content. Each fix states a contract and cites the owner. |
| Rules | No compatibility layer. §13.4 Q4 asks the owner rather than designing one. Current-state wording. Mermaid labels contain no `;`. No string dispatch after the edge (§9). No monolithic routine (§4). |

## Round 2

Reviewed ADR-012 at 8fb238b (diff from 048deb3). Round-1 verdict: REJECT.

| ID | Round-1 severity | Status | Note |
| --- | --- | --- | --- |
| FND-001 | high | resolved | §7.2 step 1 matches candidates on capability kind alone. §1.1 gives `negotiate_*` the extent rules: `requires-bound` for an unbounded extent on a bounded-only candidate with an available finite bound, `unsupported` (warned) without one, never `supported`. §5.2, §7.3 and the Alternatives entry agree. Never-narrowed rule kept. |
| FND-002 | medium | resolved | §2 makes declarations, the type environment and limits read-only. Only the meter, the diagnostic sink and the scope stack are mutable. Identity is a function of normalized content and declaration path, never a counter. The reversed-order test obligation is not listed in §3 (residue, no finding). |
| FND-003 | medium | resolved | §2 Structured outcome adds `Incomplete` for limit or meter exhaustion, and §8 maps it to the incomplete category. |
| FND-004 | medium | resolved | §4.2 is a runtime state machine with an explicit arm per (state, clause kind) pair. Out-of-order leaves the state unchanged. A refused in-order clause advances. `finish` runs a cross-clause check only over successfully checked clauses. |
| FND-005 | medium | resolved | §3 keys the entry table by a closed leading-token kind enum (seam S2), one production per entry, with no ordered trial. |
| FND-006 | medium | resolved | §9: an unknown wire tag is a typed refusal of the v2 reader, never a skipped node. |
| FND-007 | medium | resolved | §5.2 row and §7.2 unknown-backend class settle `invalid-request` naming the identity. See new FND-013 for the dual path. |
| FND-008 | medium | partial | §7.1 adds the pinned tool identity to `BackendDescriptor`. §7.4 defines absence as the pre-run probe result (missing tool or pin mismatch). It still does not state that a failure after the tool starts (crash, timeout) maps through S8, not §7.4. Residue is low. Fix: add one sentence to §7.4. |
| FND-009 | low | partial | `ReferenceEvaluation` is a separate trait, and S1 has an explicit `Relation` `unsupported` arm. §8 Replay still does not say what a `Relation` claim does at replay. `evaluate` still returns `Outcome<Value>` for state-observing families. |
| FND-010 | low | resolved | §2: `package` is all-or-nothing. |
| FND-011 | low | partial | §12.3 adds the backend change set. It does not record the effect that an overlapping registration turns requests that name no backend into `invalid-request`. Only Consequences mentions it. |

New findings:

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-012 | medium | An item without `Requirements` has no disposition path. §2 says "A claim that needs no backend capability yields no `Requirements` value and is not negotiated". §7.2 builds candidates from `Requirements`, so a proof or execute request for such an item has no candidate set and never reaches `negotiate_*`. It then gets no disposition and no accounting record. That breaks AD-016's terminal-disposition rule and its completeness test on zero records (§7.3 cites it). §12.1 also has a CG `negotiate_*` arm that returns `unsupported` for sum/case, and §12.2 has a Requirements row of "none", so these items can reach CG in the change sets but not by §2's rule. Fix: in §2, drop "and is not negotiated". State that every requested item reaches `negotiate_*` exactly once. In §7.2 step 1, add the case for an item with no capability kinds: the candidate set is the named backend, or every registrant when none is named. The IR-form arm (S6) then settles it, for example `unsupported` with a catalog code until a harness exists. | ADR-012 §2 Requirements row, §7.2, §7.3, §12.1, §12.2 · AD-016 terminal-disposition rule and completeness test |
| FND-013 | low | Two failure points for an unknown `BackendId`. §9 resolves a `BackendId` at the CLI edge by registry lookup and refuses an unknown name there. §5.2 and §7.2 settle the same case as `invalid-request` in `negotiate_*`, "unless the CLI edge refused it first". The observed outcome, and whether an accounting record exists, then depend on the entry path. Fix: in §9, let the CLI edge check only the syntax of a `BackendId`, and leave registry membership to §7.2, so `invalid-request` is the one outcome. Or state that a CLI refusal happens before any request exists and so creates no `request_index`. | ADR-012 §5.2 row 3, §7.2, §9 · QSpec FR-290 as amended by PR #133 |
| FND-014 | low | Mixed-mode ambiguity. Candidates match on kind alone, so an unbounded-mode backend and a bounded-only backend for the same kind make every unbounded request that names no backend `invalid-request`, although only one candidate could settle `supported`. This follows from the stated rule and is raised with the owner in §13.4 Q2. Fix: state this case in §7.2 or §12.3 so #212 scenario 7 records it. | ADR-012 §1.1, §7.2, §12.3, §13.4 Q2 |

Round 2 verdict: ACCEPT WITH FINDINGS
