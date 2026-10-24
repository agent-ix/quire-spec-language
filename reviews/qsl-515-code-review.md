---
id: SR-12063
title: "QSL-515 PR #930 code review: header-selectable layers, requires closure and per-layer admitted forms"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@49ad03552456bed91a194ca204bbf0bc31eeb683; PR #930: qsl-semantics/src/value/layer.rs, qsl-semantics/src/check/layer.rs, qsl-semantics/src/value/definition.rs, qsl-semantics/src/value/mod.rs, qsl-semantics/src/check/assemble.rs, qsl-semantics/src/check/check.rs, qsl-semantics/src/check/mod.rs, qsl-semantics/src/check/profile.rs, qsl-package/src/emit.rs, qsl-replay/src/spine.rs, qsl-replay/src/spine/refinement.rs, qsl-replay/src/spine/clause/tests/layers.rs, qsl-semantics/src/check/assemble/tests.rs, qsl-semantics/tests/it/complete_value_lock.rs"
review_set: subset
---

## Summary

Ticket: QSL-515. PR quire-spec-language#930, diff `origin/main...49ad03552`
(17 files, +1098/-39). Code review with the rust-review lane folded in.
Read-only: no build or test was run. The author's focused checks (EXIT=0 on
64c3bcc16) and the conflict-free main merge are taken as given, per the
brief. QSpec is private and is cited by id only.

Checked hard, per the brief:

1. Classifier. `expression_form` is one `match` over `ExprNode` with no
   wildcard arm. The nested `BinaryOperator` and `BinderQuery` matches are
   exhaustive too. The only `cfg` arm is the `seam_probe` variant. Admission
   is deterministic: a pre-order walk over an explicit stack. Children are
   pushed in reverse so the first child pops first. It stops at the first
   form outside the set. The span comes from `ExpressionSpans::child`, the
   same pairing `check/region.rs` uses, and falls back to the declaration
   span. The refusal carries `layer`, and `assembly_message` names
   `layer.identity()`.
2. Closure. `DefinitionLock::closure` is a worklist with a `reached` set, so
   it terminates even on a cyclic table (the static table is acyclic). It
   returns `BTreeSet` order, which is the enum declaration order and so the
   lock row order. Unit tests hard-code the expected closure for all five
   layers and for a union. `qualification_roles` returns `always` when the
   union holds `ValueComplete`, else `[Edition]`. That matches QSpec's lock
   `package_selection.layer` and `without_layer`, which I checked against the
   local QSpec checkout. A conformance test also asserts it.
3. `AssemblyCause::UnsupportedLayerForm` is `Code::UnsupportedConstruct`
   with cause `declaration-form` or `expression-form`.
   `refinement.rs::assembly_limit` gives `None`. `spine.rs::assembly_message`
   names the form cause and the layer. `classify_refusal` reaches
   `Prohibited` through the default branch: the cause is not
   `not-yet-implemented` and is not a limit or typed code. The test
   `a_layer_refusal_is_prohibited` pins this.
4. Lock rows. `catalog_selections` writes the closure's layers first, then
   the qualification rows, then laws, with one dedup over all of them. The
   `root` row (`quire.value.complete/v1`) dedups against the layer reference,
   since both use authority `agent-ix`. Every source-compiled unit with a
   `value.complete` header now also selects `quire.state.core/v1`, ahead of
   `quire.value.complete/v1`, so its package identity changes. FR-110-AC-1
   requires this. No committed golden holds an emitted
   `definition_selections`: the emit tests build packages with no header
   layer, which keep the old `always` order. The fixtures under
   `tests/fixtures/*.native` are recompiled, not compared byte-for-byte.
5. Public API. `DefinitionLock` gains `layers`, `layer_row`,
   `layer_reference`, `closure`, `admitted_forms`, `admits` and
   `qualification_roles`. `value` re-exports `HeaderLayer`, `LayerForm`,
   `LayerRow`, `WitnessKind` and `WitnessRequest` from a `pub(crate)`
   module. `CheckedGraph::header_layers` is new. `layer_row` indexes
   `LAYER_ROWS` by `layer as usize`. That is correct today but relies on the
   table order matching the enum. Not raised.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `expression_form` classifies `=`/`!=` as admitted under every layer whatever the operand type, so `quire.state.core/v1` and `quire.state.queries/v1` admit whole-record, sequence and option equality and reference/object identity equality. The state-core definition lists these as refused, and FR-110 says S3 never admits a form under a wider layer. | qsl-semantics/src/check/layer.rs:135-147 |

## Verdict

PASS. No high-severity bug. The classifier, the closure, the
qualification-row switch, the new assembly cause and its prohibited
classification, and the lock rows are correct for the forms they
classify. FND-001 is a medium: a type-dependent form that a syntax-only
classifier cannot see. It is recorded under the major-only bar and does not
block merge.

Author-recorded decisions are left to the planner and not raised: the
`value.complete` `ObjectNavigation` exception, FR-110-CON-1 against the form
table living in Rust, the no-header default, imported predicate calls
classified as value constructs, ungated model constructs (FR-361), and the
editor path.

Note: commit d2b72ea75 landed on the branch during this review. It is outside
the reviewed head. It adds five tests that assert each `LayerRow::forms` list
literally ("so a change to the table is a visible diff of this test"). Those
are change-detector tests that restate the table, not functional tests. They
should be deleted in the next round.
