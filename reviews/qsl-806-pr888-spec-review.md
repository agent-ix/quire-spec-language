---
id: SR-11689
title: "Spec review of quire-spec-language PR #888: the Text value type in the model document and typed frame comparison"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@5b48879f8f286b9d4091b683cb605f299df92735; spec/functional/FR-056-admit-domain-package-model-declarations.md, spec/functional/FR-106-admit-snapshots-and-invocations.md, spec/test-cases/TC-1038-snapshots-carry-text-values.md, spec/test-cases/TC-1044-text-value-types-in-the-model-document-and-typed-frame-comparison.md"
review_set: subset
---
# Spec review of quire-spec-language PR #888

## Summary

Ticket: QSL-806. One pass of spec-review with the contradiction, ambiguity,
completeness, testability and spec-integrity sub-analyses over the PR diff
(`origin/main...HEAD`). Spec-only PR, so no code review. Only major issues
block. Mediums and lows are recorded in the Verdict.

Every TC-1044 row was traced first through the filament-core-data
semantic-IR validator at the rev QSL pins (`agent_ix_semantic_ir::decide`:
`schema::decide`, then `rules::decide` only when the schema is clean), which
`validate_with_semantic_ir` (qsl-semantics/src/model/intake.rs:1402) runs
before any reader, then through the reader text of FR-056.

Oracles: FR-056 (Text paragraph, AC-11, AC-34 to AC-36), FR-098 (argument
admission, AC-8, AC-10, AC-13), FR-100 domain table, FR-106 value grammar and
check 11, FR-115 Behavior, FR-154/TC-1043 dependency marking, TC-897 step 3.
QSpec: the checked-package vectors' Text value types.

Examined:
- FR-056 Text paragraph, six-step order, value-domain paragraph, bare Text paragraph (examined)
- FR-056-AC-11, AC-37, AC-38, AC-39 (examined)
- FR-106 check 11 typed-equality sentence, FR-106-AC-40 (examined)
- TC-1044 preconditions and steps 1 to 4 (examined)
- TC-1038 preconditions (examined)
- FR-098 argument admission and AC-10 (context_only)
- FR-106 value grammar, FR-115 Behavior (context_only)
- TC-897 step 3, TC-1043 dependency marking (context_only)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The Text value type form is refused by the pinned semantic-IR validator, so every row that admits or reads one is unreachable. FR-056 requires each constraint's `appliesTo` to be the value type itself (TC-1044's `ix://P/Title`, as QSpec's fixture writes). At the pin, `rules.rs::constraint_rules` resolves that identity to a construct-kind node, `shape_of` gives `Some(Record)`, so the kind is `"construct"` and `applies_to("minLength"/"maxLength"/"format", "construct", _)` is false: `CONSTRAINT_NOT_APPLICABLE`, Error severity, which `validate_with_semantic_ir` does not filter, so intake refuses `invalid_model_binding`/`malformed-declaration` at `Title`. The same rev has no `Shape::Scalar`; QSL's own integer value-type test works around this by naming `ix://quire/native/Integer` in `appliesTo` (tests/it/model_operations.rs:735). Unreachable: AC-37 (all admits), AC-38 reader rows (c) to (k), AC-39, AC-40, the AC-11 admit/`nfx`/65-64 rows, TC-1044 steps 1, 2(a)-(k), 3, 4, and TC-1038 now that its preconditions cite the AC-37 form. Fix: state this validator dependency and mark the dependent rows as FR-154/TC-1043 do, or bump the pin to an FCD rev that applies length/format constraints to a string value-type construct. | spec/functional/FR-056-admit-domain-package-model-declarations.md:717-731 |
| FND-002 | high | Two FR-056-AC-38 / TC-1044 step 2 rows expect a Text-reader refusal that the validator's schema stage pre-empts, independent of FND-001. (a) A `textProfile` on `Title`: `schema.rs` lets `textProfile` sit only on a definition whose `kind` is the core string `"scalar"` with scalar `string`; `Title`'s kind is the construct object, so the schema refuses `malformed-declaration`, not `declaration-form` naming `textProfile`. Reader step 1's "a `textProfile` member on the value type" branch can never run. (b) The `pattern` constraint is written `{"regex":"^a"}`, but the schema requires both `regex` and `dialect`, so it refuses `malformed-declaration`, not `declaration-form` naming `pattern`. Fix: move (a) to the validator rows (and drop the value-type textProfile branch from step 1), and give (b) `"dialect":"ecma-262"`. | spec/functional/FR-056-admit-domain-package-model-declarations.md:893 |
| FND-003 | high | FR-056's value-domain rule and FR-056-AC-39 contradict FR-098. FR-056 says a replayed call argument, `Input` assignment or `Witness` entry outside the Text domain refuses `text_length_out_of_domain`/`outside-domain` before the call; TC-1044 step 3 expects that for nine pairs from `Input` and `Witness`. FR-098 states "A value outside a parameter's declared domain (`12` for `Int[0, 9]`) refuses at S6a admission ... with the same cause, `WrongValueKind` (`invalid_runtime_input`)", and FR-098-AC-10 has "a nine-scalar text" for the `Text[0, 8]` predicate refuse `WrongValueKind` from a `Witness` entry. Two tests written from the two rules fail each other. Fix: use FR-098's `WrongValueKind` for the replay surfaces, or amend FR-098's rule and AC-10 in this PR. | spec/functional/FR-056-admit-domain-package-model-declarations.md:894 |
| FND-004 | medium | TC-897 step 3 (bare `ix://quire/native/Text`, expecting `declaration-form` naming `profile`) is unreachable at the pin: FCD's native set has no `Text`, so the validator refuses `UNRESOLVED_TYPE_REF` first. FR-056 and AC-11 now state the dependency, but TC-897 itself is unchanged and unmarked, so a test written from TC-897 asserts an outcome the pin cannot produce. The precedent (TC-1043) marks its dependent rows inside the TC. Fix: mark TC-897 step 3 as dependent on the validator's native vocabulary naming `Text`, giving the outcome at this pin. | spec/test-cases/TC-897-text-is-the-native-text-name.md:35 |

## Verdict

FAIL, four blocking findings (FND-001 to FND-004).

Traced and correct:
- Validator-stage rows (l) `minLength -1`, (m) `maxLength 1.5`, (n) `text-profile:`: `expect_integer(min 0)` and `is_namespaced_name` (empty name) refuse at the schema, which `declared_node_for_pointer` reports at `Title` (a constraint pointer names its owning declaration). The AC-11 correction for `-1` is right.
- Step 2(d), duplicate `minLength` with a distinct identity: no validator rule checks a repeated keyword or `diagnosticCode`, so apart from FND-001 it reaches reader step 2.
- `text-profile:NFC`, `text-profile:nfx` and `email:rfc` pass `is_namespaced_name` and reach the reader as written.
- The field-level `textProfile` row (`label`, `typeRef` native `String`) passes the validator (`Resolved::Native("string")`; parameters share `field_schema`), so the QSL refusal is reachable. Choice (1) is sound: QSpec never uses `textProfile` (the second spelling is FCD's), and QSpec reads the profile only from `format`.
- Choice (3), both bounds required, matches QSpec, which requires both bounds. Choice (4), profile before bounds, does not conflict with QSpec's inverted-bounds vector (profile `nfc`).
- FR-106 check 11: the equality list covers exactly FR-106's value grammar (boolean, integer, absent/present, reference, sequence, record, variant, String, UUID, Timestamp, Text; no set/bag/ordered-set value exists). It agrees with the existing record-by-most-specific-type sentence and with FR-115, which delegates to check 11. Each equality is the kernel type's own, so no second equality is needed. AC-40's rows follow from it (`aliases` is `Sequence<Tag>`, so a swap is a change).
- No test parses FR-056, FR-106, TC-897, TC-1038 or TC-1044 (only FR-092 is `include_str!`-ed). No id collides with an open PR; TC-1044 is new. No ticket ids or tracking ceremony added.

Recorded (non-blocking):
- low: TC-1044 step 4 expected "absent to present refuses naming `subtitle`" omits `child`, unlike every other row.
