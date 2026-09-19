---
id: SR-459
title: "failure-domain review of ADR-010 observed architecture baseline"
type: SpecReview
analysis: failure-domain
scope: "spec/decisions/ADR-010-observed-architecture-baseline.md"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-010
    type: reviews
---

## Summary

Round 2. Reviewed ADR-010 (ARCH-00 observed architecture baseline, #206) at
quire-spec-language commit 432e615, against the round-1 findings raised at
faa1731 and the revision diff between the two. The failure-domain checklist was
adapted to a descriptive record, as in round 1. It covers trust boundaries at
the observed handoffs, identity confusion between duplicate types and digest
domains, and unstated failure modes in the cross-repository handoffs. Evidence
was re-checked at QSL de627b5, IR 553b6d1, RT d97bc0b, CG a4b2a73 (and CG
5e2a6a9 for the IT-010 chain) and QI 40cff46.

Both round-1 blocking findings are resolved. OBS-037 and a new §4.4 row now
record the wire-admission bypass into the checked-predicate and temporal-subject
handoffs to IR. `EffectiveId` is now in DA-02, §4.2, §4.4 and OBS-018, with the
correct `pub(crate)` `from_bytes` site. The five medium findings and both low
findings are resolved; one (evidence form) leaves a small residue.

The revision adds one medium problem. §3.2 and OBS-040 now say the IT-010 chain
has a single cross-revision guard (the digest equality at `:543-544`). The same
test file also pins the CG and IR revisions in `Cargo.lock` and checks
`IR_CANDIDATE_REVISION` (`:504-507`), and it leaves RT 8a4d02b with only a
length check (`:508`). Three low findings cover framing and wording.

Verdict: ACCEPT WITH FINDINGS. No high findings remain. One medium and three
low findings are open, all fixable by editing cells without changing a
conclusion or an owner.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Cross-revision guards on the IT-010 chain are misstated. §3.2 ("the only cross-revision guard is `:543-544`") and OBS-040 ("its only cross-revision guard is one digest equality") are wrong. Test `locked_backend_graph_has_one_reviewed_ir_and_pinned_kani` (IT-010-SC-01) checks that the lock holds CG 5e2a6a9 (`:504`), exactly one `quire-contract-ir` at IR 04eb6f8 (`:505-506`), and `codegen::IR_CANDIDATE_REVISION == IR_REVISION` (`:507`). So the IR 04eb6f8 ↔ CG 5e2a6a9 pairing is guarded. The unguarded link is RT 8a4d02b: `RUNTIME_REVISION` is checked only for length 40 (`:508`) and is used unchecked in the generated manifests (`:821`). The statement "the only check that IR model 53cc03c and IR 04eb6f8 agree is `:543-544`" in the §2.1 preconditions paragraph is correct and can stay. Fix: in §3.2 and OBS-040, list `:504-507` as the lock and revision guards. State that the producer ↔ consumer format agreement is checked only at `:543-544`, and that RT 8a4d02b has no revision check beyond length (`:508`). Owner unchanged (#209; DA-14 #211). | `QSL:tests/configversion_backends.rs:502-508,543-544,821` · `CG@5e2a6a9:src/oracle.rs:14,17` · ADR-010 §3.2, OBS-040, DA-14 |
| FND-002 | low | The `EffectiveId` → `NodeKey` transfer is framed only as a bypass, and its normative source is left out. `QSL:value/model_query.rs:10-16` calls the transfer "the documented canonical encoding" required by FR-143. It says a reference's `type` is literally the `quire.model.effective-declaration/v1` digest and that `NodeKey` "carr[ies] no domain tag of its own". `NodeKey::from_bytes` is documented as a "same-domain identity bridged from another 32-byte digest type" (`QSL:value/node.rs:44-48`). That conflicts with `NODE_KEY_DOMAIN = "quire.checked-semantic-node/v1"` (`QSL:value/node.rs:17-18`), which the DA-02 row cites as `NodeKey`'s domain. There is also a second reverse transfer, in `resolve_target` (`QSL:value/model_query.rs:155`), which §4.2 and OBS-018 do not list. The bypass reading hides that the code and FR-143 require this bridge, and that the two in-code domain statements disagree. That disagreement is the decision #211 has to make. Fix: in DA-02 and OBS-018, cite `model_query.rs:10-16`, FR-143 and `node.rs:17-18,44-48` as the conflicting domain statements, and add `:155` to the reverse-transfer cells. In §4.4, write "FR-143-sanctioned byte transfer across two declared domains" in place of "skips the node-id digest domain". | `QSL:value/model_query.rs:10-16,108,123,155` · `QSL:value/node.rs:17-18,44-49` · ADR-010 §4.2, §4.4, DA-02, OBS-018 |
| FND-003 | low | OBS-028 contradicts its own evidence. It now names `IR:tests/kani_replay.rs:240` as the sole caller of `replay_with_native_runtime`, and that function always runs the real `runtime::execute` (`IR:src/kani/replay.rs:88`); only `reconstruct` is caller-supplied. The next sentence says "Every call site injects a stub returning the expected verdict". That is true of the CG sites and of the IR `replay_counterexample` test sites (`IR:tests/kani_replay.rs:208,213,221`). It is not true of `:240`. Fix: scope the sentence to "every `replay_counterexample` call site (CG, and IR tests `:208-221`)". | `IR:src/kani/replay.rs:80-97` · `IR:tests/kani_replay.rs:208-221,240` · `CG:src/bounded_kani_replay.rs:57-60` · ADR-010 OBS-028 |
| FND-004 | low | Residue of round-1 FND-007 (evidence that cannot be re-checked). X4's evidence ("no consumer of `LoweredSourceGraph` outside `src/complete`") is not written in the new `absent:` form. OBS-011 and §2.2 still cite `QSL:tests/native_protocol_emission.rs` and `QSL:tests/compiled_protocol_v2.rs` with no line. The `(RT, case-insensitive)` qualifier on `absent: replay in src` departs from the convention's `git grep -n -F` definition, which is case-sensitive. The claims themselves were re-run and hold. Fix: write X4 as `absent: LoweredSourceGraph in src/` excluding `src/complete`. Give the test citations a line. Add an `absent-i:` variant (meaning `git grep -n -i -F`) to the Evidence convention. | ADR-010 §Evidence convention, §2.2, §2.6 X4, §1.2, OBS-011, OBS-038 |

## Round 1 resolution

Round 1 reviewed faa1731 (SR-459 round 1, verdict REJECT). Status against
432e615:

| Round-1 ID | Severity | Status | Reason |
| --- | --- | --- | --- |
| FND-001 | high | resolved | OBS-037 (#209, secondary #211), a §4.4 row, a §4.1 boundary row, a §2.2 note and the §6.2 "QSL f1700a9" input tags record the wire-admitted `v2::AdmittedPackage` → checked predicate / temporal subject → IR projection bypass. The citations were checked (`checked_predicate.rs:139,156`, `temporal_subject.rs:177,193`, `v2/intake.rs:476`, `IR predicate/admission.rs:92`). |
| FND-002 | high | resolved | `EffectiveId` and `ReferenceKey.type_identity` are in DA-02. §4.2 has both transfer directions with `model_query.rs:108,123`. §4.4 and OBS-018 cite `pub(crate)` `from_bytes` at `node.rs:49`, which was checked. The framing gap that remains is new FND-002 (low). |
| FND-003 | medium | resolved | §3.2 paragraph, §6.2 inputs and OBS-029 state that IR's typed handoffs take QSL f1700a9 types and that QSL main cannot reach them without serializing. |
| FND-004 | medium | resolved | §6.2 paragraph, OBS-027 and OBS-028 state that replay checks only that the witness is non-empty and reports disagreement as `Inconclusive` with `kani_native_replay_disagreement`. Line citations `:41,61,62,65,87,89,92` were checked. |
| FND-005 | medium | resolved | A9 is now "test panic (`unwrap`/`expect`)", A10 is "first `vec![…]` row only, exactly 8 bytes", A11 is "`None` outside `DOMAIN` (:262)". The revision chain is in §3.2, DA-14 and OBS-040. The new text overstates the single guard; see new FND-001. |
| FND-006 | medium | resolved | §2.2 checked-handoff rows now cite the public `checked_predicate.rs` and `temporal_subject.rs` entries. |
| FND-007 | medium | partially resolved | The Evidence convention now defines `absent: <pattern> in <path>`, and it is used in §1, §2.6, §3.2, §3.3 and the OBS rows. The presence cells named in round 1 now carry lines. The small residue is new FND-004 (low). |
| FND-008 | low | resolved | OBS-006 records the `quire/native` pseudo-package sharing the domain-package key space and the prefix-first match in `read_field_type_ref`, owner #211. |
| FND-009 | low | resolved | The §8 Notes column (first, second, third) now matches the governing order #228 → #204 → #200. |

Counts: 8 resolved, 1 partially resolved, 0 unresolved.

## Method

- Checklist, adapted to a descriptive record:
  - Trust boundaries: where admitted data comes from at each observed handoff,
    and what its type name claims.
  - Entity identity: which uniqueness key and digest domain each identity type
    uses, and where bytes move between domains.
  - Evaluation purity: whether caller-supplied closures in replay are recorded
    as what decides the verdict.
  - Topology: dependency cycles and version skew along the IT-010 chain.
- Scope: round-1 findings and the `faa1731..432e615` diff of
  `spec/decisions/`. Owner assignments follow the coordinator's constraint
  (#209, #210, #211 primary; #229 secondary only). No finding asks to change an
  owner.
- Re-checked at the pinned revisions: `QSL:value/node.rs:17-18,44-49`;
  `QSL:value/model_query.rs:10-16,108,123,155`; `QSL:model/population.rs:376`;
  `QSL:protocol_artifact/checked_predicate.rs:139,156`;
  `QSL:protocol_artifact/temporal_subject.rs:177,193`;
  `QSL:protocol_artifact/v2/intake.rs:476`;
  `QSL:tests/configversion_backends.rs:34,262,502-508,543-544,597,821,837,840,843-857,871,948-951`;
  `IR:src/kani/replay.rs:13-20,39-51,55-71,80-97`;
  `IR:src/predicate/admission.rs:92`; `IR:src/temporal/admission.rs:590`;
  `IR:tests/kani_replay.rs:208-221,240`; `CG@5e2a6a9:src/oracle.rs:14,17`.
- Absence claims re-run with `git grep -n -F`: `kani` (case-insensitive) in QSL
  `src/`; `CheckedPackageV2`, `native-run-result/2` in QSL `src/`; `mod intake`
  in QSL `src/model`; `crate::lowering` and `EXECUTABLE_PROJECTION` in the X5
  paths (plus `src/temporal.rs`, which the `src/temporal` pathspec does not
  cover); `replay` (case-insensitive) in RT `src`; `heads` in QI `Makefile`. All
  return nothing, as the record states.

## Round 2 resolution (author)

Recorded by the authoring agent; the round-2 verdict stands.

- FND-001 resolved: §2.1, §3.2 and OBS-040 name the IR ↔ CG pairing check at
  `configversion_backends.rs:504-507` and state that RT 8a4d02b is checked only
  for hash length at :508.
- FND-002 resolved: OBS-018 and §4.2 add the reverse transfer at
  `model_query.rs:155` and record the FR-143 comment (`:10-16`) against
  `NODE_KEY_DOMAIN` (`node.rs:17-18`) as the #211 decision.
- FND-003 resolved: OBS-028 limits the stub claim to `replay_counterexample`
  call sites.
- FND-004 resolved: X4 uses `absent:` with `excluding`; OBS-011 and §2.2 cite
  test lines; the convention defines `absent-i:` for case-insensitive checks.
