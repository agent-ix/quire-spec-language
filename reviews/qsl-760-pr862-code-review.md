---
id: SR-11191
title: QSL-760 PR 862 code-review
type: SpecReview
analysis: code-review
scope: agent-ix/quire-spec-language@809a3d1afdb5c8cfde561ee6073eab26a809b2e5; PR 862;
  qsl-semantics/src/check/{type_form,assemble,check,claims,lowering}.rs;
  qsl-replay/src/spine/call.rs; qsl-semantics/tests/it/native_scalar_equality.rs;
  qsl-eval/tests/it/native_scalar_equality.rs
review_set: subset
---

## Summary

Ticket: QSL-760. Code review with the rust-review lane folded in. The PR resolves `String`, `UUID` and `Timestamp` as native candidates, makes a same-named unit form an `AmbiguousTypeName` with a typed `native` flag, refuses same-type ordering on the three as `operator-ineligible`, lowers `=`/`!=` to the native equality identities, drops the dead `String` arm from `scalar_equality`, and lets replay call arguments carry `Uuid`/`Timestamp` values.

## Verdict

FINDINGS: one high, a test held ignored with a stale reason. The rest of the code is correct. Checked: the native flag is set at every producer, and its consumers use `{ .. }` only for code, catalog cause and message, so no wildcard hides a wrong result. No existing fixture declares a form named `String`, `UUID` or `Timestamp`. No existing test expectation changed: the only edits add `native: false`. All 12 ordering pairs refuse `operator-ineligible`, and unlike-type ordering still falls through to `type-mismatch`. `Value::Uuid` and `Value::Timestamp` can only be built through quire-exact's canonical constructors, so `argument_value` cannot admit a noncanonical spelling. New non-test code has no unwrap, expect or panic. Removing `String` from `scalar_equality` changes no earlier behaviour: before this PR, `String` equality was refused in lowering, so the arm was never reached. The native families are also not in `SCALAR_FAMILIES`. Four of the five qsl-eval tests fail on pre-change code, because `UUID` does not resolve there and the fixture panics.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `uuid_timestamp_and_string_fields_are_read_through_a_child` (FR-359-AC-1, TC-998) stays `#[ignore]`d. Its reason is "`=` on them is refused ... not yet admitted", which is exactly what this PR admits. The trace tag counts a test that never runs, and native-leaf equality through a child is not exercised. Un-ignore it. If it fails, that is a defect in this PR. | qsl-semantics/tests/it/child_composite.rs:506-522 |

## Coverage

Recorded medium and low items, not blocking:
- Step 3 lowering asserts `arguments` length 2, not the order `[ref(a), ref(b)]`.
- The replay `assembly_message` reads "names more than one declaration" when one candidate is the native type, which is not a declaration. Message text only.
- The new `argument_value` `Uuid`/`Timestamp` arms have no test.
- The qsl-eval E8 test checks only quire-exact's `Uuid::from_canonical_text`. See SR-11192 FND-001.

No build was run. Per the coder's logs, the focused tests and the crate tests for semantics, eval and replay (all features) pass, clippy is clean, and qsl760-red1.log shows the semantics tests failing 8 of 10 before the implementation.

Plan completion: not assessed

## Examined scope

```yaml
scope:
- id: qsl-semantics/src/check/type_form.rs
  path: qsl-semantics/src/check/type_form.rs
  role: examined
  excerpt: "([], None) => MissingName; ([], Some(native)) => Ok(native); ([value_type], None) => Ok; _ => AmbiguousName"
- id: qsl-semantics/src/check/assemble.rs
  path: qsl-semantics/src/check/assemble.rs
  role: examined
  excerpt: "None | Some(0) if native_scalar_named(name).is_some() => {} ... Some(1) if native_scalar_named(name).is_none() => {} Some(_) => AmbiguousTypeName { native: native_scalar_named(name).is_some() }"
- id: qsl-semantics/src/check/check.rs
  path: qsl-semantics/src/check/check.rs
  role: examined
  excerpt: "| (ValueType::String, ValueType::String) | (ValueType::Uuid, ValueType::Uuid) | (ValueType::Timestamp, ValueType::Timestamp) => return Err(ineligible(location)), _ => return Err(mismatch(location))"
- id: qsl-semantics/src/check/claims.rs
  path: qsl-semantics/src/check/claims.rs
  role: examined
  excerpt: "ValueType::String | ValueType::Uuid | ValueType::Timestamp => false"
- id: qsl-semantics/src/check/lowering.rs
  path: qsl-semantics/src/check/lowering.rs
  role: examined
  excerpt: "ValueType::String => \"string\", ValueType::Uuid => \"uuid\", ValueType::Timestamp => \"timestamp\""
- id: qsl-replay/src/spine/call.rs
  path: qsl-replay/src/spine/call.rs
  role: examined
  excerpt: "ValueType::Uuid => match value { Value::Uuid(value) => Ok(Value::Uuid(value)), _ => Err(wrong()) }"
- id: qsl-replay/src/spine.rs
  path: qsl-replay/src/spine.rs
  role: context_only
  excerpt: "AssemblyCause::AmbiguousTypeName { name, .. } => format!(\"`{name}` names more than one declaration\")"
- id: qsl-semantics/tests/it/child_composite.rs
  path: qsl-semantics/tests/it/child_composite.rs
  role: examined
  excerpt: "#[ignore = \"String, UUID and Timestamp now lower, but `=` on them is refused ill_typed/operator-ineligible; equality on native scalars is not yet admitted\"]"
```

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 73fb60576d7f310f60723b179b11d7b61f01e769 (`#[ignore]` removed; test body and expectations unchanged against origin/main, only the doc comment reworded; passes in qsl760-fix.log) |
