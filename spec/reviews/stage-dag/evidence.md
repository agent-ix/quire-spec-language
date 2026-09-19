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
