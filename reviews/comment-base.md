<!-- reviewer repo=agent-ix/quire-spec-language visibility=public quoin=0.28.3 module=spec-artifacts-process@b82e42e893612b4b4ee94127714b4577ee893502 id=SR-13740 method=spec-review/base lang=spec pr=quire-spec-language#1175 model=claude-opus-5-5 run=29FAD705-13ED-41F1-8BE5-6D0653015F44 -->

| FND | Severity | Check | Summary |
| --- | --- | --- | --- |
| FND-001 | medium | soundness | FR-064 says `ix-trace-rs` 'provides it beside `#[trace]` and `#[implements]`', but it does not yet |
| FND-002 | medium | soundness | FR-064's Dependencies section still says 'this requirement builds the attribute and the scan tool that #141 and #86 invoke' |
| FND-003 | medium | untestable-ac | AC-15 cannot be tested as written |
| FND-004 | low | soundness | The scan accepts look-alike markers |
| FND-005 | low | other | The owner's 'no comment form' ruling is not written down |
| FND-006 | low | other | AC-14 names the crate being removed ('no `qsl_attrs` path and no workspace member `qsl-attrs`') |
| FND-007 | low | other | The PR deletes the blank line before `## Expected Results`, so the heading now follows the paragraph 'These commands are the acceptance recipe ...' directly (TC-162 lines 147-149) |

SR file: reviews/qsl-951-pr1175-spec-review-base.md (SR-13740). Verdict: Not mergeable as it stands. FND-001 states as fact an ix-trace-rs attribute that does not exist yet and records no dependency on it. FND-002 is a contradiction inside FR-064. FND-003 is an AC that cannot be tested as written. FND-004 to FND-007 are low. All are text fixes in this PR. Follow-up for the code half (not a finding of this spec-only diff): README.md line 333 still lists `qsl-attrs`.

+++ [reviewer data]

```yaml
clean: false
scope:
  - {id: "FR-064 owner statement", path: "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md", role: examined, excerpt: "choose behaviour. The attribute `#[string_edge]` marks an edge function, and it is owned by the neutral crate `ix-trace-rs` (agent-ix/ix-trace-rs), which provides it beside `#[trace]` and `#[implements]`. QSL SHALL provide the scan that reports every string comparison or string `match` outside a marked function in the selected crates' non-test code, so this rule is enforced by a running check rather than by convention alone. QSL SHALL define no `string_edge` attribute of its own."}
  - {id: "FR-064 marker applicable", path: "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md", role: examined, excerpt: "It SHALL be applicable to a function."}
  - {id: "FR-064 marker no argument", path: "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md", role: examined, excerpt: "It SHALL take no argument; a call with any argument SHALL be a compile error that retains the annotated item, so the diagnostic is not followed by a stack of errors from the missing function."}
  - {id: "FR-064 marker identity", path: "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md", role: examined, excerpt: "It SHALL expand to the annotated item unchanged and carry no runtime behaviour: its presence or absence SHALL NOT change what the marked function compiles to."}
  - {id: "FR-064 marker denotes", path: "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md", role: examined, excerpt: "It SHALL denote the one total conversion at a permitted edge, and it is no exemption of a file or a crate."}
  - {id: "FR-064 marker crate deps", path: "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md", role: examined, excerpt: "`ix-trace-rs` SHALL have no normal dependency, no dependency on QSL, and no dependency on a model, IR or code-generation crate, so a consumer depends on it directly."}
  - {id: "FR-064 import rule", path: "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md", role: examined, excerpt: "QSL and every consumer SHALL import the attribute from `ix-trace-rs` (`use ix_trace_rs::string_edge;`) or write its qualified form `#[ix_trace_rs::string_edge]`. The QSL workspace SHALL hold no definition, re-export or alias of `string_edge`, and SHALL hold no crate whose only purpose is that marker; the scan recognises the attribute by its final path segment `string_edge` and by that name only."}
  - {id: "FR-064 consumer marker", path: "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md", role: examined, excerpt: "The marker is `#[string_edge]`, including its qualified form `#[ix_trace_rs::string_edge]`; consumers that compile marked functions SHALL obtain the `ix-trace-rs` crate by Cargo dependency on its repository (agent-ix/ix-trace-rs). A marker identifies the one total conversion at a permitted edge; it is not a consumer-defined exemption for an entire crate or file."}
  - {id: "FR-064 inputs marker", path: "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md", role: examined, excerpt: "The `#[string_edge]` attribute of `ix-trace-rs` applied to an edge function."}
  - {id: "FR-064 Dependencies", path: "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md", role: examined, excerpt: "agent-ix/quire-contract-ir#141 and agent-ix/quire-contract-codegen#86 run the same scan in their own repositories, over the same rule; this requirement builds the attribute and the scan tool that #141 and #86 invoke, not their own edge inventories."}
  - {id: "FR-064-AC-12", path: "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md", role: examined, excerpt: "FR-064-AC-12 | A fixture function marked `#[ix_trace_rs::string_edge]` and one marked with `#[string_edge]` imported by `use ix_trace_rs::string_edge;` each hold a string comparison that the library, the command and QSL's adapter do not report; an unmarked function in the same fixture is reported. | Test (TC-162)"}
  - {id: "FR-064-AC-13", path: "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md", role: examined, excerpt: "FR-064-AC-13 | A consumer crate that applies `#[string_edge]` from `ix-trace-rs` to a function compiles and runs the function with the behaviour of the unmarked function; `#[string_edge(anything)]` fails to compile with the one error \"`#[string_edge]` takes no arguments\" and no further error from the retained function. | Test (TC-162)"}
  - {id: "FR-064-AC-14", path: "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md", role: examined, excerpt: "FR-064-AC-14 | The QSL workspace holds no `proc_macro_attribute` definition named `string_edge`, no `pub use` or alias of it, no `qsl_attrs` path and no workspace member `qsl-attrs`; every `string_edge` marker in the workspace's production code resolves to `ix_trace_rs::string_edge`. | Test (TC-162)"}
  - {id: "FR-064-AC-15", path: "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md", role: examined, excerpt: "FR-064-AC-15 | `cargo metadata` over the QSL workspace lists `ix-trace-rs` as the provider of `string_edge`, and `ix-trace-rs`'s own manifest lists no normal dependency and no QSL, model, IR or code-generation package; the layer table of ADR-011 and ADR-012 §9 names `ix-trace-rs` as the exempt marker crate. | Test (TC-162)"}
  - {id: "ADR-012 section 9 marker", path: "spec/decisions/ADR-012-semantic-family-extension-contracts.md", role: examined, excerpt: "edge function carries the marker attribute `#[string_edge]`, a tool attribute owned by the neutral crate `ix-trace-rs` (agent-ix/ix-trace-rs) and imported from it; QSL defines no marker of its own. An `xtask string-edge` scan over the QSL crates reports every string comparison or string `match` outside a marked function in non-test code, and runs in the lint gate. Each allow-list entry names the user value it compares and selects no behaviour. #214 builds the scan; the attribute is `ix-trace-rs`'s."}
  - {id: "ADR-012 work table #214", path: "spec/decisions/ADR-012-semantic-family-extension-contracts.md", role: examined, excerpt: "`FamilyKind`, `FamilyContract`, `ReferenceEvaluation`, seams S1–S4, the seam probe, the thin check seam for function application, the `xtask string-edge` scan (its `#[string_edge]` attribute is `ix-trace-rs`'s) | #214"}
  - {id: "ADR-011 X-10", path: "spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md", role: examined, excerpt: "X-10 | Extract crate `qsl-replay`. **Extracted**: `bounds`, `call_site`, `execute`, `identity`, `proof_result`, `request`, `result`, `spine` and `witness` are in the crate `qsl-replay`. Its `[dependencies]` are exactly `qsl-cst` (1), `qsl-forms` (2), `qsl-semantics` (3), `qsl-package` (4), `qsl-eval` (5), `qsl-foundation` (F), `quire-semantic-value` (SV: the `InputRefusal` and `Location` the spine reports), `quire-exact` (K), the marker crate `ix-trace-rs` (ADR-012 section 9) and `thiserror`, and `qsl-source` (I3) under its feature `quire-extraction`; the facade re-exports items of `qsl-semant"}
  - {id: "TC-162 scope", path: "spec/test-cases/TC-162-string-edge-scan-coverage.md", role: examined, excerpt: "source locations. Scope: FR-064-AC-1 through FR-064-AC-15."}
  - {id: "TC-162 consumer toml", path: "spec/test-cases/TC-162-string-edge-scan-coverage.md", role: examined, excerpt: "A consumer compiling marked intake functions uses the marker crate by reference as well:  ```toml [dependencies] ix-trace-rs = { git = \"https://github.com/agent-ix/ix-trace-rs\", branch = \"main\" } ```  These commands are the acceptance recipe, not a claim that the standalone package has already been implemented or the installation run has passed."}
findings:
  - fnd: FND-001
    severity: medium
    confidence: high
    method: spec-review/base
    check: soundness
    artifact_id: "FR-064 owner statement"
    related_ids: ["FR-064 Dependencies", "ADR-012 section 9 marker"]
    path: "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md"
    lines: "22-27"
    excerpt: "choose behaviour. The attribute `#[string_edge]` marks an edge function, and it is owned by the neutral crate `ix-trace-rs` (agent-ix/ix-trace-rs), which provides it beside `#[trace]` and `#[implements]`. QSL SHALL provide the scan that reports every string comparison or string `match` outside a marked function in the selected crates' non-test code, so this rule is enforced by a running check rather than by convention alone. QSL SHALL define no `string_edge` attribute of its own."
    finding: "FR-064 says `ix-trace-rs` 'provides it beside `#[trace]` and `#[implements]`', but it does not yet. On agent-ix/ix-trace-rs main (latest tag v0.1.1), src/lib.rs defines only `trace` and `implements`. No branch, PR or Linear ticket adds `string_edge`. The IR proposal (IR-212 comment 855ebf10) says outright that it is 'a proposal to its owner, not a claim the attribute exists today'. The PR never states that QSL depends on ix-trace-rs shipping the attribute, so a coder reading FR-064 would expect `use ix_trace_rs::string_edge;` to compile now. Add an FR-064 Dependencies entry: QSL needs ix-trace-rs to provide `#[string_edge]` with the contract in 'The marker attribute', tracked by a named ix-trace-rs ticket that must land before the code half. Then word the description as the contract ('ix-trace-rs provides ...') without implying it is already published."
  - fnd: FND-002
    severity: medium
    confidence: high
    method: spec-review/base
    check: soundness
    artifact_id: "FR-064 Dependencies"
    related_ids: ["FR-064 owner statement"]
    path: "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md"
    lines: "251-254"
    excerpt: "agent-ix/quire-contract-ir#141 and agent-ix/quire-contract-codegen#86 run the same scan in their own repositories, over the same rule; this requirement builds the attribute and the scan tool that #141 and #86 invoke, not their own edge inventories."
    finding: "FR-064's Dependencies section still says 'this requirement builds the attribute and the scan tool that #141 and #86 invoke'. That contradicts the new statement 'QSL SHALL define no `string_edge` attribute of its own' in the same FR. Change it to say that this requirement builds the scan tool, and that the attribute is ix-trace-rs's."
  - fnd: FND-003
    severity: medium
    confidence: high
    method: spec-review/base
    check: untestable-ac
    artifact_id: "FR-064-AC-15"
    related_ids: ["TC-162 step 16", "TC-162 expected 16", "ADR-011 crate map", "ADR-011 X-10"]
    path: "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md"
    lines: "242"
    excerpt: "FR-064-AC-15 | `cargo metadata` over the QSL workspace lists `ix-trace-rs` as the provider of `string_edge`, and `ix-trace-rs`'s own manifest lists no normal dependency and no QSL, model, IR or code-generation package; the layer table of ADR-011 and ADR-012 §9 names `ix-trace-rs` as the exempt marker crate. | Test (TC-162)"
    finding: "AC-15 cannot be tested as written. (1) `cargo metadata` lists packages, targets and dependency edges. It does not say which package provides an attribute named `string_edge`, so 'lists `ix-trace-rs` as the provider of `string_edge`' is nothing metadata can show. (2) The PR's own ADR text contradicts the clause 'the layer table of ADR-011 and ADR-012 section 9 names `ix-trace-rs` as the exempt marker crate'. The ADR-011 layer table and crate map name neither `ix-trace-rs` nor `qsl-attrs`. Only the X-10 row lists it among qsl-replay's dependencies, and no ADR uses 'exempt'. (3) Checking ADR prose is Inspection, not Test. Restate AC-15 in terms metadata can show: every QSL package whose production code carries the marker names `ix-trace-rs` as a normal dependency, and ix-trace-rs's `[dependencies]` table is empty. Then either drop the ADR clause or add the 'exempt marker crate' text to ADR-011's layer rules."
  - fnd: FND-004
    severity: low
    confidence: medium
    method: spec-review/base
    check: soundness
    artifact_id: "FR-064 import rule"
    related_ids: ["FR-064 consumer marker", "FR-064-AC-14"]
    path: "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md"
    lines: "76-81"
    excerpt: "QSL and every consumer SHALL import the attribute from `ix-trace-rs` (`use ix_trace_rs::string_edge;`) or write its qualified form `#[ix_trace_rs::string_edge]`. The QSL workspace SHALL hold no definition, re-export or alias of `string_edge`, and SHALL hold no crate whose only purpose is that marker; the scan recognises the attribute by its final path segment `string_edge` and by that name only."
    finding: "The scan accepts look-alike markers. 'the scan recognises the attribute by its final path segment `string_edge` and by that name only', and the detector does exactly that (qsl-string-edge/src/detector.rs has_attr_named, `segments.last()`). So `#[my_macros::string_edge]`, or a consumer's own local `string_edge` proc macro, suppresses a report just as ix-trace-rs's does. Inside QSL, AC-14 closes the gap, because no other definition may exist and the code must compile. In IR, CG and driver, nothing checks the 'every consumer SHALL import ... from ix-trace-rs' rule. A consumer could keep a copied marker, the copy the owner ruled out, and the scan would not notice. The marker is self-attested anyway, since the real attribute can go on any function, so this adds no new way past the scan. It is a low finding. Either have the scanner accept only the paths `string_edge` and `ix_trace_rs::string_edge` and report any other qualified `string_edge` path, or state the limitation in FR-064 beside the detector's other documented limitations."
  - fnd: FND-005
    severity: low
    confidence: medium
    method: spec-review/base
    check: other
    artifact_id: "FR-064 import rule"
    related_ids: ["FR-064 inputs marker"]
    path: "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md"
    lines: "76-81"
    excerpt: "QSL and every consumer SHALL import the attribute from `ix-trace-rs` (`use ix_trace_rs::string_edge;`) or write its qualified form `#[ix_trace_rs::string_edge]`. The QSL workspace SHALL hold no definition, re-export or alias of `string_edge`, and SHALL hold no crate whose only purpose is that marker; the scan recognises the attribute by its final path segment `string_edge` and by that name only."
    finding: "The owner's 'no comment form' ruling is not written down. FR-064 recognises only an attribute, by its final path segment. It never says that a comment marker (`// string_edge`, `/// string_edge`) or a doc tag is not a marker, while ADR-032 shows QSL uses doc-comment tags (`/// quire:canonical`) for a similar purpose. Add one sentence: the marker is the attribute only, and the scan recognises no comment or doc-comment form."
  - fnd: FND-006
    severity: low
    confidence: medium
    method: spec-review/base
    check: other
    artifact_id: "FR-064-AC-14"
    related_ids: ["FR-064 import rule"]
    path: "spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md"
    lines: "241"
    excerpt: "FR-064-AC-14 | The QSL workspace holds no `proc_macro_attribute` definition named `string_edge`, no `pub use` or alias of it, no `qsl_attrs` path and no workspace member `qsl-attrs`; every `string_edge` marker in the workspace's production code resolves to `ix_trace_rs::string_edge`. | Test (TC-162)"
    finding: "AC-14 names the crate being removed ('no `qsl_attrs` path and no workspace member `qsl-attrs`'). That is a migration check, not a property of the system. Once the code half lands, a future reader sees a ban on a name that exists nowhere. The general clauses ('no `proc_macro_attribute` definition named `string_edge`, no `pub use` or alias', with FR-064's 'no crate whose only purpose is that marker') already cover it. Keep the qsl-attrs check in the implementation ticket's acceptance and leave it out of the AC, or keep it with the general rule first."
  - fnd: FND-007
    severity: low
    confidence: high
    method: spec-review/base
    check: other
    artifact_id: "TC-162 consumer toml"
    related_ids: []
    path: "spec/test-cases/TC-162-string-edge-scan-coverage.md"
    lines: "139-148"
    excerpt: "A consumer compiling marked intake functions uses the marker crate by reference as well:  ```toml [dependencies] ix-trace-rs = { git = \"https://github.com/agent-ix/ix-trace-rs\", branch = \"main\" } ```  These commands are the acceptance recipe, not a claim that the standalone package has already been implemented or the installation run has passed."
    finding: "The PR deletes the blank line before `## Expected Results`, so the heading now follows the paragraph 'These commands are the acceptance recipe ...' directly (TC-162 lines 147-149). It still parses (`make check-spec-validation` passes) but no other heading in the file is laid out this way. Restore the blank line."
bindings: []
```

+++
