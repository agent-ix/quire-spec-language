---
id: SR-13740
title: 'Spec review of PR #1175: ix-trace-rs owns the string_edge marker, base checklist'
type: SpecReview
analysis: base
scope: 'agent-ix/quire-spec-language@76a302455; git diff origin/main...76a302455 (PR #1175, branch qsl-951-trace-marker): ADR-011 X-10, ADR-012 section 9 and work table, FR-064, TC-162; read against agent-ix/ix-trace-rs main (Cargo.toml, README.md, src/lib.rs, tags v0.1.0/v0.1.1), the QSL workspace manifests and qsl-string-edge/src/detector.rs at origin/main, IR FR-028 on quire-contract-ir main, and the proposal in IR-212 comment 855ebf10 and IR-9 comment a884990a; ticket QSL-951'
review_set: subset
date: '2026-10-22'
relationships:
- target: ix://agent-ix/quire-spec-language/ADR-011
  type: reviews
- target: ix://agent-ix/quire-spec-language/ADR-012
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-064
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-162
  type: reviews
---

## Summary

Ticket: QSL-951. PR quire-spec-language#1175, head 76a302455, spec-only (ADR-011, ADR-012, FR-064, TC-162). Base checklist: faithfulness to the owner decision and the IR proposal, the actual ix-trace-rs API, AC testability and id uniqueness, stale statements elsewhere in spec/, wording, and gates.

Faithful to the proposal: ix-trace-rs owns `string_edge`. The marker takes no argument, expands to the item unchanged, has no runtime behaviour, and an argument is a compile error that keeps the item. It is never a file or crate exemption. QSL keeps the scanner, holds no QSL definition, re-export or alias, and has no comment form or compatibility layer. No FR-028 exception is needed: IR FR-028-AC-3 forbids any IR root-package dependency on a crate from the agent-ix/quire-spec-language repository, and ix-trace-rs lives in its own repository. AC ids FR-064-AC-12 to AC-15 are unique, and TC-162's scope now reads AC-1 through AC-15. No file in spec/ outside reviews/ still says the marker lives in `qsl-attrs` or in a QSL crate, apart from ADR-032 R-3 (see the integrity review). The `branch = "main"` git dependency in TC-162 matches the form all 15 QSL manifests already use for ix-trace-rs, so it adds no second pin source. `make check-spec-validation` exits 0 with no diagnostic on any touched file. `tools/check-index-completeness.sh` exits 0. No test or tool reads FR-064, TC-162, ADR-011 or ADR-012 by file name, so no test-parsed table changes. The only `include_str!`/`read_to_string` spec reads are FR-092 and FR-030 to FR-095 in linking.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-064 owner statement | spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md | examined |
| FR-064 marker applicable | spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md | examined |
| FR-064 marker no argument | spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md | examined |
| FR-064 marker identity | spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md | examined |
| FR-064 marker denotes | spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md | examined |
| FR-064 marker crate deps | spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md | examined |
| FR-064 import rule | spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md | examined |
| FR-064 consumer marker | spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md | examined |
| FR-064 inputs marker | spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md | examined |
| FR-064 Dependencies | spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md | examined |
| FR-064-AC-12 | spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md | examined |
| FR-064-AC-13 | spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md | examined |
| FR-064-AC-14 | spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md | examined |
| FR-064-AC-15 | spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md | examined |
| ADR-012 section 9 marker | spec/decisions/ADR-012-semantic-family-extension-contracts.md | examined |
| ADR-012 work table #214 | spec/decisions/ADR-012-semantic-family-extension-contracts.md | examined |
| ADR-011 X-10 | spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md | examined |
| TC-162 scope | spec/test-cases/TC-162-string-edge-scan-coverage.md | examined |
| TC-162 consumer toml | spec/test-cases/TC-162-string-edge-scan-coverage.md | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-064 says `ix-trace-rs` 'provides it beside `#[trace]` and `#[implements]`', but it does not yet. On agent-ix/ix-trace-rs main (latest tag v0.1.1), src/lib.rs defines only `trace` and `implements`. No branch, PR or Linear ticket adds `string_edge`. The IR proposal (IR-212 comment 855ebf10) says outright that it is 'a proposal to its owner, not a claim the attribute exists today'. The PR never states that QSL depends on ix-trace-rs shipping the attribute, so a coder reading FR-064 would expect `use ix_trace_rs::string_edge;` to compile now. Add an FR-064 Dependencies entry: QSL needs ix-trace-rs to provide `#[string_edge]` with the contract in 'The marker attribute', tracked by a named ix-trace-rs ticket that must land before the code half. Then word the description as the contract ('ix-trace-rs provides ...') without implying it is already published. | spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md:22-27; spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md:251-254; spec/decisions/ADR-012-semantic-family-extension-contracts.md:854-860 |
| FND-002 | medium | FR-064's Dependencies section still says 'this requirement builds the attribute and the scan tool that #141 and #86 invoke'. That contradicts the new statement 'QSL SHALL define no `string_edge` attribute of its own' in the same FR. Change it to say that this requirement builds the scan tool, and that the attribute is ix-trace-rs's. | spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md:251-254; spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md:22-27 |
| FND-003 | medium | AC-15 cannot be tested as written. (1) `cargo metadata` lists packages, targets and dependency edges. It does not say which package provides an attribute named `string_edge`, so 'lists `ix-trace-rs` as the provider of `string_edge`' is nothing metadata can show. (2) The PR's own ADR text contradicts the clause 'the layer table of ADR-011 and ADR-012 section 9 names `ix-trace-rs` as the exempt marker crate'. The ADR-011 layer table and crate map name neither `ix-trace-rs` nor `qsl-attrs`. Only the X-10 row lists it among qsl-replay's dependencies, and no ADR uses 'exempt'. (3) Checking ADR prose is Inspection, not Test. Restate AC-15 in terms metadata can show: every QSL package whose production code carries the marker names `ix-trace-rs` as a normal dependency, and ix-trace-rs's `[dependencies]` table is empty. Then either drop the ADR clause or add the 'exempt marker crate' text to ADR-011's layer rules. | spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md:242; spec/test-cases/TC-162-string-edge-scan-coverage.md:93-94; spec/test-cases/TC-162-string-edge-scan-coverage.md:186-188; spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md:741-746; spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md:1152 |
| FND-004 | low | The scan accepts look-alike markers. 'the scan recognises the attribute by its final path segment `string_edge` and by that name only', and the detector does exactly that (qsl-string-edge/src/detector.rs has_attr_named, `segments.last()`). So `#[my_macros::string_edge]`, or a consumer's own local `string_edge` proc macro, suppresses a report just as ix-trace-rs's does. Inside QSL, AC-14 closes the gap, because no other definition may exist and the code must compile. In IR, CG and driver, nothing checks the 'every consumer SHALL import ... from ix-trace-rs' rule. A consumer could keep a copied marker, the copy the owner ruled out, and the scan would not notice. The marker is self-attested anyway, since the real attribute can go on any function, so this adds no new way past the scan. It is a low finding. Either have the scanner accept only the paths `string_edge` and `ix_trace_rs::string_edge` and report any other qualified `string_edge` path, or state the limitation in FR-064 beside the detector's other documented limitations. | spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md:76-81; spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md:184-189; spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md:241 |
| FND-005 | low | The owner's 'no comment form' ruling is not written down. FR-064 recognises only an attribute, by its final path segment. It never says that a comment marker (`// string_edge`, `/// string_edge`) or a doc tag is not a marker, while ADR-032 shows QSL uses doc-comment tags (`/// quire:canonical`) for a similar purpose. Add one sentence: the marker is the attribute only, and the scan recognises no comment or doc-comment form. | spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md:76-81; spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md:42 |
| FND-006 | low | AC-14 names the crate being removed ('no `qsl_attrs` path and no workspace member `qsl-attrs`'). That is a migration check, not a property of the system. Once the code half lands, a future reader sees a ban on a name that exists nowhere. The general clauses ('no `proc_macro_attribute` definition named `string_edge`, no `pub use` or alias', with FR-064's 'no crate whose only purpose is that marker') already cover it. Keep the qsl-attrs check in the implementation ticket's acceptance and leave it out of the AC, or keep it with the general rule first. | spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md:241; spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md:76-81 |
| FND-007 | low | The PR deletes the blank line before `## Expected Results`, so the heading now follows the paragraph 'These commands are the acceptance recipe ...' directly (TC-162 lines 147-149). It still parses (`make check-spec-validation` passes) but no other heading in the file is laid out this way. Restore the blank line. | spec/test-cases/TC-162-string-edge-scan-coverage.md:139-148 |

## Verdict

Not mergeable as it stands. FND-001 states as fact an ix-trace-rs attribute that does not exist yet and records no dependency on it. FND-002 is a contradiction inside FR-064. FND-003 is an AC that cannot be tested as written. FND-004 to FND-007 are low. All are text fixes in this PR. Follow-up for the code half (not a finding of this spec-only diff): README.md line 333 still lists `qsl-attrs`.
