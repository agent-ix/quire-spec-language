---
id: SR-13673
title: "base review of QSL-938 FR-395 selected-call requirement-item association (PR #1147)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@4e97aeaad6d855b125b7cbd4bbcc6870007a6db2; spec/functional/FR-395-associate-the-selected-call-s-requirement-items-through-the-replay-facade.md, spec/test-cases/TC-1089-the-replay-facade-associates-the-selected-call-s-requirement-items.md, spec/functional/FR-062-implement-checked-family-contract.md, spec/functional/FR-075-compute-candidates-from-registered-backends.md, spec/functional/FR-100-run-a-named-function-through-the-spine.md, spec/spec.md"
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
