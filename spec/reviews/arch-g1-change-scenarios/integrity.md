---
id: SR-501
title: "Integrity analysis of ADR-010 to ADR-013 (ARCH-G1)"
type: SpecReview
analysis: integrity
scope: "spec/decisions/ADR-010-observed-architecture-baseline.md, spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md, spec/decisions/ADR-012-semantic-family-extension-contracts.md, spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: reviews
---
# SR-501: Integrity analysis of ADR-010 to ADR-013 (ARCH-G1)

## Summary

Reviewed the four records together on `task/212-arch-g1-gate`, including the
uncommitted ADR-011 edits (#245 proof rule and the O-03 alignment of E3 and
§10 row 8). The lens is cross-record consistency: contradictions between the
records, and agreement on identity and ownership. Line anchors are to the
working tree. The gate record SR-498 already holds MD-1, MD-2 and its
FND-001 to FND-007. They are not repeated here.

What holds:
- ADR-010 routing matches the three decision records. ADR-011 decides the 19
  #209 findings and its three secondaries. ADR-012 decides the six #210
  findings, DA-11 and L1-D1. ADR-013 decides 16 findings and 17 DA items as
  primary owner and five findings as secondary owner.
- `DeclarationKey` ownership now agrees: the domain package assigns it and
  QSL mints node ids from it (ADR-011:320, 818; ADR-013:152, 678).
- `PackageNodeKey{package: package_id, node: WireNodeId}` has one shape
  (ADR-011:195, 320; ADR-013:662). Only E4 and the `replay` facade turn a
  `WireNodeId` into a `NodeKey` (ADR-011:276-277; ADR-012:867; ADR-013:170).
- The I2 types, the checked typestate producers and R-10 agree
  (ADR-011:105-110, 436-449; ADR-013:93, 660).
- `FamilyOutcome` and `FamilyRefusal` sit in the layer-3 `check` core in all
  three records (ADR-011:337-339; ADR-012:245-255; ADR-013:370-376).
- The canonical clause kind sits in the `check` core (ADR-012:390;
  ADR-013:244), except one stale row (FND-011).
- `BackendId` on the wire, the manifest as descriptor, and T-7 data crossing
  agree (ADR-011:292-297; ADR-012:489-495, 794; ADR-013:452-468, 666).
- The SR-468 round-3 findings FND-022 to FND-025 are fixed in the text.

The defects:
- **FB-10 contradicts the new §2.3 rule (FND-001).** FB-10 forbids any proof
  whose expectation shares a helper with the code under proof. Decision 8 and
  §2.3 rule 3 allow a shared helper when its mutation fails the proof. The
  gate's scenario 1 evidence uses the second reading.
- **Kernel and refusal ownership disagree (FND-003, FND-004).** ADR-011 puts
  `CatalogCode` in the kernel; ADR-013 puts it in F. ADR-012 says the shared
  refusal part is the kernel `Refusal`; ADR-013 says it is `RefusalRecord`.
- **A limit at S3 has two types (FND-005).** ADR-012 returns `Incomplete`.
  ADR-011 and ADR-013 return `StageFailure::Limit`.
- **The #231 envelope types have no module (FND-006).** ADR-013 makes CG use
  a QSL envelope type. ADR-011 lets CG use only the `replay` facade.
- **O-20 settles every unbounded claim `requires-bound` (FND-007).** ADR-012
  §1.1 settles it `unsupported` when no finite bound exists.

Verdict: **ACCEPT WITH FINDINGS.** FND-001 is blocking: under FB-10 as
written, the evidence the gate records for scenario 1 is a forbidden bypass.
It is an editorial fix to ADR-011 in this PR. FND-002 to FND-007 (medium)
do not change a gate verdict and should be fixed before the records are
accepted. FND-008 to FND-012 (low) may follow.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Blocking. FB-10 contradicts Decision 8 and §2.3 rule 3. FB-10 forbids a gate that discharges "only propositions whose expectation shares a helper with the code under proof". Decision 8 and rule 3 allow a shared helper when a run mutation of it fails the proof. The Consequences bullet asks for an expectation "independent of `src/exact/`", a third reading. SR-498 scenario 1 lists shared helpers (kernel `Value` decode, rational normalization) with mutation evidence, so under FB-10 its evidence is forbidden. **Fix:** FB-10 Forbidden cell: "A proof gate claims a module over which it discharges no proposition, or discharges propositions whose expectation shares a helper with the code under proof without a run mutation of that helper that fails the proof". Consequences: "a harness whose expectation is derived independently of the `src/exact/` function under proof, with a run mutation of each shared helper that fails the proof, and a mutation control there." | ADR-011:134-143, 390-391, 423, 945-946 · SR-498 scenario 1 |
| FND-002 | medium | T-10 does not carry the #245 rule. This PR restates the rule in §1.1, §2.3, FB-10, §9 and Consequences. T-10, the CG harness-gate ticket (CG #88), still lists only the claimed-module list, `unreached` and the mutation control. CG #88 would build a gate without the SUCCESS-only floor or the shared-helper mutation. **Fix:** T-10: "CG generated-harness gate under §2.3: claimed-module list, `unreached` failure, SUCCESS-only discharge floor, mutation control, and a run mutation of each helper shared between oracle and code under proof". | ADR-011:384-391, 926 · SR-498 scenario 1 Tickets |
| FND-003 | medium | ADR-011 misstates the kernel contents that ADR-013 decides. ADR-011 says `diagnostic::Code` "becomes a kernel `CatalogCode`". ADR-013 T-6, O-17 and slice S-1 say `CatalogCode` and the category are F `diagnostic` types, not kernel types. The ADR-011 K row claims the AD-016 row "as amended by QC-15" but lists the pre-amendment types. It omits `EffectiveId`, `UniverseId`, `ObjectId`, `UnitId`, `VariantId` and `MemberId`, although ADR-011:594 names the kernel `EffectiveId` constructor. **Fix:** ADR-011:551-552: "and `diagnostic::Code` leaves: the kernel `Refusal` carries the kernel's own typed cause, and F `diagnostic` maps it to a `CatalogCode`". Add the six QC-15 types to the K row. | ADR-011:513, 549-552, 594 · ADR-013:411-412, 665, 791, 825 |
| FND-004 | medium | The shared refusal part has two owners. ADR-012 §2 and §13.2 Q3 say "the shared part is the kernel `Refusal` (O-17)". ADR-013 O-17, the answer to that question, says the shared part is `RefusalRecord` in F `diagnostic`, and the kernel `Refusal` carries only kernel causes. ADR-012 claims to adopt ADR-013's answer and misquotes it. **Fix:** ADR-012:214 and :829: "the shared part is `RefusalRecord` in F `diagnostic` (O-17); the kernel `Refusal` carries kernel causes only". | ADR-012:214, 829 · ADR-013:403-413 |
| FND-005 | medium | A limit reached during checking has two result types and two categories. ADR-012 has a family `check` return `Incomplete` "when a limit or the meter is exhausted", category `incomplete`. ADR-011 §2.3 and ADR-013 T-4 have S1 to S4 return `StageFailure::Limit(LimitExceeded)`, which ADR-011 calls a "limit refusal" with its own exit code. O-16 has no row for `LimitExceeded`, although it says every source value has one. O-21 adds a third rule: an exhausted meter yields `Incomplete`, but the S3 `CheckContext` meter is also a work budget under T-4. Separately, T-4 says "S6a keeps the kernel `Outcome<T>`", while O-16 and ADR-011 say S6a returns `FamilyOutcome`. **Fix:** decide in ADR-013 T-4 and O-16 which one S3 uses and its category. Align ADR-012 §2, the `check` signature and §13.5 Q210-3 to it. In T-4, say "S6a returns `FamilyOutcome`, whose `Evaluated` arm keeps the kernel `Outcome<T>`". | ADR-012:212, 214, 225-226, 626, 868 · ADR-011:333-339, 356-362, 490 · ADR-013:350, 370, 511-512, 663 |
| FND-006 | medium | The #231 envelope types have no module, and CG's use of them meets FB-05 under one reading only. ADR-013 makes the #231 counterexample envelope, replay request and proof-result envelope QSL-side types, and says "CG #50 uses #231's counterexample envelope". ADR-011 names #231 only as a ticket and places no module for these types in §6.1 or §6.2. FB-05 and T-12 (a) allow CG only the public API of the layer-6 `replay` module. §10 row 7 says a new backend "returns typed outcomes in #231 envelopes" and, in the same row, must not "depend on any QSL API other than the `replay` facade". Either the envelopes are part of the `replay` public API, or CG handles only the FR-331 and FR-323 wire forms (C-12, C-23). The records do not say which. **Fix:** ADR-011 §6.1 places the #231 types, and ADR-013 O-24 to O-26 and §7 CG #50 state whether CG reaches them as `replay` API or as wire. | ADR-011:77, 418, 523, 817, 928 · ADR-013:560-562, 633-637, 688, 699, 779, 783 |
| FND-007 | medium | ADR-013 O-20, O-21 and C-25 contradict ADR-012 §1.1. O-20 and O-21 say "An unbounded claim settles `requires-bound`", and C-25's test expects `requires-bound` for an unbounded domain. ADR-012 §1.1 settles `requires-bound` only when a finite bound is available, `unsupported` when none is, and the form's own disposition when the candidate advertises unbounded mode. ADR-011 §10 row 6 and SR-498 scenario 5 follow ADR-012. **Fix:** O-20 Validation: "IR `requires-bound` is the single predicate (AD-016). CG `negotiate_*` settles an unbounded claim by the ADR-012 §1.1 rules: `requires-bound` when a finite bound is available, `unsupported` with a warning when none is." C-25 test: "CG test with an unbounded domain and an available bound returning `requires-bound`, and with none returning `unsupported`". | ADR-013:476, 481, 512-513, 701 · ADR-012:184-192 · ADR-011:816 |
| FND-008 | low | The discharge floor has two readings. Rule 1 says the floor "counts SUCCESS checks only; UNREACHABLE checks are subtracted". If only SUCCESS counts, nothing is left to subtract. "Subtracted" suggests total checks minus UNREACHABLE, which would still count FAILURE and UNDETERMINED. **Fix:** "The floor is the number of checks whose status is SUCCESS. A check whose status is UNREACHABLE is not discharged." Apply the same wording at ADR-011:139-141. | ADR-011:139-141, 384-387 |
| FND-009 | low | ADR-011 §1 says "No stage after S3 re-resolves a name". E9 resolves the replay `QualifiedName` by name lookup in the recompiled package, and ADR-013 R-06 names that lookup as its one exception. This is the remainder of SR-468 FND-009. **Fix:** "No stage after S3 re-resolves a name, except the replay `QualifiedName` lookup at E9 (ADR-013 R-06)." | ADR-011:209, 272-275 · ADR-013:89 |
| FND-010 | low | ADR-011 still describes the packet with a typed `Witness` in two places. E8 and ADR-013 O-25 and QC-20 carry `source: ReplaySource`, which is `Witness` or `Input`. The S7 stage row and §10 row 9 say "with a typed `Witness`" and "runs S6a on the decoded witness". **Fix:** S7: "`CounterexamplePacket` with a `ReplaySource`". Row 9: "E8 builds the packet with its `ReplaySource`", and "runs S6a on the decoded witness or the stored input". | ADR-011:187, 249, 819 · ADR-013:577, 830 |
| FND-011 | low | ADR-013 §9 DA-08 says "one checked clause kind in `value::expression`". O-10 in the same record, and ADR-012 S5, put it in the layer-3 `check` core. **Fix:** "O-10: one checked clause kind in the layer-3 `check` core." | ADR-013:244, 912 · ADR-012:390 |
| FND-012 | low | ADR-012's Identity part leaves out the package scope. It says node identity is "derived from content and path alone". ADR-013 O-04 and QC-18, and ADR-011 E3, put the declaring package's `name@version` in the preimage, so a bare `NodeKey` is unique across packages. Read alone, ADR-012 gives two packages equal ids for equal paths. **Fix:** "derived from its normalized content, its declaration path and the declaring package's `name@version` (ADR-013 O-04, QC-18)". | ADR-012:210 · ADR-013:174, 828 · ADR-011:320 |

## Method

- **Scope.** The four records read together at the working tree, with the
  ADR-011 diff against `HEAD`. ADR-010 is the evidence baseline, so it was
  checked for routing and counts, not for design.
- **Routing.** The ADR-010 §9.2 owner tally and §9.3 table were compared with
  ADR-011 Context and §9, ADR-012 Context and §10 to §11, and ADR-013 Context
  and §9. Counts and ids match.
- **Identity and ownership.** Each ADR-013 O and T row that ADR-011 or ADR-012
  cites was compared with the citing cell: O-03, O-04, O-10, O-15 to O-17,
  O-20, O-24 to O-27, T-1 to T-8, and QC-15, QC-18 and QC-20.
- **Proof rule.** The new Decision 8 text was compared with every place that
  states or relies on it: §1.1, §2.3, FB-10, §9, T-10, Consequences, and
  SR-498 scenario 1.
- **Gate record.** Each finding was checked against SR-498. Only FND-001
  makes a gate cell wrong. MD-1, MD-2 and the gate's FND-001 to FND-007 are
  not repeated.
- **Compatibility.** No fix proposed here adds a shim, fallback or
  migration window.
- **Not applied.** US → FR → StR traceability and the EARS and
  hidden-assumption probes do not apply to design records with no
  requirement statements.

## Round 2

- FND-001 is resolved: FB-10 now forbids a shared helper only when no run
  mutation of it fails the proof, matching Decision 8 and §2.3 rule 3. The
  Consequences bullet now says "derived independently of the function under
  proof in `src/exact/`".
- FND-002 is resolved: T-10 carries the SUCCESS-only floor, the shared-helper
  list and the helper mutation.
- FND-007 is resolved: ADR-013 O-20, O-21 and C-25 follow ADR-012 §1.1.
- FND-008 is resolved: the counting sentence says no status but SUCCESS is
  counted.

Verdict after Round 2: ACCEPT WITH FINDINGS.

## Round 3

Dispositions after the #212 rulings, 2026-09-19 (issue #212, newest two comments).

- FND-003: the `CatalogCode` part is resolved (#212 rulings, 2026-09-19, SR-501 FND-003): ADR-011
  now follows ADR-013 T-6, with `CatalogCode` in F `diagnostic`. The K row's
  missing QC-15 id types are Remaining work: #247.
- FND-004 is Remaining work: #247.
- FND-005 is resolved (#212 rulings, 2026-09-19, SR-500 FND-007): a `check` limit is
  `StageFailure::Limit(LimitExceeded)`, category `incomplete`, and
  `Incomplete` is an S6a outcome only.
- FND-006 is Remaining work: #231.
- FND-009 and FND-010 are Remaining work: #247.
- FND-011 is resolved (#212 rulings, 2026-09-19, FND-014): DA-08 names the layer-3 `check` core.
- FND-012 is resolved (#212 rulings, 2026-09-19, SR-500 FND-002): ADR-012 §2 says the preimage
  includes the declaring package's `name@version`.
- FND-001, FND-002, FND-007 and FND-008 were resolved in Round 2.

Verdict after Round 3: ACCEPT WITH FINDINGS.

## Round 4

Dispositions after the #212 round-2 rulings, 2026-09-19 (issue #212, round-2
comment), and the #247 editorial alignments.

- FND-003 is resolved (#247): the ADR-011 K row lists `EffectiveId`,
  `UniverseId`, `ObjectId`, `UnitId`, `VariantId` and `MemberId` (QC-15).
- FND-004 is resolved (#247): ADR-012 §2 and §13.2 Q3 name `RefusalRecord`
  in F `diagnostic` as the shared part (O-17); the kernel `Refusal` carries
  kernel causes only.
- FND-006 is resolved (#212 round-2 ruling): the #231 envelopes live in the
  layer-6 `replay` module and are part of its public API. CG reaches them
  only through it (FB-05).
- FND-009 is resolved (#247): ADR-011 §1 names the replay `QualifiedName`
  lookup at E9 as the exception, as R-06 does.
- FND-010 is resolved (#247): the S7 row and §10 row 9 name `ReplaySource`.

Verdict after Round 4: ACCEPT.
