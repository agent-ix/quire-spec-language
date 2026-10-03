---
id: SR-1256
title: "Code review of quire-spec-language PR #613: adopt quire-canonical QC1 (byte-only Limits, FixedShape, Encode, shared reader)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@f407facfd2235286412b7c3a6ea7a1706e7fe750; PR #613 diff against merge base 652ae3d51: Cargo.lock, examples/config-version/spine.rs, qsl-bench/src/model.rs, qsl-eval/src/simulation/{explore,key,sample}.rs, qsl-eval/tests/it/finite_simulation.rs, qsl-package/src/{checked,checked_v2,emit}.rs, qsl-replay/src/call_site.rs, qsl-replay/src/spine/clause/tests.rs, qsl-semantics/src/check/node_key/{mod,shape,tests}.rs, qsl-semantics/src/library/{bundle,package_identity}.rs, qsl-semantics/src/model/{domain_package,intake,key,normalize,observation,population,refusal}.rs, qsl-semantics/src/value/{definition,enumeration,member,semantic_node,unit}.rs, qsl-semantics/tests/it/{library_resolution,model_intake,state_clauses}.rs, quire-semantic-value/src/{semantic_node,unit}.rs, tests/it/composed_domain_models.rs"
review_set: subset
---
# Code review of quire-spec-language PR #613

## Summary

Ticket: QSL-480 (QC1-adopt). Draft PR. QSL moves to quire-canonical
59fe4f06: push-event `Writer`, the shared reader `read()`/`Document`,
`FixedShape`/`Encode`, and no depth limit. Rust lane (rust-review) folded in.

The three sites that wait on IR-533 are not defects here. Each is marked
plainly, and no shim is committed:

- qsl-package/src/checked.rs:509-511: a comment names `Encode` for
  `CheckedPackageIdentityPreimageV2`.
- qsl-package/src/checked_v2.rs:598-600: the same comment.
- qsl-package/src/emit.rs:813-817: the comment names all seven IR types.
- No `.cargo/config.toml` is tracked. No `[patch]` table is in any manifest.
  Cargo.lock changes only the quire-canonical bump and the new
  quire-canonical-derive entry.

The coder's report, checked against the code:

1. **Lock.** Cargo.lock pins quire-canonical and quire-canonical-derive at
   59fe4f06370fbdbd8de4fa446ad78f3974926252. Confirmed.
2. **Byte-only limits.** No `Limits::MAX_DEPTH`, `max_depth()`, two-argument
   `Limits::new` or `DepthAboveMaximum` match is left in the tree. The
   remaining `DepthAboveMaximum` (quire-semantic-value/src/checking.rs) is
   QSL's own checking cap, which slice 1 owns. `IDENTITY_LIMITS` is
   `Limits::new(u64::MAX)` (see SR-1257 FND-004 for FR-259 B3).
3. **FixedShape.** Every QSL-owned preimage in the diff derives it. The
   seven hand-written impls (`NodeRef`, `WireNodeRef`, `PackageRef`,
   `FrameField`, `LiteralValue`, `OperationMode`, `Member`) each set `DEPTH`
   to a derived wire type's `DEPTH`. In each case `Serialize` delegates to
   that same wire type, so `DEPTH` is computed from the fields actually
   written. None of the seven types is recursive. This is not a literal-DEPTH
   bypass: a cycle through a wire type would still reach rustc's E0391.
   `LeafSegment` uses `<str as FixedShape>::DEPTH`, which the trait docs allow
   for a type whose hand-written `Serialize` emits a scalar. It emits one
   string. The diff adds no `into`, `serialize_with`, `with` or `remote`.
   The three mode-spelling functions and `Member::to_wire` are deleted, and
   no caller is left. The `to_wire` hits in qsl-route, qsl-replay and family
   belong to other types.
4. **Node-key Encode.** `Preimage` and `PreimageTerm` write events from an
   explicit `Vec` task stack. I compared them field by field with the
   deleted derive: `owner` is skipped when `None`, `semantic_type`,
   `declaration` and `recursion` are written as `null` when `None`, and the
   composite terms use the same member names. `preimage_bytes_are_pinned`
   (application), `a_structural_preimage_is_pinned` and node_key/tests.rs:532
   (aggregate), and `operation_and_literal_bytes_are_pinned` pin
   hand-written bytes. The binding term is checked field by field in
   check/lowering/tests.rs:1384-1400 over real preimage bytes. The node-key
   body is still built by a recursive walk, which slice 1 owns (B2).
5. **`Value` encode sites.** Intake, observation, package identity,
   node_key/shape.rs and the replay `call_site` test read through `read()`.
   `Rfc8785Numbers`, `ByteScan` and the UTF-16 helpers are gone. No
   `quire_canonical` encode call over `serde_json::Value` is left in the tree.
   The shape.rs rewrite matches the deleted `serde_json` version: the
   placeholder is detected the same way and its other members are not
   entered, ordinals are collected in canonical order, and the anonymous
   shape drops top-level `owner` and writes `declaration` as `null`.
   `Preimage` always writes `declaration`, so this matches the old insert.
6. **ReadError mapping.** As described: intake `Malformed` maps to
   `malformed-declaration` at `$` with the offset, `Limit` to `input_bytes`,
   and `Allocation` to the new `IntakeLimit::Memory`. Package identity maps
   to `Malformed { offset }` or `Unreadable`. Observation maps `Malformed` to
   the raw digest and the rest to `memory-exhausted`. `ReadError` is
   `#[non_exhaustive]`, so the catch-all arms are needed. Three of these names
   are not catalogued (FND-001 to FND-003).
7. **Simulation.** `TransitionSystem::TransitionId` and `Key` are bound on
   `Encode`. The only implementers are in qsl-eval/tests.

## Verdict

Changes requested: three medium and three low findings. Apart from the IR
wait, the PR is not mergeable until the team-leader ruling on new refusal
names is applied (FND-001 to FND-003). The encoding work itself is sound.
The node-key and shape rewrites are byte-equivalent to what they replace and
pinned by independent bytes. The FixedShape hand-impls are honest.
`tc_730_a_lone_low_surrogate_refuses_at_its_byte_offset` has an independent
oracle: it runs on a 512 KiB stack and takes the offset from `text.find`.

Gates: one focused run under the build lock, `cargo test -p qsl-semantics`
filtered to intake, package_identity, value::member, node_key, l08_, the
second-selection test, big_integers_canonicalize and model::observation. It
passed: 114 lib tests and 11 integration tests, 0 failed. That includes
tc_730, the_one_parse, the three pinned node-key byte tests, l08, the
big-integer golden vectors and canonicity_admits_a_non_ascii_member_name.
Everything else was checked by reading. Crates above qsl-package were not built: they cannot compile until
IR lands.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `IntakeLimit::Memory` (`limit: "memory"` under `resource_exhausted`/`intake-limit-exceeded`) is not catalogued. FR-056, FR-260 and FR-255 name only the input-bytes limit, plus FR-056-AC-2's depth limit, and FR-255 gives each limit a setting. A failed heap reservation is not a limit and has no setting, and the cause's `bound` field here holds the bytes requested, not a bound. This breaks the team-leader ruling that every refusal uses catalogued codes and causes. Fix: add an allocation-failure outcome to the spec catalog in this PR, one cause shared with observation (FND-002) and package identity (FND-003), for example in FR-259 B4 and FR-056/FR-260. Give it its own `ModelRefusalCause` variant with a `requested` field instead of overloading `IntakeLimitExceeded`. | qsl-semantics/src/model/refusal.rs:711-713; qsl-semantics/src/model/refusal.rs:722; qsl-semantics/src/model/intake.rs:396-407 |
| FND-002 | medium | Observation's `resource_exhausted`/`memory-exhausted` is not catalogued. FR-106 check 1 lists only `missing-required-artifact`, `input-bytes-exceeded` and `content-mismatch` before the member checks, and no other spec or doc names `memory-exhausted`. The cause is a string literal made up at the call site. Fix: catalogue the shared allocation outcome from FND-001 in FR-106 check 1 and FR-261 B2 in this PR, and use that one name here. | qsl-semantics/src/model/observation.rs:542-547; qsl-semantics/src/model/observation.rs:551; qsl-semantics/src/model/observation.rs:555 |
| FND-003 | medium | `PreimageDefect::Unreadable(quire_canonical::ReadError)` is not in FR-261, whose B2 names only the malformed-input and limit outcomes. Through `LibraryRefusal::code`/`cause` (library/mod.rs:449-455 and 471-475) it reports as `invalid_package`/invalid-value, so a memory failure looks like a defective package. Its payload is a foreign `#[non_exhaustive]` error that can also hold `Malformed` or `Limit`, although its doc says it means want of memory. The canonicity encode's allocation failure at line 402 still becomes `NonCanonical`, so one failure has two defect names. Fix: catalogue the allocation outcome (FND-001) and map it to `resource_exhausted`. Type it as `{ requested: usize }`. Send line 402's `Error::Allocation` to it too, instead of `.ok()` folding it into `NonCanonical`. | qsl-semantics/src/library/package_identity.rs:68-72; qsl-semantics/src/library/package_identity.rs:389-395; qsl-semantics/src/library/package_identity.rs:402 |
| FND-004 | low | Observation digests under `IDENTITY_LIMITS` and turns every encode error into `memory-exhausted`, `Error::Limit` included. The read's `Err(_)` arm does the same for `ReadError::Limit`. Neither can happen today (u64::MAX). But FR-259 B3 gives `IDENTITY_LIMITS` a 16 MiB default and says a site with its own stage budget passes that budget. Once that lands, an observation document over 16 MiB of canonical text would refuse as memory-exhausted, against FR-259 B4/AC-2 (a byte error is the site's input-bytes outcome). Fix: encode under `Limits::new(u64::MAX)`, since the doc comment at lines 531-533 says the bytes are already held to `observation.input_bytes`. Match `Allocation` explicitly, and send any `Limit` to FR-106 check 1.2's `stage_limit_exceeded`/`input-bytes-exceeded`. | qsl-semantics/src/model/observation.rs:549-555 |
| FND-005 | low | `SemanticTypePreimage::Group(PreimageLeaf<'a>)` can hold any leaf (`Literal`, `Frame`, `DependencyReference`, ...), but its doc says it is "always a `PreimageLeaf::GroupReference`". The type does not state the invariant, so a wrong leaf in `semantic_type` would compile and be keyed. Fix: give `Group` the one shape it can have, for example a `GroupReference { ordinal }` wire struct written with `term: "group_reference"`. | qsl-semantics/src/check/node_key/mod.rs:1487-1493 |
| FND-006 | low | Test oracle: in `the_one_parse_reads_what_serde_json_reads`, `assert_eq!(canonical(document.tree()), canonical(&expected))` comes right after `assert_eq!(document.tree(), &expected)`. It is the same function over equal inputs, so it cannot fail. The production digest now comes from the reader's tree (intake.rs:327), not from `tree`, and this test never reaches it. So the doc claim that the JCS bytes "equal what `serde_json::from_slice` reads" is unchecked for -0, 1e-400, 5e-324, 4.9e-324, the long fraction and the escapes. The golden vectors cover only the big integers. Fix: assert `document.jcs_digest() == digest_of(&canonical(&expected))`. | qsl-semantics/src/model/intake.rs:5569 |

## New findings (disposition pass 1)

Round 1, reviewed at 7f5ded6050a87f07920fb65c18eda2afee730b2f.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | low | `read_defect`'s catch-all reports every other `ReadError` as `PreimageDefect::Malformed { offset: 0 }`, so it makes up a byte offset. It cannot happen today: the read's limit is u64::MAX, so `Limit` never occurs, and no other variant exists. But `ReadError` is `#[non_exhaustive]`, so a future variant would be reported as malformed input at byte 0. Intake's `read_refusal` handles the same arm honestly ("could not be read: {other}", no offset). Fix: give the remainder a defect that claims no offset, or match `Limit` explicitly and leave a message-only remainder, as intake does. | qsl-semantics/src/library/package_identity.rs:388-390 |
| FND-008 | low | Two test doc comments still say the repeated-name document "refuses `content-mismatch`", but their tests assert `byte-digest-mismatch` (`ModelRefusalCause::ByteDigestMismatch`, and the observation record's cause). The rename commits 0284fa39b and 7f5ded605 changed only spec files. Fix: change both comments to `byte-digest-mismatch`. | qsl-semantics/src/model/intake.rs:5599-5604; qsl-semantics/src/model/observation.rs:604-608 |

## New findings (disposition pass 2)

Round 2, reviewed at 7420d36d86d4901cca21d0a5a213687a175e1348. New scope this round:
- b460fca63, the IR move to 9631d7e;
- 2cb854bfd, the IR-wait markers removed;
- 555f921e9, A3k's cherry-picked CyclicEquality gap retirement;
- d632cb13c, QTY1/QSL-247;
- 7420d36d8, TC-186 status.

No new code-review findings:
- **Lock.** Cargo.lock has one quire-canonical entry (59fe4f06) and one quire-canonical-derive entry. quire-contract-model at 9631d7e depends on that same entry, and QVC stays at ead78f3. deny.toml sets `multiple-versions = "allow"`, so the one-copy rule holds by the lock alone, not by a deny ban.
- **IR-wait sites.** 2cb854bfd only deletes the three markers and adds a doc line on `WireV2`'s `Encode`. QSL gains no shim, wrapper or literal `DEPTH`. IR's own `Encode` for the three `Value`-holding types writes from an explicit heap stack (quire-contract-model/src/checked_package/v2/encode.rs at 9631d7e). `package_id_matches_its_golden_vector` passes unedited.
- **Admission corpus.** In 555f921e9, the CyclicEquality row now asserts admission at the emitted `package_id`, with an `expression`/`binary` node applying `quire.op.structural.eq`. The zip of `graph().nodes` with `node_kinds()` relies on IR's documented contract that both are "in graph order". Classification still counts each family once. FR-093-AC-19 and TC-416 step 11 drop the STD-129 exception to match.
- **QTY1.** d632cb13c removes the `#[ignore]` from `tc_440_quantity_extent_agrees_with_ir_requires_bound`, which passes, and updates the TC-440, FR-097 and tests.md status to match.

## Dispositions

Round 1, reviewed at 7f5ded6050a87f07920fb65c18eda2afee730b2f. The branch was rebased onto 8d1deba43 (#612). `git range-diff` shows the four reviewed commits are patch-identical, f407facfd = fedc3708f. So the fix rounds are fedc3708f..7f5ded605 (a5ef1516b, d5cd29192, d08987f74, 4a60d5bfb, 6dabe354d, 0284fa39b, 7f5ded605), and I checked those by reading. I ran no build of my own. The coder's log ~/dev/worktrees/logs/qsl-480-fixround-test.log (written 19:58, after the last code commit at 19:42; the two later commits change only spec) shows `cargo test` passing: qsl-semantics 432 lib and 250 integration tests, and quire-semantic-value 13 unit and 23 doctests, with no warnings. `quire validate` passes (exit 0) on the 10 changed spec files; its two EARS warnings at FR-106:166 predate this diff.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | d5cd29192 (spec a5ef1516b): `IntakeLimit::Memory` is gone. It is replaced by `ModelRefusalCause::AllocationFailed { requested }`, whose code and cause come from the one constant `qsl_foundation::diagnostic::ALLOCATION_FAILED` (`resource_exhausted`/`allocation-failed`). The outcome is catalogued as FR-259 B6 and stated in FR-056, FR-106 check 1, FR-260 B5 and FR-261 B2, with FR-259-AC-5 tested through TC-729 step 3. `admit` returns it early, as it does the limit refusal. |
| FND-002 | fixed | d5cd29192: observation's `allocation_failed(requested)` builds `AdmissionRecord::new(Code::ResourceExhausted, ALLOCATION_FAILED.cause()).with("requested", ...)`, and the `memory-exhausted` literal is gone. FR-106 check 1.3 states the outcome. `an_allocation_failure_refuses_allocation_failed` asserts it against the literal "allocation-failed". |
| FND-003 | fixed | d5cd29192: `Unreadable(ReadError)` is replaced by `PreimageDefect::AllocationFailed { requested }`. `LibraryRefusal` reports it as `Code::ResourceExhausted` and `LibraryCause::AllocationFailed`. The canonicity encode's `Error::Allocation` now goes to it through `encode_defect` instead of `.ok()` into `NonCanonical`. `an_allocation_failure_is_its_own_defect` asserts the code and the cause "allocation-failed". |
| FND-004 | fixed | d5cd29192: observation reads and encodes under `Limits::new(u64::MAX)`. `digest_read_refusal` and `digest_encode_refusal` send `Limit` to check 1.2's `stage_limit_exceeded`/`input-bytes-exceeded` (the same record document.rs builds) and `Allocation` to allocation-failed. A test drives a real encoder byte error (`to_vec("over", Limits::new(1))`) into `input_bytes_exceeded()`. FR-106 check 1.2 states the mapping. |
| FND-005 | fixed | d08987f74: `SemanticTypePreimage::Group(GroupReference)`, with `#[serde(tag = "term", rename = "group_reference")] struct GroupReference { ordinal }`. No other leaf fits. check/lowering/tests.rs:516-520 asserts G9's preimage `semantic_type` is `{"term": "group_reference", "ordinal": 1}` over real preimage bytes. |
| FND-006 | fixed | 4a60d5bfb: `the_one_parse_reads_what_serde_json_reads` now asserts `document.jcs_digest() == digest_of(&canonical(&expected))`, which reaches the production digest. |


Round 2, reviewed at 7420d36d86d4901cca21d0a5a213687a175e1348. I checked 7f5ded605..7420d36d8 by reading and ran no build of my own. Coder logs:
- ~/dev/worktrees/logs/qsl-480-fixround2-test.log, written 20:37: qsl-semantics 433 lib and 250 integration tests pass. The last qsl-semantics change was at 20:19, and qsl-semantics does not depend on quire-contract-model.
- ~/dev/worktrees/logs/qsl-480-ir-sites-test.log, written 21:08 after the last code commit at 20:40: qsl-package 105 and qsl-replay 265 tests pass. That includes `a_package_document_refusal_keeps_its_cause`, `package_id_matches_its_golden_vector`, `every_emitted_node_family_is_admitted_at_its_package_id` and the TC-440 quantity test.

These logs do not cover qsl-eval, qsl-bench, the root crate or the example at this head. The coordinator's `make ci` covers them.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-007 | fixed | 84888133d: new `PreimageDefect::ReaderRefused { reason }` for any refusal that is neither a located `Malformed` nor `Allocation`. It claims no offset, and its reason is the reader's own `Display`. `a_reader_refusal_with_no_offset_claims_none` drives a real `ReadError::Limit` (`read(b"[]", 1)`) into it and checks that `{` still gives `Malformed { offset: 1 }`. |
| FND-008 | fixed | 6e899fd77: both doc comments (intake.rs:5604, observation.rs:608-609) now say `byte-digest-mismatch`. No `content-mismatch` is left in any `.rs` file. |
