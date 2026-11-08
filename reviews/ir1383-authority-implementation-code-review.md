---
id: SR-202611070458156
title: "IR-1383 authority code-review"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; IR-1383 IR-1388 IR-1389 QSL-1599 PR2489 combined full origin/main...HEAD, including PR2542"
review_set: subset
---

## Summary

Reviewed the implementation at 818ff2bcc9c20208c530d2cbb43a2ee994414248 against the merged FR-251/TC-1081 authority contract and actual observation PR49 API. Original native errors, structural NodeIdentity/ClaimOccurrence and owner View values are retained; no mirror authority or document-byte substitute was found.

## Verdict

**CONDITIONAL** — Fix each recorded finding and qualify genuine positive admission before merge. The code lead owns the one final aggregate gate.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | A correct existing claim with a different independently selected observation source is filtered out before check_time_source_fields and returns Selection(Claim). The public Source code documents this actual mismatch but is unreachable through admit_authority. Distinguish no matching claim from a matching claim whose observation source differs, and assert exact Selection(Source). | qsl-analyze/src/monitor_root/progress.rs:390-403 |

## Coverage

Plan completion: not assessed

Rust review covers API ownership, read-only PlanRoot/AdmittedPlanAuthority, typed errors, finite reader/canonical limits, structural node/occurrence equality, original versus final-emission joins, public getters and complete owner value retention. QSL SourceIdentity and observation Identity are deliberately distinct domains and are independently selected. The final package is strictly read from this immutable root's own final emission/evidence; its AdmissionBinding cannot be replaced through a public setter. Source and test changes contain no unsafe or CI weakening. First-party dependency manifests remain branch=main. No applicable AssuranceProfile was found.

The positive fixture genuinely constructs a StaticProducerBundle, uses owner admission, canonical record/population identities, derives and strict-reads progress before supplying View. Runtime success still requires the positive tests to pass; no owner capability is inferred from a name, digest, arbitrary JSON or component-only comparison.

Computed matrix was attempted with a 30-second bound; result is recorded separately. Manual criterion/test/code correspondence checks direct paired trace tags. No whole-repository completion or computed coverage credit is asserted. No spec/plan file is changed relative to current main, so spec-review methods are not applicable; merged FR-251/TC-1081 are the context.

## Validation evidence

Coder compile/check passed in /tmp/IR-1383-authority-check.log. At the original runtime checkpoint /tmp/IR-1383-authority-tests.log reports three passing negatives and two genuine positive failures at strict IR admission. No reviewer Cargo build or aggregate gate was started in the shared ticket worktree. Later fixes require this reviewer's explicit disposition.

## Examined scope

- `FR-251-AC-45` — `spec/functional/FR-251-derive-a-tick-based-monitor-plan-for-an-embedded-target.md`: From a genuine authored tick-monitor unit, the public plan-root handoff returns the complete QSL-admitted model-step order and the observation owner's original canonical association VALUE, bound to the same final emission as its plan and two timing obligations. A public consumer compares the complete values and original source, structural clause node, claim role and ordinal, selected subject and checked model-time facts; the final emission temporal binding join equals its plan-root clause and the strict reader declaration. Both values and owner capabilities remain usable after temporary compil
- `FR-251-AC-46` — `spec/functional/FR-251-derive-a-tick-based-monitor-plan-for-an-embedded-target.md`: Independently omit the checked order premise and progress association; substitute a foreign admitted emission; change the structural clause node, claim role, claim ordinal or original source; change the selected subject, observation source, clock identity/revision, model, structural model/unit/dimension/digital-operation node, unit, source kind, digital operation or exact period; and substitute order or progress values. Each candidate returns its typed missing-premise, owner refusal or mismatched-selection condition and no partial admitted context. Exact owner canonical-association byte bounds
- `FR-251-AC-47` — `spec/functional/FR-251-derive-a-tick-based-monitor-plan-for-an-embedded-target.md`: After original source, model and request inputs are dropped, a public consumer reads the actual combined checked unit retained by the plan root: its exact source includes the authored unit and generated entries, its selected clause equals the plan subject, and its checked time source retains the selected model node, source kind and unit. These are explicitly original checker facts, not final renamed identities or admitted order/progress.
- `FR-251-AC-48` — `spec/functional/FR-251-derive-a-tick-based-monitor-plan-for-an-embedded-target.md`: A genuine dense model-time monitor root yields the complete specified canonical model-step value, with the actual checked model/unit/dimension identities. A digital source retains its exact period and checked tick-operation anchor. Changed source kind, unit or period changes the value. Exact-byte-ceiling encoding succeeds and one byte less returns Encoding with the original native canonical limit and no partial order value.
- `Cargo.lock` — `Cargo.lock`: Reviewed the complete changed file and relevant callers/owner contracts.
- `qsl-analyze/Cargo.toml` — `qsl-analyze/Cargo.toml`: Reviewed the complete changed file and relevant callers/owner contracts.
- `qsl-analyze/src/monitor_root.rs` — `qsl-analyze/src/monitor_root.rs`: Reviewed the complete changed file and relevant callers/owner contracts.
- `qsl-analyze/src/monitor_root/order.rs` — `qsl-analyze/src/monitor_root/order.rs`: Reviewed the complete changed file and relevant callers/owner contracts.
- `qsl-analyze/src/monitor_root/progress.rs` — `qsl-analyze/src/monitor_root/progress.rs`: Reviewed the complete changed file and relevant callers/owner contracts.
- `qsl-analyze/src/monitor_root/progress/tests.rs` — `qsl-analyze/src/monitor_root/progress/tests.rs`: Reviewed the complete changed file and relevant callers/owner contracts.
- `qsl-analyze/tests/it/main.rs` — `qsl-analyze/tests/it/main.rs`: Reviewed the complete changed file and relevant callers/owner contracts.
- `qsl-analyze/tests/it/monitor_authority.rs` — `qsl-analyze/tests/it/monitor_authority.rs`: Reviewed the complete changed file and relevant callers/owner contracts.
- `qsl-analyze/tests/it/monitor_authority/observation.rs` — `qsl-analyze/tests/it/monitor_authority/observation.rs`: Reviewed the complete changed file and relevant callers/owner contracts.
- `qsl-analyze/tests/it/monitor_root.rs` — `qsl-analyze/tests/it/monitor_root.rs`: Reviewed the complete changed file and relevant callers/owner contracts.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The source mismatch is distinguished from absent claim before canonical authority issuance. The genuine independent selector control asserts exact Selection(Source) and passes. |

## Current outcome

**PASS** on c7368900224daca67789bc6c03f8b25e4b9df868 — All original findings are fixed. Re-reviewed the full implementation and the new writer evidence delta. The final context consumes one immutable original PlanRoot and retains that exact final emission, original checked unit, actual strict-reader package, QSL model-step value, original observation View and its complete bounded canonical association value. QSL source and observation source are distinct domains; no equality or invented field conflates them. Full structural node and actual claim occurrence fields are compared, then the retained temporal row joins to the strict reader's final clause. No mirror authority, synthetic evidence, source re-emission or arbitrary document-byte substitution is introduced.

The producer evidence fix clones the actual CheckedPackageEvidence supplied to the original strict base read, preserving admitted domain/dependency capabilities. The private writer's final package serializes the same base lock, so supporting base.lock.required_features at this boundary names precisely that final lock; no additional fabricated record or authority is synthesized. New strict reads use only returned emission evidence.

## Final focused evidence

/tmp/IR-1383-authority-tests.log: seven genuine authority tests pass (0 failures, 8.62s), including real tick positive/full owner values, dense/tick retained evidence without reinjection, isolated independent claim controls, exact source diagnostic, foreign/missing selections and canonical exact/one-short bounds. /tmp/IR-1383-authority-clippy.log: qsl-analyze library/test-target Clippy passes with warnings denied (16.72s). Read-only compile-fail documentation covers root and admitted-order immutability; the lead's final gate includes qsl-analyze documentation and the existing monitor-root suite. No duplicate reviewer Cargo or aggregate gate was run. Computed matrix timed out at 30s with no JSON (exit124); manual scoped trace review is recorded, without computed coverage credit or whole-repository completion claim.

## Additional examined scope

- `qsl-package/src/emit/monitor_plan.rs`: actual evidence retention at the publication boundary and unchanged final lock.

Additional focused results verified on the same clean c73689002 source: nineteen existing monitor-root tests, one private checked-source component test, and both immutable-context compile-fail doctests pass in the corresponding /tmp/IR-1383-authority-{existing-roots,components,docs}.log files. No further source changes or reviewer builds.

## Final combined-head verification

**PASS** on 199530b4a62b008d1e3101da0690181d5742c508 — Rechecked the composed current-main diff. The only intervening upstream delta is five unrelated specification Markdown files under FR-290/FR-529/FR-530 and TC-1329/TC-1330. All Rust sources, manifests and Cargo.lock are identical to the genuinely tested c73689002 head. No new findings; all round-one dispositions remain fixed. No duplicate Cargo test or review comment is needed for this source-equivalent composition.

## Combined stack final examination

Frozen candidate `a122d51a2e6d1e4a2f250b801bbddc9cdde1c5d0`; full `origin/main...HEAD` against main `d32cfd898`. PR2542 `b698d5d40` is included. This supersedes narrower historical scope without rewriting historical findings.

FAIL / HOLD — new medium allocation-boundary defect FND-002 and reproduced high wrong-claim binding FND-003. Rust/style, public API, wire conversion, original typed refusal paths, shared ledgers and conflict composition inspected. No new unsafe code, CI weakening, compatibility layer, vendor or pin found. Inline large-error allowances in authority modules correctly avoid denial allocation. Frozen branch not edited. Historical FND-001 remains fixed.

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | medium | SourceWitnessSupplier boxes the compact NativeRowError after identity_rows or selected_source_enum_rows has denied the caller's storage/work charge. With a genuine emission and zero remaining storage, identity_rows returns its allocation-free Rows denial, but supply immediately executes Box::new(rows), allocating an uncharged error after refusal. Keep the compact denial inline through WitnessSupplyFailure and cover the actual supplier boundary with allocation observation. | qsl-package/src/checked_v2.rs:1122; qsl-package/src/checked_v2.rs:1152 |
| FND-003 | high | HybridProcessRequest::read selects the first raw slot-form node_id match, so two different genuine hybrid clauses whose local slot IDs have identical bytes bind to the first clause. An unchanged public-API probe compiling One (h.v < 30) and Two (h.v < 29) produces distinct graph positions 24 and 25 with equal wire own IDs and claim ordinal 0; reading the second item succeeds but returns position 24. This can solve or settle a different obligation. Resolve the request as a document-held identity or require an unambiguous holder; never choose the first byte-equal node. | qsl-replay/src/hybrid/process_request.rs:324 |

## Dispositions — combined round 2

| FND | outcome | reason |
| --- | --- | --- |
| FND-002 | still-open | New defect verified by the reachable unconditional Box::new in the actual source supplier; no frozen-source mutation or runtime allocation probe was performed. |
| FND-003 | still-open | Reproduced by the non-mutating standalone public-API probe in hybrid_probe.rs; hybrid-probe.log records second expected graph position 25 becoming actual position 24. Current freshly built qsl_replay rlib was linked, no source edits or review clone. |

## Examined combined scope

```yaml
scope:
- id: FR-251-AC-45
  path: spec/functional/FR-251-derive-a-tick-based-monitor-plan-for-an-embedded-target.md
  role: examined
  excerpt: From a genuine authored tick-monitor unit, the public plan-root handoff
    returns the complete QSL-admitted model-step order and the observation owner's
    original canonical association VALUE, bound to the same final emission as its
    plan and two timing obligations. A public consumer compares the complete values
    and original source, structural clause node, claim role and ordinal, selected
    subject and checked model-time facts; the final emission temporal binding join
    equals its plan-root clause and the strict reader declaration. The handoff exposes
    the actual emitting package's original opaque IR Ad
- id: FR-251-AC-46
  path: spec/functional/FR-251-derive-a-tick-based-monitor-plan-for-an-embedded-target.md
  role: examined
  excerpt: Independently omit the checked order premise and progress association;
    substitute a foreign admitted emission; change the structural clause node, claim
    role, claim ordinal or original source; change the selected subject, observation
    source, clock identity/revision, model, structural model/unit/dimension/digital-operation
    node, unit, source kind, digital operation or exact period; and substitute order
    or progress values. Each candidate returns its typed missing-premise, owner refusal
    or mismatched-selection condition and no partial admitted context. Owner admission
    at its exact supported docume
- id: FR-251-AC-47
  path: spec/functional/FR-251-derive-a-tick-based-monitor-plan-for-an-embedded-target.md
  role: examined
  excerpt: 'After original source, model and request inputs are dropped, a public
    consumer reads the actual combined checked unit retained by the plan root: its
    exact source includes the authored unit and generated entries, its selected clause
    equals the plan subject, and its checked time source retains the selected model
    node, source kind and unit. These are explicitly original checker facts, not final
    renamed identities or admitted order/progress.'
- id: FR-251-AC-48
  path: spec/functional/FR-251-derive-a-tick-based-monitor-plan-for-an-embedded-target.md
  role: examined
  excerpt: A genuine dense model-time monitor root yields the complete specified canonical
    model-step value, with the actual checked model/unit/dimension identities. A digital
    source retains its exact period and checked tick-operation anchor. Changed source
    kind, unit or period changes the value. Exact-byte-ceiling encoding succeeds and
    one byte less returns Encoding with the original native canonical limit and no
    partial order value.
- id: FR-251-AC-49
  path: spec/functional/FR-251-derive-a-tick-based-monitor-plan-for-an-embedded-target.md
  role: examined
  excerpt: 'The authority handoff and order materialization receive the caller''s
    one mutable NativeJoinBudget, precharge every specified selection/materialization
    work and logical-storage step, and preserve previous consumption when used before
    row adaptation and native closure preparation. Measure genuine dense and tick
    handoffs at ample allowance: exact combined work and storage succeed; one short
    independently preserves the first unpaid native kind, full amount, original bound
    and non-owning locus without a context. Precharge part of the same budget before
    entry, then demonstrate that two individually '
- id: FR-251-AC-50
  path: spec/functional/FR-251-derive-a-tick-based-monitor-plan-for-an-embedded-target.md
  role: examined
  excerpt: A genuine dense authority returns its original checked ModelTimeSource
    by borrow and no IR tick period; a genuine tick authority returns its original
    checked source and an actual borrowed IR CheckedRational with the exact same checked
    numerator/denominator. The source borrow names the same element of the immutable
    retained checked-unit source slice. The IR period comes from the selected same-package
    clock's validated canonical period components, compared against the checked association
    and copied once under the original shared budget; exact storage for the actual
    IR container and two component
- id: FR-531-AC-28
  path: spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md
  role: examined
  excerpt: Producer and bytes-only equality admission pass the caller's same `NativeJoinBudget`
    through IR's admitted equality reader after identity/type projection. Decisions
    and non-resource witness refusals are retained for later unmetered selection and
    projection. Successful reads consume native work and storage cumulatively; exact
    measured bounds admit, and one unit less in either resource refuses with the original
    typed IR equality denial and recorded first unpaid charge, never an unknown witness
    or internal fault. Reusing an admitted slot consumes no additional native allowance.
- id: FR-531-AC-29
  path: spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md
  role: examined
  excerpt: Materializing an authentic emission's equality witness with a shared native
    budget preserves every application, fact and source-unit row, including original
    identities and order. The source admission and handoff paths use the same budgeted
    row builder. Exact measured work and storage allowances return the complete witness;
    one unit less in either resource refuses without allocating the denied material
    or a diagnostic, retains the first unpaid resource, amount and original locus,
    and returns no partial witness. Two individually affordable materializations that
    exceed one shared allowance refuse
- id: FR-531-AC-30
  path: spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md
  role: examined
  excerpt: 'Source-enum materialization and QSL-to-IR model-enum-shape and selected-collection
    conversion preserve original structural/declaration/member identities, canonical
    case order and complete Exact types on one shared native budget. Exact measured
    bounds admit; work or storage one short and zero storage return allocation-free
    compact denials at the first unpaid locus with prior consumption intact. A later
    conversion cannot reset a preceding adapter''s charges or replace its first denial.
    A selected source map preserves its original optional key node and complete Exact
    map value type through actual '
- id: FR-531-AC-31
  path: spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md
  role: examined
  excerpt: An authentic emission with source records, record members and unions produces
    complete identity rows equal to the original producer claims after native joining
    and QSL materialization. The measured work and storage include the QSL producer
    input copies, the single native join and each later QSL declaration, field, record/member
    and union copy. Exact allowances admit; one unit below either measured allowance
    refuses at its first unpaid step with no partial identity rows, the original configured
    bound and the admitted prefix intact. Denied external row copies retain AdapterRow(i)
    at their origin
- id: FR-531-AC-32
  path: spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md
  role: examined
  excerpt: A real tick-monitor plan's retained context supplies the same emission's
    plan, obligations, admitted order and progress association to the actual native
    row join and selected type-closure projection. The resulting native nodes, original
    declaration/member identities and Exact types equal those of that emission. Substituting
    another genuine emission's context, even with identical authored names, or omitting
    required order/progress authority returns a typed refusal and no executable context.
    Borrowing retained context leaves native storage consumption unchanged.
- id: FR-531-AC-33
  path: spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md
  role: examined
  excerpt: Allocation observation encloses only the adapter/conversion call, with
    fixture preparation and assertions outside it. A successful owned materialization
    performs observed allocations; a denial before its first allocation performs none,
    and a later denial allocates neither the denied material nor an owned diagnostic.
    The controls include fresh owner-produced lazy numeric payloads, values `2^127`
    and `-(2^127 + 1)`, and larger heap-backed values without prior warming or formatting.
    These values remain exact on successful copies; insufficient work/storage retains
    the first real row locus and orig
- id: FR-071-AC-7
  path: spec/functional/FR-071-implement-typed-replay-request.md
  role: examined
  excerpt: A request whose encoded size exceeds the configured reader bound refuses,
    and no truncated or partially-populated request is returned.
- id: FR-098-AC-7
  path: spec/functional/FR-098-execute-a-replay-request.md
  role: examined
  excerpt: A request whose `dependencies` entries are swapped, or repeat one identity,
    refuses `ReplayRefusal::DependencySelections` (`invalid_package`/`invalid-value`
    at `/dependencies`) before any recompile; one carrying an extra entry no import
    reaches refuses `DependencySelections` after the recompile; two entries whose
    sources share one authority and identity refuse `ReplayRefusal::DependencyInput`
    (`invalid_package`/`conflicting-definition`); one lacking an entry refuses `ReplayRefusal::Recompile`
    carrying `missing_import`/`missing-selection` at the import; an entry naming two
    sources of one librar
- id: FR-098-AC-20
  path: spec/functional/FR-098-execute-a-replay-request.md
  role: examined
  excerpt: 'A request for a proved package whose refinement `CasTwice` has the internal
    list `[{"refinement": "CasTwice", "internal": ["Spec::Twice::Done"]}]` carries
    it at `/package/lock/refinement_internals` and recompiles. A well-shaped list
    naming an unknown refinement or an unresolved internal class (`Spec::Twice::zz`
    at index 0) refuses `RefinementInternals`, retaining the complete native compiler
    refusal with its original code, cause and relative locus, and separately locating
    the request at `/package/lock/refinement_internals` followed by that relative
    pointer. A root value of `{}` is instead a bo'
- id: FR-350-AC-16
  path: spec/functional/FR-350-run-the-conformance-corpus-against-qsl.md
  role: examined
  excerpt: 'A `replay-request` file whose `package` is the checked-package document
    named `orders` with `contract_version` `quire.checked-package/v2`, whose `lock.sources`
    holds the one `{authority, identity}` entry `("repo", "orders.qsl")`, and whose
    `dependencies` holds `{identity: "lib/money", sources: [("repo", "money.qsl")]}`,
    decodes to a request with package name `orders`, that one source and one dependency
    entry with `identity` `lib/money` and its one source, and no version member; the
    same file with `package` removed, with `package` set to the string `"orders"`,
    with `contract_version` set to `qu'
- id: FR-350-AC-17
  path: spec/functional/FR-350-run-the-conformance-corpus-against-qsl.md
  role: examined
  excerpt: 'A `byte_provision` row whose `bytes_path` is `missing.bin` (absent from
    the vector directory), `unlisted.bin` (present but not listed as `kind: bytes`),
    `/etc/hostname` (absolute) or `../x.bin` (parent-traversing) is a `tool-failure`
    with cause `corpus-decode` and pointer `/byte_provision/0/bytes_path`, and no
    request is constructed; a listed `kind: bytes` path is read as raw bytes.'
- id: FR-350-AC-18
  path: spec/functional/FR-350-run-the-conformance-corpus-against-qsl.md
  role: examined
  excerpt: A `stage_limits` map holding the key `depth` (no FR-255 setting of that
    name) is a `tool-failure` with cause `corpus-decode` and pointer `/stage_limits/depth`;
    the key `s1.input_bytes` with the value `"1024"` decodes to the entry `("s1.input_bytes",
    1024)`.
- id: FR-480-AC-7
  path: spec/functional/FR-480-compile-and-link-a-package-from-several-source-units.md
  role: examined
  excerpt: 'AC-1''s package replays: an FR-098 request whose package reference lists
    `U1` and `U2` in canonical unit order, with both sources in the byte provision,
    recompiles to the same structural package and replays the D10 call. The same reference
    with the two sources reversed recompiles to the same structural package and replays;
    with one source repeated it refuses before any recompile, `invalid_package`/`invalid-value`
    at `/package/lock/sources`. A reference naming a definition document refuses before
    any recompile.'
- id: FR-484
  path: spec/functional/FR-484-identify-a-function-value-parity-claim-and-retain-its-replay-inputs.md
  role: examined
  excerpt: 'A function-value parity claim says that the generated code of a

    non-`Boolean` function `f` returns QSL''s exact `f(b)` at the parameter

    bindings `b` it was run at (FR-357, function level). Its obligation is the

    whole function: on every argument vector the declared signature of `f`

    admits, the generated outcome equals QSL''s exact one. A counterexample is

    one such vector together with the generated outcome observed there.


    QSL SHALL own this obligation''s identity. It SHALL define the preimage,

    mint it as the preimage value itself, compared by structural equality

    (ADR-013 O-09), hand it to the cla'
- id: FR-488
  path: spec/functional/FR-488-emit-a-checked-hyper-clause-as-a-v2-hyperproperty-node.md
  role: examined
  excerpt: "The S4 package emitter (ADR-011 layer 4 `package`, edge E4) SHALL emit\
    \ each\nchecked hyper clause over `behaviours` (FR-172) as one\n`quire.checked-semantic-graph/v2`\
    \ node with `node_tag: \"claim\"` and\n`semantic_form: \"hyperproperty\"`, whose\
    \ body is the seven-binding aggregate\nQSpec FR-402 states, together with the\
    \ nodes that aggregate references: the\nclause's object-parameter and subject\
    \ model nodes, its `match` conjunct\nnodes, its `align skip` operation anchors\
    \ and its body's formula nodes.\nQSL's I2 reader reads the node back, and the\
    \ checked clause exposes its\n`claim` occurrence (\xA7 \"Outputs\")."
- id: TC-1081
  path: spec/test-cases/TC-1081-monitor-builds-from-the-plan-alone-and-refuses-tampered-or-unsupported-plans.md
  role: examined
  excerpt: 'Verify that the executable monitor plan of FR-251 is self-sufficient,
    that admission refuses any plan not derived from the selected clause and target,
    that built monitors do not share state, that executing the plan agrees with QSL''s
    reference evaluator, and that every build, admission and execution fault is a
    typed non-verdict result.


    Scope: FR-251-AC-4 to FR-251-AC-12, FR-251-AC-14, FR-251-AC-15, FR-251-AC-17,
    FR-251-AC-19, FR-251-AC-21, FR-251-AC-23 to FR-251-AC-25, FR-251-AC-27 to FR-251-AC-30,
    FR-251-AC-32 to FR-251-AC-39, FR-251-AC-41, FR-251-AC-42, FR-251-AC-43 through
    FR-251-AC-50.'
- id: TC-1325
  path: spec/test-cases/TC-1325-the-runner-decodes-a-replay-request-file.md
  role: examined
  excerpt: 'Verify FR-350-AC-11 to FR-350-AC-14 and FR-350-AC-16 to FR-350-AC-18 (AC-15
    is retired) over fixture `replay-request` files. A wrong implementation this test
    catches: an adapter that executes no replay vector, one that keeps revision labels,
    one that reports a malformed corpus file as a QSL refusal.'
- id: TC-1331
  path: spec/test-cases/TC-1331-evaluate-from-a-checked-package-read-from-its-bytes-alone.md
  role: examined
  excerpt: 'Verify the bytes-only entry and its selected-monitor preparation boundary.

    Scope: FR-531-AC-1 through FR-531-AC-33. **R** is the package of FR-517.

    The isolated bytes-only consumer test crate depends on `quire-contract-model`,

    `qsl-package` and `qsl-eval` only. Producer transport and selected-monitor

    preparation tests additionally obtain authentic final emissions, admitted model

    inputs and the FR-251 plan context through their public QSL APIs before handing

    rows or bytes to the consumer.'
- id: QSpec-FR-402-AC-5
  path: agent-ix/quire-specification/spec/objects/interfaces/FR-402-hyperproperty-obligation-identity.md
  role: examined
  excerpt: A `behaviours` clause's `quantifiers` binding is a Tuple of one quantifier
    Group per quantifier, each Group binding `variable`, `quantifier` and `alias`
    to text literals and then holding one `reference` per fairness constraint to its
    `temporal`/`fairness` node; with `fair weak V::Vault::step` on `a` of TC-337 HB-01,
    `a`'s Group holds one reference and `b`'s none; the body passes strict wire validation
    of FR-322's grammar with no `malformed_wire`, and the reader admits the package.
    A finite-domain clause's Groups bind `variable` and `quantifier` only and its
    `subjects` is empty.
- id: QSpec-FR-402-AC-6
  path: agent-ix/quire-specification/spec/objects/interfaces/FR-402-hyperproperty-obligation-identity.md
  role: examined
  excerpt: 'Each `subjects` binding references the `model`/`model_root` node of the
    package its alias selects: two aliases'' clauses over one object type of each
    package reference two roots, and a clause whose alias binds no object parameter
    has that alias''s root as its subject; a `subjects` target that is a `model`/`object_type`
    node refuses `invalid_model_binding`/`malformed-declaration` at the target; a
    quantifier `alias` naming no subject refuses `missing_declaration`/`missing-name`
    at the `alias` binding; a `subjects` binding no Group names, and a body of six
    bindings, each refuse `invalid_semantic_gr'
- id: QSpec-FR-402-AC-7
  path: agent-ix/quire-specification/spec/objects/interfaces/FR-402-hyperproperty-obligation-identity.md
  role: examined
  excerpt: 'Each quantifier fairness refusal is located at the reference `/semantic_graph/nodes/{n}/body/members/3/value/members/{g}/members/{m}`:
    a fairness node whose target is a `protocol_root` target refuses `invalid_model_binding`/`malformed-declaration`;
    one whose `operation` target names no effective operation refuses `missing_declaration`/`missing-name`;
    one whose declaring node belongs to another alias''s package refuses `invalid_model_binding`/`malformed-declaration`;
    and a non-empty quantifier fairness set under a body wrapper whose profile is
    `quire.temporal.event-position.false-extension/v1` r'
```

## Validation boundaries

The lead owns the single final combined gate after fixes. No aggregate gate, workspace build, make ci or Kani was run by this reviewer. Full mutation testing was prohibited on the frozen branch; static old-code sensitivity was checked from the oracles. Coder receipts were inspected as secondary evidence and are not substituted for reviewer runtime results.


Combined additional examined unit:

```yaml
scope:
- id: FR-447-AC-4
  path: spec/functional/FR-447-check-a-hybrid-declaration-and-lower-it-to-a-hybrid-provider-request.md
  role: examined
  excerpt: The request writer writes for `SlowEnough` one item of kind `temporal-satisfaction`,
    extent `bounded`, carrying the hybrid-dynamics requirement, whose `claim` names
    `SlowEnough`'s emitted `temporal_clause` node and its `claim` occurrence; with
    an in-process fixture hybrid provider registered the item routes to it, which
    reads `Braking` from the checked package; with only a process fixture hybrid provider
    registered the item routes to it, which receives the package's v2 bytes and reads
    `Braking` and `SlowEnough` from their QSpec FR-481 nodes; with none registered
    it settles `unsupported`, `unsu
```

## Current evidence and limitations

Computed `quire matrix --scope . --format json` produced zero JSON bytes after more than seven minutes of CPU work and was explicitly stopped with SIGTERM. A scoped manual tag/test/source inventory was used; repository-wide untagged counts, matrix completeness and a global matrix PASS are unavailable. Plan completion: not assessed.

Focused reviewer runtime receipts are in this scratchpad. Coder clippy/fmt receipts are secondary; the user assigned the aggregate checks, cargo deny and final gate to the lead, so they were not duplicated. Standalone hybrid_probe.rs uses public APIs linked to the freshly built current qsl_replay library, outside the frozen tree, and reproduces wrong-claim binding without mutating the candidate. No review clone, source edit, commit or push was made.

Focused frozen-head checks: original AdapterRow output, sparse retained field index, tick complete three-resource handoff, root refinement false replay, dependency-isolated refinement false replay, hyper fairness references and two-alias model-root subjects each passed one exact test. Map/sequence actual native projection and cold equality payload tests also passed. The model-enum native closure check FAILED at monitor_root.rs:1308 (expected one Tier row; actual []); historical IR1388 gap FND001 remains still-open. These are focused checks only.
