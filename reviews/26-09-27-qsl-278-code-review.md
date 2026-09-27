---
id: SR-750
title: "QSL-278 code review (with rust-review lane) of PR 495"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@c7454d7c50d906dbbca511f91ae1ef4696c72f62; qsl-semantics/src/model/observation.rs; qsl-semantics/src/model/observation/document.rs; qsl-semantics/src/model/observation/frame.rs; qsl-semantics/src/model/observation/ordered_json.rs; qsl-semantics/src/model/key.rs; qsl-semantics/src/model/intake.rs; qsl-semantics/src/model/object_environment.rs; qsl-semantics/src/check/state_clause.rs; qsl-semantics/src/check/mod.rs; qsl-eval/src/value/expression/evaluate.rs; qsl-eval/src/value/expression/mod.rs; qsl-eval/src/value/expression/causes.rs; qsl-eval/src/value/expression/s6a/mod.rs; qsl-eval/src/value/expression/s6a/protocol_clause.rs; qsl-replay/src/spine/clause.rs; qsl-replay/src/spine/call.rs; quire-exact/src/accounting.rs; xtask/src/seam_probe.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-107
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-109
    type: reviews
---
## Summary

Ticket: QSL-278. PR: quire-spec-language#495 at c7454d7c, base 5cbc77d6.
Methods: code-review with the rust-review lane folded in. The caller's own
`make ci` run at this head (exit 0) was read; no build was re-run.

History checks. `model/observation*` imports no `check` item (1). The
evaluator reads the typed `CheckedStateClause::binds_result()`, derived from
the kind and operation result in `StateClauseDeclaration::parameters` (2).
`read_model` now refuses a missing member or a wrong prefix, but the same
defects remain in `read_document_ref` and the identity labels: FND-003 (3).
`check_package_digest` is generalized, and an encoder error becomes a typed
`byte-digest-mismatch` refusal (4). `population_universe_for` reuses
normalization's `object_universe_of`, with an agreement test (5).
`sha256_and_len` produces the same bytes as before (6):
`quire_canonical::sha256` is plain SHA-256, with no domain prefix, fed by the
same `encode` that `to_vec` uses. `ByteDigest::of` is plain SHA-256 too.
`canonical_len` returns the encoder's `produced` count, which counts every
byte exactly once (the object close re-emits buffered spans through `emit`,
not `produce`), so it equals `to_vec(..).len()`. The `ProtocolClauseFamily::evaluate` seam is genuine: the
ci log confirms it among 15 probe locations (7). Item 8 is partly closed:
document order is preserved (`OrderedJson`), and the frame check runs end to
end for 11.1, 11.2, 11.3 and 11.4. `allInstances` and `lookup` refuse with a
typed family refusal. Item 9 is confirmed in `object_field_kind_matches` and
`edge_targets`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Checks 7 and 8 do not work. `required` is built only from reference targets that are present in `population_of`, which holds admitted objects. A dangling target never makes its population required, and `self`'s population is never required at all. Check 8 skips every target missing from `population_of`, so it can never fire. The dangling case falls through to `ObjectEnvironment::new`, which refuses whatever the completeness, without naming the population. TC-465 row 23 (`complete: false`, `child.parent` to `missing`) therefore refuses `dangling_reference` where FR-106-AC-4 requires `Incomplete`, and row 24 does not name `config_history`. `admit_scalar` also ignores a reference's `population` member. | qsl-semantics/src/model/observation/document.rs:940-975; qsl-semantics/src/model/observation/document.rs:991-1000; qsl-semantics/src/model/observation/document.rs:740-750 |
| FND-002 | high | `hex_bytes` slices `&text[at..at + 2]` by byte index, so a non-ASCII digest string panics on a char boundary. An example is an invocation whose `pre.digest` is `"sha256-jcs:aé…"`. This is untrusted input, and it is reached after the invocation's own digest check passes. Check `is_ascii_hexdigit` first, or decode over bytes. | qsl-semantics/src/model/observation/document.rs:401-409 |
| FND-003 | medium | History item 3 is only partly fixed. `read_document_ref` still uses `trim_start_matches("sha256-jcs:")`, so a bare hex digest or a doubled prefix is accepted. The identity labels still use `unwrap_or_default()`, so a missing or non-string label reports `blank-label` rather than `missing-member`. A population's missing `complete` becomes `false` (so the result is Incomplete, not a refusal), and a missing `population`, `key` or `type` becomes `""`. | qsl-semantics/src/model/observation/document.rs:385; qsl-semantics/src/model/observation/document.rs:273-279; qsl-semantics/src/model/observation/document.rs:476-500 |
| FND-004 | medium | These document defects return `AdmissionFailure::Fault` (exit 30) instead of a refusal: a non-object document; non-JSON bytes whose raw digest matches; `identity`, `model`, `anchor`, `populations`, `objects` or `fields` of the wrong JSON kind; an unknown anchor kind; and a field, parameter or result value that is not one of the six forms (for example `{"integer": 1}`, which FR-106 check 6.5 calls `wrong-value-kind`). FR-106 settles every input defect at admission. | qsl-semantics/src/model/observation/document.rs:208; qsl-semantics/src/model/observation/document.rs:212; qsl-semantics/src/model/observation/document.rs:272; qsl-semantics/src/model/observation/document.rs:440-444; qsl-semantics/src/model/observation/document.rs:470; qsl-semantics/src/model/observation/document.rs:513 |
| FND-005 | medium | The check order departs from FR-106. `admit_operation` runs check 4 (model) and check 5 (operation) on the invocation before check 1 reads the pre and post snapshots. It also completes checks 6 to 8 on pre before check 6 on post. Within check 6, duplicate key (6.3) runs before type membership (6.1) and the set field (6.2), and an undeclared field (6.4) is checked after the value checks (6.5). | qsl-semantics/src/model/observation.rs:727-750; qsl-semantics/src/model/observation.rs:769-775; qsl-semantics/src/model/observation/document.rs:838; qsl-semantics/src/model/observation/document.rs:917 |
| FND-006 | medium | Check 9 requires `self` in the post snapshot for a precondition too. FR-106 requires it in post only for a postcondition, so a precondition of an operation that deletes `self` wrongly refuses `wrong-role-mapping`. | qsl-semantics/src/model/observation.rs:777-782 |
| FND-007 | medium | Check 11 reimplements the frame check rather than running `enforce_frame` through `admit_invocation`, as FR-106 states. The copy differs from it. `modifies` is matched on the last path segment of the field key, across all types, so a grant on one type's field authorizes a same-named field on any other type. `created`/`deleted` entries that name a population in neither snapshot are never checked for `delta-disagreement`. | qsl-semantics/src/model/observation/frame.rs:41-43; qsl-semantics/src/model/observation/frame.rs:78; qsl-semantics/src/model/observation/frame.rs:95; qsl-semantics/src/model/population.rs:1259 |
| FND-008 | medium | Every `Attribute` read charges `model.deref` and `model.navigate`, including `self.f`, where there is no `deref`. QSpec TC-197 Y08 charges `self.home` one `model.navigate`. The arm is shared, so every Value-family attribute read now also charges two units. | qsl-eval/src/value/expression/evaluate.rs:1072-1073 |
| FND-009 | medium | The `ObservationLimits` fields `objects_per_document` and `values_per_document` are never read. Check 8 scans all objects for each reference (O(n²)), so a 1 MiB document is bounded only by the byte limit. `run_function` also re-normalizes every domain package once per reference argument. | qsl-semantics/src/model/observation.rs:88-92; qsl-semantics/src/model/observation/document.rs:970; qsl-replay/src/spine/clause.rs:507 |
| FND-010 | low | When `pre(e)` has no pre environment, `objects_for` silently reads the current environment. FR-107 requires `FamilyResult::Refused` `wrong_snapshot`/`wrong-anchor` for that case. | qsl-eval/src/value/expression/evaluate.rs:1449-1457 |
| FND-011 | low | String-dispatch residue. `AnchorKind` values are compared through `as_str()`. `exit_code` maps the record's `&str` code back through a `Code::all()` string search, with a silent `map_or(20)` default. `AdmissionRecord.code` should be a typed `Code`. `is_protocol_clause` is the sentinel `reads.is_some()`. | qsl-semantics/src/model/observation.rs:642; qsl-replay/src/spine/clause.rs:205; qsl-replay/src/spine/clause.rs:224-227; qsl-eval/src/value/expression/evaluate.rs:1467 |
| FND-012 | low | Minor items. `canonical_len(..) as usize` is a lossy cast (use `try_from`). `check_model` accepts a model that matches any selection, not the clause alias's. `ClauseDeclarations`, `ClauseClaim` and `ClaimSubject` became `pub` with `pub` fields, wider than needed. The cause `protocol-clause-population-read` is not a `native-diagnostics` 1-draft.8 cause. | qsl-semantics/src/model/key.rs:514; qsl-semantics/src/model/observation.rs:688-700; qsl-semantics/src/check/state_clause.rs:127; qsl-eval/src/value/expression/causes.rs:155 |

## Verdict

Request changes. FND-001 and FND-002 block merge, because they give a wrong
result and a panic on untrusted input. FND-003 to FND-009 are real FR-106 and
FR-107 deviations and should be fixed in this PR. FND-010 to FND-012 can be
fixed in this PR or deferred with a ticket.

## Round 2 Dispositions

<!-- reviewer-dispositions repo=agent-ix/quire-spec-language visibility=public quoin=0.24.1 module=spec-artifacts-process@v0.26.0 id=SR-750 pr=quire-spec-language#495 round=2 reviewed=0b7bc7581582f5d87d6bb2430e0cd460c53b1c76 date=2026-09-27 -->

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | open | Partly fixed cc72b43c/10ffb4cd. A dangling reference in a non-required incomplete population is still refused by `ObjectEnvironment::new` (document.rs:1184-1189); confirmed by running it |
| FND-002 | fixed | cc72b43c |
| FND-003 | fixed | cc72b43c |
| FND-004 | open | `"model": "x"` and `"key": ""` still return InternalFault (document.rs:351, observation.rs:950); confirmed by running both |
| FND-005 | open | Cross-document order fixed in 10ffb4cd. Inside check 6, the order is still 6.3 before 6.1/6.2 and undeclared-field after 6.5 (document.rs:947, 1033) |
| FND-006 | fixed | 10ffb4cd |
| FND-007 | open | Type-scoped `modifies` and unseen-population deltas fixed in 5ff4dbba. Still not `enforce_frame` via `admit_invocation`; no cross-type test |
| FND-008 | open | Value family now matches main (fe94f9ae), but `model.deref` is now never charged (evaluate.rs:1079-1081) |
| FND-009 | fixed | 8b715594 (the per-argument re-normalize is accepted-no-change) |
| FND-010 | fixed | 1f61cdb4 |
| FND-011 | open | The `&str` code with `map_or(20)` (clause.rs:319) and the `reads.is_some()` sentinel (evaluate.rs:1492) remain |
| FND-012 | open | `as usize`, alias and cause fixed (06a30ea3, 0b7bc758, 4de15b0d); the `pub` widening remains (state_clause.rs:127,152,163) |

New in round 2: FND-013 (low) `{"absent": <any>}` accepted (observation.rs:337); FND-014 (low) duplicated `required` block (document.rs:1073-1095); FND-015 (low) a non-object document refuses 1.5 `missing-member` rather than 1.4 `unsupported-wire` (document.rs:217-221).

+++ [reviewer data]

```yaml
dispositions:
  - {fnd: FND-001, outcome: open, partial_fix_shas: [cc72b43c, 10ffb4cd], reason: "dangling reference in a non-required incomplete population still refused via ObjectEnvironment::new (document.rs:1184-1189, 1221-1230); confirmed by running it"}
  - fnd: FND-002
    outcome: fixed
    fix_sha: cc72b43c
    after_excerpt: |-
      let bytes = text.as_bytes();
          if !bytes.len().is_multiple_of(2) || !bytes.iter().all(u8::is_ascii_hexdigit) {
              return None;
          }
  - fnd: FND-003
    outcome: fixed
    fix_sha: cc72b43c
    after_excerpt: |-
      let digest_hex = digest_raw
              .strip_prefix("sha256-jcs:")
              .ok_or_else(|| wrong_kind(field))?;
  - {fnd: FND-004, outcome: open, partial_fix_shas: [cc72b43c], reason: "model non-object (document.rs:351) and empty key (observation.rs:950) still Fault; confirmed by running both"}
  - {fnd: FND-005, outcome: open, partial_fix_shas: [10ffb4cd], reason: "order inside check 6 unchanged (document.rs:947, 1033)"}
  - fnd: FND-006
    outcome: fixed
    fix_sha: 10ffb4cd
    after_excerpt: |-
      let post_self_population = match kind {
              StateClauseKind::Postcondition => Some(self_object.population.as_str()),
              StateClauseKind::Invariant | StateClauseKind::Precondition => None,
          };
  - {fnd: FND-007, outcome: open, partial_fix_shas: [5ff4dbba], reason: "frame check still reimplemented, not enforce_frame via admit_invocation (population.rs:1259); no cross-type grant test"}
  - {fnd: FND-008, outcome: open, partial_fix_shas: [fe94f9ae], reason: "model.deref never charged; FR-107 requires it for each deref(...) (evaluate.rs:1079-1081)"}
  - fnd: FND-009
    outcome: fixed
    fix_sha: 8b715594
    after_excerpt: |-
      object_count += 1;
                  if object_count > limits.objects_per_document {
  - fnd: FND-010
    outcome: fixed
    fix_sha: 1f61cdb4
    after_excerpt: |-
      self.pre_objects.ok_or_else(|| {
                          Halt::Family(FamilyResult::Refused(Box::new(
                              ProtocolClauseSnapshot::WrongAnchor {
  - {fnd: FND-011, outcome: open, partial_fix_shas: [4c1da0a0], reason: "&str code + map_or(20) at clause.rs:319; reads.is_some() sentinel at evaluate.rs:1492"}
  - {fnd: FND-012, outcome: open, partial_fix_shas: [06a30ea3, 0b7bc758, 4de15b0d], reason: "pub widening remains at state_clause.rs:127,152,163"}
new_findings:
  - {fnd: FND-013, severity: low, path: qsl-semantics/src/model/observation.rs, lines: "337", finding: "absent tag accepts any payload"}
  - {fnd: FND-014, severity: low, path: qsl-semantics/src/model/observation/document.rs, lines: "1073-1095", finding: "duplicated required-set block and comment; stale FND numbers in comments"}
  - {fnd: FND-015, severity: low, path: qsl-semantics/src/model/observation/document.rs, lines: "217-221", finding: "non-object document refuses 1.5 missing-member instead of 1.4 unsupported-wire"}
```

+++
