---
id: SR-752
title: "QSL-289 code review (with rust-review lane) of PR 498"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@ebcc62d94065581527c7a13a14ebe462afed7f68; qsl-semantics/src/model/intake.rs; qsl-semantics/src/model/domain_package.rs; qsl-semantics/src/check/assemble.rs; qsl-semantics/src/model/index.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: reviews
---
## Summary

Ticket: QSL-289. PR: quire-spec-language#498 at ebcc62d9, base a62bd8b4.
Methods: code-review with the rust-review lane folded in. The diff touches one
file, `qsl-semantics/src/model/intake.rs` (+235/-1).

Checked and sound:

- Dispatch. `read_type_node` now matches `meaning::VALUE_TYPE` before the
  generic `other if meaning::ALL.contains(&other)` arm (intake.rs:2232-2236).
  The old refusal was that generic arm, not a dedicated branch, so nothing is
  left behind as dead code. The generic arm still serves EVENT_TYPE,
  STATE_MACHINE, PROCESS and the rest.
- Downstream. `ScalarTypeRecord { key, lower: i64, upper: i64 }`
  (domain_package.rs:214) and `check/assemble.rs:704` are not in the diff.
  `ValueTypeRef::Package` resolves a field's `typeRef` to the record, and
  `assemble` builds `ValueType::Int(IntegerInterval::new(lower, upper))`.
  The coder's claim that nothing downstream needed touching holds.
- Structural reading. The bound is read from `constraints[].keyword` and
  `operands.value` via `Value::as_i64`. No rendered `Int[lo,hi]` string is
  parsed anywhere. Each refusal case the PR body lists has a real branch.
- `#[qsl_attrs::string_edge]` is FR-064's marker for wire-edge string
  dispatch (`scalar != "integer"`, `match keyword`). It is used the same way
  as on `read_relationship`, `read_population`, `read_connection` and
  `read_field_member`. It is not a lint suppression.
- `missing_docs`: no new public item. `read_value_type` and the test helpers
  are private.
- No `unwrap`, `expect`, `panic!` or lossy cast in the production code.
  `as_i64` returns `None` for a float or a value above `i64::MAX`, and that
  refuses rather than truncating.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The pinned FCD rev (033e228) binds `quire.meaning.model.value-type/v1` to record-shaped constructs: `agent-ix/spec-objects-business` `value_object` (`fields: required`) and `event` (`fields`, `occurrenceField` required), in `fixtures/semantic/v1/positive/semantic-ir-v2-constructs.json`. Before this PR, such a type refused `unsupported_construct`/`declaration-form`. Now it reaches `read_value_type`, which refuses it `invalid_model_binding`/`malformed-declaration` ("a value type declares no fields"). An upstream-valid document is now blamed as malformed. The ticket said to settle the wire shape with the FCD owner first. Nothing in the PR records that. Either refuse a value type with `fields` as `unsupported_construct` (a record-shaped value type QSL cannot read yet), or record the FCD owner's ruling and align FR-056's export-records row with it. | qsl-semantics/src/model/intake.rs:1761-1764 |
| FND-002 | low | A missing `min` or `max` refuses `malformed-declaration`, but a half-bounded integer is valid FCD wire (FCD's own config-version-v2 `VersionNumber` carries only `min`). `ScalarTypeRecord` cannot represent it, so it is a known-but-unsupported form, and `unsupported_construct` is the honest code. The same PR already refuses `exclusiveMax` as unsupported. | qsl-semantics/src/model/intake.rs:1819-1823 |
| FND-003 | low | The doc comment says a value type's other members refuse "rather than being silently dropped", but only `fields`, `operations`, `relationships` and `supertypes` are checked. `variants`, `clauses` and `abstract` are optional on every construct kind upstream and are ignored without comment. Either check them or narrow the comment. The refusal codes are also mixed without a stated rule: `operations` is `unsupported_construct`, while `fields`, `relationships` and `supertypes` are `malformed-declaration`. | qsl-semantics/src/model/intake.rs:1748-1752; qsl-semantics/src/model/intake.rs:1761-1779 |
| FND-004 | low | Stale comment. The generic arm's comment still gives RECORD_VALUE_TYPE as its example of a meaning "with no QSL record shape yet", but RECORD_VALUE_TYPE has its own reader two arms above. This predates the PR, but the PR edits this match. | qsl-semantics/src/model/intake.rs:2258-2260 |

## Verdict

Request changes on FND-001. It is a real misclassification of upstream-valid
input, and the ticket made the upstream shape a precondition. FND-002 to
FND-004 are small and can be fixed in the same round. Gates were re-run by the reviewer
at ebcc62d9: `make ci` exit 0, `cargo clippy -p qsl-semantics --all-targets
--all-features -- -D warnings` clean, and `cargo test -p qsl-semantics` all
passed. Test findings are in SR-753.
