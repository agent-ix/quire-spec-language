---
id: SR-800
title: "Base review of ADR-017 protocol/frame, refinement and abstraction-relation boundaries"
type: SpecReview
analysis: base
scope: "spec/decisions/ADR-017-protocol-refinement-abstraction-boundaries.md, spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md, spec/decisions/ADR-012-semantic-family-extension-contracts.md, spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md, spec/spec.md"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-017
    type: reviews
---
# SR-800: Base review of ADR-017 protocol/frame, refinement and abstraction-relation boundaries

## Summary

Reviewed the uncommitted ADR-017 draft on `spec/16-arch43-mapping`, and its amendment pointers in ADR-011 §1, ADR-012 §3 and §13.5
Q210-3, ADR-013 R-06, O-08 and O-25, and `spec/spec.md`. The review applied
the base checklist as it fits an ADR: id formats, cross-references, link
validity, terminology, verifiable decisions, and catalog codes checked against
QSpec `main` (`native-diagnostics.md`, revision `1-draft.8`).

What holds:

- Local item ids (`PF-`, `RF-`, `AR-`, `G-`, `Q-`, `TK-`) are unique and are
  declared in Status.
- The `spec/spec.md` index row and `contains` edge are present. The
  relationship targets resolve. SR-800 is unused elsewhere.
- Most code locations are correct. They include `OperationEffect`
  (`model/domain_package.rs:244-253`), `CheckedOperationFrame`
  (`check/state_clause.rs:62-68`), `CheckedAttempt`
  (`check/protocol_clause.rs:91-103`), `FrameCounterexample`
  (`qsl-replay/src/witness/frame.rs:133-152`), the `WitnessEnvelope` members
  (`witness.rs:343-359`), `CheckedGraph::operation_frame`
  (`check/mod.rs:1807-1831`), the unused `check::identity` frame types, and
  `FamilyKind::Relation` (`family/mod.rs:101`). `replay_frame` reads no
  envelope selection, occurrence or clause member (`execute/frame.rs`).
- These catalog pairs exist in the catalog: `stale_dependency`/
  `revision-mismatch` and `byte-digest-mismatch`,
  `invalid_model_binding`/`wrong-model-selection`, `conflicting-binding` and
  `malformed-declaration`, `missing_declaration`/`missing-name`, and
  `frame_violation`/`unauthorized-change`.
- The Linear states cited in Context are current: IR-370 and IR-89 Done;
  IR-339, IR-32, IR-33, IR-93 and QSL-316 Backlog.

What does not hold:

- Bare FR ids break the ADR's own convention, and two of them resolve to
  unrelated QSL requirements.
- §7 lists an O-26 amendment that the diff does not make.
- `OperationName` is defined two different ways.
- Several citations point at the wrong place: a phrase quoted from ADR-013
  that ADR-013 does not contain, and the wrong callers and section numbers.
- §4 counts a test as landed, but it is `#[ignore]`d at HEAD.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Status says "A bare FR id is a QSL requirement", but the ADR cites QSpec ids bare. The FR-353 summary in Context and AR-2 ("FR-012 pre/post/result anchors") write FR-012 and FR-013 bare. In QSL those are FR-012 "audit fixtures in Rust" and FR-013 "link formal environments", not QSpec's operation observations and frames. FR-196 (AR-6), FR-301 (RF-5), FR-323 and FR-331 (AR-5, AR-6) and FR-340 (Context, PF-1) are also bare QSpec ids. Fix: prefix each one with "QSpec". | ADR-017 Status, Context, PF-1, RF-5, AR-2, AR-5, AR-6; QSL `spec/functional/FR-012-*`, `FR-013-*` |
| FND-002 | medium | §7 says "ADR-013 O-25 and O-26: the selection member is PF-4's `ReplaySelection`". The diff amends only the O-25 packet list. The O-26 public type still says "the selected function's `QualifiedName` (OQ-5 ruling)". Its Conversions cell still says it "resolves the `QualifiedName` … and calls the selected function". C-13 still says "select by `QualifiedName` lookup (OQ-5)". Fix: add the ADR-017 PF-4 amendment pointer to O-26 (public type and conversions) and to C-13, or remove O-26 from §7. | ADR-017 PF-4, §7; ADR-013 O-26, C-13 |
| FND-003 | medium | `OperationName` means two different things. PF-3 defines it as "`QualifiedName` of the object type plus the operation `Identifier`", which is the shape of `FrameOperation` (`qsl-replay/src/witness/frame.rs:33-38`). The existing `qsl_replay::spine::OperationName` (`spine/clause.rs:93-100`) and QSL FR-115 Inputs define it as `{model, object, operation}` Strings. PF-4's `ReplaySelection::Frame(OperationName)` and TK-1 could therefore mean either type. Fix: name the one type PF-3, PF-4 and TK-1 mean. If it is the spine type, say that TK-1 retypes its members to `QualifiedName` and `Identifier`. | ADR-017 PF-3, PF-4, G-1, TK-1; FR-115 Inputs |
| FND-004 | low | G-1 and TK-1 name the wrong callers. `operation_frame` has one production call site, in `resolve_frame` (`qsl-replay/src/spine/clause.rs:890`). `resolve_frame` is reached from `spine/clause.rs:867` (FR-115 run) and `execute/frame.rs:187` (FR-116 replay). The cited `execute/frame.rs:348-357` is `operation_name`, which converts `FrameOperation` into `OperationName` and formats no string. TK-1's "its three callers" matches none of this. Fix: cite `resolve_frame` as the only caller and its two entry paths, and restate TK-1's scope. | ADR-017 G-1, TK-1 |
| FND-005 | low | PF-6's last row quotes ADR-013 "prefer content to identity". ADR-013 contains no such phrase. Fix: cite the rule actually relied on, ADR-013 O-02 (`package_id` is computed from the content) and O-04 (content-addressed node ids). | ADR-017 PF-6; ADR-013 O-02, O-04 |
| FND-006 | low | RF-6 item 2 cites "ADR-011 §2.3 rule 8". Rule 8 is Decision item 8 (ADR-011 lines 140-150), a rule for proof gates, and §2.3 carries only its detail. Fix: cite "ADR-011 Decision item 8 (§2.3)", and say it is applied to a test gate by analogy. | ADR-017 RF-6; ADR-011 Decision item 8, §2.3 |
| FND-007 | low | PF-7 cites `unsupported_construct`/`not-yet-implemented` as the forced sites. Catalog `1-draft.8` gives `unsupported_construct` only the causes `declaration-form` and `expression-form`, so `not-yet-implemented` is not a catalog cause. RF-4 justifies its `unsupported` class with AD-003's "prohibited by the selected profile", but the code uses the same code for "QSL has not implemented this checker" (`check/protocol_clause.rs:409-418`). Fix: record the catalog gap and its owning ticket. State that RF-4's `unsupported` class covers both meanings on purpose. | ADR-017 PF-7, RF-4; QSpec `native-diagnostics.md` `unsupported_construct` row |
| FND-008 | low | PF-7 scenario 1 offers `compensate` as a new construct that needs "an S2 production in `forms::protocol_clause`". S2 already parses it (`ProtocolNodeKind::CompensateTemplate`, `qsl-forms/src/syntax.rs:1225`), and S3's `covered_kind` refuses it (`check/protocol_clause.rs:389-405`). Fix: use a construct that has no S2 production, or reword scenario 1 as "an S2-parsed construct gains its check": classify it in `covered_kind`, check it, and lower it. | ADR-017 PF-7 |
| FND-009 | medium | §4 lists TC-463 as landed. It is `#[ignore = "blocked on IR-370 …"]` (`qsl-replay/src/spine/clause/tests.rs:4931`), and QSL still pins quire-contract-ir, while Context says IR-370 is Done. The ticket forbids claiming unfinished behaviour. Fix: say TC-463 is ignored until the IR pin moves past IR-370, and name the ticket that moves it. | ADR-017 Context, §4 |
| FND-010 | low | AR-2 calls the operation key an "O-06 pair" but types it as (`DeclarationKey`, `Identifier`). The O-06 operation member is `operation{declaration: NodeKey, name}`, and O-06 equality uses the receiver's static type node. Fix: either key by the O-06 member (a `NodeKey`) or drop the O-06 label and cite O-03 plus the identifier. | ADR-017 AR-2; ADR-013 O-06 |
| FND-011 | low | The front matter omits artifacts that the decisions amend or depend on: QSL FR-070, FR-071, FR-098, FR-100, FR-104 and FR-109, and QSpec AD-006, FR-012, FR-013, FR-177, FR-196, FR-301 and FR-331. Fix: add `depends_on` or `relates_to` edges for them. | ADR-017 front matter, PF-3, TK-2, RF-5, AR-2, AR-6 |
| FND-012 | low | The §6 ticket-text corrections leave out one error. QSL-40 and QSL-39 (ticket text, read as data) name QSpec FR-177 as their "normative contract", but Context says FR-177 is a different claim, protocol implementation refinement. Fix: add that correction to the list for the owner. | ADR-017 Context, §6; Linear QSL-40, QSL-39 |

## Resolution

Resolved by the author on `spec/16-arch43-mapping`. FND-001: QSpec ids
prefixed throughout. FND-002: the `ReplaySelection` amendment is dropped
(PF-4, Q-4); O-26 and C-13 are unchanged, and O-25 gains a frame-packet
pointer. FND-003: PF-3 and TK-1 name one `OperationName` of three
identifiers. FND-004: G-1 and TK-1 list `resolve_frame`, its two entry paths,
`operation_name` and the test. FND-005: PF-6 cites O-02 and O-04. FND-006:
RF-5 cites ADR-011 Decision item 8. FND-007: the `not-yet-implemented` gap is
Q-6, and RF-2 classifies it apart from a profile prohibition. FND-008: PF-7
scenario 1 is an S2-parsed construct gaining its check. FND-009: Context and
§4 mark TC-463 partial; TK-4 moves the pin. FND-010: `OperationKey` cites
O-03 plus the identifier. FND-011: front matter extended. FND-012: added to
the §6 ticket-text corrections.

An independent second pass over the revision found no remaining base
defect.
