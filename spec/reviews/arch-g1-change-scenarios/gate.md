---
id: SR-498
title: "ARCH-G1 change-scenario gate over ADR-011, ADR-012 and ADR-013"
type: SpecReview
analysis: gap-analysis
scope: "spec/decisions/ADR-010-observed-architecture-baseline.md, spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md, spec/decisions/ADR-012-semantic-family-extension-contracts.md, spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md, spec/functional/FR-057-admit-shared-capability-kinds.md"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: reviews
---
# SR-498: ARCH-G1 change-scenario gate (#212)

## Summary

This record walks the seven #212 change scenarios through ADR-011, ADR-012 and
ADR-013 as merged on QSL `origin/main` 457a131, plus the changes this PR makes
(below), including the #212 rulings of 2026-09-19 (issue #212: the round-1,
round-2 and round-3 comments). Cross-repository inputs are QSpec
`origin/main` 1e8bb50 (FR-290, amended AD-016 from
agent-ix/quire-specification#140) and Contract IR `origin/main` 65aa282.

Line anchors are to this PR's head. `ADR-011:135` means line 134 of
`spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md`. FR-057
anchors are unchanged from `origin/main`.

| # | Scenario | Verdict |
|---|---|---|
| 1 | Exact scalar operator | PASS |
| 2 | Sum type and exhaustive `case` | PASS |
| 3 | Protocol clause with a frame or scoped anchor | PASS |
| 4 | Model-bound identity change that preserves provenance | PASS |
| 5 | Unsupported unbounded proof request | PASS |
| 6 | Nested counterexample, replayed natively | PASS (on the MD-1 ruling) |
| 7 | Backend added without syntax or checking authority | PASS (on the MD-2 ruling) |

Gate verdict: **passed.** MD-1 (#209) and MD-2 (#210) are decided by the
#212 rulings, and this PR applies them. Cross-repository work that the
rulings name is cited as remaining work in each cell.

## Changes in this PR

- **#245, ADR-011 §2.3 rule 8.** Decision 8 (ADR-011:135) now requires a proof
  expectation derived independently of the function under proof. A run mutation
  of each helper shared between the expectation and the function under proof
  must fail the proof. The discharge floor counts SUCCESS checks only;
  UNREACHABLE checks, and every status other than SUCCESS, are not counted as
  discharged. The rule is restated in the §1.1 skeleton bullet (ADR-011:228),
  §2.3 proof-stage acceptance (ADR-011:403-456), FB-10 (ADR-011:482), the §9
  agent-ix/quire-contract-runtime#53 row and the Consequences bullet.
- **Editorial, ADR-013 O-03 alignment.** ADR-011 E3 (ADR-011:348) and §10
  row 8 (ADR-011:889) said that S3 mints `DeclarationKey`s. ADR-013 O-03
  (ADR-013:153) says the domain package assigns the key and QSL never mints
  one. Both cells now say that I1 intake admits the domain package's keys and
  S3 mints node ids from them (C-02, ADR-013:682).
- **Spec-review fixes (editorial).** ADR-011 FB-10 (ADR-011:482) now forbids
  only a shared helper with no failing run mutation, matching rule 8 (SR-500,
  SR-501, SR-506). The counting sentence now says no status but SUCCESS is
  counted. §2.3 defines a shared helper and requires a published
  shared-helper list (SR-503 FND-002). T-10 and the Consequences bullet carry
  the #245 rule. ADR-013 O-20, O-21 and C-25 (ADR-013:485, 517, 705) now
  settle an unbounded claim as ADR-012 §1.1 does (SR-500 FND-004, SR-501
  FND-007).
- **#212 rulings, #209 (ADR-011).** MD-1: QSL owns the replay crossing test and
  reads IR's agreement vectors as raw bytes through one accessor in the pinned
  `quire-contract-model` (§7.1, ADR-011:764; OBS-040, ADR-011:866). SR-500
  FND-001 and FND-006 in §2.3 (ADR-011:417, 442). FND-008: S6a, `replay` and
  `route` result types and the E9 internal-fault case (ADR-011:364, 389).
  FND-010: `CheckedPackage` accessors, one `pub use` and a `compile_fail` test
  (§4). FND-011: the E5 expected `package_id` (ADR-011:253). FND-012 and
  FND-013: E9 recompile limits and dependency source maps (ADR-011:268, 298).
  SR-501 FND-003: `CatalogCode` in F `diagnostic`. SR-502 FND-001: #212 does not
  wait on agent-ix/quire-contract-codegen#87. SR-502 FND-002 and SR-504 FND-006:
  lane deletions name the implementation tickets.
- **#212 rulings, #210 (ADR-012).** MD-2: the run-level refusal
  `invalid_capability`/`unknown-backend` (ADR-012:564). SR-500 FND-007: a
  `check` limit is `StageFailure::Limit(LimitExceeded)` (ADR-012:214).
  SR-505 FND-001: `route` computes and routes, CG `negotiate_*` settles
  (ADR-011:305, ADR-012:493, 543).
- **#212 rulings, #211 (ADR-013).** SR-500 FND-002: content-addressed node ids
  with O-07 occurrence keys in the packet and the obligation identity
  (ADR-013 O-04, O-07, O-09, O-25; ADR-011 E3, E9; ADR-012:210). FND-003: the
  `ModelOwner` `name@version` (ADR-013 O-04). FND-005: a vacuous `Proved`
  maps to `KaniOutcomeKind::Inconclusive` with cause `kani_vacuous_proof`
  (ADR-013:361, 689). FND-009: the outcome→verdict map is fixed by QSpec per
  O-16 category (O-26, O-27, C-12, QC-8). FND-014: DA-04, DA-08 and the
  sibling citation. SR-500 FND-001: O-13 (ADR-013:291).
- **#212 round-2 rulings.** The S1 to S4 stage limits travel in the #231
  replay request, and the recompile runs under them (ADR-011:257, 262, 268;
  ADR-013 O-26, QC-8). `InputRefusal` is an O-26 refusal that `replay`
  carries as a `StageFailure::Refused` cause, never `inconclusive`
  (ADR-011:272). The #231 envelopes live in the layer-6 `replay` public API,
  and CG reaches them only through it (FB-05; ADR-011:586). The `replay`
  allow-list row gains I3 under `quire-extraction`. T-13, the orchestrating
  driver crate, is implemented by #248, and #225 accepts its design
  (ADR-011:588, 1004). The run-mutation evidence is checked in, a check is
  discharged only on proved or SUCCESS, and a non-failing mutant fails the
  gate (ADR-011:426-440). The #185 and #217 end-to-end disposition test runs
  in CG over wire and never calls `route` (ADR-012:458). `ProjectionTarget`
  and `--target` are deleted with SEAM-1, and the backend is chosen only by
  `BackendId`. A duplicate `BackendId` registration is refused and the
  existing one stands (FR-057:214-218; ADR-012:428). `route` runs its
  routing step after E7 and returns a `BackendId` per `supported` item; CG
  settles every disposition (ADR-011:305-317; ADR-012:543). The
  `capability_report` is keyed by occurrence key, and `request_index` is the
  bytewise order of those keys (ADR-011:255).
- **#247 editorial alignments.** The K row QC-15 ids; `RefusalRecord` in F
  `diagnostic`; the E9 `QualifiedName` exception in ADR-011 §1 and R-06; S7
  and row 9 name `ReplaySource`; IR arms over IR's own `ValueType`; #185
  removes `requests`, and #213 S-1 removes the `negotiate_*` copies; row 7
  names CG as the settler; T-2 names #243 and
  agent-ix/quire-contract-codegen#87; ADR-012 §8 and §12.3 cite the `replay`
  facade; SR-506 FND-005 terminology; SR-506 FND-006 closed as satisfied.
- **#244 (FND-003).** ADR-013 O-27, QC-7 and QC-14 now describe AD-016 as
  amended by agent-ix/quire-specification#140 (ADR-013:645, 820, 827).
- **#212 round-3 rulings.** SR-503 FND-008: the T-12 API-surface check
  scans every crate that depends on `quire-exact`; #215 ships it as one
  reusable tool, and RT and CG run it in their lint gates under
  agent-ix/quire-contract-runtime#56 and agent-ix/quire-contract-codegen#89.
  SR-503 FND-014: the #219 and #224 gate walks check that the #187 and #218
  landing PRs change only their table's paths until #226 lands, and #226
  enforces it after that; each `xtask string-edge` allow-list entry names the
  user value it compares and selects no behaviour (ADR-012 §9, §12).
- **Other review fixes.** Provenance and spans are keyed by occurrence key
  in E3, E4 and §10 row 8. O-07 and O-09 use the ruled identity wording.
  O-13 names TK-03 as agent-ix/quire-contract-runtime#56 and
  agent-ix/quire-contract-codegen#89. IR owns `WitnessBinding`, and CG builds
  the bindings. The ADR-013 #222 row runs in parallel with #212 (FND-006).
  ADR-011 §3 and ADR-013 R-09 name the interim inspection of R-09 (SR-503
  FND-013). Cross-repository references use the full `agent-ix/<repo>#N`
  form.

## Scenario walk-throughs

### Scenario 1: exact scalar operator

ADR-011 §10 row 1 (ADR-011:882); ADR-012 Consequences row 1 (ADR-012:956),
§3 `Value` row (ADR-012:280), §4.3 (ADR-012:356-378).

| Field | Record |
|---|---|
| Owners | QSL `Value` family: the `forms` builder arm, the `check` arm and the `value::expression` evaluator arm. The kernel operation lives in the `quire-exact` crate, layer K (ADR-011:576; ADR-013 O-13, ADR-013:291). IR owns the `Operator` row (C-05, ADR-013:685). RT owns the exact op arm. CG owns the render and oracle arm. QSpec owns the operation catalog and vector (AD-016 scenario 1). |
| Stages | S2 form (E2), S3 `Value` check (E3), S4 v2 arm (E4, C-03), E5 IR `Operator` (C-05, total `From`, no `_` arm), S6a kernel operation (E6), E7 CG render arm, then S6b. The seam probe runs at S2 and S3 (ADR-012 §5.1 S2 and S3, ADR-012:390-391). |
| Capability | A nested operator records no kind (FR-057:159-162). Its clause records `value-validity` (FR-057:166). FR-290 and FR-057 are unchanged. The `Requirements` hook adds nothing. |
| Failure | S3 refuses with `ill_typed`/`operator-ineligible` (ADR-012:280; catalog line 84). At S4 a missing v2 operation refuses `invalid_package`/`unknown-operation`. S6a outcomes are the kernel `Refused` or `Undefined` with their catalog codes, for example `undefined_expression`/`unproved-nonzero`, mapped category-preserving by C-08 (ADR-013:688; O-16, ADR-013:337). E7 settles a form that has no arm as `unsupported`, warned, with `unsupported_projection`/`unsupported-requested-capability` (FR-057:264). |
| Version impact | The operation catalog grows in place while v2 is prerelease (ADR-012:764). No contract version bump. `quire-exact` gains one operation. |
| Tests and evidence | Arm-level unit test per stage. The QSpec operator vector through `quire-exact`, RT and QSL. The seam probe at S2 and S3. `cargo mutants` on the C-05 map. **Proof evidence under ADR-011 §2.3 rule 8 (ADR-011:135, #245).** The proof's expectation is the QSpec operator vector or a specification model that calls no `quire-exact` operation. A CG oracle arm that calls `quire-exact` is not independent (ADR-013 O-13). The shared-helper set is computed from the build as the functions reachable from both the expectation and the function under proof, for example the kernel `Value` decode and rational normalization that the agent-ix/quire-contract-runtime#57 harness shared through `decode`. It is checked in, and #219 compares it with the computed set (ADR-011:442). For each helper in the set, a run mutation of that helper fails the proof. A check inside the claimed module is discharged only when the backend reports it proved or SUCCESS; a backend with no per-check status counts one check per proved obligation. UNREACHABLE checks, and every status other than SUCCESS, are not counted as discharged. Each run mutation is checked in as evidence (the diff, the exact command and the failing check output), and a mutant that does not fail the proof fails the gate (ADR-011:426-440). One mutation control inside the operation's module turns the gate red (ADR-011:423-442). |
| Tickets | #213 S-1 (`quire-exact`, X-1); #214 (thin `Value` arms); #217 (function-application exemplar); agent-ix/quire-contract-runtime#53 (§2.3 gate on RT `src/exact/`); agent-ix/quire-contract-runtime#55 (T-9 agreement retarget); agent-ix/quire-contract-codegen#88 (ADR-011 T-10 harness gate, its scope amended with the §2.3 rule-8 items: claimed modules, `unreached`, the proved-or-SUCCESS discharge floor, mutation control, the shared-helper list, a failing run mutation of each shared helper, and the checked-in run-mutation evidence); agent-ix/quire-contract-codegen#89 (TK-03). |

Verdict: PASS.

### Scenario 2: sum type and exhaustive `case`

ADR-011 §10 row 2 (ADR-011:883); ADR-012 §12.1 (ADR-012:741-775).

| Field | Record |
|---|---|
| Owners | QSL `SumCase` family: `forms::sum_case`, `check::sum_case` and `value::expression::sum_case`. The kernel `ValueType` gains a sum shape (ADR-013 O-14, ADR-013:311; C-26, ADR-013:706). agent-ix/quire-specification#115 owns the spelling, FR-143 and FR-146. IR owns the v2 decode arm. CG owns the `negotiate_*` arm. RT owns the variant op. |
| Stages | S2 builder (E2), S3 family checker with exhaustiveness as its own obligation (E3), S4 v2 variant and case nodes (E4), S6a `case` evaluation (E6), E7 only when a backend supports it. The §12.1 "Seam forced" column forces S1, S3, S4 and S6 (ADR-012:748-768). |
| Capability | A clause containing `case` records `value-validity` (FR-057:167). The exhaustiveness obligation records no kind, because language admission discharges it (FR-057:168; ADR-012 §7.2 step 1, ADR-012:535-539). No new FR-290 kind. |
| Failure | S3 refuses non-exhaustive, unreachable-arm and wrong-variant causes. An unproved exhaustiveness obligation refuses `undefined_expression`/`unproved-exhaustiveness` (FR-057:168). The v2 reader refuses an unknown node kind with a named code (QC-19, ADR-013:832). IR and CG arms return `unsupported` with a catalog code until the harness exists (ADR-012:765-766). |
| Version impact | v2 node-kind set revised in place, no bump (owner ruling, ADR-012:764). The kernel `ValueType` gains one shape. |
| Tests and evidence | The ADR-012 §12.1 test list (ADR-012:770-774): arm tests, the non-exhaustive and unreachable-arm refusals, builder order, evaluation per variant, the agent-ix/quire-specification#115 vectors, a v2 round trip and one typed `unsupported` ledger case. The seam probe at S1 to S4. The #187 landing PR changes only the paths in the §12.1 table, checked by the #219 and #224 gate walks until #226 lands and by #226 after that (ADR-012:735-739). |
| Tickets | #221 (design); #187 (implementation); agent-ix/quire-specification#115; #213 S-3 (C-26); agent-ix/quire-contract-ir#141 (v2 intake); agent-ix/quire-contract-codegen#86 (`negotiate_*` arms). |

Verdict: PASS.

### Scenario 3: protocol clause with a frame or scoped anchor

ADR-011 §10 row 4 (ADR-011:885); ADR-012 §12.2 (ADR-012:777-807).

| Field | Record |
|---|---|
| Owners | QSL `ProtocolClause` family. The canonical clause kind lives in the `check` core (ADR-013 O-10, ADR-013:241; seam S5, ADR-012:393). IR owns `ClauseKind` and frame lowering (agent-ix/quire-contract-ir#109). RT owns the observation kind (C-06, ADR-013:686). CG owns the frame harness (agent-ix/quire-contract-codegen#49). QSpec owns FR-340 and the v2 spellings. |
| Stages | S2 `FrameForm` and `ScopedAnchorForm`; S3 builder states `Anchored` and `Framed`; S4 v2 `state`/`frame` and anchor nodes; E5 IR `ClauseKind`; E7 frame obligation, which settles `unsupported` until agent-ix/quire-contract-ir#109 lands; S6a runtime frame check. S5 is forced in QSL, IR, RT and CG, with compile-forced arms in `TemporalTrace` and `Relation` (ADR-012:787). |
| Capability | The frame obligation records `operation-contract`, an existing kind (FR-057:170; ADR-012:794). The scoped anchor records no claim of its own. The vocabulary is unchanged, so S7 is unchanged. |
| Failure | S3 raises FR-340 frame-target, anchor-scope and clause-order causes. S6a refuses `frame_violation`/`unauthorized-change` (catalog line 94). E7 settles `unsupported`, warned, with `unsupported_projection`/`unsupported-requested-capability` while agent-ix/quire-contract-ir#109 and agent-ix/quire-contract-codegen#49 are open (ADR-012:797). C-06 maps IR → RT totally or refuses with a typed cause. Emitting through `protocol_artifact` is FB-03 (ADR-011:475). |
| Version impact | The v2 node-kind set is revised in place. `ClauseKind` gains `Frame` and `ScopedAnchor` in QSL, IR, RT and CG. No contract version bump. |
| Tests and evidence | The ADR-012 §12.2 test list (ADR-012:800-805), including the S5 seam probe and the `operation-contract` backend-absence corpus case. C-06 RT test over every IR kind. The #218 landing PR changes only the paths in the §12.2 table, checked by the #219 and #224 gate walks until #226 lands and by #226 after that. |
| Tickets | #223 (protocol, frame and relation design); #218 (frames); agent-ix/quire-contract-ir#109; agent-ix/quire-contract-codegen#49; agent-ix/quire-contract-runtime#56 (TK-03, C-06). |

Verdict: PASS.

### Scenario 4: model-bound identity change that preserves provenance

ADR-011 §10 row 8 (ADR-011:889, as fixed here); ADR-013 O-01, O-03, O-04,
O-12 (ADR-013:122, 149, 165, 276).

| Field | Record |
|---|---|
| Owners | FCD produces the semantic IR bytes. QSL `model::intake` is the only translation point and owns domain-package identity (O-01). The domain package assigns each `DeclarationKey` (O-03). QSL `check` mints node ids from `ModelOwner` (C-02). QSL `package` mints `package_id` (O-02). `library` owns the I2 binding. QSL is the only span minter (O-12). |
| Stages | I1 intake re-admits the domain package (C-01). E3 re-resolves: nodes bound to changed declarations get new node ids through C-02, and so do the nodes that reference them. Source regions keep their `RawSourceRef` and byte range, keyed by node id (O-12). E4 mints a new `package_id`: FR-322 `identity_preimage` includes `model_selections` and excludes source regions. Consumers pinned to the old identity refuse at the I2 binding (§4). |
| Capability | None added. Each item keeps its FR-057 kind. |
| Failure | Intake refuses `stale_dependency`/`byte-digest-mismatch` or `digest-domain-mismatch` (O-01, ADR-013:131). A second selection of one identity refuses under QC-5 (code pending in TK-08, agent-ix/quire-specification#139). A consumer lock naming the old identity refuses `DependencyIdentityMismatch`, `stale_dependency` (ADR-011:384). A replay of an old packet refuses on `package_id` inequality (O-26, ADR-013:633). |
| Version impact | The domain package's `DomainPackageRef` (identity, version, digest) enters `package_id` through the v2 lock `model_selections` (O-01, C-17). A model-bound node id changes when its preimage content or its `ModelOwner` changes (C-02). For a `ModelOwner` node the declaring package is the domain package, and its `name@version` is the `DomainPackageRef` identity and version (ADR-013 O-04, #212 rulings), so a domain-package version bump changes every model-bound node id of that package. The v2 contract version is unchanged. |
| Tests and evidence | The #131 intake test over a pinned FCD fixture (C-01). The model-owned node-identity vectors (QC-3, #213 S-2). The `name@version` uniqueness vector (QC-18). A test that a changed domain-package selection changes `package_id` and leaves the regions of unchanged source identical (C-14 lookup over the v2 source map). The consumer-lock refusal test at I2. |
| Tickets | #131 (QSL PR #200); #213 S-2; #120; #132 (re-vendor QSpec); agent-ix/quire-specification#139 (TK-08). |

Verdict: PASS. The E3 and row 8 cells contradicted O-03 on `origin/main`;
this PR fixes them.

### Scenario 5: unsupported unbounded proof request

ADR-011 §10 row 6 (ADR-011:887); ADR-012 §1.1 and §7.2 (ADR-012:167-202,
509-563).

| Field | Record |
|---|---|
| Owners | The family records the extent and any authored bound in `Requirements` (ADR-012:174-175). IR `requires-bound` is the single boundedness predicate. CG `negotiate_*` settles (ADR-012:176-180; O-20, ADR-013:476). #222 owns the extent vocabulary and the "available finite bound" predicate. |
| Stages | E3 records the extent. E4 carries v2 `bounded_domain` (ADR-011:349). E5 derives `requires-bound` once (ADR-011:350). E7 settles in `negotiate_*` over the `route` candidate set. S6b does not run. |
| Capability | The item keeps its one kind. The candidate is matched on kind alone, and the mode is compared in `negotiate_*` (ADR-012:181-183). |
| Failure | With a finite bound available: `requires-bound` (FR-057:265). With none: `unsupported`, warned, with `unsupported_projection`/`unbounded-extent` (FR-057:266). With no extent classification: `invalid-request`, `invalid_capability`/`absent-extent` (FR-057:245-246). No stage narrows the domain (C-22, C-25, ADR-013:702, 705). No stage reports `proved` for an unbounded claim. |
| Version impact | None. A supplied bound makes a new bounded request with its own identity (ADR-012:195-197). |
| Tests and evidence | CG `negotiate_*` tests: `requires-bound` with a bound, `unsupported` without one, `supported` only for a bounded extent (ADR-012:959). The bound round trip from IR `KaniOutcome` to the FR-331 record. The C-25 CG tests (aligned to ADR-012 §1.1 in this PR). The IR `requires-bound` row (AD-016 scenario 4). |
| Tickets | #222 (predicate); agent-ix/quire-contract-codegen#86; agent-ix/quire-contract-ir#141; #189. |

Verdict: PASS. Every outcome is typed whichever way #222 decides the
predicate. Remaining work: #222.

### Scenario 6: nested counterexample, replayed natively

ADR-011 §10 row 9 (ADR-011:890), E8 and E9 (ADR-011:256-257, 354, 389),
§1.1 (ADR-011:216); ADR-012 §8 Witness and Replay rows (ADR-012:629-630);
ADR-013 O-25, O-26, O-27 (ADR-013:572, 633, 645).

| Field | Record |
|---|---|
| Owners | IR owns the packet and `Witness` (E8, C-10, C-11). CG owns reconstruction, builds the replay request (C-12) through the layer-6 `replay` public API, where the #231 envelopes live (FB-05; ADR-011:586), and owns the parity comparator, which holds the sealed backend-evidence verdict (amended AD-016). The QSL layer-6 `replay` facade recompiles and executes (TK-01, C-13). QSL resolves the failing node's occurrence key to nested regions (C-14). QSL owns the crossing test (amended AD-016, MD-1 ruling). IR owns the agreement vectors as data, exported as raw bytes through one accessor in `quire-contract-model`. |
| Stages | S6b → E8 builds `CounterexamplePacket{source: ReplaySource}` (`Witness` or `Input`; QC-20, landed by agent-ix/quire-specification#140). E9 calls `replay`, which recompiles S1 to S4 from digest-addressed source (QC-1), checks `package_id` and source digests, selects by `QualifiedName` (ADR-013 T-8), and runs S6a. The replay request carries the S1 to S4 stage limits of the proving run, and the recompile runs under them (ADR-011:262, 268; ADR-013 O-26, QC-8). `CheckedPackage::call` admits the arguments before evaluation. The node id is looked up as a `NodeKey`, and the failing node's occurrence key, which the packet carries (QC-8), resolves to its nested span through the source map of the package that holds the node, including a recompiled dependency package (ADR-011:297-300). |
| Capability | `finite-replay` for a requested replay claim (FR-057:171). The replay of a proof counterexample is part of the proved item's evidence and requests no second kind. |
| Failure | E9 refuses on identity mismatch, including `stale_dependency` for a dependency. It refuses on a `Witness::parse` or `decode` failure. A recompile limit refusal carries its `LimitExceeded` cause and yields no verdict. An argument that `CheckedPackage::call` does not admit is an `InputRefusal`: an O-26 refusal that `replay` carries as a `StageFailure::Refused` cause, never `inconclusive` (ADR-011:272). An internal fault yields no verdict and a distinct outcome kind. A disagreement settles `inconclusive` with a typed cause and is never repaired (ADR-011:389). The outcome→verdict map is fixed by QSpec per O-16 category; `Undefined` and `FamilyOutcome::Refused` never count as agreement (ADR-013 O-26). A tag that names no node refuses at replay (O-12). An `Input`-sourced replay settles `reproduced-without-witness` (QC-8). |
| Version impact | Packet `source: ReplaySource` replaces `witness: Option<Witness>` (QC-20, landed). O-27 and QC-7 describe the parity carrier as the `arm` sum of Witness-arm and Input-arm results, each with its own `settlement` (ADR-013:645, 820; #244, fixed in this PR). |
| Tests and evidence | The QSL replay crossing test (counterexample → native replay → parity), over IR's agreement vectors read as raw bytes through the `quire-contract-model` accessor at the revision QSL's lock already pins. It adds no crate edge and reads no path inside a cargo checkout (ADR-011:764). It discharges IR FR-031-AC-3 in its own row once agreement vectors exist. Remaining work: agent-ix/quire-contract-ir#146 (accessor and vectors), after agent-ix/quire-contract-ir#145. The parity comparator is agent-ix/quire-contract-codegen#50. The CG skeleton spine (agent-ix/quire-contract-codegen#87) is the end-to-end run; its result is cited as evidence where one exists, and #212 does not wait on it. The #231 byte-for-byte envelope round trip; the TK-01 executor tests (C-13); the C-14 source-map lookup by occurrence key (Remaining work: agent-ix/quire-specification#141). |
| Tickets | #243 (ADR-013 TK-01 `replay` facade); #231; #217; agent-ix/quire-contract-codegen#50; agent-ix/quire-contract-codegen#87; agent-ix/quire-contract-ir#144 (TK-04); agent-ix/quire-contract-ir#145; agent-ix/quire-contract-ir#146; agent-ix/quire-specification#137 (TK-06); agent-ix/quire-specification#141; #244. |

Verdict: PASS, on the MD-1 ruling.

### Scenario 7: backend added without syntax or checking authority

ADR-011 §10 row 7 (ADR-011:888), FB-05 (ADR-011:477); ADR-012 §5.2
(ADR-012:420-432), §7 (ADR-012:485-617), §12.3 (ADR-012:809-829); ADR-013
T-7 (ADR-013:670), C-27 to C-29 (ADR-013:707-709).

| Field | Record |
|---|---|
| Owners | The backend repository owns its FR-331 provider manifest, runner and probe. QSL `route` (#185, layer R) converts the manifest into a `BackendDescriptor` (C-28) and writes candidate sets (C-29). CG owns one backend kind variant, its `negotiate_*` arm, which settles every disposition, and its generation arm (seam S9). The orchestrating driver crate (ADR-011 T-13, implemented by #248; #225 accepts its design) builds the registry value and passes each routed `BackendId` to CG generation (ADR-011:588, 1004). |
| Stages | No QSL S0 to S4 change. Registration enters at the `route` candidate step after S4. E7 consumes S5 IR and the candidate sets, and CG `negotiate_*` settles every item. After E7 the `route` routing step reads the FR-331 dispositions as wire and returns a `BackendId` per `supported` item (ADR-011:305-317; ADR-012:543). Replay, if any, goes through E9 and the `replay` facade. A backend depends on no QSL API other than `replay` (FB-05). |
| Capability | The backend advertises (kind, mode) pairs from the FR-290 vocabulary. It adds no kind. |
| Failure | Registration refuses with `invalid_capability` and one of `duplicate-backend`, `unknown-kind`, `absent-kind` or `unknown-mode` (FR-057:214-218); a duplicate `BackendId` registration is refused and the existing registration stands (ADR-012:428). A request naming an unregistered backend settles `invalid-request`, `invalid_capability`/`unknown-backend` (FR-057:239). A second registrant for the same kind makes an unnamed request settle `invalid-request`, `invalid_capability`/`ambiguous-backend` (FR-057:242; ADR-012:823-829). Tool absence at probe settles FR-331 `unsupported` with `unsupported_projection`/`tool-unavailable` (FR-057:268). A registered `BackendId` with no CG kind: the orchestrating driver's conversion before negotiation refuses the run with `invalid_capability`/`unknown-backend`, naming each unconverted `BackendId` in bytewise order, as a command-level refusal that #225 renders (ADR-012:564, MD-2 ruling). FR-290's per-item row stays as CG's guard for requests formed outside the driver. |
| Version impact | None in QSL. The manifest is QSpec FR-331 data. |
| Tests and evidence | The §12.3 touch set. A metamorphic test that the checked package has the same bytes with and without the new descriptor registered. The layer check that `check` and the family modules do not depend on `route` (ADR-012:961). The FB-05 direction and API-surface checks (T-12): #215 ships the API-surface check as one reusable tool that scans every crate that depends on `quire-exact`, and RT and CG run it in their lint gates (agent-ix/quire-contract-runtime#56, agent-ix/quire-contract-codegen#89). The #185 and #217 end-to-end disposition test in CG, over the candidate-set wire and the v2 bytes as data at the pinned QSL revision, never calling `route`; QSL tests only `route`'s candidate-set output (ADR-012:458). |
| Tickets | #185; agent-ix/quire-contract-codegen#86; #225; #248 (T-13); #226; QSpec FR-331 (agent-ix/quire-specification#134, closed). |

Verdict: PASS, on the MD-2 ruling.

## Pass conditions

| Condition | Verdict | Evidence |
|---|---|---|
| Stage, module and crate DAG is acyclic | PASS | ADR-011 §6.1 (ADR-011:574-588) is an exhaustive allow-list in which each layer depends only on layers listed before it. `route` depends on 4, 3, F and K. `replay` depends on 1 to 5, and on I3 under feature `quire-extraction`. The driver (T-13, #248) is a separate crate downstream of CG. §7.1 (ADR-011:722-776): QSL depends on `quire-exact`, `quire-contract-model` and FCD. CG → QSL is through the `replay` facade only. IR root → QSL is removed (M-6d, agent-ix/quire-contract-ir#141). QSL → CG dev and QSL tests → RT are removed (M-6b). No cycle remains over normal, dev or test edges (FB-11). |
| Checked and unchecked inputs cannot be confused | PASS | ADR-013 R-10 (ADR-013:94) and T-1 (ADR-013:664): one nominal type per stage output, with private constructors. ADR-011 §4 (ADR-011:486-539) and FB-03 and FB-04. |
| The capability condition is met | PASS | #229 produced FR-057 (merged in QSL #237). QSpec FR-290 at 1e8bb50 carries the same ten labels, the claim-form table and the candidate-set table. ADR-012 §7 and §13.3 cite both. #185 has no capability decision left open. |
| Witness and replay are lossless | PASS | ADR-013 O-25 `ReplaySource`, and C-11 lossless widening (ADR-013:691). The amended AD-016 (agent-ix/quire-specification#140) carries `ReplaySource`, both reproduced arms and the CG parity comparator. The packet carries the failing node's occurrence key (ADR-013 O-25, QC-8). Remaining work: agent-ix/quire-specification#141. The O-27 parity carrier matches the amended AD-016 (#244, fixed in this PR). |
| Cross-repository changes have a named QSpec owner | PASS | ADR-013 QC-1 to QC-20 (ADR-013:814-833), filed as TK-06 to TK-10 (agent-ix/quire-specification#136 to agent-ix/quire-specification#139; QC-20 landed by agent-ix/quire-specification#140). |
| Slices are bounded tickets | PASS | Each scenario cites existing tickets. ADR-011 T-1 to T-13 and ADR-013 TK-01 to TK-10 are filed (#240 to #243; #248; agent-ix/quire-contract-codegen#86 to agent-ix/quire-contract-codegen#90; agent-ix/quire-contract-runtime#55 and agent-ix/quire-contract-runtime#56; agent-ix/quire-contract-ir#144; agent-ix/quire-specification#136 to agent-ix/quire-specification#139). The slices for agent-ix/quire-contract-ir#141 and agent-ix/quire-contract-codegen#86 are named below (SR-504 FND-008). |
| OBS-212-2: owner of the crossing replay test | DECIDED | Amended AD-016, arrows 6 and 7 (agent-ix/quire-specification#140): QSL owns the crossing test (counterexample → native replay → parity). IR owns the agreement vectors, as data only. CG owns a TC for emitting the harness and the packet. IR FR-031-AC-3 has its own row, discharged by the QSL test. Per-criterion rows also make a criterion's obligation observable under the current engine. This resolves agent-ix/quire-contract-ir#143 item 1. MD-1 is ruled and applied: ADR-011 §7.1 and OBS-040 (ADR-011:764, 866) and ADR-013 TK-04 and the agent-ix/quire-contract-ir#137 row (ADR-013:894, 965). Remaining work: agent-ix/quire-contract-ir#146. |
| OBS-212-1: IR coverage | RED, limits cross-repository evidence only | On IR `origin/main` 65aa282, `quire coverage --scope . --strict` backs 196 of 226 minted rows and exits 1. It lists 16 unbacked rows and 0 contradicted. The 16 are the FR-036 and FR-037 functional-coverage rows of `spec/contract-test-matrix.md`, TC-045, TC-046 and TC-047, FR-035-AC-5, FR-036-AC-1 to AC-5 and FR-037-AC-1 to AC-5. The 30 unbacked of the 196/226 ratio are the 14 TC and AC ids among those 16, plus eight NFR acceptance criteria (NFR-001-AC-2, NFR-002-AC-1 and AC-2, NFR-004-AC-6 and AC-7, NFR-005-AC-1, AC-4 and AC-5) and the eight suite rows SUITE-001 to SUITE-008 of `spec/evidence/suites.md`, which the tool leaves out of its unbacked-row list. The tool reports 16 unbacked before and after agent-ix/quire-contract-ir#147. FR-031-AC-3 is unbacked per criterion, and the TC-221 row proxy hides it: that is a second false-backing path in agent-ix/quire-rs#467. The honest count is pending: Agent E confirms it when the FR-031-AC-3 own-row split lands (agent-ix/quire-contract-ir#143 item 1, after agent-ix/quire-contract-ir#147). This red limits IR evidence cited by scenarios 1, 3 and 6. It does not affect any QSL decision. |
| Combined spec review | ACCEPT WITH FINDINGS | SR-499 base and SR-500 failure-domain accept after Round 3. SR-501 integrity, SR-502 dependency, SR-503 evidence, SR-504 risk-complexity, SR-505 scope-boundary and SR-506 EARS accept with findings after Round 3. After Round 4 every finding is resolved by a #212 ruling (round 1 or round 2) or an earlier round, or is remaining work with a ticket. The #247 ADR text alignments and #244 are fixed in this PR. Implementation tickets: #185, #213, #214, #215, #216, #219, #225, #226, #231, #240, #243 and #248; agent-ix/quire-contract-codegen#86, agent-ix/quire-contract-codegen#87 and agent-ix/quire-contract-codegen#88; agent-ix/quire-contract-ir#141 and agent-ix/quire-contract-ir#144. Cross-repository remaining work: agent-ix/quire-specification#141 (QC-8 occurrence key in the packet and obligation identity, the FR-323 outcome→verdict map per O-16 category, the O-16 vacuity row, the replay-request stage limits, and the AD-016 text at arrows 1, 4 and 5) and agent-ix/quire-contract-ir#146 (the agreement-vector accessor, the `kani_vacuous_proof` cause on `KaniOutcomeKind::Inconclusive`, and FR-031-AC-3 in its own row). |

## Implementation slices

SR-504 FND-008 asks for bounded slices of the two largest cross-repository
tickets. Each slice below is sized for one to three agent sessions and is
filed as a sub-issue of its parent: IR-141-1 to IR-141-6 are
agent-ix/quire-contract-ir#148 to agent-ix/quire-contract-ir#153, and CG-86-1 to
CG-86-5 are agent-ix/quire-contract-codegen#91 to
agent-ix/quire-contract-codegen#95.

### agent-ix/quire-contract-ir#141 (v2 reader decodes closed enums once)

| Slice | Issue | Scope | Waits on |
|---|---|---|---|
| IR-141-1 | agent-ix/quire-contract-ir#148 | Name every closed vocabulary that crosses the v2 seam, and decode each into an enum in the reader, with an exhaustive `match` and no catch-all arm (asked items 1 and 4). | — |
| IR-141-2 | agent-ix/quire-contract-ir#149 | Re-measure, then move the `src/kani/provenance.rs` and `src/kani/objects.rs` `as_str()` sites (15 at filing) onto the decoded enums. | IR-141-1 |
| IR-141-3 | agent-ix/quire-contract-ir#150 | Move the `src/kani/abi.rs` and `src/kani/profile.rs` `as_str()` sites (12 at filing) onto the decoded enums. | IR-141-1 |
| IR-141-4 | agent-ix/quire-contract-ir#151 | The string-edge scan in the lint gate, plus a test that fails when a wire string is matched after intake (asked item 3; ADR-012 §9). | IR-141-2, IR-141-3 |
| IR-141-5 | agent-ix/quire-contract-ir#152 | v2 intake of value and expression nodes (ADR-011 T-5). | IR-141-1 |
| IR-141-6 | agent-ix/quire-contract-ir#153 | v2 intake of temporal nodes, and removal of the IR root → QSL edge with a `cargo tree` check over normal and dev edges (ADR-011 M-6d, FB-05). | IR-141-5 |

### agent-ix/quire-contract-codegen#86 (`negotiate_*` is the only settlement point)

| Slice | Issue | Scope | Waits on |
|---|---|---|---|
| CG-86-1 | agent-ix/quire-contract-codegen#91 | The closed backend-kind enum, `negotiate_*` with one arm per variant and no catch-all, and the test that fails when a capability settles outside a `negotiate_*` arm (asked items 1 and 5; seam S9). | agent-ix/quire-specification#134 (merged) |
| CG-86-2 | agent-ix/quire-contract-codegen#92 | Settlement over the #185 candidate set and the claim extent, one of the four dispositions per item; the `requires-bound` extent-against-modes test and the multi-match `invalid-request` test (asked items 2 to 4; ADR-012 §1.1). | CG-86-1 |
| CG-86-3 | agent-ix/quire-contract-codegen#93 | The solver-absence fault-injection test: tool missing from `PATH` and a mismatched pin, replacing the IT-010 `expect` panic (asked item 6; ADR-012 §7.4). | CG-86-1 |
| CG-86-4 | agent-ix/quire-contract-codegen#94 | CG S6 matches on IR's decoded tag and form enums in place of string compares, and the string-edge scan in the lint gate (asked items 7 and 8). | IR-141-1 |
| CG-86-5 | agent-ix/quire-contract-codegen#95 | The #185 and #217 end-to-end disposition test over the candidate-set wire and the v2 bytes as data, at the pinned QSL revision, never calling `route` (ADR-012 §5.3). | CG-86-2, #185 |

## Missing decisions

The first run of this gate found two missing decisions. Both are decided by
the #212 rulings (2026-09-19, issue #212), and this PR applies them. The
review of this PR at 348256d found further missing decisions. The #212
round-2 comment rules on each of them, and this PR applies them (Changes in
this PR, "#212 round-2 rulings").

### MD-1: where the crossing replay test runs and how it reads IR vectors

- **Owner:** #209 (ADR-011). Secondary: #211 (ADR-013 TK-04).
- **Ruling:** The QSL crossing test reads IR's agreement vectors as data,
  through the already-pinned `quire-contract-ir` dependency (package
  `quire-contract-model`). IR exports them from that crate through one data
  accessor that returns raw bytes only and no IR type. The test never reads a
  path inside a cargo checkout. This adds no crate edge. The parity comparator
  stays in agent-ix/quire-contract-codegen#50. ADR-011 §7.1 and OBS-040 name QSL
  as the test's home, and the end-to-end run is
  agent-ix/quire-contract-codegen#87. ADR-013 TK-04 and the
  agent-ix/quire-contract-ir#137 row give FR-031-AC-3 its own row, discharged by
  the QSL test, after agent-ix/quire-contract-ir#145 removes the stub tags.
- **Applied:** ADR-011:764, 862, 866; ADR-013:894, 965. Remaining work:
  agent-ix/quire-contract-ir#146.

### MD-2: a registered backend with no CG kind

- **Owner:** #210 (ADR-012).
- **Ruling:** The orchestrating driver's conversion before negotiation refuses
  the run with `invalid_capability`/`unknown-backend`. It names each
  unconverted `BackendId` in bytewise order, as a command-level refusal that
  #225 renders. FR-290's per-item row stays as CG's guard for requests formed
  outside the driver.
- **Applied:** ADR-012 §7.2 (ADR-012:559-569).

## Findings

| ID | Severity | Summary | Refs |
|---|---|---|---|
| FND-001 | high | Fixed in this PR. ADR-011 E3 and §10 row 8 said S3 mints `DeclarationKey`s, which contradicted ADR-013 O-03. | ADR-011:348, 889 |
| FND-002 | medium | FR-057 names its actor "the QSL composed linker", which SEAM-2 and M-6e delete. Remaining work: #185; the FR-036 and FR-057 re-homing goes through `/specify` before #185 (ADR-012 §14.1). | FR-057:27, 83 |
| FND-003 | medium | Fixed in this PR (#244). ADR-013 O-27, QC-7 and QC-14 described the pre-amendment AD-016; they now describe AD-016 as amended by agent-ix/quire-specification#140. | ADR-013:645, 820, 827 |
| FND-004 | low | QSL's vendored diagnostic catalog lacks `invalid_capability`, `unproved-exhaustiveness`, `unbounded-extent`, `absent-extent`, `unknown-backend` and `tool-unavailable`, which FR-057 cites. QSpec `origin/main` has all six. Remaining work: #132. | `resources/complete-value/…/native-diagnostics.md` |
| FND-005 | low | AD-016 scenario 4 and the #189 title name `NativeModelProfile`, which ADR-011 retires with SEAM-1. AD-016 arrow 1 still says S3 "mints `DeclarationKey`", and arrows 4 and 5 carry stale text. Both are QSpec-owned. Remaining work: agent-ix/quire-specification#141. | AD-016; ADR-011:884 |
| FND-006 | low | Fixed in this PR. ADR-013 §7 said #222 waits on #212, while ADR-012 §1.1 lets #222 run in parallel. The ADR-013 #222 row now runs in parallel with #212. | ADR-012:201-202; ADR-013:781 |
| FND-007 | low | Fixed in this PR on the round-2 ruling. ADR-012 §5.2 said registry construction refuses a duplicate `BackendId`. It now follows FR-057:214-218: the duplicate registration is refused and the existing one stands. #185 implements FR-057. | ADR-012:428 |
| FND-008 | medium | ADR-013 O-13 listed CG oracles as `quire-exact` consumers without restating rule 8 for an oracle over a kernel operation. Fixed in this PR on the #212 rulings (SR-500 FND-001): O-13 limits CG oracles to kernel types, and the expectation for a kernel operation comes from the QSpec operation vectors or a checked-in specification model that calls no `quire-exact` operation. | ADR-013:295 |
