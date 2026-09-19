---
id: SR-500
title: "Failure-domain analysis of ADR-010 to ADR-013 (ARCH-G1)"
type: SpecReview
analysis: failure-domain
scope: "spec/decisions/ADR-010-observed-architecture-baseline.md, spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md, spec/decisions/ADR-012-semantic-family-extension-contracts.md, spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: reviews
---
# SR-500: Failure-domain analysis of ADR-010 to ADR-013 (ARCH-G1)

## Summary

Reviewed state: ADR-010, ADR-012 and ADR-013 as merged on QSL `origin/main`
457a131, and ADR-011 at 457a131 plus the uncommitted #245 and O-03 edits on
branch `task/212-arch-g1-gate`. The four records were read together, as one
architecture, against the gate record SR-498 (`gate.md` in this directory).
QSpec inputs are at `origin/main` 1e8bb50. ADR-010 is the observed baseline.
It decides nothing, and no finding is raised against it.

Anchors: `ADR-011:320` is line 320 of the ADR-011 file in this working tree.
`SR-498:73` is line 73 of `gate.md`.

The checklist was applied across record boundaries. Trust boundaries: the
proof oracle, the replay parity map, the E5 binding and the stage failure
carriers. Entity identity: node ids, occurrences and model-bound ids. Purity
and independence: the #245 rule against the kernel's consumer list. Topology:
dependency nodes at replay, and the limits a replay recompile runs under.
SR-498 MD-1, MD-2 and FND-001 to FND-007 are not repeated.

Verdict: **BLOCK**. Three findings make SR-498 wrong as written:

- FND-001: the scenario 1 evidence cell says the CG oracle is independent of
  the `quire-exact` operation under proof. ADR-013 O-13 makes CG oracles
  consumers of that kernel.
- FND-003: the scenario 4 version cell credits ADR-013 O-04 with a preimage
  member that O-04 does not name.
- FND-004: the scenario 5 evidence cell cites the ADR-013 C-25 test. That test
  expects `requires-bound` for every unbounded domain, which contradicts the
  scenario's own failure cell.

Each of the three can be fixed with an edit to the text. None needs a new
decision. The other eleven findings do not change any SR-498 verdict. FND-002
needs a #211 decision and should be listed as remaining work under scenario 6.

## Findings

| ID | Severity | Summary | Refs |
|---|---|---|---|
| FND-001 | high | **Blocking.** The #245 independence rule conflicts with the kernel's consumer list. ADR-011 Decision 8 and §2.3 item 3 require the proof's expectation to be independent of the function under proof. ADR-013 O-13 names "CG oracles" as `quire-exact` consumers. §7.1 draws CG → `quire-exact`, and ADR-012 §12.1 lists `CG:src/oracle.rs` as a kernel consumer. After TK-03, RT ops also call `quire-exact`. So a CG oracle for an exact operator evaluates through the operation under proof. That is the agent-ix/quire-contract-runtime#57 `decode` defect at the scale of the whole operation, not one helper. SR-498 scenario 1 says the CG oracle is independent of the `quire-exact` operation. The ADRs as written contradict that. **Fix:** add to ADR-011 §2.3, after the independence paragraph: "An oracle that evaluates through a `quire-exact` operation shares that operation with any code under proof that calls it (ADR-013 O-13). For a proof of a `quire-exact` operation, or of an RT op that calls one, the expectation comes from the QSpec operation vectors or from a checked-in specification model that calls no `quire-exact` operation." In SR-498 scenario 1, Tests and evidence, replace "The CG oracle's expectation is derived independently of the `quire-exact` operation under proof." with "The proof's expectation is the QSpec operator vector or a specification model that calls no `quire-exact` operation. A CG oracle arm that calls `quire-exact` is not independent (ADR-013 O-13)." | ADR-011:134-143, 375-379, 390-391, 675-676; ADR-013:294, 890; ADR-012:737; SR-498:73 |
| FND-002 | high | Entity identity: the records disagree on what a node id identifies. ADR-011 E3 says "One id per node occurrence within a package". ADR-012 §2 derives the id from "normalized content and its declaration path". ADR-013 O-04 says equal ids mean "the same declaration position". ADR-013 O-07 keys occurrences by (node id, `role`, `ordinal`), so one id may have several occurrences. The QSpec preimage at 1e8bb50 is content-addressed. Its `ApplicationNode` has a nullable `declaration` and no occurrence member, so two identical anonymous subexpressions share one id. Two things then fail. First, E9 resolves a counterexample node "to its nested span", but a shared id has several regions, and the packet carries only the clause's occurrence key (O-25). Second, the obligation identity (E7) keys on a clause node id that can be shared. **Fix (needs #211):** choose content addressing with O-07 occurrences, or add an occurrence discriminator to the preimage (a QC-18 extension). With the first choice, ADR-011:320 reads "Node ids are content-addressed over the ADR-013 O-04 preimage. Structurally identical nodes share one id, and each source occurrence is keyed by (node id, role, ordinal) (ADR-013 O-07)", and QC-8 adds the occurrence key of the failing node to the packet. List this under SR-498 scenario 6 remaining work. | ADR-011:320, 326; ADR-012:210, 268; ADR-013:107, 174, 210, 594; QSpec `proposals/checked-package-v2/node-identity-preimage.schema.json` `ApplicationNode` |
| FND-003 | medium | **Blocking.** SR-498 scenario 4, Version impact, says "The domain package's `name@version` is part of the node-identity preimage (O-04)". ADR-013 O-04 and QC-18 say "the declaring package's declared name and version". They never say which package that is for a `ModelOwner` node: the QSL package being checked, or the domain package. `ModelOwner` in the QSpec schema has `identity` and `node` only, with no version. Under one reading, a domain-package version bump changes no model-bound node id unless the declaration content changes. Under the other, it changes every model-bound id of that package. Either way the cell states an O-04 rule that O-04 does not state. ADR-011 row 8, "new node ids for nodes bound to changed declarations", holds under content addressing (FND-002), but it leaves out the nodes that reference those nodes transitively. **Fix:** in SR-498:121, replace the first sentence with "The domain package's `DomainPackageRef` (identity, version, digest) enters `package_id` through the v2 lock `model_selections` (O-01, C-17). A model-bound node id changes when its preimage content or its `ModelOwner` changes (C-02). Which package's `name@version` QC-18 adds for a `ModelOwner` node is SR-500 FND-003." In SR-498:118, replace "and the preimage carries `name@version` (QC-18)" with "and so do the nodes that reference them". In ADR-013 O-04 Equality (ADR-013:174), add: "For a `ModelOwner` node the declaring package is the domain package, and its `name@version` is the `DomainPackageRef` identity and version." Owner #211 confirms the reading. | ADR-013:154, 156, 174, 828; ADR-011:818; SR-498:118, 121 |
| FND-004 | medium | **Blocking.** The unbounded-claim outcome differs between records. ADR-012 §1.1 settles an unbounded claim as `requires-bound` only when a finite bound is available. With no bound it settles `unsupported` with a warning, and with an unbounded-mode candidate it settles the form's own disposition. ADR-011 row 6 and FR-057:265-266 agree with ADR-012. ADR-013 O-20 says "An unbounded claim settles `requires-bound`", O-21 repeats it, and the C-25 test is "CG test with an unbounded domain returning `requires-bound`". SR-498 scenario 5 cites "The C-25 CG test" as evidence, and that test contradicts the scenario's own no-bound failure row. **Fix:** in ADR-013:481, replace "An unbounded claim settles `requires-bound`" with "An unbounded claim settles by ADR-012 §1.1: `requires-bound` when the candidate is bounded-only and a finite bound is available, `unsupported` with a warning when none is, and the form's own disposition when the candidate advertises unbounded mode". In ADR-013:513, replace "an unbounded proof claim settles `requires-bound` (O-20)" with "an unbounded proof claim settles as O-20 states". In ADR-013:701, set the C-25 test to "CG tests with an unbounded domain: `requires-bound` with an available finite bound, `unsupported` (warned) without one, and no narrowing". | ADR-012:184-190; ADR-011:816; ADR-013:481, 513, 701; SR-498:138, 140 |
| FND-005 | medium | Trust boundary: a per-item `proved` result can be vacuous. The #245 edit makes the gate floor count SUCCESS checks only. That rule covers #205 gate evidence only. ADR-013 O-16 maps IR `Proved` to FR-331 `proved` with no reachability condition (C-09). A Kani run in which every check of the obligation is UNREACHABLE therefore reaches the user as `proved`. **Fix:** the O-16 proof column maps a `Proved` run with zero SUCCESS checks in the obligation to `inconclusive` with a typed vacuity cause. IR distinguishes the case in `KaniOutcomeKind`. File it with the QC-16 amendment. | ADR-011:139-141, 384-387; ADR-013:354, 685, 826 |
| FND-006 | medium | The list of shared helpers is declared by the gate itself. §2.3 item 3 requires a mutation of "each helper that the oracle or expectation shares" with the code under proof. Nothing derives or checks that list. The census that #219 runs covers claimed-module lists only. A gate that leaves a helper off its list passes item 3 with no mutation run, which repeats the agent-ix/quire-contract-runtime#57 defect. **Fix:** in §2.3, say that the shared-helper set is computed from the build: the functions reachable from both the oracle and the code under proof. It is checked in, and #219 compares it with the computed set as it does the claimed-module list. | ADR-011:381-396; SR-498:73 |
| FND-007 | medium | Budget exhaustion at S3 has two carriers. ADR-012 §2 has a family `check` return `Incomplete` when "a limit or the meter is exhausted", and it gives `CheckContext` a meter. ADR-012 §13.5 maps check `Incomplete` to category `incomplete`. ADR-011 §2.3 and ADR-013 T-4 make every S1 to S4 limit a `StageFailure::Limit(LimitExceeded)`, and T-4 says `Incomplete` is an S6a meter outcome only. No conversion joins the two (ADR-013 R-01). ADR-011 §5 gives limit refusal its own exit code, so the two carriers can reach different exit codes. **Fix:** in ADR-012 §2 Structured outcome and the §13.5 Q210-3 row, write "A family `check` that reaches a limit returns `StageFailure::Limit(LimitExceeded)` with limit kind work budget (ADR-013 T-4). `Incomplete` is an S6a outcome only." | ADR-012:212, 214, 226, 464, 868; ADR-011:333-339, 490; ADR-013:84, 663 |
| FND-008 | medium | Some stages have no failure carrier. ADR-011 §2.3 and ADR-013 T-4 give `StageFailure` (refused, limit, fault) to S1 to S4 and the I2 reader only. S6a returns `FamilyOutcome { Evaluated, Refused }`, which has no fault arm. Yet ADR-013 O-16 says the executor produces `failed` when a runtime invariant breaks, and ADR-012 §13.5 relies on that. The layer-6 `replay` facade and layer-R `route` have no stated result type, although both refuse (E9 identity mismatch, C-28 unknown capability). The E9 on-error row has no internal-fault case. **Fix:** in ADR-011 §2.3, S6a's entry returns `Result<FamilyOutcome, InternalFault>`. `replay` and `route` return `Result<Staged<T>, StageFailure<C>>` with their own cause types. The E9 row adds "internal fault: no verdict, distinct outcome kind (§5)". | ADR-011:333-339, 354, 364-366; ADR-012:246, 868; ADR-013:361, 663, 704 |
| FND-009 | medium | Trust boundary at E9: the request supplies the rule that decides parity. The replay request carries the #231 "outcome→verdict map", and CG copies it into the request (C-12). CG is the party whose backend evidence the replay checks. ADR-011:278 fixes three S6a results as never agreement. The mapping for `Undefined`, a `Completed` value and `FamilyOutcome::Refused` comes from request data. No record says who authors the map, that it is total, or that it is pinned. **Fix:** the map is fixed by QSpec per O-16 category (FR-323, with QC-8), not per request. If it must travel in the request, pin its digest in the request identity and state that `Undefined` and `FamilyOutcome::Refused` never count as agreement. | ADR-011:250, 254-256, 278-279; ADR-013:635, 639, 647, 688 |
| FND-010 | medium | Typestate: the S4 type and its `call` entry sit in different modules. `CheckedPackage` is defined in layer-4 `package` with constructors private to that module (ADR-011 §4, ADR-013 T-1). `call` is implemented in layer-5 `value::expression` and must read the package's evaluable content, such as bodies and dispatch tables. If the fields are `pub(crate)`, any QSL module can build the type with a struct literal, which breaks §4. The AD-016 path `value::expression::CheckedPackage::call` also needs a `pub use` of the type in `value::expression`. ADR-011:106 and :236, and ADR-013 §9 DA-04, still name `value::expression::CheckedPackage` as the type. **Fix:** in ADR-011 §4, `package` keeps the fields and the constructor private and exposes read-only accessors. `value::expression` names the type at the AD-016 path through one `pub use`, and that path is the contract, not a second type. A `compile_fail` test shows that `value::expression` cannot construct it. | ADR-011:106, 236, 431-433, 470-472; ADR-013:329, 660, 908 |
| FND-011 | low | Trust boundary at E5: the third condition of the verified binding has no source. The E5 row admits v2 bytes "under the §4 verified binding", including "identity pinned by the request". §4 defines that binding for I2 in QSL `library`, which IR cannot call (FB-05). Nothing names the request that pins the expected `package_id` on the IR side. **Fix:** in the E5 row, IR's reader enforces conditions 1 and 2 under FR-322 (IR TC-048). The expected `package_id` for condition 3 is the one the driver received from E4's `EmittedPackage`, passed beside the bytes. | ADR-011:246, 290, 436-446; ADR-013:142 |
| FND-012 | low | Topology at E9: the recompile limits are not stated. The replay request carries the S6a accounting limits only. `replay` recompiles S1 to S4 under stage limits that no member of the request carries. A recompile can therefore hit a limit that the original compile did not, and the E9 on-error row does not list a limit refusal. **Fix:** the replay recompile runs under the stage limits recorded for the proved package, or under limits no lower than those. A limit refusal at E9 yields no verdict with its `LimitExceeded` cause, never `inconclusive`. | ADR-011:250, 354, 356-362; ADR-013:635, 638 |
| FND-013 | low | Topology at E9: the source map for a dependency node is not named. A nested counterexample can fail inside a dependency's function. E9 resolves the node "through the v2 source map", but each package keeps its own source map (E4, O-12), and C-14 is tested over one package. **Fix:** in the E9 row, a `WireNodeId` of a dependency node resolves through the source map of the dependency package that `replay` recompiled, under the `RawSourceRef` digests the packet carries (O-25). | ADR-011:264-271, 321, 326; ADR-013:281, 596, 690 |
| FND-014 | low | Stale cross-references in ADR-013. §9 DA-08 says "one checked clause kind in `value::expression`", but O-10 and ADR-012 S5 put it in the layer-3 `check` core. §9 DA-04 names `value::expression::CheckedPackage` (see FND-010). The Context section cites ADR-011 at `a4ce336` and ADR-012 at `e71986a`, but the section citations match 457a131. **Fix:** DA-08 becomes "O-10: one checked clause kind in the layer-3 `check` core". DA-04 becomes "O-15, T-1: `CheckedPackage`, defined in layer-4 `package`, canonical for S4". Update the two sibling heads. | ADR-013:64-66, 244, 908, 912; ADR-012:390 |

## Method

- Checklist, applied across records:
  - Trust boundaries: E5, E9, the proof oracle under ADR-011 §2.3, and the
    failure carrier of each stage and facade. For each one, what admits the
    input, who supplies the deciding data, and what happens on a fault.
  - Entity identity: node id, occurrence, `DeclarationKey`, `ModelOwner` and
    obligation identity, across ADR-011 §2.2, ADR-012 §2 and ADR-013 O-03,
    O-04, O-07 and O-09.
  - Purity and independence: the #245 rule against the kernel consumers in
    ADR-013 O-13 and the crate graph in ADR-011 §7.1.
  - Topology: replay over a dependency closure, and the limits of a replay
    recompile.
- Disposition vocabulary: ADR-012 §1.1 and §7, ADR-013 O-16, O-20 and O-21,
  and ADR-011 row 6 were compared with FR-057:265-266 as SR-498 cites them.
- Evidence outside the records: the QSpec `node-identity-preimage.schema.json`
  at `origin/main` 1e8bb50, for FND-002 and FND-003.
- Not repeated: SR-498 MD-1 (crossing test home), MD-2 (a `BackendId` with
  no CG kind) and FND-001 to FND-007. FND-009 touches the parity comparator
  that MD-1's ruling keeps in CG, and it does not reopen MD-1.
- No finding proposes a compatibility layer. Every fix states the current
  design.

## Round 2

- FND-004 is resolved: ADR-013 O-20, O-21 and C-25 now settle an unbounded
  claim as ADR-012 §1.1 does.
- FND-003 is resolved in SR-498: scenario 4 no longer claims which
  `name@version` enters a `ModelOwner` preimage, and cites this finding.
- FND-001 is resolved in SR-498: scenario 1 states rule 8 as the requirement
  on the oracle, and SR-498 FND-008 carries the proposed O-13 text as
  Remaining work: #244.

Verdict after Round 2: ACCEPT WITH FINDINGS.

## Round 3

Dispositions after the #212 rulings, 2026-09-19 (issue #212, newest two comments).

- FND-001 is resolved (#212 rulings, 2026-09-19, SR-500 FND-001): ADR-011 §2.3 carries the fix
  text, ADR-013 O-13 limits CG oracles to kernel types and never the
  operation under proof, and SR-498 scenario 1 uses the fix wording.
- FND-002 is resolved (#212 rulings, 2026-09-19, SR-500 FND-002): node ids are content-addressed
  with O-07 occurrences in ADR-011 E3, ADR-012 §2 and ADR-013 O-04. The packet
  and the E7 obligation identity carry the failing node's occurrence key, and
  E9 resolves spans by it. Remaining work: agent-ix/quire-specification#141
  (QC-8).
- FND-003 is resolved (#212 rulings, 2026-09-19, FND-003): ADR-013 O-04 names the domain package's
  `name@version` for a `ModelOwner` node, and SR-498 scenario 4 cites it.
- FND-004 was resolved in Round 2.
- FND-005 is resolved (#212 rulings, 2026-09-19 and addendum, FND-005): a vacuous `Proved` maps to
  `KaniOutcomeKind::Inconclusive` with cause `kani_vacuous_proof` in O-16
  and C-09, and a run mutation of the map turns C-09 red. Remaining work:
  agent-ix/quire-contract-ir#146, agent-ix/quire-specification#141.
- FND-006 is resolved (#212 rulings, 2026-09-19, FND-006): ADR-011 §2.3 computes the shared-helper
  set from the build, and #219 compares it with the checked-in set.
- FND-007 is resolved (#212 rulings, 2026-09-19, SR-500 FND-007): a family `check` limit is
  `StageFailure::Limit(LimitExceeded)` in ADR-012 §2, §6, §8 and §13.5 and in
  ADR-013 T-4.
- FND-008 is resolved (#212 rulings, 2026-09-19, FND-008): S6a returns
  `Result<FamilyOutcome, InternalFault>`, `replay` and `route` return
  `Result<Staged<T>, StageFailure<C>>`, and the E9 row has an internal-fault
  case.
- FND-009 is resolved (#212 rulings, 2026-09-19, FND-009): the outcome→verdict map is fixed by
  QSpec per O-16 category and is not a request member (ADR-011 E9, ADR-013
  O-26, O-27, C-12, QC-8). Remaining work: agent-ix/quire-specification#141.
- FND-010 is resolved (#212 rulings, 2026-09-19, FND-010): ADR-011 §4 keeps `CheckedPackage`'s
  fields and constructor private, `value::expression` names it by one
  `pub use`, and a `compile_fail` test shows it cannot construct it.
- FND-011 is resolved (#212 rulings, 2026-09-19, FND-011): the E5 row passes the expected
  `package_id` from E4's `EmittedPackage` beside the bytes.
- FND-012 is resolved (#212 rulings, 2026-09-19, FND-012): the E9 recompile runs under the proved
  package's recorded stage limits, and a limit refusal carries
  `LimitExceeded`.
- FND-013 is resolved (#212 rulings, 2026-09-19, FND-013): a dependency's `WireNodeId` resolves
  through the recompiled dependency package's source map.
- FND-014 is resolved (#212 rulings, 2026-09-19, FND-014): ADR-013 DA-04, DA-08 and the sibling
  record citation are fixed.

Verdict after Round 3: ACCEPT.

## Round 4

Dispositions after the #212 round-2 rulings, 2026-09-19 (issue #212, round-2
comment), and the #247 editorial alignments.

- FND-009 is restated: the outcome→verdict map is fixed by QSpec per O-16
  category, and the request carries no map (ADR-011 E9, ADR-013 O-26,
  QC-8).
- FND-012 is refined by the round-2 ruling: the S1 to S4 stage limits travel
  in the #231 replay request (ADR-013 O-26, QC-8), and the recompile runs
  under them. Remaining work: agent-ix/quire-specification#141.
- `CheckedPackage::call` admits arguments before evaluation. An
  `InputRefusal` is an O-26 refusal that `replay` carries as a
  `StageFailure::Refused` cause, never `inconclusive` (ADR-011 E9).

Verdict after Round 4: ACCEPT.
