---
id: SR-13673
title: "base review of QSL-938 FR-395 selected-call requirement-item association (PR #1147)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@952dcf836b98e1e6acaabc092bf2e5214e227793; spec/functional/FR-395-associate-the-selected-call-s-requirement-items-through-the-replay-facade.md, spec/test-cases/TC-1089-the-replay-facade-associates-the-selected-call-s-requirement-items.md, spec/functional/FR-062-implement-checked-family-contract.md, spec/functional/FR-075-compute-candidates-from-registered-backends.md, spec/functional/FR-100-run-a-named-function-through-the-spine.md, spec/spec.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-395
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1089
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-062
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-100
    type: reviews
---
# SR-13673: base review of QSL-938 FR-395 selected-call requirement-item association (PR #1147)

## Summary

Ticket: QSL-938. Base checklist over `git diff origin/main...4e97aeaad6d855b125b7cbd4bbcc6870007a6db2` (6 files,
+226): new FR-395 (10 ACs) and TC-1089, FR-062's owner paragraph and
FR-062-AC-14, FR-075's identity paragraph and FR-075-AC-17, FR-100's pointer
sentence and `references` edge, and the spec index row. Spec-only diff, so
code-review and gap-analysis do not apply.

Checked against the code on origin/main, with my own probe (an uncommitted
integration test in my clone that checks fixture K through `spine::check`
and calls `spine::run` for every TC-1089 vector):

- `spine::run` (qsl-replay/src/spine/call.rs:423) returns
  `Result<(PackageId, CallOutcome), Box<RunRefusal>>`, as FR-100 pins.
- `RunRefusal` has the arms FR-395 lists; `Fault` codes `runtime_invariant`
  (FR-100 exit 30); `select` refuses empty, multi-segment, non-identifier and
  unknown names `MissingDeclaration`, and admits Boolean, integer and
  Reference results.
- `items_from_requirements` (qsl-route/src/request.rs:486) enumerates the
  records' `BTreeMap`, so an item's request index is its position in
  occurrence-key order, as FR-075/FR-395 state.
- Fixture K checks to exactly four `value-validity` records. `inc`, `sq`,
  `id` and `two` complete (4, 16, 3, 2); `sq` at `x = 0` and `x = 9`
  complete; `work_units = 0` gives `Incomplete` on `work_units` at
  `FunctionCall`; a Boolean `x` refuses `WrongValueKind { position: 0 }`;
  `nope`, ``, `inc.x`, `1inc` refuse `MissingDeclaration`; a non-parsing
  source refuses `Compile`; a pre-cancelled handle refuses `Cancelled`.
  So AC-3, AC-4, AC-5, AC-7 and AC-8 are constructible and give the stated
  result.
- The probe shows K's `x + 1` is one hash-consed node shared by `inc` and
  `sq`: `(bd63..., expression, 0)` is `inc`'s narrowed site, and ordinals 1
  and 2 are `sq`'s. This confirms the owner design is needed (the key cannot
  recover the function), and breaks AC-1's ordinals (FND-001).

Examined units: FR-395 (Description, Terms, Outputs, every Behavior bullet,
AC-1 to AC-10), TC-1089, FR-062 Owner paragraph and FR-062-AC-14, FR-075
identity paragraph and FR-075-AC-17, FR-100's new paragraph and edge, the
spec.md row. `make check-spec-validation` and
`tools/check-index-completeness.sh` exit 0 on the PR head and on main, with
the same 17 warnings and none in a changed file.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-395-AC-1 says `sq`'s two `integer.add` sites are at ordinal 0 and 1 of one node. In fixture K they are not. `inc`'s body `x + 1` and `sq`'s `x + 1` lower to the same hash-consed node (`bd63...`), and lowering numbers `inc` first. A probe that checks K gives `(bd63, expression, 0)` for `inc` (bound `Int[0, 10]`, narrow context) and ordinals 1 and 2 for `sq`. The `integer.mul` node (`b804...`) sorts before them, so `sq`'s key order is mul, add 1, add 2. A test written to the AC fails on a correct implementation. State the ordinals as 1 and 2, or drop ordinal values and say the two add members share one node with distinct ordinals, and do not list the members in a key order the digests do not give. | spec/functional/FR-395-associate-the-selected-call-s-requirement-items-through-the-replay-facade.md:140 |
| FND-002 | high | TC-1089 runs "from an integration test that reaches only the `qsl_replay` root", but steps 1 and 6 need the checked graph: step 1 writes K's items with `items_from_requirements` over the package's records, and step 6 reads each record's owner at the checked graph. The `qsl_replay` root exposes no checked graph or records: `CompiledPackage` has only `bytes()` and `package_id()` (qsl-replay/src/compile.rs:28-35) and `AdmittedCompilation` only the package and the enum context. So the procedure cannot be run as written. Split the procedure: the facade steps root-only (AC-9), and the item-pairing and owner steps through the checked graph (`spine::check` or `qsl_package`), saying which. | spec/test-cases/TC-1089-the-replay-facade-associates-the-selected-call-s-requirement-items.md:19-32 |
| FND-003 | medium | The pairing contract does not say where the consumer gets the items it pairs members with. `run_with_requirements` returns identities only, and no `qsl_replay` root entry returns records or items. The driver's `drive()` compiles separately (QSL-938), so its items come from a second compile. A request index is a position over the whole package's records, so it pairs correctly only when that compile has the same `package_id`. The spec does not state this. Say how the consumer obtains the items (and through which allowed API), and that it pairs only when that package's `package_id` equals the returned one. Otherwise a driver pairing against another build silently mis-associates items. Or have the entry return the items, which would make the result self-contained for IR-672. | spec/functional/FR-395-associate-the-selected-call-s-requirement-items-through-the-replay-facade.md:67-72 |
| FND-004 | medium | "The entry and its types are reached through the `qsl_replay` root alone". The entry's inputs and outputs are FR-100's call vocabulary: `Call`, `CallArgument`, `SpineLimits`, `CallOutcome`, `RunRefusal`, `PackageId`. Today these exist only under `qsl_replay::spine` (`qsl-replay/src/lib.rs` re-exports none of them). So the facade rule silently requires re-exporting all of FR-100's carriers at the root. An implementer who re-exports only the new types leaves consumers naming `spine::Call`, which T12-A forbids. Name the types that must be reachable from the root. | spec/functional/FR-395-associate-the-selected-call-s-requirement-items-through-the-replay-facade.md:114-118 |
| FND-005 | low | AC-6 compares `CallOutcome` with `compile_package`, which returns no outcome. It also takes different inputs (`StageLimits` and `ReplayLimits`, not `SpineLimits`, `Call` or `&Cancel`), and its `package_id()` is a `DigestRecord`, not a `PackageId`. Say that the `package_id` is compared with `compile_package`'s (as a record, under default limits) and the outcome with `run`'s only. | spec/functional/FR-395-associate-the-selected-call-s-requirement-items-through-the-replay-facade.md:145 |
| FND-006 | low | "no item has two calls" is unclear. The intended check is that no item is a member of both `inc`'s and `sq`'s lists. Say that. | spec/functional/FR-395-associate-the-selected-call-s-requirement-items-through-the-replay-facade.md:141 |

## Verdict

Not mergeable. FND-001 (AC-1's ordinals contradict the code's node
sharing) and FND-002 (TC-1089's root-only procedure cannot reach the records
it reads) block. FND-003 and FND-004 are real gaps for the driver consumer.

The association rule itself, its owner-based design, outcome independence,
the refusal list, and every other TC-1089 vector hold against the code.
Specifically: the owner design is necessary, because K's `x + 1` node is
shared across functions; the empty member list for `id` and `two` is right;
`work_units = 0` reaches `Incomplete`; the refusal arms and codes match;
package order equals request index. The QSL-655 independence statement is
consistent: settlement takes an item by key and index and needs nothing
here. The member shape (key and index) is enough identity for the driver to
settle an item once it holds that package's items (FND-003).

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | medium | AC-12 and the 'Pairing with items' SHALLs (and FR-075's new consumer SHALL) obligate a consumer outside QSL: no QSL function performs the pairing or raises the refusal. `RequirementItem` carries no `package_id`, qsl-replay does not depend on qsl-route, and the entry returns identities only. So TC-1089 step 8 would exercise code the test itself writes, a test that cannot fail on any QSL change. The cause is real (`Code::StaleDependency`, and `content-mismatch` is QSL's digest-recompute cause), so it suits a package-identity mismatch if QSL raises it. Owner-level choice: give QSL a pairing operation (for example a root `pair(&CallRequirements, package_id, &[RequirementItem])` returning the refusal, which needs a qsl-route edge or items carrying their package id), or make the rule consumer guidance and move AC-12 to the driver's spec. | spec/functional/FR-395-associate-the-selected-call-s-requirement-items-through-the-replay-facade.md:97-103, 169 |
| FND-008 | low | "Every one of these was observed on the unit by the review's probe, with package id `d181dfe3...a934`" is a build-log sentence, not system behavior. The package id also depends on the source identity, which the fixture does not state (the probe used `(a, u, git, 1)`). Drop the sentence, or state the source identity and make the id an expected value of the AC. | spec/functional/FR-395-associate-the-selected-call-s-requirement-items-through-the-replay-facade.md:153-154 |
| FND-009 | low | AC-11 says "Two `Call`s of `sq` whose arguments arrive in different orders", but `sq` has one parameter. The vector is `add` of RR-3, a separate unit. Write "Over FR-062 RR-3's unit, two `Call`s of `add` with arguments `x = 2, y = 5` and `y = 5, x = 2` are equal calls and return equal members". A probe gives both calls `Completed(7)` under one package id, with one record. | spec/functional/FR-395-associate-the-selected-call-s-requirement-items-through-the-replay-facade.md:168 |


## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-010 | low | AC-9 says a consumer reaches "FR-100's call types through the `qsl_replay` root alone". The Facade boundary list still leaves out run-signature and result types that are not on the root today. `Cancel` (the `&Cancel` argument) is not re-exported: qsl-replay/src/lib.rs has no `Cancel`. The author's note that it is already exported is wrong; `DependencyInput` and `SourceIdentity` are. `CallRefusal` and `CallIncomplete`, the payloads of `CallOutcome::Refused` and `::Incomplete`, are also missing. Add them, or say that `Cancel` comes from `quire_exact` and that the list covers the named carriers only. Nothing breaks either way; this is an AC-9 scope gap. | spec/functional/FR-395-associate-the-selected-call-s-requirement-items-through-the-replay-facade.md:126-131, 167 |


## New findings (disposition pass 3)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-011 | low | Every listed name checks against origin/main. `Call`, `CallArgument`, `CallOutcome`, `CallValue`, `CallRefusal`, `CallIncomplete` and `RunRefusal` are in qsl-replay/src/spine/call.rs. `SpineLimits` and `DependencyInput` are in spine.rs. `PackageId` is `qsl_semantics::library::PackageId`, `ObjectEnvironment` is `qsl_semantics::model::object_environment`, `SuppliedLimits` is `qsl_semantics::value::supplied`, and `Cancel`/`CancelCause` are in `quire_exact`. `DependencyInput` and `SourceIdentity` are already root re-exports. The bullet now says the root SHALL re-export "every type of `run`'s signature and result that it does not export today", and then lists them. Two types defined only under `qsl_replay::spine` are missing from that list: `CompileRefusal` (the payload of `RunRefusal::Compile`, spine.rs:105) and `CallLocus` (`CallRefusal::Record.locus`, spine/call.rs:112). So the "every" sentence is false as written, and a consumer that names either type must name `qsl_replay::spine`. Add both. The other nested payloads (`ModelRefusal`, `SuppliedStop`, `UnsupportedHelperStop`, `EnvironmentFailure`, `Location`, `CatalogCode`) come from crates other than the spine, and `Incomplete` is already on the root; alternatively scope "every" to types defined under `qsl_replay::spine`. | spec/functional/FR-395-associate-the-selected-call-s-requirement-items-through-the-replay-facade.md:126-141 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | 29f11fe9b: Over K, `inc` with `x = 3` returns one member, request index 1; `sq` with `x = 3` returns three members in ascending key order, request indices 0, 2 and 3, the `integer.mul` first and then the two `integer.add` sites at ordinals 1 and 2 of the shared `x + 1` node; the two member sets are disjoint and each key is a key of K's records. |
| FND-002 | fixed | 29f11fe9b: Test layer: an integration test in `qsl-replay/tests`, like `admitted_context_facade.rs`. The facade calls under test reach only the `qsl_replay` root. The independent oracle (the checked graph's records and their owners, and the items `items_from_requirements` writes) is read through `qsl_replay::spine` and the checked package inside the test, as `admitted_context_facade.rs` reads the spine's graph; consumer code does not. |
| FND-003 | fixed | 29f11fe9b: - **Pairing with items.** A member names its item by (`package_id`, request index). A consumer that writes items with `items_from_requirements` SHALL pair a member only with an item written from a compile of the same `package_id`, at the member's request index, whose `occurrence()` equals the member's key; for any other `package_id` the consumer SHALL refuse the pairing as `stale_dependency`/`content-mismatch` (QSpec FR-272) and SHALL NOT pair by occurrence key alone. |
| FND-004 | still-open | The root re-export list names Call, CallArgument, CallOutcome, CallValue, RunRefusal, PackageId and SpineLimits (all real names in qsl-replay/src/spine/call.rs), but omits two carriers the entry takes: `ObjectEnvironment` (run's required `&qsl_semantics::model::object_environment::ObjectEnvironment` argument) and `SuppliedLimits` (`Call.supplied` is `qsl_semantics::value::supplied::SuppliedLimits`). Neither is re-exported at the qsl_replay root today (lib.rs re-exports only ObjectEnvironmentDocument/Ref/Refusal/WriteError and read_supplied_limits), so AC-9's 'through the qsl_replay root alone' cannot hold. Add both to the list (and `Cancel`/`CancelCause` if the root is meant to be self-sufficient), or say they are reached from their owning crates. |
| FND-005 | fixed | 29f11fe9b: For the calls of AC-1, AC-3 and AC-5 over K, the returned `package_id` equals the `package_id` of `run` and of `compile_package` over the same source, models, libraries and limits, and the returned outcome equals `run`'s outcome. |
| FND-006 | fixed | 29f11fe9b: Over K, each member's request index is the position of its key in the four-record package order, and the items `items_from_requirements` writes over K's records, at indices 0 to 3, have the members' keys; each of the four items is a member of exactly one of the calls of `inc` and `sq`. |
| FND-004 | fixed | a4c01ebd3: - **Facade boundary.** The facade SHALL expose the entry and its types at the `qsl_replay` root, where a consumer reaches them, and the root SHALL re-export FR-100's call types that the entry takes or returns (`Call`, `CallArgument`, `CallOutcome`, `CallValue`, `RunRefusal`, `PackageId`, `SpineLimits`, `ObjectEnvironment` and `SuppliedLimits`) beside `run_with_requirements`, `RunWithRequirements`, `CallRequirements` and `CallRequirement`. `qsl_replay::spine` stays internal, a consumer names neither it nor `qsl_semantics`' checked graph (FR-060, T12-A), as for `compile_admitted` (FR-394), and ` |
| FND-007 | fixed | a4c01ebd3: - **Pairing key.** The facade SHALL return, with each member, the request index of its item, and with the members the `package_id` of the compile that produced them, so that the pairing key of a member and its item is (`package_id`, request index). Consumers (informative): a consumer that writes items with `items_from_requirements` pairs a member with the item at its request index, written from a compile of the same `package_id`, whose `occurrence()` equals the member's key; a different `package_id` is the consumer's refusal, which the consumer's own specification states. |
| FND-008 | fixed | a4c01ebd3: K has four records, in package order: the `integer.mul` of `sq` (request index 0); the `x + 1` node's ordinal 0, which is `inc`'s `integer.add` with result bound `Int[0, 10]` (index 1); and the same node's ordinals 1 and 2, which are `sq`'s two `integer.add` sites (indices 2 and 3). The `id` and `two` functions have none. |
| FND-009 | fixed | a4c01ebd3: Two `Call`s of `add` of FR-062 RR-3 (`function add using v(x: Int[0, 9], y: Int[0, 9]): Int[0, 18] pure { x + y }`) whose arguments arrive as `x`, `y` and as `y`, `x` are equal calls and return equal members. |
| FND-010 | fixed | 952dcf836: - **Facade boundary.** The facade SHALL expose the entry and its result types (`run_with_requirements`, `RunWithRequirements`, `CallRequirements` and `CallRequirement`) at the `qsl_replay` root, where a consumer reaches them. The root SHALL also re-export every type of `run`'s signature and result that it does not export today, so that a consumer calls the entry and reads its result without naming `qsl_replay::spine`: `Call`, `CallArgument`, `CallOutcome`, `CallValue`, `CallRefusal` and `CallIncomplete` (the payloads of `CallOutcome::Refused` and `::Incomplete`), `RunRefusal`, `SpineLimits`, ` |
