---
id: SR-202611070458152
title: "ears-conformance review of PR2481 selected-monitor subsystem specification"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language; FR-531 AC29–33 and associated new body; TC-1331 selected-monitor procedures; PR2481 415f43c97a30bf4c3a5abf32927858912157a1d1"
review_set: subset
---

## Summary

Added requirements use explicit triggers/conditions and SHALL obligations. MAY is limited to permitted borrowed access and caller resume behavior; denial requirements are unconditional within their stated phase.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Scope and evidence

No applicable enforced AssuranceProfile was found. Review covers the changed subsystem only. Existing FR531 AC28 supplies shared-budget integration context; FR251 and IR357/358 are owner boundaries, not rewritten here. Published IR943 public API was inspected. Runtime implementation, Rust review and Cargo checks are intentionally outside this SPEC-only review; absence of new runtime tests is not a finding. No cancelled IR1392–1394 rows were added. Quire validation of the changed documents: 2/2 grammar-clean, zero findings, all 33 ACs parsed.

- FR-531-AC-29 (spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md): Materializing an authentic emission's equality witness with a shared native budget preserves every application, fact and source-unit row, including original identities and order. The source admission and handoff paths use the same budgeted row builder. Exact measured work and storage allowances return the complete witness; one unit less in either resource refuses without allocating the denied material or a diagnostic, retains the first unpaid resource, amount and original locus, and returns no partial witness. Two individually affordable materializations that exceed one shared allowance refuse
- FR-531-AC-30 (spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md): Source-enum materialization and QSL-to-IR model-enum-shape and selected-collection conversion preserve original structural/declaration/member identities, canonical case order and complete Exact types on one shared native budget. Exact measured bounds admit; work or storage one short and zero storage return allocation-free compact denials at the first unpaid locus with prior consumption intact. A later conversion cannot reset a preceding adapter's charges or replace its first denial. A selected source map whose key has no representation in the target IR row returns a typed unsupported result in
- FR-531-AC-31 (spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md): An authentic emission with source records, record members and unions produces complete identity rows equal to the original producer claims after native joining and QSL materialization. The measured work and storage include both the native join and each later QSL declaration, field, record/member and union copy. Exact allowances admit; one unit below either measured allowance refuses at its first unpaid step with no partial identity rows, the original configured bound and the admitted prefix intact. Two materializations on one budget compose without reconstructing or resetting it.
- FR-531-AC-32 (spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md): A real tick-monitor plan's retained context supplies the same emission's plan, obligations, admitted order and progress association to the actual native row join and selected type-closure projection. The resulting native nodes, original declaration/member identities and Exact types equal those of that emission. Substituting another genuine emission's context, even with identical authored names, or omitting required order/progress authority returns a typed refusal and no executable context. Borrowing retained context leaves native storage consumption unchanged.
- FR-531-AC-33 (spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md): Allocation observation encloses only the adapter/conversion call, with fixture preparation and assertions outside it. A successful owned materialization performs observed allocations; a denial before its first allocation performs none, and a later denial allocates neither the denied material nor an owned diagnostic. The controls include fresh owner-produced lazy numeric payloads, values `2^127` and `-(2^127 + 1)`, and larger heap-backed values without prior warming or formatting. These values remain exact on successful copies; insufficient work/storage retains the first real row locus and orig
- FR-531 (spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md): When materializing selected-monitor rows from an `Emission`, `identity_rows`, `equality_witness` and `selected_source_enum_rows` SHALL take the pass's caller-owned `NativeJoinBudget` by mutable reference. Their shared row-building helpers SHALL use that same allowance on source-view admission and handoff writing paths.
- FR-531 (spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md): Each adapter SHALL charge its inspections and every owned allocation through Contract IR's public accounted-charge API before the corresponding inspection or allocation. The charged material includes vector and temporary-container storage, copied node and occurrence identities, paths, source-map entries, owner/type/unit payloads, enum declarations, canonical case rows and source-unit rows. A borrowed row accessor incurs no owned-storage charge.
- FR-531 (spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md): The `identity_rows` adapter SHALL charge its QSL-owned materialization in addition to the native identity join. This material includes converted source record and member rows, copies of joined declarations and fields, and copied source union rows. A successful native join SHALL NOT count as payment for those later QSL allocations. A denied materialization SHALL return no partial `ReadIdentityRows` and SHALL retain the native charges already admitted.
- FR-531 (spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md): Footprint inspection SHALL use borrowed owner data without allocating. When inspection itself requires an allocation, that allocation SHALL be charged before inspection proceeds. A fresh or lazily represented numeric payload SHALL NOT be materialized merely to calculate an uncharged footprint. Charges SHALL cover every owned numeric representation that is actually copied, including heap-backed values at the signed 128-bit boundary.
- FR-531 (spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md): These QSL materialization charges count one logical container and one unit per initialized entry, plus each copied text or numeric payload byte and its owned buffer. A shared Exact handle contributes its containing entry rather than a deep copy of the shared payload. The charge does not depend on host pointer width, allocator capacity or artifact JSON byte length.
- FR-531 (spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md): QSL's conversion of its model-enum shapes and collection rows to Contract IR's selected-monitor row types SHALL use the same budget for all copied containers and payloads. An admitted conversion SHALL preserve complete original node, declaration and member identities, canonical variant order and Exact value types. A collection that the target row contract cannot represent SHALL return a typed unsupported refusal without omitting its key or inventing a different collection kind. Collection conversion SHALL inspect only the retained row the caller selected; an unsupported map elsewhere in the em
- FR-531 (spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md): A denied adapter charge SHALL return a fixed-size typed refusal without allocating the denied material or an owned diagnostic. The refusal SHALL retain the owner's first unpaid resource, amount, original configured bound and locus, together with the admitted budget prefix. Per-row copying SHALL identify the actual retained row or caller-supplied selection locus; a package-wide container allocation MAY use the package locus. No partial witness or converted row population is authoritative after a refusal.
- FR-531 (spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md): The adapters SHALL continue the pass through the owner's accounted-charge and resume surface without reconstructing, resetting or enlarging the allowance. Materialization charges are separate from the subsequent native validation and join charges: both operations spend the same budget once for the work and owned storage each actually performs.
- FR-531 (spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md): An explicit caller resume MAY start a new phase without restoring spent allowance. An adapter SHALL NOT implicitly resume over an existing first denial. After a denied charge, another attempted charge or adapter call in that phase SHALL retain that first resource, amount and locus and SHALL perform no further inspection or allocation.
- FR-531 (spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md): When preparing a selected tick-monitor plan, QSL SHALL keep the [FR-251](./FR-251-derive-a-tick-based-monitor-plan-for-an-embedded-target.md) plan, obligations, admitted order and progress association bound to the same final emission and its original selected package/model identities. The order and progress authority remain FR-251's admitted values; selected-row adapters SHALL NOT recreate them from names, source spans, bytes, guessed node indices or tracking values.
- FR-531 (spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md): The public selected-monitor preparation entry SHALL return a typed refusal when required order or progress authority is absent or belongs to another emission. It SHALL NOT produce an executable context with an empty or invented authority value. Retained context accessors MAY borrow the already admitted values without owned-storage charges; any owned copy SHALL use the same caller budget as row materialization and native closure validation.
- FR-531 (spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md): QSL SHALL pass the complete converted rows to the actual Contract IR native join and selected type-closure projection before treating them as admitted monitor inputs. Materialization alone SHALL NOT confer native identity or execution authority. Successful composition SHALL retain the original plan, obligation, node, declaration and member identities and the native projected Exact types. A typed unsupported row SHALL remain unsupported at its requested locus rather than becoming a partial or differently typed witness.
- TC-1331 (spec/test-cases/TC-1331-evaluate-from-a-checked-package-read-from-its-bytes-alone.md): For AC-29, compare the complete equality application, fact and source-unit rows
of an authentic final emission at ample and exact measured native allowances.
Exercise its source-admission and handoff-writing paths. Repeat on one budget,
then lower each resource by one and try zero storage, asserting the typed first
unpaid kind, amount and locus, cumulative admitted prefix and absence of an
authoritative partial witness. Observe the adapter's allocations so a denied
materialization step and compact refusal cannot allocate their payloads.
- TC-1331 (spec/test-cases/TC-1331-evaluate-from-a-checked-package-read-from-its-bytes-alone.md): For AC-30, use source ordered and unordered enums, a model enum shape and
sequence, set and bounded-collection rows from one final emission. Compare
complete native rows and original Exact identities after conversion, including
canonical variant order. Compose equality, enum and collection adapters on one
budget and exercise exact, one-short and zero-storage boundaries. Preserve an
earlier first-denial locus across a later attempted conversion. A map with a key
that the target IR collection row cannot carry refuses with its typed
unsupported cause; it never becomes a keyless collection. Re-rea
- TC-1331 (spec/test-cases/TC-1331-evaluate-from-a-checked-package-read-from-its-bytes-alone.md): For AC-31, obtain an authentic emission with nonempty source records, members
and unions. Materialize its identity rows and compare every retained original
declaration, field, member, union key and ordering to the producer's claims;
then pass the rows through the actual native identity join and type projection.
Measure the native join prefix separately from the adapter's later declaration,
field, record/member and union copies, requiring their sum to equal the shared
budget's cumulative consumption. Repeat with exact allowances, one below each
resource and zero storage. A refusal returns no pa
- TC-1331 (spec/test-cases/TC-1331-evaluate-from-a-checked-package-read-from-its-bytes-alone.md): For AC-32, build two genuine tick-monitor emissions with the same authored
names but distinct original package/model identities. Through FR-251's public
plan context, obtain the first emission's plan, obligations, admitted order and
progress association. Join its converted selected rows through Contract IR and
perform the actual selected type-closure projection. Compare complete native
values, original node/declaration/member identities and Exact types. Present
the second emission's association or omit each required authority in turn;
require the typed refusal and no executable context, withou
- TC-1331 (spec/test-cases/TC-1331-evaluate-from-a-checked-package-read-from-its-bytes-alone.md): For AC-33, use a real allocation observer, warmed independently from its payload
fixtures. Prepare fresh owner-produced lazy numeric values, including `2^127`,
`-(2^127 + 1)` and larger heap-backed values, without displaying or inspecting
them before the measured call. Observe only materialization or conversion;
retain its result and perform assertions after observation ends. Require
observed allocations on a successful owned copy. Denial before the first
allocation observes none; a later denial may retain allocations admitted before
its first stop but allocates no denied payload or owned diag
