---
id: SR-780
title: "QSL-317 code review of PR 517 (qsl-replay call-site facade and request-type re-exports)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@5a233b04000b2447b723400fcfbfe570c5a83505; qsl-replay/src/call_site.rs; qsl-replay/src/lib.rs; qsl-replay/src/execute.rs (read, unchanged); tools/arch-lint/api_surface.rs (read, unchanged); xtask/src/typestate_scan.rs (read, unchanged)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-088
    type: reviews
---
## Summary

Ticket: QSL-317. PR: quire-spec-language#517 at 5a233b04. This review covers
code and Rust (the rust-review lane is folded in here). The diff adds
`qsl-replay/src/call_site.rs` (a public `call_site` entry, `CallSite` and
`CallSiteRefusal`) and seven re-exports in `qsl-replay/src/lib.rs`.

What was checked:

- **Architecture boundary (ADR-011 §6.1, FB-05, T-12; ADR-013 TK-01).** The
  `replay` facade is "the only QSL surface CG uses" and is widened by
  tickets. `call_site` adds one function to that surface. It does not expose
  `spine`: its signature names only `SourceIdentity`, `QualifiedName`,
  `CallSite` and `CallSiteRefusal`. `CallSiteRefusal::Compile` flattens
  `spine::CompileRefusal` into its rendered `String`, so no `spine` type
  reaches the public error. That is stricter than the precedent:
  `ReplayRefusal::Recompile(Box<CompileRefusal>)` already carries a `spine`
  type publicly. `Fault(InternalFault)` follows the precedent of
  `ReplayRefusal::Fault`, which already exposes the same type. No re-exported
  type names `qsl_eval` (FR-100-AC-8). `WireNodeId::from_digest` is the wire
  node id, not the kernel `NodeKey::from_digest` that T12-B governs, and the
  pattern `NodeKey::from_digest` does not match it.
- **Faithful to the executor.** The lookup (`segments()` has one segment,
  `graph().callable`, `semantic_graph().node(..).function_parameters()`, the
  length check against the signature, `WireNodeId::from_digest(*key.as_bytes())`)
  is the same one `execute.rs:560-598` and `execute.rs:687` use, with the same
  fault invariant names.
- **FR-088-AC-6 / TC-258.** `xtask/src/typestate_scan.rs:1200`
  `tc_258_a_qualified_name_is_never_an_identity_on_its_own` fails any struct
  or enum variant whose only field is a `QualifiedName`. Pairing `selection`
  with the `package` it was looked up in is the shape
  `ReplayRefusal::UnknownFunction` already has, and it is the right identity
  (a name is scoped to a package). It is a correct use of the rule, not a way
  around it.
- **Tests exercise the real path.** Both tests call `call_site` over a real
  native unit through the real `spine::compile`. There is no mock.
- **Rust idioms.** `Box<CallSiteRefusal>` keeps the `Result` small. No
  `unsafe`, no integer casts, no panics in shipped code (`let [segment] = ..
  else` and `ok_or_else`). `thiserror` messages follow the crate's style.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The package-id/parameter test builds its expected value by re-running the same lookup chain `call_site` runs (`callable` → `semantic_graph().node` → `function_parameters` → `WireNodeId::from_digest`). It can fail only if `call_site` stops matching its own copied code. It cannot catch a wrong node being picked, and it does not show that the returned ids are the ones `replay` accepts. A stronger oracle: build a `ReplayRequestWire` whose `CanonicalAssignment`s are keyed by `site.parameters` and assert `replay` does not refuse `UnknownParameter`/`UnboundParameter`, or assert against a fixed hex vector. | qsl-replay/src/call_site.rs:151-187 |
| FND-002 | low | The parameter-key lookup is copied from `execute.rs:560-598,687` instead of shared. If the executor's key derivation changes, `call_site` silently drifts, and FND-001's test would not notice. Extract one crate-private helper that both call. | qsl-replay/src/call_site.rs:90-128 |
| FND-003 | low | The `call_site` doc says a source that imports a library or needs non-default stage limits "refuses here (`DependencyInput`)". `CallSiteRefusal` has no such variant. Such a source refuses as `CallSiteRefusal::Compile(String)`, carrying for example `missing_import/missing-selection` (an `ImportRefusal`), not a `DependencyInput` refusal. | qsl-replay/src/call_site.rs:67-72 |
| FND-004 | low | The re-export comment says "Read-only, like the occurrence key and source region above: no constructor this crate does not already carry is added." That is not true. Unlike `OccurrenceKey`/`SourceRegion`, whose constructor inputs stay unexported, these re-exports make `DigestRecord::mint`, `WireNodeId::from_digest`/`from_hex`, `SourceIdentity::new`, `Identifier::new` and `ByteDigest::of` callable through `qsl_replay`. CG needs exactly those; its skeleton-spine test calls `DigestRecord::mint` and `SourceIdentity::new`. None is a T-12-governed constructor, so no rule is broken. Only the comment is wrong. | qsl-replay/src/lib.rs:60-66 |

## Verdict

PASS with one medium and three low findings. The facade addition is
architecturally sound: `spine` stays out of the public signature and error,
no T-12 rule is touched, and the `UnknownFunction` shape is a correct use of
FR-088-AC-6. FND-001 is the one worth fixing before merge. The others are
doc and duplication nits.
