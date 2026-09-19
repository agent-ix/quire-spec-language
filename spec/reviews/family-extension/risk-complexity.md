---
id: SR-479
title: "Risk and complexity review of ADR-012 semantic-family extension contracts"
type: SpecReview
analysis: risk-complexity
scope: "spec/decisions/ADR-012-semantic-family-extension-contracts.md; spec/spec.md ADR-012 index row and relationship"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: reviews
---
# SR-479: Risk and complexity review of ADR-012

## Summary

Reviewed commit: 048deb3 (branch `task/210-family-extension`), plus the
uncommitted two-line Context edit that adds the `ADR-012 S<n>` citation form.
That edit changes no finding. The `spec/spec.md` index row (line 388) and the
`ADR-012` relationship (line 39) are present and consistent with the record.

The record meets most of #210's acceptance bullets. Families form a closed set
with a DAG. The closed seams S1–S8 name owners. Registration refuses
explicitly, with no first-wins default. The four admissions are separated. The
registry is an explicit value. Solver absence has no fallback. The two change
sets are tabled.

One high problem remains. The record says CG `negotiate_*` is the single
negotiation point, yet §7.2 and Consequences give each backend "its own
negotiator". The record also does not say how a QSL-held registry routes
items after a CG settlement without a QSL → CG normal dependency. AD-016 WP9
makes CG → QSL a normal dependency, so that edge would form a cycle. This
contradicts AD-016 and QSpec PR #133 (FND-001).

The medium findings are about whether slices can be built as sized:

- the builder typestate cannot refuse an out-of-order clause at run time
  (FND-003);
- `Expression` and `infer_form` mix `Value` and `StateModel` forms, so #214's
  thin-seam work spills past its one-family slice (FND-004);
- the trait's required `evaluate` conflicts with `Relation` having no
  `evaluate` (FND-005);
- the compile-failure test has no stated mechanism (FND-007);
- the §12 change sets leave out catalog and wire-version rows (FND-008);
- the OBS-003 removal breaks FR-036-AC-5 and AC-6, and no requirement edit
  covers them (FND-009);
- #185, #188 and #189 wait on a CG ticket that has not been opened (FND-002).

Verdict: REJECT (1 high, 7 medium, 5 low). Every fix is local to the record.

## Method

- Read ADR-012 at 048deb3 and its working-tree diff, the `spec/spec.md` index
  row, ADR-010 (§4.3 dispatch sites, OBS/DA routing) and the issue bodies of
  #210, #205, #212, #185, #213, #214, #221 and #229.
- Read accepted QSpec AD-016 (`origin/main`): terminal-disposition rule,
  arrows 1–7, shared-type strategy and crate graph. Read QSpec PR #133
  (FR-290/AD-010 single negotiation point) and its SR-604 and SR-605.
- Checked the record's code claims against the worktree:
  - `src/value/expression/check.rs:683-966`: `infer_form`, 284 lines. It has
    arms for `Value` forms and for `StateModel` forms (`AllInstances`,
    `Lookup`, `Dispatch`, `Pre`).
  - `src/value/expression/ir.rs:214`: `NodeKind`.
  - `src/linking/composed/requests.rs:36,82`: the four-kind `Capability` and
    `Backend { identity: &str }`.
  - `src/lowering/target.rs:39-80`: the `ProjectionTarget` closed enum with a
    total `FromStr`.
  - `src/value/division.rs:223` and `src/value/ieee.rs:830,882`: the
    negotiate copies.
  - `tests/composed_admission_stages.rs:201-653`: TC-115, which traces
    FR-036-AC-5 and AC-6 and depends on `requests::report`'s `Backend` and
    its `Unsupported*` dispositions.
- Scored each decision item on technical risk and volatility. For each slice
  that the record hands on (§14), judged whether it is buildable in 1–3
  focused sessions and whether any abstraction is speculative.

## Risk register

| Item | Tech risk | Volatility | Drivers | Mitigation named in the record? |
|---|---|---|---|---|
| §7.2 candidate set → CG `negotiate_*` → registry routing | High | High | Crosses QSL, CG and any third-party backend (quire-analyze #40). It adds a new input to AD-016 arrow 4, and the crate direction is undecided (§13.1 Q1–Q2) | No. FND-001, FND-002 |
| §4.2 staged builder typestate | Medium | Low | Clause order in source is known only at run time, and diagnostics must aggregate after a refusal | Partly. FND-003 |
| §4.3 / S2 thin `infer_form` seam (#214) | Medium | Medium | One 284-line `match` over one `Expression` enum that spans two families | No. FND-004 |
| §2 `FamilyContract` trait, S1 `FamilyKind` | Low | Medium | A static trait with no generic consumer, a required hook that one family must omit, and a payload-free enum whose seams duplicate S2 and S3 | No. FND-005, FND-006 |
| §5.3 compile-failure test obligation | Medium | Low | A variant cannot be added to an in-crate enum from a `trybuild` crate | No. FND-007 |
| §12 change sets | Medium | High | Depend on #229 Q3 (frame kind), the QSpec catalog, the v2 wire and QSpec #115 | Partly. FND-008, FND-013 |
| §10 OBS-003 linker negotiation removal | Medium | Low | Removes behaviour that FR-036-AC-5 and AC-6 (TC-115) specify | No. FND-009 |
| §1.1 / §2 `Requirements` mode | Low | High | Mode placement is open in #229 Q2 and #222 | Partly (§13.3 Q2). FND-010 |
| §9 string edges for open identities | Low | Low | `BackendId` belongs to an open set, but the §9 rule requires a closed enum | No. FND-011 |
| §11 L1-D1 relaxed edges | Low | Medium | Five tickets relaxed; #188 and #189 kept | Yes. The critical path is incomplete: FND-002 |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The record contradicts the single negotiation point and leaves a crate cycle open. §7.2's "exactly one" row settles "that backend's own settlement of the IR form". Consequences says "A new backend costs one descriptor plus its own negotiator". For a backend outside CG, such as quire-analyze's implication backend (FR-290 registrant, SR-605 row quire-analyze #40), that is a second negotiation point, which AD-016 ("CG `negotiate_*` is the single capability negotiation point") and PR #133's FR-290 text forbid. The alternative is that CG depends on every backend, which is not viable. Separately, step 3 has the #185 registry in QSL route items after CG settles them. If a QSL crate makes that call, QSL gains a normal dependency on CG. AD-016's crate graph makes CG → QSL a normal dependency in WP9, so the two edges form a cycle. §13.1 Q2 also offers "a field of the checked package" as a carrier for the candidate set. That contradicts §6 ("a checked package is the same bytes whatever backends are registered") and the rejected "Selection inside the checker". Fix: (1) Delete "its own negotiator" from Consequences. Rewrite the "exactly one" row as "CG `negotiate_*` settles the IR form against that candidate's advertised (kind, mode) pairs". State that a backend contributes only a descriptor, and that no backend-side negotiator exists. (2) Add a constraint for #209: no QSL library crate calls CG, and the caller that holds the registry sits above both (CLI or orchestration crate), or CG receives the registry value. (3) Strike the checked-package option from §13.1 Q2 and name the QSpec contract owner for the candidate-set type (#212 pass condition "named QSpec contract owner"). | ADR-012 §7.2, §13.1 Q1–Q2, Consequences; AD-016 Decisions, Crate graph; QSpec PR #133 FR-290, AD-010 |
| FND-002 | medium | The critical path runs through tickets that do not exist. §14 gives "`negotiate_*` taking the candidate set" and "IR tag and form enums at the v2 reader edge" to tickets "to be opened" by the CG and IR owners. Under §7.3, the `unsupported` settlement for an empty candidate set happens in CG. So #185's exit criterion ("corpus case for a claim with no registrant settles `unsupported` with the warning") cannot close without the unopened CG ticket. The §11 "kept" edges for #188 and #189 then wait on it too, but §11 names only #185. #212 requires "bounded implementation tickets" for each scenario. Fix: open, or name by number, the CG and IR tickets before #212. Add them to the §11 reason cells for #188 and #189 ("closure waits on #185 and CG #NNN"). State in §14 that #185's absent-capability corpus case needs the CG ticket. | ADR-012 §7.3, §11, §14; #185 exit; #212 pass conditions |
| FND-003 | medium | The builder in §4.2 cannot be the stated typestate. A Rust typestate encodes order in types, so an out-of-order transition does not compile. But clause order in QSL source is known only at run time. Driving a compile-time typestate from a parsed clause list needs exactly the runtime sequencer that the rule "No sequencer inspects the clause list afterwards" forbids. "A refused clause does not stop sibling clauses" also conflicts with transitions that return the next state or refusals, because after a refusal there is no next state to continue from. As written, #214 and #220–#223 each have to redesign the builder. Fix: specify the builder as a closed runtime state enum with one `accept(state, clause) -> (State, Vec<Refusal>)` transition per clause kind. The out-of-order refusal is the `(state, clause)` arm with no legal successor. The state always advances, so siblings are still checked. Keep typestate only for `begin` → `finish`, if at all. Alternatively, make clause order a grammar property (syntax admission) and drop the out-of-order refusal from the builder. | ADR-012 §4.2, §12.2 Check row; #210 acceptance bullet 2 |
| FND-004 | medium | The `Value` / `StateModel` boundary runs through one enum and one `match`, and #214's slice crosses it. `infer_form` (`check.rs:683-966`) matches the single `Expression` enum. It holds `Value` arms (literals, `Let`, `If`, `Binary`, `Call`, `Record`, `Convert`, `Count`, `Sum`) and `StateModel` arms (`AllInstances` at `:950`, `Lookup` at `:953-958`, `Dispatch` at `:959-963`, and `Pre`, `Deref`, `Present`). §3 gives lookups and dispatched calls to `StateModel`, but S2 is "the one `Expression` enum" owned by "QSL, owning family", and §4.3 has #214 turn the whole `infer_form` into a thin seam. That is more than #214's scope ("function application as the sole representative family"; "does not modify unrelated family internals"), and there is no rule for which family owns a shared enum. Fix: choose one rule and state it. (a) Split `Expression` into per-family sub-enums (`Expression::Value(ValueForm)`, `Expression::State(StateForm)`); S2 is then the outer enum, owned by QSL, and each sub-enum is owned by its family. Or (b) keep one enum, and state that #214 thins only the `Value` arms, leaving each `StateModel` arm as one call into its existing helper for #220. Either way, list the `StateModel` arms by name. | ADR-012 §3, §4.3, §5.1 S2; `src/value/expression/check.rs:683-966`; #214 scope and acceptance |
| FND-005 | medium | The `FamilyContract` trait contradicts itself. The design shape declares `fn evaluate(...)` as a required method. The text then says `Relation` "implements no `evaluate` hook" and must not stub it. A required method cannot be omitted, and a default body is the forbidden stub. The trait is also dispatched only through closed-enum `match` arms (§2: "not an object-safe plug-in interface", no `dyn`), and no generic code is bounded on it. Its only effect is to force per-family signature conformance, so the `const KIND` and associated types are mostly speculative. Fix: split evaluation into a separate `ReferenceEvaluation` trait, implemented only by the five families that evaluate. Give S1's evaluation seam an explicit `Relation` arm that returns a typed `unsupported` with a catalog code. Also state what the base trait buys, namely compile-checked per-family signatures used by the S1–S3 arms, so that #214 does not add generic plumbing. | ADR-012 §2 trait sketch and closing paragraph, §5.1 S1, §8 Replay row |
| FND-006 | low | S1 duplicates S2 and S3. `FamilyKind` carries no payload, but the seams listed for it ("check dispatch, package emission dispatch, reference evaluation dispatch, requirement derivation") actually match on the parsed form (S2) or the checked node (S3). A `match` on a bare `FamilyKind` cannot call `check(form)` without the form. So adding a `FamilyKind` variant fails only in `catalog_code()`'s prefix, not at the dispatch seams. Fix: limit S1 to what matches on `FamilyKind` itself (the catalog prefix and any per-family registration table). Move the dispatch seams to S2 and S3, and state that a new family adds a variant to S1, S2 and S3 together. | ADR-012 §5.1 S1–S3, §5.3 |
| FND-007 | medium | §5.3's test obligation has no mechanism. "#214 adds one test-only variant and shows a compile failure at each of S1–S4" cannot be an ordinary unit test. A `trybuild` or `compile_fail` crate cannot add a variant to an enum defined in `quire-spec-language`. A `#[cfg(test)]` variant would break the crate's own test build. Without a stated mechanism, the #214 acceptance bullet ("explicit exhaustive failure at every required core seam") risks becoming a one-off manual check. Fix: specify the mechanism. For example, a `--cfg qsl_seam_probe` gate adds the probe variant, and an `xtask seam-probe` builds with the flag and asserts that the compiler reports E0004 at exactly the listed seam functions and nowhere else. State that the job runs in the full gate. | ADR-012 §5.3; #214 acceptance bullet 2 |
| FND-008 | medium | The §12 change sets leave out rows that the record's own rules require. §3 requires every `Cause` variant to have "a catalog code in the vendored diagnostic catalog". AD-016 makes that catalog QSpec `native-diagnostics.md`, vendored with `VENDOR.json` digests. So §12.1's non-exhaustive, unreachable-arm and wrong-variant causes and §12.2's frame, anchor-scope and clause-order causes each need a QSpec catalog row plus a re-vendor. Neither table lists one. Neither table states the `quire.checked-package/v2` schema or version impact (QSpec I04), which #212 requires per scenario. §12.1's Requirements row cites "S7 (explicit arm)", but adding sum/case adds no capability kind, so S7 is not forced. §12 says "A row outside the table is a defect … reopens #210". Fix: add a "Diagnostic catalog" row (QSpec `native-diagnostics.md` plus the QSL, IR and CG re-vendor) and a "Wire version" row (QSpec I04 schema change and the v2 version decision) to both tables. Change §12.1's Requirements seam cell to "none". | ADR-012 §3, §12.1, §12.2; AD-016 Shared-type strategy (catalog), Arrow 2; #212 scenario record fields |
| FND-009 | medium | The OBS-003 removal breaks specified QSL requirements, and no requirement edit covers it. §10 removes `requests::report`'s `Backend` parameter and its `UnsupportedCapability` and `UnsupportedFamily` dispositions (`requests.rs:82`). FR-036-AC-5 ("changing … backend leaves static meaning unchanged") and FR-036-AC-6 ("a supported state request and an unsupported … projection both remain in the request report") specify that behaviour. TC-115 (`tests/composed_admission_stages.rs:201-653`, seven traced tests) exercises it. After the removal these acceptance criteria describe nothing, and #185 would have to rewrite a QSL FR it does not own. Fix: in §10 OBS-003 and §14, name the FR-036-AC-5 and AC-6 amendment (the linker records requests as data; dispositions move to the `negotiate_*` ledger) and the TC-115 rewrite, and assign both to #185 with its `/specify` cycle. | ADR-012 §10 OBS-003, §14; `spec/functional/FR-036-link-composed-native-packages.md:136-137`; `tests/composed_admission_stages.rs:201-653` |
| FND-010 | low | `Requirements { capabilities, mode, bound }` gives one mode and one bound per checked node. Advertisements and selection, however, key on (kind, mode) pairs. A node whose claims need different modes (a bounded range obligation beside an unbounded population claim) cannot be expressed, so it would be forced into the stronger mode or split without a rule. Fix: make `Requirements` a set of (kind, mode, bound) entries, or state the invariant "one mode per checked node" and name the refusal for a node that would need two. Keep the decision independent of #229 Q2 by saying the pair is the selection key either way. | ADR-012 §1.1, §2 Requirements row, §7.1, §13.3 Q2 |
| FND-011 | low | The §9 edge rule does not fit open identities, and the `--target` row misdescribes the code. §9 requires "one total conversion into a closed enum", but §5.2 makes backends an open set, so a CLI string cannot convert totally into `BackendId`. The step is a registry lookup with a typed unknown-backend refusal. Also, `--target` already converts once, through a total `FromStr` into the closed `ProjectionTarget` enum (`target.rs:39-80`). It is not string dispatch. Its three values (`boolean-oracle/v1`, `integer-ir/v1`, `state-scalar-ir/v1`) are QSL lowering domains, not backends. Fix: add an edge rule for open identities ("lookup in the registry value; unknown name → typed refusal naming it"). In the `--target` row, state whether the three projection targets become three descriptors or stay a closed QSL lowering enum next to the registry. | ADR-012 §9 rule and `--target` row, §5.2; `src/lowering/target.rs:39-80` |
| FND-012 | low | OBS-012's wording conflicts with accepted AD-016's text. OBS-012 says "Language admission is called semantic admission (§6) and is not a capability". AD-016's Shared-type row reads "QSL `Capability` = language admission (FR-290, six kinds)", arrow 1 records `capability_report` at language admission, and PR #133's SR-604 repeats "QSL `Capability` stays language admission". The substance matches, since §6 records `Requirements` as data at semantic admission, but a reader could take the record to reopen AD-016. Fix: reword as "Per AD-016, QSL's `Capability` is the FR-290 kind that semantic (language) admission records in `capability_report`; it grants nothing and is not a disposition". | ADR-012 §10 OBS-012; AD-016 Arrow 1, Shared-type strategy; QSpec PR #133 SR-604 |
| FND-013 | low | §12.2 is bounded only once #229 Q3 is answered. Its Requirements row needs "the frame obligation's capability kind (from #229)". §13.3 Q3 says that if none of the six kinds fits, "the vocabulary question returns to QSpec". If that happens, the change set gains a QSpec FR-290 edit and an S7 variant in every repository, and #212 scenario 3 cannot be walked. Fix: mark §12.2's Requirements row "blocked on #229 Q3", and add a conditional row "QSpec FR-290 kind + S7 arms in QSL, IR, CG" so the bounded set stays explicit in both outcomes. | ADR-012 §12.2, §13.3 Q3; #212 scenario 3; #229 |

## Top hazards

1. FND-001: the negotiation point and crate direction. This is the only item
   that contradicts AD-016. It also decides whether #185, the CG ticket and
   #209's crate DAG can be built at all.
2. FND-002: the #185, #188 and #189 closure path goes through an unopened CG
   ticket.
3. FND-004: the family boundary inside `Expression` and `infer_form` decides
   whether #214 stays a single-family slice.
4. FND-003: the builder mechanism is reused by #220–#223. A wrong shape here
   is repeated four times.

## Failure-domain gaps

`spec/reviews/family-extension/failure-domain.md` covers the same record. The
overlaps with this review are FND-001 (the terminal disposition when the
candidate set and the negotiator disagree) and FND-003 (whether diagnostics
still aggregate after a clause is refused).
