---
id: SR-470
title: "Evidence analysis of ADR-011 stage DAG and dependency architecture"
type: SpecReview
analysis: evidence
scope: "spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md, spec/spec.md (ADR-011 index row)"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: reviews
---
# SR-470: Evidence analysis of ADR-011

## Summary

Round 1. Reviewed commit 944a1c8 on `task/209-stage-dag`. ADR-011 is a design
record, not an FR. It has no acceptance-criterion rows, and `quoin advise`
returns no obligation for it. So every method below is a reviewer judgement,
not a catalog verdict. The review asks two questions of each decision. Can it
be verified? Does the record name the mechanism and the owner that verify it?
It also spot-checks the record's factual claims against ADR-010, the issues it
cites and `src/` at 944a1c8.

What holds:

- The 19 ADR-010 findings routed to #209 match ADR-010 §9.2.
- The §6.2 module table covers all 31 top-level modules in `QSL:lib.rs:13-44`.
- The AD-016 arrow 7 executor, the crate graph deltas and the Owner decisions
  are cited correctly.
- RT `src/exact/` is 11,519 lines at RT d97bc0b.
- The index row in `spec/spec.md:388` matches the record.

What does not hold:

- Two structural claims in the module DAG are false against the code:
  - `model` depends on check-stage types today (FND-001).
  - S3 `check` must consume an output of the layer-4 `package` reader
    (FND-002).
  As written, the §6.1 layer rule cannot be satisfied, and so neither can
  #212's "DAG remains acyclic".
- The forbidden bypasses (FB-01 to FB-12) have no per-bypass verifier.
- The proof-stage acceptance rule names CG defect tickets as its "gates", and
  names no gate for `quire-exact` after the kernel is extracted.

Verdict: **REVISE**. Two `high` findings need a change to §6 before #212 can
walk the DAG. The seven `medium` findings each name a missing or mis-cited
verifier. None reopens AD-016, #210 or #211.

## Findings

| ID | Severity | Summary | Refs |
|----|----------|---------|------|
| FND-001 | high | The M-2 and §6.1 claim is false against the code. The claim: `model` "depends on K and F only", "never depends on `value` or `check`", and the kernel move "breaks SCC S2". But `QSL:model/checked_dispatch.rs:109-112` imports `Expression`, `FunctionDeclaration`, `PackageDeclarations`, `DispatchTable`, `DispatchCandidate` and `DispatchOperation`. `model::conformance` uses `expression::established_field_fact` (`QSL:value/mod.rs:115-121`). All of these are `value::expression` check-stage types (`QSL:value/mod.rs:106-113`). None of them is in AD-016's kernel row, which keeps expression types out of `quire-exact`. So after X-1, `model` → `check` remains, and S2 is still a cycle. §6.2 maps neither `model::checked_dispatch` (ADR-010 lane C `X`, the only non-test producer of checked expressions, OBS-007) nor `model::conformance`. **Fix:** add §6.2 rows that place `model::checked_dispatch` and the FR-151 conformance obligation in layer 3 `check`, or else state which types they consume from K only. Then restate M-2's dependencies and public API to match. | ADR-011 §6.1, §6.2, §7.3 M-2; ADR-010 §3.1 S2, OBS-007 |
| FND-002 | high | Layer inversion on E3. E3 admits "dependency linked packages (I2)" (§2.1). §1 says I2 enters S3 "as an S4 linked package", and the I2 reader is owned by layer-4 `package`. But `check` is layer 3 and may depend only on lower layers (§6.1). `package` depends on layer 3, so `check` → `package` closes a cycle. The §6.2 row for `complete::package` (resolution) says "3 `library` / I2", which splits one module across the two layers. A #226 drift gate written to §6.1 would fail the first E3 implementation, or it would be written with an exemption. **Fix:** name the layer that owns the I2 read type and verifier. Either put it at or below layer 3 (for example a layer-3 `library` view of a verified package identity), or state that E3 admits a layer-3 type that `package` converts to. Then correct the §6.1 table and the E3 admitted-input cell. | ADR-011 §1 I2, §2.1 E3, §6.1, §6.2 `complete::package` |
| FND-003 | medium | The order inside the F layer is not stated, so the diagnostic↔source break cannot be checked. §6.1 says the foundation locus breaks diagnostic↔source. But `source.rs:3` imports `Code`, `Diagnostic` and `Phase`, and `diagnostic.rs:3` imports `LocatedSpan`, `Source`, `SourceIdentity` and `Span`. Both modules are in F, and F's "Depends on" is "K". Under the rule, no F module may import another F module, so `source` could not emit a `Diagnostic`. The record also says only the diagnostic cycles are broken by the locus. It does not say that the other S1 two-cycles, checking↔linking and linking↔native_model, retire with SEAM-1. **Fix:** give the order inside F, for example `source` < `diagnostic` < `wire_format` …, with its allowed intra-layer edges. Add one sentence that the remaining S1 two-cycles are removed by SEAM-1. | ADR-011 §6.1; ADR-010 §3.1 S1, two-cycles |
| FND-004 | medium | The §3 bypass table has no "Verified by" column. For cross-repository bypasses, nothing named can detect the bypass. The Consequences name #226 for all of §3, §6 and §7. But #226 is Layer 5 and depends on #215 and #216, so Layers 2 to 4 are built with no enforcement of the module DAG or of the bypass rules. #226 is also a QSL-repository ticket. FB-05 (a backend depends on QSL types) and FB-11 (a test-time cycle across repositories) can only be seen across repositories. None of AD-016's seven `heads/` drift checks tests dependency direction or cycles. **Fix:** add a "Verified by / owner" column. Suggested owners by bypass: FB-01, FB-02 and FB-09 → #216 "no consumer reconstructs meaning" evidence, then the #226 display-text gate. FB-03 and FB-04 → #216 typestate evidence, then the #226 checked-input entry-point gate. FB-06 → the #226 module-DAG check. FB-07 and FB-08 → #217 and #219 with IR #140. FB-10 → §2.3 through RT #53 and #219. FB-12 → #210 and #185. FB-05 and FB-11 → a named cross-repository check (#215 or QI `heads/`) with an owner. Also state which gate enforces §6.1 before #226 lands. | ADR-011 §3, §6.1, Consequences; #226; AD-016 Current-head integration |
| FND-005 | medium | §2.3 names the wrong owners. It says the rule "binds … the CG generated-harness gates (CG #58, #59, #60 and #73)". Those are defect tickets: no Kani timeout, verdicts by string match, classification stops at the first refusal, and a corpus identity collision. None is a gate that can report `unreached` or run a mutation control. #219 (ARCH-G3, the executable proof, witness and replay gate) is the natural spine owner, and it is cited in §2.3 but missing from the implementing-tickets table. **Fix:** name the CG gate by its make target or script, or open a CG ticket that adds the unreached-module and mutation-control rule to CG's generated-harness gate. Add #219 to the table as the owner that verifies §2.3 on the spine. | ADR-011 §2.3, Context table; CG #58, #59, #60, #73; QSL #219 |
| FND-006 | medium | The FB-02 "Observed today: none known" entry is false at the scope the row states ("Any consumer"). CG #59 records that CG decides Kani verdicts by string-matching the backend's human-readable output. That is a consumer branching on rendered text instead of a typed cause. **Fix:** cite CG #59 in the FB-02 "Observed today" cell, and link its retirement to #231's typed proof-result envelope. | ADR-011 §3 FB-02; CG #59; QSL #231 |
| FND-007 | medium | The proof-stage acceptance rule has no subject for `quire-exact`, and nothing says where each gate declares the modules it claims. X-1 moves the RT `src/exact/` kernel parts into `crates/quire-exact` in the QSL repository. After that, RT #53's harness and mutation control over `src/exact/` either move with the code or no longer apply. No gate is named that claims `quire-exact` and discharges a proposition in it, although AD-016 names CG oracles and RT as its consumers. The rule "every module it claims" also needs a place where each gate declares its claims. **Fix:** name the Kani gate that owns `quire-exact` and its owning change (X-1 / AD-016 WP5a). State that each proof gate publishes its claimed-module list in its census, for example RT `scripts/check_kani_harnesses.py`, and that `unreached` is computed against that list. | ADR-011 §2.3, §7.3 X-1; RT #53; AD-016 Shared-type strategy |
| FND-008 | medium | The IR root → QSL removal has no IR-side owner. §7.1 gives its owner as "#218 and #223, with IR #109". IR #109 is "Represent frame clauses in the Contract IR". It is not predicate or temporal admission from v2. No open IR issue at 2026-09-19 removes IR's QSL dependency or rewrites predicate and temporal admission onto v2 forms. FB-05's only live violation (OBS-029) therefore has no owner in the repository that must change. **Fix:** open or name the IR ticket for v2 predicate and temporal admission and removal of the QSL dependency, and cite it in §7.1 and FB-05. Keep IR #109 on the frame row only. | ADR-011 §7.1, FB-05, §9 OBS-029; IR #109 |
| FND-009 | medium | The single-revision lock rule has no working check. §7.1 says "The QSL lock resolves exactly one revision per quire-ecosystem crate. This matches AD-016 heads drift check 6". Drift check 6 runs `cargo tree -d` in the `heads/` workspace, where `[patch]` maps every pinned revision to one head. So it cannot see two revisions in QSL's own `Cargo.lock`, which is exactly the OBS-041 case (PR #200 adding a second quire-rs revision). **Fix:** name a check on QSL's own lock, for example a duplicate-revision check in the QSL exact-pin lane owned by #215 or in #226, and keep drift check 6 for head drift only. | ADR-011 §7.1 rules, §9 OBS-041; AD-016 drift check 6 |
| FND-010 | low | §4 and §5 give rules with no verification method. §4: the constructors of S3 and S4 types are private to their stage modules. Inside one crate, only module privacy holds this, and `pub(crate)` does not. Across crates it can be shown by `compile_fail` doctests. §5: the exit-code map has no `_` arm, which can be checked by `clippy::wildcard_enum_match_arm` on that function or by a closure test. **Fix:** add one sentence per rule that names the method and the gate. Suggested gates: #216 "checked typestate prevents unchecked values" evidence for §4, and #230 for §5. | ADR-011 §4, §5; #216; #230 |
| FND-011 | low | SEAM-1 quotes "#205 Layer 2: 'No deprecated producer path remains reachable'". That text is not in #205. The gate wording is #216's: "Old producer/bypass paths are unreachable." **Fix:** quote and cite #216. | ADR-011 §6.2 SEAM-1; #205; #216 |
| FND-012 | low | `state` imports IR today, which the target layer rule forbids, and the record does not say so. `QSL:state/evaluation.rs:15` has `use quire_contract_ir as ir;`, which is the `quire-contract-model` crate, and uses `ir::ValueType` at `:1352,1357`. §6.1 layer 5 depends on "4, 3, K" only. The §6.2 `state` row mentions only FB-03. **Fix:** note the import in the `state` row, and name its retirement: the K `ValueType` from X-1, typed by #211. | ADR-011 §6.1 layer 5, §6.2 `state` |
| FND-013 | low | The RT figure in the Context has no revision. "Compiles 11,519 lines of it" does not say which revision was counted, and RT #53 does not give the figure. It holds at RT d97bc0b (origin/main, 2026-09-18). **Fix:** add "at RT d97bc0b", following the ADR-010 evidence convention. | ADR-011 Context (RT #53) |

## Verification by decision

This table is reviewer judgement. The catalog matched no rule, because ADR-011
has no criterion rows.

| Decision | Verifiable by | Named verifier in ADR-011 | Gap |
|---|---|---|---|
| 1 Stage DAG, every module mapped | Inspection of §6.2 against `lib.rs`, then a #226 module-to-stage check | #226 (Consequences) | Submodules that cross stages are unmapped (FND-001) |
| 2 Edge contracts §2.1–§2.2 | Typed round-trip and adverse tests per edge | #213, #222, #231; #212 re-walk | None beyond FND-002 |
| 3 Forbidden bypasses | Seeded-bypass adverse tests (#226 acceptance) | #226 only | FND-004, FND-006, FND-008 |
| 4 Checking before lowering | `compile_fail` typestate tests; #216 evidence | #211 and #213 (design and implementation) | Method not named (FND-010) |
| 5 CLI orchestration | #230 conformance slice; exhaustive exit map | #225, #230 | Method not named (FND-010) |
| 6 Module and crate DAG | Module-import graph check and `cargo tree` direction check | #226 | FND-001, FND-002, FND-003, FND-009 |
| 8 Proof-stage acceptance | Transcript check-location census and a mutation control per claimed module | RT #53, CG #58–#73, #217, #219 | FND-005, FND-007 |

## Method

- **ADR-011.** Read at 944a1c8 (`git -C` log confirms the head). The index row
  was checked at `spec/spec.md:37-39,388`.
- **`quoin advise --json`.** Run in the worktree. It returned zero rows for
  ADR-011.
- **ADR-010 claims checked:**
  - §9.2 owner column: 19 primary #209 rows, and secondary rows OBS-005,
    OBS-017 and OBS-034.
  - §3.1: SCCs S1–S3 and the 7 two-cycles.
  - §2: lane labels.
  - The X4 row.
- **QSL `src/` at 944a1c8:**
  - `lib.rs:13-44`
  - `model/checked_dispatch.rs:101-112,747`
  - `value/mod.rs:106-121`
  - `source.rs:3` · `diagnostic.rs:3`
  - `state/evaluation.rs:15,1352,1357`
  - `value/division.rs:223` · `value/ieee.rs:882`
  - `complete/package.rs:1340` · `complete/{editor,edit}.rs`
  - `value/expression/mod.rs:71,650`
  - `Cargo.toml:36,43-44`
  - A search for diagnostic-text branching in `src/` found none (FB-02
    in-repository).
- **AD-016.** Read from `quire-specification` `origin/main`: arrow 7, the
  Shared-type row, the crate graph, the `heads/` drift checks and the Owner
  decisions.
- **GitHub, read-only `gh`:**
  - Issues: #205, #209, #212, #216, #226, RT #53 and IR #140.
  - Titles of #213–#232, #185, #122, #133, #29, CG #58, #59, #60, #73, IR #109
    and QSL PR #200.
  - The full open-issue list of IR.
- **RT.** The `src/exact/` line count was taken at d97bc0b.

## Round 2 (HEAD 5cbd853)

### Round 2 summary

Round 2 reviews commit 5cbd853, which revises the record against the round-1
findings. Round-1 content above is unchanged. Code claims were checked against
`src/` at 5cbd853, and the #215, #216, #219 and #226 bodies were read with
`gh`. Eleven of the 13 round-1 findings are resolved and two are partly
resolved. The revision introduced one new `high` finding: its new
`semantic_value` split leaves the `quire-exact` kernel depending on layer 3.

### Resolution of round-1 findings

| Finding | Status | Reason |
|---|---|---|
| FND-001 | Resolved | §6.1 and the new §6.2 row move `model::checked_dispatch` and `check_field_refinement_obligation` to `check` (M-2). At 5cbd853 the only other `model` → `value::expression` imports are `Location` and `Origin`, which are in the kernel row, and the helpers of the moved obligation (`conformance.rs:741-893`). |
| FND-002 | Resolved | §1 I2 row: the reader is in layer-4 `package`, the view type is in layer-3 `library`, and `check` never calls `package`. The E3 cell names layer-3 import views. |
| FND-003 | Resolved | §6.1 gives the F order `json_number` < … < `source` < `diagnostic` < `located_json`, and `source` no longer constructs a `Diagnostic` (M-1). The checking↔linking and linking↔native_model two-cycles retire with SEAM-1. The imports at `diagnostic.rs:3`, `source.rs:3` and `located_json.rs:4` fit that order once M-1 lands. |
| FND-004 | Partly | §3 now has a "Verified by" column and an interim rule until #226. But FB-05 and FB-11 are assigned to "#215 direction check", and #215's scope and acceptance hold no direction check. The Owner questions do not list a #215 amendment. |
| FND-005 | Resolved | §2.3 names CG #58, #59, #60 and #73 as defects and not the gate. The CG gate rule goes to Owner question 7. #219 is in the ticket table, and its body carries the §2.3 rule. |
| FND-006 | Resolved | FB-02 cites CG #59 and its retirement with the #231 envelope. |
| FND-007 | Resolved | §2.3 requires a checked-in claimed-module list for each gate. The `quire-exact` gate goes to the X-1 change and Owner question 6. |
| FND-008 | Resolved | §7.1 and FB-05 now route the IR removal to Owner question 4. IR #109 is scoped to scenario 4 only. |
| FND-009 | Partly | §7.1 moves the lock check off drift check 6 and onto "#215's exact-pin lane". #215's scope preserves the exact-pin lane as it is and adds no duplicate-revision check. No Owner question routes the change. |
| FND-010 | Resolved | §4 names `compile_fail` tests as #216 evidence. §5 names `clippy::wildcard_enum_match_arm` and #230. |
| FND-011 | Resolved | SEAM-1 quotes #216. Both quoted sentences match the #216 body. |
| FND-012 | Resolved | The `state` row cites the IR import (`state/evaluation.rs:15`) and the `native_model` import (`:21`), and M-6c removes both. |
| FND-013 | Resolved | The Context now reads "At RT d97bc0b (origin/main, 2026-09-18)". |

### Verification by decision (round 2)

This table replaces the round-1 table for this revision. It is still reviewer
judgement, because ADR-011 has no criterion rows.

| Decision | Verifiable by | Named verifier in ADR-011 | Gap |
|---|---|---|---|
| 1 Stage DAG, every module mapped | Inspection of §6.2 against `lib.rs`, then the #226 module-to-stage check | #226; §3 interim walk by #216 and #219 | FND-014, FND-015 |
| 2 Edge contracts §2.1–§2.2 | Typed round-trip and adverse tests per edge | #213, #222, #231; #212 re-walk | None |
| 3 Forbidden bypasses | Seeded-bypass adverse tests | Per-row "Verified by" column: #216, #217, #219, #230, #231, #215, #226 | FB-05 and FB-11 name a #215 check that #215 does not hold (FND-004) |
| 4 Checking before lowering | `compile_fail` typestate tests | #216 | None |
| 5 CLI orchestration | #230 conformance slice; `clippy::wildcard_enum_match_arm` | #225, #230 | None |
| 6 Module and crate DAG | Module-import graph check and `cargo tree` direction check; QSL lock duplicate-revision check | #226; #215 | FND-009, FND-014, FND-015 |
| 8 Proof-stage acceptance | Transcript check-location census and a mutation control per claimed module | #219; RT #53; X-1 change and CG gate through Owner questions 6 and 7 | FND-016 |

### New findings

| ID | Severity | Summary | Refs |
|---|---|---|---|
| FND-014 | high | The revision creates a kernel ↔ layer-3 cycle. §6.2 now maps `composite`, `collection`, `equality` and `outcome` to K `quire-exact`, and maps `enumeration`, `quantity`, `unit`, `key` and `reference` to layer-3 `semantic_value`. But the kernel `Value` and `ValueType` (`value/composite.rs:50-67,144-158`) have `Quantity`, `Enum`, `Reference` and `Population` variants. Their payload types are imported from `quantity`, `enumeration`, `reference` and `model::population::PopulationBinding` (`composite.rs:22,27,29,31`). `collection.rs:25` imports `key::compare_keys`. `equality.rs:23,26` imports `enumeration` and `quantity`. `outcome.rs:10,12,13` imports `expression::WrongSnapshotCause`, `reference` and `diagnostic::Code`. K "depends on none in the ecosystem", so X-1 cannot build a leaf crate from this map. The claim that M-2 breaks SCC S2 then fails, because `semantic_value` and `model` import K while K imports them back. AD-016 limits the kernel to "exactly the types listed in this row plus `Undefined`". **Fix:** state which layer owns the payload types of `Value`'s `Quantity`, `Enum`, `Reference` and `Population` variants. Either put their closure in K, or state that the kernel `Value` is narrower than QSL's and name where the extended variants live. If the choice is a type decision, route it to #211 and AD-016 WP5a. Then correct the §6.2 kernel and `semantic_value` rows and the X-1 "Leaf" claim. Also place `WrongSnapshotCause` and the `Code` that `outcome` uses at or below K. | ADR-011 §6.1 layers K and 3, §6.2 `value` rows, §7.3 X-1 and M-2; AD-016 Shared-type strategy |
| FND-015 | medium | Layer inversion inside the `value::library` row. §6.2 maps `value::library` to layer-3 `library` and `value::package_identity` to layer-4 `package`. But `value/library.rs:25` imports `project_declarations`, `PreimageDefect` and `ProjectedDeclarations` from `package_identity`. Layer 3 may not depend on layer 4. The row dates from 944a1c8 and round 1 missed it. The revision makes §6.1 an exhaustive allow-list, so a #226 gate written to it would fail here. **Fix:** place the declaration projection that `library` uses in layer 3, or state that `library` receives it as a layer-3 type. Then correct the row. | ADR-011 §6.1, §6.2 `value::library` row |
| FND-016 | low | §2.3 says "#226 checks each claimed-module list against the transcript census". #226's gate list holds no proof-transcript or claimed-module check. #219 does carry the rule. **Fix:** drop the #226 clause or route it as a #226 amendment. Keep #219 as the verifier. | ADR-011 §2.3; #226; #219 |

### Round 2 verdict

**REVISE.** FND-014 is a structural error that the revision introduced. With the
§6.2 kernel split as written, X-1 cannot produce a leaf crate. Each remaining
item needs a short text change or routing only:

- FND-004 and FND-009: route the direction check and the lock
  duplicate-revision check as a #215 amendment in the Owner questions, or
  assign them to #226.
- FND-015 and FND-016.

No other round-1 finding needs more work. None of the new findings reopens
AD-016, #210 or #211.

### Round 2 method

- **ADR-011.** Read at 5cbd853. `git -C` log confirms the head. The round-1
  commit was 944a1c8.
- **QSL `src/` at 5cbd853:**
  - `diagnostic.rs:3` · `source.rs:3` · `source_map.rs:3` · `located_json.rs:4`
  - `model/conformance.rs:89-91,741-894`
  - `model/*` imports of `crate::value`
  - `value/mod.rs:95-121`
  - `value/composite.rs:18-31,50-67,144-158`
  - `value/collection.rs:19-26` · `value/equality.rs:16-30`
    · `value/outcome.rs:8-13`
  - `value/{enumeration,quantity,unit,key,reference,definition}.rs` imports
  - `value/library.rs:24-26` · `value/package_identity.rs:10`
  - `state/evaluation.rs:15,21,1352,1357`
- **AD-016.** Read the Shared-type strategy row from `quire-specification`
  `origin/main` with `gh api`.
- **GitHub, read-only `gh`:** the #215, #216, #219 and #226 bodies.
