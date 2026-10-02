---
id: SR-1085
title: "Spec review of PR #580 (checked-input and canonical-types drift gates, ADR-032, FR-270 to FR-273)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@dcf73683dbb09cd48a60f820480a45b8d9626d5f; spec/decisions/ADR-032-checked-input-and-duplicate-canonical-type-gates.md, spec/functional/FR-270 to FR-273, spec/usecase/US-028, spec/test-cases/TC-745 to TC-748, spec/spec.md, spec/tests.md"
review_set: subset
---

## Summary

Ticket: QSL-391. Review of `git diff origin/main...HEAD` at dcf73683
against the four owner rulings (R-1 on demand until no namesakes remain,
then `make ci`; R-2 delete or rename each namesake by meaning, with the
verdict for each type listed in the spec; R-3 a doc-comment tag, not a
dependency or a reserved-name list; R-4 no QSpec half) and the wave-B/wave-C
owner rulings.

FR-273's verdict table was checked against quire-spec-language origin/main
(9647e5a8). Every namesake path in the table exists on main. Seven rows were
checked for meaning by reading the definitions. Four are right:
`qsl_semantics::model::accounting::LimitKind` (a `ModelNormalizationLimits`
counter, rename), root `command::LimitKind` (a command intake ceiling,
rename), `qsl_replay::identity::RawSourceRef` (a digest in any FR-201 domain,
rename), and the two `QualifiedName`s in `qsl-eval` and `qsl-replay` (the
same O-11 meaning, delete both and keep one). Three are wrong (FND-002). The
table is also incomplete (FND-001, FND-003).

What is right: the canonical set comes from the `/// quire:canonical` tag
alone, with no name list and no new dependency (R-3). There is no QSpec half
(R-4). The checked-input gate's stage-entry and pre-check sets come from
crate and module membership. Findings name the file and line. There are no
caps, pins or compat paths. Every AC has a behaviour TC (TC-745 to TC-748).
`quire validate` on the changed files exits 0, and
`tools/check-index-completeness.sh` passes.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-273's table gives no verdict for the `re-export` findings DT-2 reports on main. `qsl-replay/src/lib.rs` has `pub use` of canonical types from crates other than their owners: `OccurrenceKey` (owner `qsl-foundation`, line 71), `Origin` and `Integer` (owner `quire-exact`, lines 88 and 101) and `DeclarationKey` (owner `qsl-semantics`, line 120). With every table row carried out, `canonical-types` still reports these, so FR-273-AC-1 and TC-748 cannot pass. | spec/functional/FR-273-resolve-each-canonical-type-namesake.md:50-99, :105 |
| FND-002 | high | Three `delete` verdicts are on namesakes whose meaning differs from the canonical type, so R-2 makes them renames. Root `runtime::evaluation::value::Value<'a>` is a borrowed arena view over IR value nodes, with `i64` integers, `&str` text and `quire_contract_model` references; it is not the kernel `Value` with its unbounded `Integer`. Root `state::input::Value` carries a nominal wire type index (`value_type: u32`) beside its kind. Root `checking::CheckedPackage<'a>` holds native-v1 checked clauses that keep the linked AST; it is not the v2 `qsl_package::CheckedPackage`. The table itself labels two of them "lane-private, ADR-013 §6", and ADR-013 §6 keeps lane-private types apart from the canonical ones. | spec/functional/FR-273-resolve-each-canonical-type-namesake.md:67, :68, :94 |
| FND-003 | medium | The table misses a namesake on main. `qsl_semantics::check::termination::Member` (`pub(crate)`, "what termination needs of one checked function") shares the identifier of the O-06 member identity `qsl_semantics::value::member::Member`. DT-2 reports it whatever its visibility, so FR-273-AC-1 fails until it has a verdict (rename). | spec/functional/FR-273-resolve-each-canonical-type-namesake.md:50-99 |
| FND-004 | medium | DT-4 and FR-272 scan "every workspace member's `src/`". `tools/arch-lint` is a workspace member whose sources sit in `tools/arch-lint/` (`[[bin]] path = "main.rs"`), so its shipped code is never scanned. Take each member's target source roots from `cargo metadata` instead. Once it is scanned, `tools/arch-lint/canonical_encoder.rs:290` `pub(crate) struct Outcome` is a namesake of `quire_exact::Outcome` with no verdict in FR-273. | spec/functional/FR-272-fail-on-a-second-definition-of-a-canonical-type.md:28-31; spec/decisions/ADR-032-checked-input-and-duplicate-canonical-type-gates.md:128 |
| FND-005 | medium | FR-272's Description says `canonical-types` "is a prerequisite of `make ci`; ADR-032 R-1 records when it joins". R-1 says it runs on demand until no namesakes remain and joins `make ci` after that. The sentence states a joining that has not happened as a present fact, and puts a schedule pointer inside an FR. State the gate's behaviour and leave the `make ci` membership to FR-273-AC-1. | spec/functional/FR-272-fail-on-a-second-definition-of-a-canonical-type.md:23-24 |
| FND-006 | low | The subject of FR-273's Behavior is "The implementing change SHALL delete/point/rename". That is a one-time work instruction, not a property of the system, and it is not EARS. State the property that holds afterwards: each canonical identifier has one module-level definition in the workspace, and each table row's item carries the name the row gives. | spec/functional/FR-273-resolve-each-canonical-type-namesake.md:42-47 |
| FND-007 | low | ADR-032 Status still reads "Proposed" although all four questions have recorded owner rulings folded into the rules. | spec/decisions/ADR-032-checked-input-and-duplicate-canonical-type-gates.md:29 |

## Verdict

Not mergeable as it stands. FND-001 and FND-002 mean the verdict table, the
deliverable R-2 asks for, is incomplete and wrong in three rows, so
FR-273-AC-1 cannot pass as written. FND-003 to FND-005 are real gaps. The
checked-input gate (FR-270) and the tag and gate rules (FR-271, FR-272) are
sound apart from FND-004 and FND-005.
