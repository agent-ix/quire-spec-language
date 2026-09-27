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
| FND-013 | low | Round 2 (reviewed 0b7bc758): `read_raw_value` accepts `{"absent": <any payload>}`, for example `{"absent": null}` or `{"absent": 5}`, where FR-106 spells the form `{"absent": {}}`. Checked by running it: `{"absent": 5}` admits. | qsl-semantics/src/model/observation.rs:337 |
| FND-014 | low | Round 2 (reviewed 0b7bc758): `check_population_completeness` declares `let mut required` twice, with its explanatory comment duplicated (a copy-paste leftover). Several new code comments also cite the wrong SR-750 FND numbers: the per-document limits cite "FND-011" (it is FND-009), and the `pre` fallback cites "FND-013" (it is FND-010). | qsl-semantics/src/model/observation/document.rs:1073-1095 |
| FND-015 | low | Round 2 (reviewed 0b7bc758): bytes that are not JSON, or JSON that is not an object, now refuse `missing-member` at `format`. FR-106 runs check 1.4 (format, `unknown_wire`/`unsupported-wire`) before check 1.5, and a document with no readable `format` fails 1.4 first. | qsl-semantics/src/model/observation/document.rs:217-221 |
| FND-016 | medium | Round 3 (reviewed 488a2c30): `ObjectEnvironment::new_tolerating_incomplete_population_dangling` turns off the closure check for every reference, not only those into incomplete populations. `admit_scalar` still types a reference by the field's declared type and ignores the wire `population`, so the wire-level check 8 and the typed environment can disagree. I ran a case to confirm. `child.parent` is set to `{archive, k}`, where `archive` is complete and holds `k` of type `Sub`, and `config_history` has no `k`. That snapshot now admits with a typed reference `(U, ConfigVersion, k)` that names no admitted object. At round 2 it was refused. Any `deref` of it at S6a is then an InternalFault (exit 30) instead of an admission refusal. Use one constructor that takes the exact set of dangling targets check 8 tolerated, so closure stays enforced for everything else. Also resolve a reference against the wire population's admitted object. | qsl-semantics/src/model/object_environment.rs:123-152; qsl-semantics/src/model/observation/document.rs:1230; qsl-semantics/src/model/observation/document.rs:740-750 |
| FND-017 | low | Round 3 (reviewed 488a2c30): in the reordered check 6.4, the undeclared-field test (`unknown-member`) runs before the missing-field test (`missing-member`). FR-106 check 6.4 lists missing first, and "inside a check, the conditions run in the order listed". | qsl-semantics/src/model/observation/document.rs:1032-1042 |
| FND-018 | low | Round 3 (reviewed 488a2c30): `Code::parse_str`, added in round 2, duplicates the existing `Code::from_code`, which has the same body. Delete `parse_str` and call `from_code`. | qsl-foundation/src/diagnostic.rs:230; qsl-foundation/src/diagnostic.rs:288 |
| FND-019 | low | Round 4 (reviewed bec2d93c): a reference whose wire `population` names no population of the snapshot (for example `{"population": "bogus", "key": "x"}`) lands in `References::unresolved`. `finish_populations` then tolerates it, because `completeness.get("bogus")` is `None`, not `Some(true)`. Unless the referencing object is in a required population, which makes check 7 return Incomplete, the snapshot admits holding a dangling reference into a population the package does not declare. FR-106 names no check for this case. Either refuse it at 6.5 (`wrong-value-kind`) or tolerate only populations the snapshot actually lists as incomplete. | qsl-semantics/src/model/observation/document.rs:1304-1309; qsl-semantics/src/model/observation/document.rs:875-879 |

## Verdict

Request changes. FND-001 and FND-002 block merge, because they give a wrong
result and a panic on untrusted input. FND-003 to FND-009 are real FR-106 and
FR-107 deviations and should be fixed in this PR. FND-010 to FND-012 can be
fixed in this PR or deferred with a ticket.

## Dispositions

Round 2, reviewed 0b7bc7581582f5d87d6bb2430e0cd460c53b1c76. The caller's `make ci` log
(qsl-278-ci-r10.log) has that SHA as its first line and exit 0 as its last. Every item
below was checked against the code at that head, not taken from the coder's
claims. Three items were checked by running scratch tests in a throwaway
worktree, since removed: the dangling reference into an incomplete
population, the empty key and `model` given as a string.

| ID | Disposition |
| --- | --- |
| FND-001 | open (partly fixed cc72b43c, 10ffb4cd). Fixed: required populations now come from the wire's `reference.population` plus `self`'s population; check 8 reads `keys_by_population`; rows 23 and 24 pass. Still open: a dangling reference in an incomplete population that is not required is still refused. Check 8 skips it correctly, but `finish_populations` then calls `ObjectEnvironment::new`, which refuses every dangling reference (document.rs:1184-1189, 1221-1230). Running it confirmed this: `archive` with `complete: false` and `a1.parent` to `{archive, missing}` gives `Refused dangling_reference`, where FR-106 says "skip the dangling check over an incomplete population". |
| FND-002 | fixed cc72b43c: `hex_bytes` now works over `as_bytes()` and checks `is_ascii_hexdigit` first; `a_non_ascii_digest_refuses_rather_than_panics` covers it (document.rs:1358). |
| FND-003 | fixed cc72b43c: `read_document_ref` uses `strip_prefix` (document.rs:439-443). The labels, `population`, `complete`, `key`, `type`, `context` and `operation` all refuse `missing-member` rather than defaulting. |
| FND-004 | open (mostly fixed cc72b43c). Two document defects still return `AdmissionFailure::Fault`, both confirmed by running them: `"model": "x"` gives `Fault model-member-not-an-object` (document.rs:351), and an object with `"key": ""` gives `Fault empty-object-identity` (observation.rs:950, reached from document.rs:1047). |
| FND-005 | open (partly fixed 10ffb4cd). Fixed: all documents are read (check 1) before checks 3 to 5; check 4 runs over each document in read order; checks 6, 7 and 8 interleave across pre and post; `tc465_check_order_reports_the_earlier_numbered_check_across_pre_and_post` covers it. Still open: the order inside check 6 for each object is unchanged. 6.3 (duplicate key, document.rs:947) runs before 6.1 and 6.2, and 6.4's undeclared-field test (document.rs:1033) runs after the 6.5 value checks. |
| FND-006 | fixed 10ffb4cd: `post_self_population` and the post `resolve_self` apply to a postcondition only (observation.rs:830-873); `precondition_does_not_require_self_in_the_post_snapshot` covers it. |
| FND-007 | open (partly fixed 5ff4dbba). Fixed: `modifies` is matched against the object's declaring type by conformance (frame.rs:56-73), and `created`/`deleted` entries for a population in neither snapshot now go through 11.4 (test `undeclared_population_in_created_still_refuses_delta_disagreement`). Still open: check 11 is still a reimplementation, not `enforce_frame` through `admit_invocation` (population.rs:1259, 1381), as FR-106 check 11 states. No test covers the cross-type grant this finding named. |
| FND-008 | open (partly fixed fe94f9ae). Fixed: Value-family attribute reads charge nothing, the same as main, and `model_reference_queries.rs:1277` covers it. Still open: `model.deref` is now never charged anywhere (the only `ModelDeref` reference left is a test comment). A protocol-clause `deref(r).f` charges only `model.navigate` (evaluate.rs:1079-1081), where FR-107 says "charge `model.deref` for each `deref(...)`". The `Attribute` node needs to tell `deref(r).f` apart from `self.f`, so that the deref charge applies only to the first. |
| FND-009 | fixed 8b715594: `objects_per_document` and `values_per_document` are enforced (document.rs:937-1030), with tests at state_clauses.rs:2413 and 2429. The check-8 scan uses a key set now. The per-argument re-normalization in `run_function` remains (clause.rs:681). That is low impact, accepted-no-change. |
| FND-010 | fixed 1f61cdb4: `objects_for` refuses `wrong_snapshot`/`wrong-anchor` (evaluate.rs:1467-1480); `a_pre_read_with_no_pre_observation_refuses_wrong_anchor` covers it. |
| FND-011 | open (partly fixed 4c1da0a0). Fixed: `AnchorKind` is compared as an enum, and `Code::parse_str` is shared. Still open: `AdmissionRecord.code` is still a `&'static str` that goes back through a string lookup with a silent `map_or(20)` (clause.rs:319). `is_protocol_clause` is still the `reads.is_some()` sentinel (evaluate.rs:1492-1494), and it now also gates charging. |
| FND-012 | open (mostly fixed). Fixed: `usize::try_from` (06a30ea3), `check_model` scoped to the clause alias (0b7bc758), and the catalog cause `unknown_required_feature`/`unsupported-feature` per the coordinator's ruling (4de15b0d). Still open: `ClauseDeclarations`, `ClaimSubject` and `ClauseClaim` are still `pub` with `pub` fields (state_clause.rs:127, 152, 163). |

### Round 3 (reviewed 488a2c3036da167de416faf0562f1c54a879c929)

The caller's `make ci` log (qsl-278-ci-r11.log) starts with the head SHA and
ends `exit=0`. I checked every item against the code. FND-016 was confirmed
by running a scratch test in a throwaway worktree, since removed.

| ID | Disposition |
| --- | --- |
| FND-001 | fixed 0c4d86d1: a dangling reference in an incomplete population that nothing requires now admits (`a_dangling_reference_into_an_incomplete_population_still_admits`). The mechanism has a problem of its own, recorded as new FND-016. |
| FND-004 | fixed 503e16d3: `model` of the wrong kind now refuses `wrong-value-kind` (a missing `model` refuses `missing-member`), and an empty key refuses `invalid-value` at `key` (observation.rs, `helpers::object_reference`). |
| FND-005 | fixed e9ba0e46: each object now runs 6.1, 6.2, 6.3, 6.4 and 6.5 in that order. The two halves of 6.4 are swapped, recorded as new FND-017 (low). |
| FND-007 | open. Of the coder's three blockers to delegating, two are real. `PopulationMember.field_values` holds reference identities only (`Vec<String>`, population.rs:466-483), and `admit_invocation` requires a check-time `GeneralizationClosure` (population.rs:1164-1176). The third is not a blocker: `enforce_frame` already computes `computed_created`/`computed_deleted` itself (population.rs:1399-1480), so returning them is a signature change. The two implementations have also already drifted. `frame.rs` authorizes `creates`/`deletes` by exact type-name match (frame.rs:146-147, 181, 204), while `enforce_frame` uses conformance (population.rs:1409, 1455). It authorizes `modifies` by display name plus owner conformance (frame.rs:97-111), while `enforce_frame` uses `redefinition_reaches` (population.rs:1348-1354). So the module doc's claim that it "matches `enforce_frame`'s documented behavior and order exactly" is false. A documented separate copy is not acceptable, because it has already diverged. Extract the frame decision (created/deleted/retype/field-write/delta over a neutral per-object view of key, most-specific type, and comparable field values, with `ModelIndex` conformance) into one function that both `enforce_frame` and `frame::enforce` call and that returns `(created, deleted)`. The admission-specific inputs (`PopulationDocument` shape, `GeneralizationClosure`) stay in their callers. |
| FND-008 | fixed 82fba19f: `NodeKind::Attribute { derefed }` is set by the checker. Only an explicit `deref(...)` charges `model.deref`; every protocol-clause attribute read charges `model.navigate`. The Value family charges nothing. `a_protocol_clause_deref_charges_model_deref_then_model_navigate` covers it. |
| FND-011 | open. `is_protocol_clause` now reads a typed `EvaluationFamily`, which is fixed (82fba19f). But the silent `map_or(20)` became `unreachable!()` inside `ClauseRunReport::exit_code` (qsl-replay/src/spine/clause.rs:346-349). That is production code, not test-only, so it is a panic path where the standing rule wants a typed exit-30 fault. Every admission code literal parses today (checked against `Code::as_str`), so the panic is latent, but nothing enforces that. Make `AdmissionRecord.code` a `qsl_foundation::diagnostic::Code` so the lookup and the panic both go away. `evaluate_exit_code` still uses `map_or(20)` (clause.rs:370, 373), which does match FR-100's own `refusal_exit_code` (src/command/output.rs:391-392). |
| FND-012 | fixed 7dcb3a75: the fields of `ClauseDeclarations` and `ClauseClaim` are `pub(crate)`. The types stay `pub` because they are `FamilyContract` associated types (E0446). That reason holds. |
| FND-013 | fixed eb92b61e: `absent` requires an empty-object payload (observation.rs, `read_raw_value`). |
| FND-014 | fixed 70d2e06e: the duplicated `required` block is removed. |
| FND-015 | fixed 70d2e06e: bytes that are not JSON or not an object refuse `unknown_wire`/`unsupported-wire` (check 1.4). |

### Round 4 (reviewed bec2d93cbd63b50e93fa9c3ceed4738381d5b95e)

The caller's qsl-278-ci-r12.log starts with the head SHA and ends `exit=0`,
and it lists every test named below as `ok`. The coder deleted the worktree's
`target/`, so I did not rebuild. The FND-016 probe is now the test
`a_reference_to_an_admitted_object_of_another_type_refuses_wrong_value_kind`,
which uses the same document as my round-3 probe.

| ID | Disposition |
| --- | --- |
| FND-007 | fixed 832ae4a0: one decision, `population::decide_frame` (population.rs, `FrameDecision`/`FrameObject`/`FrameDelta`), covers created, retype, deleted, field writes and the declared-delta check, and returns `(created, deleted)`. `enforce_frame` is now a thin adapter (`frame_objects` over `PopulationDocument`, with `field_values_equal`). `frame::enforce` is the other adapter (`wire_object` over `RawObject`, with field keys built from each attribute's declaring owner and `raw_values_equal`). Creates and deletes go through `granted`, using `ModelIndex::conforms`. Modifies goes through `field_write_covered` (`redefinition_reaches`). The old display-name `field_write_authorized`, `owner_of` and the exact-name `creates`/`deletes` sets are gone. The drift cases are tested: `a_creation_of_a_subtype_of_a_creates_grant_admits`, `a_write_to_a_field_that_redefines_a_modifies_grant_admits`, and `a_same_named_field_of_an_unrelated_type_is_never_authorized`. Deletes by conformance has no test of its own but shares `granted` with creates. |
| FND-011 | fixed f17fda85: `AdmissionRecord.code: Code`, and `exit_code` is `record.code.exit_code()` with no lookup and no panic (clause.rs:336-338). A grep of the production parts of `clause.rs`, `observation.rs`, `observation/{document,frame,ordered_json}.rs` and `object_environment.rs` finds no `unreachable!`, `expect`, `unwrap`, `panic!`, `todo!` or `unimplemented!`. `evaluate_exit_code` keeps `Code::from_code(..).map_or(20, ..)` (clause.rs:356, 359), which matches FR-100's own `refusal_exit_code` (src/command/output.rs:391-392). That was accepted in round 3. |
| FND-016 | fixed 5690b68a. There is one constructor, `ObjectEnvironment::new(types, objects, tolerated_dangling: &[ObjectReference])` (object_environment.rs), and closure is checked for every reference not in that exact set. `References::resolve` looks up the wire `(population, key)` among admitted objects first, and a hit must pass `ValueType::Reference(declared).admits`. Only a miss is resolved by declared type and recorded as unresolved, and `finish_populations` tolerates only the unresolved targets whose population is not complete. The `{archive, k}` to `Sub` probe now refuses `invalid_runtime_input`/`wrong-value-kind` at `child`/`parent`. That is the right check under FR-106's order. The wire reference names an existing object in a complete population, so check 8 has nothing to report. The value is a `Sub` reference where the field declares `Reference<ConfigVersion>`, and the kernel admits a reference only at its exact object type (`quire-exact/src/value.rs:246-248`; TC-465's amended note says a `Sub` object cannot be validly referenced by a `ConfigVersion`-typed field). That is check 6.5's "the value's kind does not fit the declared type", which runs before checks 7 and 8. The tolerated case still admits (`a_dangling_reference_into_an_incomplete_population_still_admits`, `ok` in r12). One remaining edge is recorded as new FND-019 (low). |
| FND-017 | fixed 64e8f97e: inside 6.4, the missing-field test now runs before the undeclared-field test. |
| FND-018 | fixed dc8dc8f5: `Code::parse_str` is deleted and callers use `Code::from_code`. |

### Round 5 (reviewed 9ba7652c6dbce0dc86ef8ccc13935ee012e97d67)

The caller's qsl-278-ci-r13.log starts with the head SHA and ends `exit=0`.
Both tests below appear in it as `ok`.

| ID | Disposition |
| --- | --- |
| FND-019 | fixed c3e121e3: `finish_populations` now tolerates an unresolved reference only when the snapshot lists its population with `complete: false` (`completeness.get(..) == Some(false)`, document.rs:1310). A reference into an unlisted population now fails the closure check and refuses `dangling_reference` (`a_reference_into_an_unlisted_population_refuses_dangling_reference`, state_clauses.rs:3364). The tolerated-incomplete case still admits (`a_dangling_reference_into_an_incomplete_population_still_admits`). |
