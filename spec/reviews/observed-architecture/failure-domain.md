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

Reviewed ADR-010 (ARCH-00 observed architecture baseline, #206) and its
`spec/spec.md` index row at quire-spec-language commit faa1731. The failure-domain
checklist was adapted to a descriptive record. It covers three things: trust
boundaries at the observed handoffs, identity confusion between duplicate types
and digest domains, and unstated failure modes in the cross-repository handoffs.
It also checks whether the record's own claims could mislead a Layer 1 owner
(#209, #210, #211). Evidence was spot-checked at QSL de627b5, IR 553b6d1,
CG a4b2a73 and QSpec 3a79dce.

Most spot-checked citations are accurate. The record keeps intended design apart
from observed code. Two omissions are blocking. First, the wire-admission bypass
also feeds the cross-repository checked-predicate and temporal-subject handoffs
to IR, and the record does not say so. Second, one identity domain
(`EffectiveId`) is missing from the ownership table, and QSL code converts it
into the semantic-node domain. Both fixes add rows or cells and change no
conclusion elsewhere.

Verdict: REJECT. Two high findings (FND-001, FND-002) block acceptance at the
#208 gate. Once they are fixed and the medium findings are dispositioned, the
expected verdict is ACCEPT WITH FINDINGS.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Trust boundary missing at a cross-repository handoff. OBS-015 and §4.4 record `protocol_artifact::read` admitting untrusted wire data only into the `state` and `temporal` evaluators. That same wire-admitted `v2::AdmittedPackage` is also the only input to `checked_predicate::derive`/`read` and `temporal_subject::derive`/`read`. Their outputs (`ValidatedCheckedPredicate` and the temporal subject) are the inputs of IR `predicate::project` and `temporal::project`, which IR documents as "constructor-private QSL checked leaves". A Layer 1 owner reading §2.2 and §6.2 would think "checked" here means compiler-checked, but these values have no source-compile provenance. This fails the #206 criterion that bypasses be explicit. Fix: extend OBS-015 (or add OBS-037, owner #209, secondary #211) and add a §4.4 row: "wire-admitted `AdmittedPackage` → checked-predicate / temporal-subject → IR projection; skips source compile and check". Cite the evidence in Refs. | `QSL:protocol_artifact/mod.rs:1-8`; `QSL:protocol_artifact/v2/intake.rs:476`; `QSL:protocol_artifact/checked_predicate.rs:139,156`; `QSL:protocol_artifact/temporal_subject.rs:176,192`; `IR:src/predicate/admission.rs:91-92`; ADR-010 §2.2, §4.4, §6.2, OBS-015 |
| FND-002 | high | Identity domain missing from the ownership table, and the cross-domain conversion is misdescribed. `model::key::EffectiveId` (domain `quire.model.effective-declaration/v1`, `QSL:model/key.rs:26,157`) is a shared identity type, but it appears in neither §5 nor DA-01/DA-02. The only production call of `NodeKey::from_bytes` is `QSL:value/model_query.rs:108`. There, the 32 bytes of `ReferenceKey.type_identity: EffectiveId` (`QSL:model/population.rs:376`) are reused as a `NodeKey`, whose domain is `quire.checked-semantic-node/v1`. §4.2 and §4.4 instead say "any digest → NodeKey" and give no call site. They cite `QSL:value/node.rs:22`, which is the struct declaration, not the function (the function is at :48). They also omit that `from_bytes` is `pub(crate)`. As a result, the record implies a public bypass that does not exist and hides the cross-domain conversion that does. Fix: add `EffectiveId` (with `ReferenceKey` and `ObjectReference`) to DA-02, or as a new DA row owned by #211. Rewrite the §4.2 and §4.4 rows as "`EffectiveId` → `NodeKey` byte transfer, `pub(crate)`, `QSL:value/model_query.rs:108`, `QSL:value/node.rs:48`". Add the conversion to OBS-018. | `QSL:value/node.rs:18,22,48`; `QSL:value/model_query.rs:108`; `QSL:model/key.rs:26,157,166`; `QSL:model/population.rs:376`; ADR-010 §4.2, §4.4, §5 DA-02, OBS-018 |
| FND-003 | medium | The typed in-memory handoffs from IR to QSL are tied to QSL f1700a9, but the record states that only as a staleness count. IR main's `predicate::project`, `temporal::project` and `replay_with_native_runtime(&NativePackage)` take QSL Rust types from the pinned QSL f1700a9 (`IR:Cargo.toml:24`). A value produced by QSL main (de627b5) is a different type from a different crate, so QSL main cannot pass it to IR main without serializing it. §6.2 names these input types without a revision, and OBS-029 describes the edge only as "78 behind". A Layer 1 owner could therefore read these as live handoffs from QSL main. Fix: tag the §6.2 inputs with "QSL f1700a9 types". In OBS-029, state that the typed handoffs in this edge cannot be reached from QSL main. | `IR:Cargo.toml:24`; `IR:src/kani/replay.rs:3-6,80`; `IR:src/predicate/admission.rs:91-92`; ADR-010 §6.2, OBS-029 |
| FND-004 | medium | The record does not state how replay fails at the handoff. IR `validate_packet` checks only that `witness` is non-empty. Neither `replay_counterexample` nor `replay_with_native_runtime` passes the witness to the executor. The executor gets `FiniteInput` or the caller's `reconstruct`. Agreement is accepted when `kind == Counterexample` or `truth() == Some(false)`, and a native result that disagrees is reported as `Inconclusive`. OBS-027 describes the witness only as a type-shape problem (`String` vs `Option<Witness>`). #211 could then decide on a typed witness while assuming replay already reads it. Fix: add to OBS-027 (owner #211) and OBS-028 (owner #209) that replay never reads the witness content. Also record that a disagreement is reported as `Inconclusive` with code `kani_native_replay_disagreement`. | `IR:src/kani/replay.rs:39-51,55-71,80-97`; ADR-010 OBS-027, OBS-028 |
| FND-005 | medium | The IT-010 handoff (A9–A11) misstates its failure modes and leaves out its only version guard. The A9 "none typed" cell hides that each consumer step calls `.unwrap()` and panics (`:543,545,597,871`). `playback_i64` reads only the first `vec![…]` row and needs exactly 8 bytes (`:843-857`), so a harness with more than one nondeterministic value is silently cut to the first value. `native_verdict` returns `None` outside `DOMAIN` (`:261`). The projection's format constant comes from IR model 53cc03c, but the consumer is IR 04eb6f8, which is also the IR that CG 5e2a6a9 pins. The only check that the two revisions agree is the digest-equality assertion at `:545`, and it appears at one of the three consumer sites. Fix: change the Refusal cells of A9 and A10 to "test panic (`unwrap`)" and "first row only; `None`". Add the IT-010 revision chain (IR model 53cc03c → IR 04eb6f8 → CG 5e2a6a9 → RT 8a4d02b) to §3.2 and DA-14, with `:545` as the single cross-revision guard. | `QSL:tests/configversion_backends.rs:32-33,261,543-545,597,843-857,871`; `CG@5e2a6a9:Cargo.toml:17,26`; ADR-010 §2.1 A9–A11, §3.2, DA-14 |
| FND-006 | medium | Wrong evidence path for the checked-handoff rows. The §2.2 table "Checked handoffs inside lane B" cites derive and read at `QSL:protocol_artifact/checked_handoff.rs:139,156,177,193`. That is a private module (`QSL:protocol_artifact/mod.rs:12`), and at those lines are `exhausted()`, a `Usage` field and `Report` accessors. The public entries are in `checked_predicate.rs:139,156` and `temporal_subject.rs:176,192`, with the private implementations at `checked_handoff.rs:1687,1753`. Because these rows are a handoff, #206's evidence-link criterion is not met for them. Fix: correct the four citations. The format rows at `:1585-1586` are correct. | `QSL:protocol_artifact/checked_predicate.rs:139,156`; `QSL:protocol_artifact/temporal_subject.rs:176,192`; `QSL:protocol_artifact/checked_handoff.rs:1687,1753`; ADR-010 §2.2 |
| FND-007 | medium | Absence claims cannot be re-checked. The Evidence convention requires `<prefix>:<path>:<line>`. Absence claims instead cite a file with no line and no search method, so a Layer 1 owner cannot confirm the absence at the pinned revision. Examples: X1, X5 and X7; §1 rows 2–3; OBS-005, OBS-026 and OBS-031 (`QSL:Cargo.toml`, `QSL:wire_format.rs`, `QSL:model/` tree, `QI:Makefile`). Some presence claims also lack a line: OBS-014 `QSL:temporal.rs`, DA-02 `QSL:model/checked_dispatch.rs` (the map is at `:657`), DA-05 `QSL:checking/types.rs`, DA-15 `QSL:model/key.rs`, and the §1 Kani sha row (`QSL:tests/configversion_backends.rs:34`). Fix: add a negative-evidence form to the Evidence convention, for example "`git grep -n '<pattern>' <sha> -- <path>` returns nothing", and use it in every ABSENT cell. Add line numbers to the presence cells listed. | ADR-010 §Evidence convention, §1, §2.6, §3.3, OBS-005, OBS-014, OBS-026, OBS-031, DA-02, DA-05, DA-15 |
| FND-008 | low | A possible identity collision in PR #200 is not recorded. PR #200 maps `ix://quire/native/<Name>` to `DeclarationKey{package:"quire/native", …}`. This is a pseudo-package inside the same key space as real domain packages. The diff reserves no `quire/native` package identity, and `read_field_type_ref` matches the native prefix before it resolves a package's own types. OBS-006 records only that the PR disagrees with AD-016. Fix: add the collision to OBS-006 as an observed PR failure mode, owner #211. | PR #200 `src/model/intake.rs` (`read_field_type_ref`); ADR-010 OBS-006, §8 |
| FND-009 | low | §8 contradicts itself on merge order. The table's Notes column says #228 "merges first" and #204 "merges second". The paragraph under it sets #204 → #228 → #200 and says the coordinator's order governs. A reader of the table alone gets the wrong order for the single-writer files. Fix: make the Notes column match the governing order, or remove the ordinal notes from the table. | ADR-010 §8 |

## Method

- Checklist, adapted to a descriptive record. The four checks were:
  - Extension points and trust boundaries: at each observed handoff, where does
    admitted data come from, and what does its type name claim?
  - Entity identity: which uniqueness key and digest domain each identity type
    uses, and where bytes move between domains.
  - Evaluation purity: whether caller-supplied closures in replay are recorded
    as what decides the verdict.
  - Topology: dependency cycles and version skew along the IT-010 chain.
- Spot checks confirmed as accurate: `QSL:linking.rs:69`, `QSL:model/key.rs:72`,
  `QSL:value/node.rs:18`, `QSL:value/package_identity.rs:13,15,334`,
  `QSL:checking.rs:263`, `QSL:value/expression/mod.rs:71,635`,
  `QSL:diagnostic.rs:306-324`, `QSL:protocol_artifact/mod.rs:1-8`,
  `QSL:package/view.rs:39-46`, `QSL:lowering/wire.rs:51`, `QSL:Cargo.toml:36,43,44`,
  `QSL:tests/configversion_backends.rs:32-33,252,259,843-857,946`,
  `QSL:linking/composed/definition_source.rs:240`, `QSL:wire_format.rs` (/1 only),
  `QSL:command/compilation.rs:~95`, `IR:Cargo.toml:24,37,38`,
  `IR:src/kani/replay.rs:13-20,55,80-97`, `CG:Cargo.toml:17,18,28`,
  `CG:src/bounded_kani_replay.rs:11`, `CG:tests/bounded_kani_corpus.rs:204-210,376`,
  `QSpec:spec/assurance/AD-016-semantic-family-extension-path.md:238`.
- Spot checks that failed: `QSL:value/node.rs:22` as the location of `from_bytes`
  (FND-002), and `QSL:protocol_artifact/checked_handoff.rs:139,156,177,193`
  (FND-006).
- Collision check: `linking::DeclarationKey` and `model::key::DeclarationKey` are
  never imported in the same source file at de627b5, so the name clash does not
  cause confusion inside the crate. The crate root re-exports only native-v1
  names. No finding was raised.
- RT and FCD citations were not spot-checked because no local clone at the pinned
  revision was available.
