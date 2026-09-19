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

This record walks the seven #212 change scenarios through ADR-011, ADR-012
and ADR-013 as merged on QSL `origin/main` 457a131, plus the two ADR-011
changes this PR makes (below). Cross-repository inputs are QSpec `origin/main`
1e8bb50 (FR-290, amended AD-016 from QSpec #140) and Contract IR `origin/main`
65aa282.

Line anchors are to this PR's head. `ADR-011:134` means line 134 of
`spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md`. ADR-012,
ADR-013 and FR-057 anchors are unchanged from `origin/main`.

| # | Scenario | Verdict |
|---|---|---|
| 1 | Exact scalar operator | PASS |
| 2 | Sum type and exhaustive `case` | PASS |
| 3 | Protocol clause with a frame or scoped anchor | PASS |
| 4 | Model-bound identity change that preserves provenance | PASS (after the editorial fix below) |
| 5 | Unsupported unbounded proof request | PASS |
| 6 | Nested counterexample, replayed natively | FAIL: evidence cell stops on MD-1 |
| 7 | Backend added without syntax or checking authority | FAIL: failure cell stops on MD-2 |

Gate verdict: **not passed.** Two missing decisions, MD-1 (owner #209) and
MD-2 (owner #210), stop one cell each. Every other cell of scenarios 6 and 7
is recorded below.

## Changes in this PR

- **#245, ADR-011 §2.3 rule 8.** Decision 8 (ADR-011:134) now requires a proof
  expectation derived independently of the function under proof. A run
  mutation of each helper shared between oracle and code under proof must fail
  the proof. The discharge floor counts SUCCESS checks only, and UNREACHABLE
  checks are subtracted. The rule is restated in the §1.1 skeleton bullet
  (ADR-011:221), §2.3 proof-stage acceptance (ADR-011:368-404), FB-10
  (ADR-011:428), the §9 RT #53 row and the Consequences bullet.
- **Editorial, ADR-013 O-03 alignment.** ADR-011 E3 (ADR-011:320) and §10
  row 8 (ADR-011:823) said that S3 mints `DeclarationKey`s. ADR-013 O-03
  (ADR-013:152) says the domain package assigns the key and QSL never mints
  one. Both cells now say that I1 intake admits the domain package's keys and
  S3 mints node ids from them (C-02, ADR-013:678).
- **Spec-review fixes (editorial).** ADR-011 FB-10 (ADR-011:428) now forbids
  only a shared helper with no failing run mutation, matching rule 8 (SR-500,
  SR-501, SR-506). The counting sentence now says no status but SUCCESS is
  counted. §2.3 defines a shared helper and requires a published
  shared-helper list (SR-503 FND-002). T-10 and the Consequences bullet carry
  the #245 rule. ADR-013 O-20, O-21 and C-25 (ADR-013:481, 513, 701) now
  settle an unbounded claim as ADR-012 §1.1 does (SR-500 FND-004, SR-501
  FND-007).

## Scenario walk-throughs

### Scenario 1: exact scalar operator

ADR-011 §10 row 1 (ADR-011:816); ADR-012 Consequences row 1 (ADR-012:932),
§3 `Value` row (ADR-012:277), §4.3 (ADR-012:353-375).

| Field | Record |
|---|---|
| Owners | QSL `Value` family: the `forms` builder arm, the `check` arm and the `value::expression` evaluator arm. The kernel operation lives in the `quire-exact` crate, layer K (ADR-011:518; ADR-013 O-13, ADR-013:290). IR owns the `Operator` row (C-05, ADR-013:681). RT owns the exact op arm. CG owns the render and oracle arm. QSpec owns the operation catalog and vector (AD-016 scenario 1). |
| Stages | S2 form (E2), S3 `Value` check (E3), S4 v2 arm (E4, C-03), E5 IR `Operator` (C-05, total `From`, no `_` arm), S6a kernel operation (E6), E7 CG render arm, then S6b. The seam probe runs at S2 and S3 (ADR-012 §5.1 S2 and S3, ADR-012:387-388). |
| Capability | A nested operator records no kind (FR-057:159-162). Its clause records `value-validity` (FR-057:166). FR-290 and FR-057 are unchanged. The `Requirements` hook adds nothing. |
| Failure | S3 refuses with `ill_typed`/`operator-ineligible` (ADR-012:277; catalog line 84). At S4 a missing v2 operation refuses `invalid_package`/`unknown-operation`. S6a outcomes are the kernel `Refused` or `Undefined` with their catalog codes, for example `undefined_expression`/`unproved-nonzero`, mapped category-preserving by C-08 (ADR-013:684; O-16, ADR-013:336). E7 settles a form that has no arm as `unsupported`, warned, with `unsupported_projection`/`unsupported-requested-capability` (FR-057:264). |
| Version impact | The operation catalog grows in place while v2 is prerelease (ADR-012:746). No contract version bump. `quire-exact` gains one operation. |
| Tests and evidence | Arm-level unit test per stage. The QSpec operator vector through `quire-exact`, RT and QSL. The seam probe at S2 and S3. `cargo mutants` on the C-05 map. **Proof evidence under ADR-011 §2.3 rule 8 (ADR-011:134, #245).** The proof's expectation SHALL be derived independently of the `quire-exact` operation under proof. ADR-013 O-13 lets a CG oracle use kernel types. Under rule 8 the oracle cannot compute its expectation with the operation under proof. The gate publishes its shared-helper list, for example the kernel `Value` decode and rational normalization that the RT #57 harness shared through `decode`. For each listed helper, a run mutation of that helper fails the proof. The discharge floor counts SUCCESS checks inside the claimed module only; UNREACHABLE checks are subtracted. One mutation control inside the operation's module turns the gate red (ADR-011:381-396). |
| Tickets | #213 S-1 (`quire-exact`, X-1); #214 (thin `Value` arms); #217 (function-application exemplar); RT #53 (§2.3 gate on RT `src/exact/`); RT #55 (T-9 agreement retarget); CG #88 (T-10 harness gate: claimed modules, `unreached`, mutation control); CG #89 (TK-03). |

Verdict: PASS.

### Scenario 2: sum type and exhaustive `case`

ADR-011 §10 row 2 (ADR-011:817); ADR-012 §12.1 (ADR-012:723-757).

| Field | Record |
|---|---|
| Owners | QSL `SumCase` family: `forms::sum_case`, `check::sum_case` and `value::expression::sum_case`. The kernel `ValueType` gains a sum shape (ADR-013 O-14, ADR-013:310; C-26, ADR-013:702). QSpec #115 owns the spelling, FR-143 and FR-146. IR owns the v2 decode arm. CG owns the `negotiate_*` arm. RT owns the variant op. |
| Stages | S2 builder (E2), S3 family checker with exhaustiveness as its own obligation (E3), S4 v2 variant and case nodes (E4), S6a `case` evaluation (E6), E7 only when a backend supports it. The §12.1 "Seam forced" column forces S1, S3, S4 and S6 (ADR-012:729-750). |
| Capability | A clause containing `case` records `value-validity` (FR-057:167). The exhaustiveness obligation records no kind, because language admission discharges it (FR-057:168; ADR-012 §7.2 step 1, ADR-012:524-528). No new FR-290 kind. |
| Failure | S3 refuses non-exhaustive, unreachable-arm and wrong-variant causes. An unproved exhaustiveness obligation refuses `undefined_expression`/`unproved-exhaustiveness` (FR-057:168). The v2 reader refuses an unknown node kind with a named code (QC-19, ADR-013:829). IR and CG arms return `unsupported` with a catalog code until the harness exists (ADR-012:747-748). |
| Version impact | v2 node-kind set revised in place, no bump (owner ruling, ADR-012:746). The kernel `ValueType` gains one shape. |
| Tests and evidence | The ADR-012 §12.1 test list (ADR-012:752-756): arm tests, the non-exhaustive and unreachable-arm refusals, builder order, evaluation per variant, the QSpec #115 vectors, a v2 round trip and one typed `unsupported` ledger case. The seam probe at S1 to S4. The #187 landing PR changes only the paths in the §12.1 table (ADR-012:718-721). |
| Tickets | #221 (design); #187 (implementation); QSpec #115; #213 S-3 (C-26); IR #141 (v2 intake); CG #86 (`negotiate_*` arms). |

Verdict: PASS.

### Scenario 3: protocol clause with a frame or scoped anchor

ADR-011 §10 row 4 (ADR-011:819); ADR-012 §12.2 (ADR-012:759-788).

| Field | Record |
|---|---|
| Owners | QSL `ProtocolClause` family. The canonical clause kind lives in the `check` core (ADR-013 O-10, ADR-013:240; seam S5, ADR-012:390). IR owns `ClauseKind` and frame lowering (IR #109). RT owns the observation kind (C-06, ADR-013:682). CG owns the frame harness (CG #49). QSpec owns FR-340 and the v2 spellings. |
| Stages | S2 `FrameForm` and `ScopedAnchorForm`; S3 builder states `Anchored` and `Framed`; S4 v2 `state`/`frame` and anchor nodes; E5 IR `ClauseKind`; E7 frame obligation, which settles `unsupported` until IR #109 lands; S6a runtime frame check. S5 is forced in QSL, IR, RT and CG, with compile-forced arms in `TemporalTrace` and `Relation` (ADR-012:768). |
| Capability | The frame obligation records `operation-contract`, an existing kind (FR-057:170; ADR-012:775). The scoped anchor records no claim of its own. The vocabulary is unchanged, so S7 is unchanged. |
| Failure | S3 raises FR-340 frame-target, anchor-scope and clause-order causes. S6a refuses `frame_violation`/`unauthorized-change` (catalog line 94). E7 settles `unsupported`, warned, with `unsupported_projection`/`unsupported-requested-capability` while IR #109 and CG #49 are open (ADR-012:778). C-06 maps IR → RT totally or refuses with a typed cause. Emitting through `protocol_artifact` is FB-03 (ADR-011:421). |
| Version impact | The v2 node-kind set is revised in place. `ClauseKind` gains `Frame` and `ScopedAnchor` in QSL, IR, RT and CG. No contract version bump. |
| Tests and evidence | The ADR-012 §12.2 test list (ADR-012:781-786), including the S5 seam probe and the `operation-contract` backend-absence corpus case. C-06 RT test over every IR kind. The #218 landing PR changes only the paths in the §12.2 table. |
| Tickets | #223 (protocol, frame and relation design); #218 (frames); IR #109; CG #49; RT #56 (TK-03, C-06). |

Verdict: PASS.

### Scenario 4: model-bound identity change that preserves provenance

ADR-011 §10 row 8 (ADR-011:823, as fixed here); ADR-013 O-01, O-03, O-04,
O-12 (ADR-013:121, 148, 164, 275).

| Field | Record |
|---|---|
| Owners | FCD produces the semantic IR bytes. QSL `model::intake` is the only translation point and owns domain-package identity (O-01). The domain package assigns each `DeclarationKey` (O-03). QSL `check` mints node ids from `ModelOwner` (C-02). QSL `package` mints `package_id` (O-02). `library` owns the I2 binding. QSL is the only span minter (O-12). |
| Stages | I1 intake re-admits the domain package (C-01). E3 re-resolves: nodes bound to changed declarations get new node ids through C-02, and so do the nodes that reference them. Source regions keep their `RawSourceRef` and byte range, keyed by node id (O-12). E4 mints a new `package_id`: FR-322 `identity_preimage` includes `model_selections` and excludes source regions. Consumers pinned to the old identity refuse at the I2 binding (§4). |
| Capability | None added. Each item keeps its FR-057 kind. |
| Failure | Intake refuses `stale_dependency`/`byte-digest-mismatch` or `digest-domain-mismatch` (O-01, ADR-013:130). A second selection of one identity refuses under QC-5 (code pending in TK-08, QSpec #139). A consumer lock naming the old identity refuses `DependencyIdentityMismatch`, `stale_dependency` (ADR-011:349). A replay of an old packet refuses on `package_id` inequality (O-26, ADR-013:629). |
| Version impact | The domain package's `DomainPackageRef` (identity, version, digest) enters `package_id` through the v2 lock `model_selections` (O-01, C-17). A model-bound node id changes when its preimage content or its `ModelOwner` changes (C-02). Which package's `name@version` QC-18 adds for a `ModelOwner` node is SR-500 FND-003. The v2 contract version is unchanged. |
| Tests and evidence | The #131 intake test over a pinned FCD fixture (C-01). The model-owned node-identity vectors (QC-3, #213 S-2). The `name@version` uniqueness vector (QC-18). A test that a changed domain-package selection changes `package_id` and leaves the regions of unchanged source identical (C-14 lookup over the v2 source map). The consumer-lock refusal test at I2. |
| Tickets | #131 (QSL PR #200); #213 S-2; #120; #132 (re-vendor QSpec); QSpec #139 (TK-08). |

Verdict: PASS. The E3 and row 8 cells contradicted O-03 on `origin/main`;
this PR fixes them.

### Scenario 5: unsupported unbounded proof request

ADR-011 §10 row 6 (ADR-011:821); ADR-012 §1.1 and §7.2 (ADR-012:167-202,
509-559).

| Field | Record |
|---|---|
| Owners | The family records the extent and any authored bound in `Requirements` (ADR-012:174-175). IR `requires-bound` is the single boundedness predicate. CG `negotiate_*` settles (ADR-012:176-180; O-20, ADR-013:472). #222 owns the extent vocabulary and the "available finite bound" predicate. |
| Stages | E3 records the extent. E4 carries v2 `bounded_domain` (ADR-011:321). E5 derives `requires-bound` once (ADR-011:322). E7 settles in `negotiate_*` over the `route` candidate set. S6b does not run. |
| Capability | The item keeps its one kind. The candidate is matched on kind alone, and the mode is compared in `negotiate_*` (ADR-012:181-183). |
| Failure | With a finite bound available: `requires-bound` (FR-057:265). With none: `unsupported`, warned, with `unsupported_projection`/`unbounded-extent` (FR-057:266). With no extent classification: `invalid-request`, `invalid_capability`/`absent-extent` (FR-057:245-246). No stage narrows the domain (C-22, C-25, ADR-013:698, 701). No stage reports `proved` for an unbounded claim. |
| Version impact | None. A supplied bound makes a new bounded request with its own identity (ADR-012:195-197). |
| Tests and evidence | CG `negotiate_*` tests: `requires-bound` with a bound, `unsupported` without one, `supported` only for a bounded extent (ADR-012:935). The bound round trip from IR `KaniOutcome` to the FR-331 record. The C-25 CG tests (aligned to ADR-012 §1.1 in this PR). The IR `requires-bound` row (AD-016 scenario 4). |
| Tickets | #222 (predicate); CG #86; IR #141; #189. |

Verdict: PASS. Every outcome is typed whichever way #222 decides the
predicate. Remaining work: #222.

### Scenario 6: nested counterexample, replayed natively

ADR-011 §10 row 9 (ADR-011:824), E8 and E9 (ADR-011:249-250, 326, 354),
§1.1 (ADR-011:211); ADR-012 §8 Witness and Replay rows (ADR-012:612-613);
ADR-013 O-25, O-26, O-27 (ADR-013:568, 629, 641).

| Field | Record |
|---|---|
| Owners | IR owns the packet and `Witness` (E8, C-10, C-11). CG owns reconstruction, the replay request (C-12) and the parity comparator, which holds the sealed backend-evidence verdict (amended AD-016). The QSL layer-6 `replay` facade recompiles and executes (TK-01, C-13). QSL resolves a node id to nested regions (C-14). |
| Stages | S6b → E8 builds `CounterexamplePacket{source: ReplaySource}` (`Witness` or `Input`; QC-20, landed by QSpec #140). E9 calls `replay`, which recompiles S1 to S4 from digest-addressed source (QC-1), checks `package_id` and source digests, selects by `QualifiedName` (T-8), and runs S6a. The node id is looked up as a `NodeKey` and resolved to its nested span through the v2 source map. |
| Capability | `finite-replay` for a requested replay claim (FR-057:171). The replay of a proof counterexample is part of the proved item's evidence and requests no second kind. |
| Failure | E9 refuses on identity mismatch, including `stale_dependency` for a dependency. It refuses on a `Witness::parse` or `decode` failure. A disagreement settles `inconclusive` with a typed cause and is never repaired (ADR-011:354). A tag that names no node refuses at replay (O-12). An `Input`-sourced replay settles `reproduced-without-witness` (QC-8). |
| Version impact | Packet `source: ReplaySource` replaces `witness: Option<Witness>` (QC-20, landed). O-27 and QC-7 still describe the pre-amendment parity carrier. Remaining work: #244. |
| Tests and evidence | **Stopped on MD-1.** Recorded without MD-1: the #231 byte-for-byte envelope round trip; the TK-01 executor tests (C-13); the C-14 source-map lookup; the CG skeleton spine as the first run (ADR-011 §1.1, CG #87). The cell cannot name where the crossing test runs, or how it reads IR's vectors. |
| Tickets | #243 (TK-01 `replay` facade); #231; #217; CG #50; CG #87; IR #144 (TK-04); QSpec #137 (TK-06); #244. |

Verdict: FAIL on MD-1, evidence cell only.

### Scenario 7: backend added without syntax or checking authority

ADR-011 §10 row 7 (ADR-011:822), FB-05 (ADR-011:423); ADR-012 §5.2
(ADR-012:416-428), §7 (ADR-012:478-600), §12.3 (ADR-012:790-810); ADR-013
T-7 (ADR-013:666), C-27 to C-29 (ADR-013:703-705).

| Field | Record |
|---|---|
| Owners | The backend repository owns its FR-331 provider manifest, runner and probe. QSL `route` (#185, layer R) converts the manifest into a `BackendDescriptor` (C-28) and writes candidate sets (C-29). CG owns one backend kind variant, its `negotiate_*` arm and its generation arm (seam S9). The driver builds the registry value (§7.1; placement is #225's). |
| Stages | No QSL S0 to S4 change. Registration enters at `route` after S4. E7 consumes S5 IR and the candidate sets. Replay, if any, goes through E9 and the `replay` facade. A backend depends on no QSL API other than `replay` (FB-05). |
| Capability | The backend advertises (kind, mode) pairs from the FR-290 vocabulary. It adds no kind. |
| Failure | Registration refuses with `invalid_capability` and one of `duplicate-backend`, `unknown-kind`, `absent-kind` or `unknown-mode` (FR-057:214-218). A request naming an unregistered backend settles `invalid-request`, `invalid_capability`/`unknown-backend` (FR-057:239). A second registrant for the same kind makes an unnamed request settle `invalid-request`, `invalid_capability`/`ambiguous-backend` (FR-057:242; ADR-012:803-810). Tool absence at probe settles FR-331 `unsupported` with `unsupported_projection`/`tool-unavailable` (FR-057:268). **Stopped on MD-2:** a registered `BackendId` with no CG kind. |
| Version impact | None in QSL. The manifest is QSpec FR-331 data. |
| Tests and evidence | The §12.3 touch set. A metamorphic test that the checked package has the same bytes with and without the new descriptor registered. The layer check that `check` and the family modules do not depend on `route` (ADR-012:937). The FB-05 direction and API-surface checks (T-12). |
| Tickets | #185; CG #86; #225; #226; QSpec FR-331 (QSpec #134, closed). |

Verdict: FAIL on MD-2, failure cell only.

## Pass conditions

| Condition | Verdict | Evidence |
|---|---|---|
| Stage, module and crate DAG is acyclic | PASS | ADR-011 §6.1 (ADR-011:516-530) is an exhaustive allow-list in which each layer depends only on layers listed before it. `route` depends on 4, 3, F and K. `replay` depends on 1 to 5. The driver is a separate crate downstream of CG. §7.1 (ADR-011:661-710): QSL depends on `quire-exact`, `quire-contract-model` and FCD. CG → QSL is through the `replay` facade only. IR root → QSL is removed (M-6d, IR #141). QSL → CG dev and QSL tests → RT are removed (M-6b). No cycle remains over normal, dev or test edges (FB-11). |
| Checked and unchecked inputs cannot be confused | PASS | ADR-013 R-10 (ADR-013:93) and T-1 (ADR-013:660): one nominal type per stage output, with private constructors. ADR-011 §4 (ADR-011:432-480) and FB-03 and FB-04. |
| The capability condition is met | PASS | #229 produced FR-057 (merged in QSL #237). QSpec FR-290 at 1e8bb50 carries the same ten labels, the claim-form table and the candidate-set table. ADR-012 §7 and §13.3 cite both. #185 has no capability decision left open. |
| Witness and replay are lossless | PASS | ADR-013 O-25 `ReplaySource`, and C-11 lossless widening (ADR-013:687). The amended AD-016 (QSpec #140) carries `ReplaySource`, both reproduced arms and the CG parity comparator. The O-27 parity-carrier cell is stale: Remaining work: #244. |
| Cross-repository changes have a named QSpec owner | PASS | ADR-013 QC-1 to QC-20 (ADR-013:811-830), filed as TK-06 to TK-10 (QSpec #136 to #139; QC-20 landed by #140). |
| Slices are bounded tickets | PASS | Each scenario cites existing tickets. ADR-011 T-1 to T-12 and ADR-013 TK-01 to TK-10 are filed (#240 to #243; CG #86 to #90; RT #55 and #56; IR #144; QSpec #136 to #139). |
| OBS-212-2: owner of the crossing replay test | DECIDED | Amended AD-016, arrows 6 and 7 (QSpec #140): QSL owns the crossing test (counterexample → native replay → parity). IR owns the agreement vectors, as data only. CG owns a TC for emitting the harness and the packet. IR FR-031-AC-3 has its own row, discharged by the QSL test. This resolves IR #143 item 1. It opens MD-1 against ADR-011. |
| OBS-212-1: IR coverage | RED, limits cross-repository evidence only | On IR `origin/main` 65aa282, `quire coverage --scope . --strict` backs 196 of 226 rows and exits 1. Sixteen rows are unbacked: FR-036, FR-037, TC-045, TC-046, TC-047, FR-035-AC-5, FR-036-AC-1 to AC-5 and FR-037-AC-1 to AC-5. None is contradicted. This red limits IR evidence cited by scenarios 1, 3 and 6. It does not affect any QSL decision. |
| Combined spec review | ACCEPT WITH FINDINGS after this PR's editorial fixes | SR-499 base; SR-500 failure-domain, SR-501 integrity, SR-503 evidence and SR-506 EARS each blocked on the rule-8 wording or the O-20 conflict, fixed in this PR (Round 2 in each file); SR-502 dependency, SR-504 risk-complexity and SR-505 scope-boundary accept with findings. |

## Missing decisions

### MD-1: where the crossing replay test runs and how it reads IR vectors

- **Owner:** #209 (ADR-011). Secondary: #211 (ADR-013 O-23, TK-04).
- **Gap:** ADR-011 §7.1 (ADR-011:703-706) puts cross-repository end-to-end
  evidence "in CG for proof and replay". ADR-011 OBS-040 (ADR-011:800) says
  QSL tests depend on QSpec vectors and QSL crates only. The amended AD-016
  puts the crossing test in QSL, over agreement vectors that IR owns as data.
  No decision says how a QSL test obtains IR-owned vectors. O-23
  (ADR-013:528) vendors QSpec bytes only. QSL cannot take a CG dev edge,
  because CG → QSL is normal (FB-11). ADR-013 TK-04 and the §9 IR #137 row
  (ADR-013:891, 962) still give FR-031-AC-3 to IR.
- **Proposed ruling:** The QSL crossing test reads IR's agreement vectors
  (counterexample packets with their bindings) as data files. It reads them
  from the IR repository at the revision QSL's `Cargo.lock` already pins for
  `quire-contract-model`, under the O-23 Cargo-pin rule. This adds no crate
  edge, so FB-11 holds. The test asserts native reproduction and the replay
  result. The parity comparator and the backend-evidence verdict stay in CG
  (CG #50). ADR-011 §7.1 and OBS-040 name QSL as the home of the crossing
  test, with the CG skeleton spine (CG #87) as the end-to-end run. ADR-013
  TK-04 and the IR #137 row move FR-031-AC-3 to the QSL test.

### MD-2: a registered backend with no CG kind

- **Owner:** #210 (ADR-012).
- **Gap:** ADR-012 §7.2 (ADR-012:547-551) says the driver refuses the whole
  run when a registered `BackendId` has no CG kind, and gives no catalog code
  or outcome category. FR-057:239 and FR-290 settle the same case item by
  item: "a registered backend with no negotiation arm" settles
  `invalid-request`, `invalid_capability`/`unknown-backend`. The two
  decisions disagree on scope, and ADR-012 has no code.
- **Proposed ruling:** The driver's pre-negotiation conversion refuses the run
  with `invalid_capability`/`unknown-backend`. It names every unconverted
  `BackendId` in bytewise order, as a command-level refusal that #225
  renders. FR-290's item-level row stays as CG's totality guard for requests
  formed outside the driver. ADR-012 §7.2 cites the code.

## Findings

| ID | Severity | Summary | Refs |
|---|---|---|---|
| FND-001 | high | Fixed in this PR. ADR-011 E3 and §10 row 8 said S3 mints `DeclarationKey`s, which contradicted ADR-013 O-03. | ADR-011:320, 823 |
| FND-002 | medium | FR-057 names its actor "the QSL composed linker", which SEAM-2 and M-6e delete. Remaining work: #185; the FR-036 and FR-057 re-homing goes through `/specify` before #185 (ADR-012 §14.1). | FR-057:27, 83 |
| FND-003 | medium | ADR-013 O-27, QC-7 and QC-14 describe the pre-amendment AD-016. Remaining work: #244. | ADR-013:641, 817, 824 |
| FND-004 | low | QSL's vendored diagnostic catalog lacks `invalid_capability`, `unproved-exhaustiveness`, `unbounded-extent`, `absent-extent`, `unknown-backend` and `tool-unavailable`, which FR-057 cites. QSpec `origin/main` has all six. Remaining work: #132. | `resources/complete-value/…/native-diagnostics.md` |
| FND-005 | low | AD-016 scenario 4 and the #189 title name `NativeModelProfile`, which ADR-011 retires with SEAM-1. AD-016 arrow 1 still says S3 "mints `DeclarationKey`". Both are QSpec-owned. | AD-016; ADR-011:818 |
| FND-006 | low | ADR-013 §7 says #222 waits on #212, while ADR-012 §1.1 lets #222 run in parallel. Scenario 5 passes either way. | ADR-012:201-202 |
| FND-007 | low | ADR-012 §5.2 says registry construction refuses a duplicate `BackendId`. FR-057:214-218 refuses only that registration and keeps the earlier one. #185 implements FR-057. | ADR-012:424 |
| FND-008 | medium | ADR-013 O-13 lists CG oracles as `quire-exact` consumers without restating rule 8 for an oracle over a kernel operation. Proposed text: "A CG oracle never computes its expectation with the kernel operation under proof or a kernel helper it calls; the expectation for a kernel operation comes from the QSpec operation law or vectors." Remaining work: #244. | ADR-013:294 |
