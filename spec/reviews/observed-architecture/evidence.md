---
id: SR-462
title: "Evidence analysis of ADR-010 observed architecture baseline"
type: SpecReview
analysis: evidence
scope: "spec/decisions/ADR-010-observed-architecture-baseline.md"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-010
    type: reviews
---
# SR-462: Evidence analysis of ADR-010

## Summary

Reviewed commit faa1731 on `task/206-observed-architecture`: ADR-010 and its
index row in `spec/spec.md` (line 385, which is correct). ADR-010 is a
descriptive record, not a requirement set. It has one obligation, from #206:
"every assertion has an evidence path and revision". The only way to verify that
obligation is to inspect the cited code at the pinned revisions. `quoin advise`
applies to requirement obligations, so this review does not use it. Every
finding below comes from reading the cited code.

Citations checked: 253. They cover QSL (187), IR (27), CG (20), RT (13),
QSpec (3) and QI (3). Of these, 241 held and 12 failed. A citation held when the
file exists at the pinned sha, the named symbol is within 5 lines of the cited
line, and the claim is true. The 12 failures are:

- Four derive/read cells in the checked-handoff table name the wrong file.
- The C5 `InputRefusal` cell names the wrong file.
- The IR root → IR model Cargo edge has the wrong dependency kind.
- ValueNode is described as "i64 only", which is false.
- The claim about which files call B8 is false.
- Three S2/S3 example edges cite doc comments, not `use` items.
- The `Code` variant count is off by one.

Four more cells hold but are imprecise. Some presence claims have no line
number. A few assertions have no citation. Several PR #200 citations have no
revision.

The data verified end to end: pin staleness (9 of 9), the vendored-tree lag
(144 and 10), the open-issue count (68, which matches the §7 rows), the
largest-file line counts, and the §1, §6.1, §5 and §9 tallies.

Verdict: REJECT. The three `high` findings (FND-001, FND-002, FND-003) are
blocking under this review's rules. Each one needs only a corrected citation or
reworded sentence. No structural rework is needed. With those fixed, the record
would be ACCEPT WITH FINDINGS.

## Findings

| ID | Severity | Summary | Refs |
|----|----------|---------|------|
| FND-001 | high | §2.2 checked-handoff table cites derive/read at `QSL:protocol_artifact/checked_handoff.rs:139`, `:156`, `:177`, `:193`. Those lines are a closing brace, the field `json_depth`, `fn result` and `self.usage`. The functions live in other files. Corrected: checked predicate derive `QSL:protocol_artifact/checked_predicate.rs:139`, read `QSL:protocol_artifact/checked_predicate.rs:156`. Temporal subject derive `QSL:protocol_artifact/temporal_subject.rs:177`, read `QSL:protocol_artifact/temporal_subject.rs:193`. The format constants at `checked_handoff.rs:1585-1586` are correct. | ADR-010 §2.2 |
| FND-002 | high | §3.2 row "IR root → IR model, normal, 53cc03c, `IR:Cargo.toml:37`" is false. Line 37 is under `[dev-dependencies]` (`quire-contract-model-owner`, rev 53cc03c). IR root's normal dependency on the model crate is a workspace path dependency. Corrected: normal `IR:Cargo.toml:21` (`path = "crates/quire-contract-model"`). Dev-only pin 53cc03c `IR:Cargo.toml:37`. §6.3's "IR model 53cc03c" row should also say the IR-side use is dev. | ADR-010 §3.2, §6.3 |
| FND-003 | high | DA-06 "`runtime::input::ValueNode` (i64 only)" and OBS-019 "Runtime values are i64 only" are false. `ValueNode` has 10 variants: Boolean, Integer, Text, Enum, Record, Absent, Present, Sequence, Reference, Object (`QSL:runtime/input.rs:94`). Only its integer payload is `i64` (`QSL:runtime/input.rs:101-104`). AD-016 itself says the relevant gap is "no function variant" (`QSpec:spec/assurance/AD-016-semantic-family-extension-path.md:238`). Corrected wording: "runtime `ValueNode` integers are fixed-width `i64` and it has no function variant". | ADR-010 §5 DA-06, §9.2 OBS-019 |
| FND-004 | medium | C5 cites `InputRefusal` at `QSL:value/expression/evaluate.rs:105`, which is the body of `charge_call`. Corrected: `QSL:value/expression/mod.rs:105`. | ADR-010 §2.3 C5 |
| FND-005 | medium | "Callers of B8: only `examples/protocol-handoff/producer.rs:1200-1566` and `tests/compiled_protocol_v2.rs`" is false. At least 20 test files call `native::admit`/`emit`/`admit_v2`/`admit_v3`, for example `QSL:tests/native_protocol_emission.rs`, `QSL:tests/composed_state_evaluation.rs` and `QSL:tests/native_temporal_owner.rs`. Also, `producer.rs:1200` is `linking::admit_namespace`, not B8. The underlying point, that B8 has no non-test and non-example caller, holds. Corrected: "B8 has no caller in `src/` outside `protocol_artifact::native`. Callers are the example (`QSL:examples/protocol-handoff/producer.rs:1411,1415,1566`) and tests under `QSL:tests/`". OBS-011 uses the same citation. | ADR-010 §2.2, §9.2 OBS-011 |
| FND-006 | medium | §3.1 cites the S2/S3 closing edges at doc comments, not `use` items: `QSL:value/collection.rs:170`, `QSL:model/accounting.rs:5` and `QSL:temporal.rs:15`. The stated method is `use crate::…` edges. The edges are real. The counts value→model ×39 and model→value ×27 match every textual mention including comments (38 and 27). They do not match non-comment lines (8 and 7). Corrected examples: value→model `QSL:value/composite.rs:31`, model→value `QSL:model/domain_package.rs:14`, temporal→protocol_artifact `QSL:temporal.rs:29`. State what the ×39 and ×27 counts count. | ADR-010 §3.1 |
| FND-007 | medium | Some citations for presence claims have no line number, which breaks the record's own `<prefix>:<path>:<line>` convention. Corrected: quire-rs optional dependency `QSL:Cargo.toml:26` (feature `:17`). Wire formats `QSL:wire_format.rs:27` (native-run/1), `:29` (native-compile/1), `:31` (native-run-result/1), `:37` (native-state-input/1), `:39` (native-linked-package/1). `CLOCK_PREFIX` `QSL:temporal.rs:50`. Catalog 1-draft.3 `QSL:complete/diagnostic.rs:13`. `object_keys` bridge `QSL:model/checked_dispatch.rs:657`. `NativeType` `QSL:checking/types.rs:12`. "sha256-jcs" `QSL:model/key.rs:23`. `quire.value.accounting/v1` `QSL:value/accounting.rs:2`. KANI_SHA256 `QSL:tests/configversion_backends.rs:34`. CG pins.json claim `CG:assurance/pins.json:26`. FR-322 16 codes `IR:tests/fixtures/checked-package/PROVENANCE:117-127`. Absence claims (`QSL:Cargo.toml` with no dependency, `QSL:model/` tree, `QI:Makefile`) can reasonably stay without a line. | ADR-010 §1, §3.2, §3.3, §4.2, §4.3, §5, §9.2 |
| FND-008 | low | Some assertions have no citation. Exit codes 20/21/22/30: cite `QSL:main.rs:24-25,113-115,138` and `QSL:command.rs:227-232`. "`NativePackage::read_verified` recompiles from source": cite `QSL:package.rs:233-246`. "Reached through `RunSelection::Extracted`": cite `QSL:command/compilation.rs:26,36`. The §3.2 IR-type use counts (`ValueType` 212 and others) and the §3.1 fan-in, fan-out and public-item tables name a method but no reproducible command. | ADR-010 §2.1, §3.1, §3.2 |
| FND-009 | low | Citations to PR #200 (`PR #200 src/model/intake.rs`, "2104 lines") carry no revision, although #206 requires one for every assertion. The claims hold at PR head 13b6687 (`src/model/intake.rs` +2104). Corrected: `PR #200@13b6687:src/model/intake.rs`. | ADR-010 §1, §8, §9.2 OBS-006 |
| FND-010 | low | DA-10 says `Diagnostic` has a "44-variant `Code`". The enum at `QSL:diagnostic.rs:51` has 45 variants (IoError … CardinalityOutOfBound). Corrected: 45. | ADR-010 §5 DA-10 |
| FND-011 | low | A11 "asserted :946" is 3 lines early. `:946` closes the `post` binding, and the verdict assertion is `QSL:tests/configversion_backends.rs:948-951` (`native_verdict` at `:949`). The same applies in §1 and OBS-002. | ADR-010 §1, §2.1 A11, §9.2 OBS-002 |
| FND-012 | low | "`replay_with_native_runtime` … sole caller is one IR unit test" (§6.2, OBS-028). The sole caller is an integration test, `IR:tests/kani_replay.rs:240` (TC-042), not a unit test. Corrected: "sole caller is IR integration test `IR:tests/kani_replay.rs:240`". | ADR-010 §6.2, §9.2 OBS-028 |

## Method

Each citation was read with `git -C <clone> show <sha>:<path>` or
`git grep -n` at the pinned sha. Local clones were used for all 7 repositories.
The pinned shas are QSL de627b5, IR 553b6d1, CG a4b2a73, RT d97bc0b, QSpec
3a79dce and QI 40cff46. Nothing was checked out and nothing was built. PR and
issue facts came from read-only `gh`. A citation held when the file exists at
the sha, the symbol is within 5 lines of the cited line, and the claim is true.
Variant counts were taken by listing the enum's variant lines.

Citations checked (✗ = failed, ~ = held with a note):

- **QSL §1 / §2.1 (lane A):** `value/package_identity.rs:13,15,334` ·
  `Cargo.toml:36,43,44` · `source_map.rs:30` · `value/expression/refusal.rs:32` ·
  `linking/composed/requests.rs:20,36,80,282` · `value/division.rs:223` ·
  `value/ieee.rs:830,882` · `value/expression/mod.rs:71,234,635,659` ·
  `tests/configversion_backends.rs:32-33,34,252,259,543,597,789,831,843-857,871,900`,
  `:946` ~ · `source.rs:94,189` · `diagnostic.rs:306` · `parser.rs:13,25` ·
  `syntax.rs:57,324` · `model_source.rs:225,248` · `linking.rs:190,202,295,319` ·
  `command/compilation.rs:~95` · `checking.rs:263,302` · `checking/proof.rs:660` ·
  `package.rs:141,202,224,252` · `package/view.rs:39-46` ·
  `lowering.rs:140,228,274,283,385` · `lowering/wire.rs:51` ·
  `lowering/target.rs:39-46` · `runtime/execution.rs:23,43,97` ·
  `runtime/validation.rs:181` · `runtime/evaluation.rs:40` · `cli.rs:59-126` ·
  `command.rs:309,315,321,326` · `mapped.rs:110,138` · `quire_source.rs:312`.
- **QSL §2.2 (lane B):** `parser.rs:43,55` · `syntax/composed.rs:10,18` ·
  `linking/composed.rs:303,375` · `linking/composed/binding.rs:16,79,180` ·
  `linking/composed/models.rs:70,278` · `checking/composed.rs:266,311` ·
  `checking/composed/proofs.rs:157,217` · `protocol_artifact/native/mod.rs:82,104` ·
  `protocol_artifact/native/temporal_v2.rs:170` ·
  `protocol_artifact/native/temporal_v3.rs:49` · `protocol_artifact/intake.rs:520` ·
  `protocol_artifact/v2/intake.rs:476` · `protocol_artifact/v3/intake.rs:211` ·
  `protocol_artifact/mod.rs:387` · `protocol_artifact/v2/refusal.rs:141` ·
  `protocol_artifact/v3/refusal.rs:38` · `state/evaluation.rs:35,45` ·
  `state/input.rs:277,316` · `temporal.rs:58,145` · `temporal/result.rs:213` ·
  `protocol_artifact/checked_handoff.rs:139` ✗, `:156` ✗, `:177` ✗, `:193` ✗,
  `:1585-1586` · `protocol_artifact/native_temporal/request.rs:1317` ·
  `protocol_artifact/native_temporal/result.rs:754` ·
  `protocol_artifact/native_temporal/common.rs:13-14` ·
  `protocol_artifact/native_temporal/v2.rs:22-24,397,535` ·
  `examples/protocol-handoff/producer.rs:1200-1566` ✗ (the "only callers" claim).
- **QSL §2.3–§2.6 (lanes C, D, absences):** `complete/mod.rs:103` ·
  `complete/cst.rs:189` · `complete/diagnostic.rs:183` ·
  `complete/package.rs:587,625,690,847,861,1340` ·
  `value/expression/refusal.rs:363` · `value/expression/evaluate.rs:51`,
  `:105` ✗ · `value/outcome.rs:18,103` · `model/normalize.rs:218,230,2102` ·
  `value/library.rs:181,374` · `model/checked_dispatch.rs:154,653` ·
  `simulation/explore.rs:59,97` · `simulation/sample.rs:95` ·
  `simulation/trace.rs:97` · `model/domain_package.rs:441`. "No consumer of
  `LoweredSourceGraph` outside `src/complete`" and "`checked_dispatch_operation`
  is the only non-test `Expression` producer" were confirmed by `git grep`.
- **QSL §3–§5:** `diagnostic.rs:3,320,324`, `:51` ✗ (count) · `source.rs:3` ·
  `linking/composed/models.rs:11` · `native_model/admission.rs:12` ·
  `value/collection.rs:170` ✗ · `model/accounting.rs:5` ✗ · `temporal.rs:15` ✗ ·
  `protocol_artifact/native_temporal/common.rs:11` · `linking.rs:69` ·
  `model/key.rs:72` · `value/node.rs:18,22` · `value/composite.rs:36,67,132,158` ·
  `checking/types.rs:462` · `state/input.rs:43,207,307` ·
  `runtime/input.rs:94` ✗ (claim) · `syntax.rs:294` ·
  `value/expression/syntax.rs:86` · `value/outcome.rs:171,175` ·
  `model/systems.rs:269` · `protocol_artifact/validate.rs:287,291` ·
  `state/evaluation.rs:2478-2484,2781-2783` · `protocol_artifact/mod.rs:1-8,49,57` ·
  `protocol_artifact/v2/mod.rs:19` · `protocol_artifact/v3/mod.rs:14` ·
  `wire_format.rs:33-35` · `native_model.rs:27-28` ·
  `linking/composed/definition_source.rs:240` ·
  `checking/composed/proofs/engine.rs:579` ·
  `resources/native-v1/VENDOR.json:20` · `resources/complete-value/VENDOR.json:64`
  (the sha256 values at :21 and :65 differ, as claimed) ·
  `value/accounting.rs:143,392,465` · `model/accounting.rs:127,201,244` ·
  `tests/fixtures/native-lowering/Cargo.toml:14`. Line counts:
  `state/evaluation.rs` 3110, `model/normalize.rs` 2189.
- **IR:** `Cargo.toml:24,38`, `:37` ✗ (kind) · `src/kani/replay.rs:13-20,55,80,88` ·
  `crates/quire-contract-model/src/canonical.rs:68` ·
  `crates/quire-contract-model/src/expression.rs:189` (8 variants) ·
  `crates/quire-contract-model/src/identity.rs:833` (6 variants) ·
  `crates/quire-contract-model/src/checked_package/dispatch.rs:28,65` ·
  `checked_package/v2/mod.rs:104,622` · `checked_package/v2/lower.rs:117,135` ·
  `checked_package/shared.rs:65` (13 variants), `:269` ·
  `src/predicate/admission.rs:92` · `src/temporal/admission.rs:590` ·
  `src/kani/arithmetic.rs:40` · `src/kani/dispatch.rs:88` ·
  `src/kani/outcome.rs:8-29` (10 variants), `:33` · `src/kani/abi.rs:21` ·
  `spec/contract/FR-031-bounded-kani-dispatch-replay-provenance.md:15,23,31,39` ·
  `tests/fixtures/checked-package/PROVENANCE` ·
  sole caller of `replay_with_native_runtime` ~ (FND-012).
- **CG:** `Cargo.toml:17,18,27,28` · `src/exact_scalar.rs:571,1370` ·
  `src/oracle.rs:14,17,504` · `src/composite_equality.rs:568,799` ·
  `src/kani.rs:357` · `src/kani_obligations.rs:292,454,828` (the only
  `negotiate_*` in `src/`) · `src/kani_execution.rs:454` ·
  `src/bounded_kani_replay.rs:11,57-60` ·
  `tests/bounded_kani_corpus.rs:204-210,376` ·
  `tests/exact_scalar_support/agreement.rs:24` · `assurance/pins.json`.
- **RT:** `src/exact/ieee.rs:819` · `src/exact/division.rs:217` ·
  `conformance/qsl-agreement/Cargo.toml:18` ·
  `src/exact/composite.rs:42` (13 variants), `:135` (12 variants) ·
  `src/exact/outcome.rs:83` (13 variants, against QSL's 15) ·
  `src/observation.rs:26` (5 variants) · `src/lib.rs:55-56` ·
  `src/exact/expression.rs:291,583,742` · `src/exact/mod.rs:6-11` ·
  `conformance/qsl-agreement/tests/tc_191_function_application.rs:54`.
- **QSpec:** `proposals/checked-package-v2/schema.json` (exists) ·
  `spec/objects/protocol/FR-290-protocol-claim-kind.md` (exists) ·
  `spec/assurance/AD-016-semantic-family-extension-path.md:238`.
- **QI:** `src/lib.rs:4` · `Cargo.toml:16-18` (empty `[dependencies]`) ·
  `Makefile` (no `heads` target; tree has no `heads/`).
- **Derived data:** §6.3 pin staleness via `git rev-list --count` (26, 7, 33,
  6, 13, 15, 78, 19, 5: all match) · vendored lag 4d6230e..3a79dce = 144 and
  d227270..3a79dce = 10 · 68 open QSL issues (`gh issue list`) match the §7
  rows one to one · §1 tally 5/1/8 · §6.1 24/2/6 · §5 13/4/1 · §9.2 owner
  tally 13/6/17.
