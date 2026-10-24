---
id: SR-11412
title: "Code and Rust review — QSL-791/QSL-796 PR #879 native payload objects"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@84c9a9084; QSL-791; QSL-796; qsl-semantics/src/model/observation.rs; qsl-semantics/src/model/observation/document.rs; qsl-semantics/src/model/observation/tree.rs; qsl-semantics/src/model/observation/encode.rs; qsl-replay/src/spine/request.rs; qsl-replay/src/spine/clause.rs"
review_set: subset
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-106, type: references }
  - { target: ix://agent-ix/quire-spec-language/FR-360, type: references }
---

## Summary

Ticket: QSL-791, QSL-796. Reviewed public PR #879 at head `84c9a9084` against `origin/main` (three-dot diff; merge-base 4bce56aea), with the `rust-review` lane folded in. No build was run by this reviewer. The PR merges cleanly with current main (no conflicts). Main gained #876 (Text value form, spec only) after this PR's merge-base.

## Verdict

FAIL: one blocking finding. The reader and `admit_native` follow FR-106's 7-step native order for String, UUID and Timestamp. The order is kind type, kind name, missing member, unknown member (first in document order), payload type, declared-type fit, then the kernel constructor, where noncanonical text beats the range check inside quire-exact. Each payload object is one `observation.values` charge, because the reader pushes one node per `charge()`. The optional-slot branch and `object_field_kind_matches` both pass a native object to the envelope check before fit. The `kernel: Option<ConstructionCause>` field is a typed carrier for the class that FR-106 step 7 requires beside `invalid-value`. Every struct-literal and destructuring site in the workspace is updated, and no crate outside qsl-semantics and qsl-replay matches `RawForm` or `RawNode`. `ValueEncoder::encode` returns `None` for String, UUID and Timestamp values through its `Read(_)` arm, so it cannot panic. It is used only in tests. `SnapshotValue::to_json` is also test-only. `Uuid::from_canonical_text` checks the length before indexing, so a non-ASCII digit cannot panic.

The blocking defect: `kind: "text"` is read as an unknown kind. On merged main, FR-106 step 2 makes `text` a known kind.

## Examined Scope

```yaml
scope:
  - id: FR-106
    path: spec/functional/FR-106-admit-snapshots-and-invocations.md
    role: examined
    excerpt: "2. A `kind` string other than `string`, `uuid`, `timestamp` and `text` (including a letter-case variant such as `String`, `UUID` or `Text`) refuses `invalid-value`, naming `kind`. The members of such an object are not examined."
  - id: FR-106-AC-35
    path: spec/functional/FR-106-admit-snapshots-and-invocations.md
    role: examined
    excerpt: "a Text object in the String field `label`, the UUID field `token`, the Timestamp field `stamp`, the integer field `versionNumber` and the `Reference` field `parent` ... each refuse `invalid_runtime_input`/`wrong-value-kind` at the nested path"
  - id: FR-360
    path: spec/functional/FR-360-type-a-field-by-a-variant-type-through-intake-check-evaluation-and-replay.md
    role: examined
    excerpt: "its String, UUID, Timestamp and Text payload forms, which a variant slot refuses `wrong-value-kind`."
  - id: qsl-semantics/src/model/observation/document.rs
    path: qsl-semantics/src/model/observation/document.rs
    role: examined
    excerpt: "fn native_payload(native: RawNative<'_>) -> Result<(NativeKind, &str), AdmissionRecord> ... fn admit_native(native: RawNative<'_>, value_type: &quire_exact::ValueType)"
  - id: qsl-semantics/src/model/observation/tree.rs
    path: qsl-semantics/src/model/observation/tree.rs
    role: examined
    excerpt: "if let Some(kind) = object.member(\"kind\") { let kind = match kind.as_str() { None => RawNativeKind::NotText, Some(\"string\") => ..., Some(_) => RawNativeKind::Unknown, };"
  - id: qsl-replay/src/spine/request.rs
    path: qsl-replay/src/spine/request.rs
    role: examined
    excerpt: "Node::String(_) => RawNativeKind::Unknown, _ => RawNativeKind::NotText,"
  - id: qsl-semantics/src/model/observation.rs
    path: qsl-semantics/src/model/observation.rs
    role: examined
    excerpt: "pub kernel: Option<quire_exact::ConstructionCause>,"
  - id: qsl-semantics/src/model/observation/encode.rs
    path: qsl-semantics/src/model/observation/encode.rs
    role: examined
    excerpt: "RawNativeKind::Unknown | RawNativeKind::NotText => Json::Null,"
```

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `kind: "text"` is read as an unknown kind, so a Text object refuses `invalid-value` naming `kind`. Merged FR-106 step 2 makes `text` a known kind, and AC-35 and FR-360 item 9 require `wrong-value-kind` for a Text object in a String, UUID, Timestamp, integer, Reference or variant slot. Main refuses those rows `wrong-value-kind` today, so main plus this PR is worse than main. | qsl-semantics/src/model/observation/tree.rs:205-211; qsl-replay/src/spine/request.rs:150-155 |

## Review Notes

- Recorded, medium: a native envelope defect sitting directly in an optional slot (not under `present`) has its own branch in `admit_value`, but no test covers it.
- Recorded, low: `encode.rs` writes an unknown `kind` as `null`, so a round-trip turns `invalid-value` into `wrong-value-kind`. Only tests call `to_json`.
- Recorded, low: the replay report path never renders `AdmissionRecord.kernel`. FR-106 requires only that the refusal carry it.
- Rust lane: no new `unwrap` or `expect` in production code. The new `match` on `ValueType` in `admit_native` lists every variant, so a new kernel type forces a decision. `#[qsl_attrs::string_edge]` is on both wire readers, and arch-lint passed in the coder's log.
