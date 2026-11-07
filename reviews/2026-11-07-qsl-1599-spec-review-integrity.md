---
id: SR-20261107070951249610
title: QSL-1599 spec-review-integrity frozen PR2514
type: SpecReview
analysis: integrity
review_set: subset
scope: agent-ix/quire-spec-language; QSL-1599 PR2514 changed replay/package/corpus/spec
  surface at 599ca2b9c1dd73144167764f2440cb4929217403
relationships:
- type: references
  target: ix://agent-ix/quire-spec-language/FR-350
---

## Summary

Ticket: QSL-1599. Checked source/dependency location consistency across FR098, FR350, FR480, FR484 and TC1325. Whole raw package retention, bounded metadata decoding and native admission are distinct phases. FR098AC3 remains retired; fresh source evaluation does not assert offered/recompiled equality. Required authority is explicit; dependency lists remain optional. Root malformed list and semantic list failures have distinct specified owners. Existing relationships and verification ownership remain intact; no new dependency relationships or domain objects require separate analyses.

## Verdict

PASS (static scope only; dynamic HOLD). Final dynamic PASS awaits actual positive-admission receipts after owner PR946 lands. No Cargo, source edits, commits, pushes or aggregate gate performed by reviewer.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - | correct-requirement-no-evidence |

## Coverage

Checked source/dependency location consistency across FR098, FR350, FR480, FR484 and TC1325. Whole raw package retention, bounded metadata decoding and native admission are distinct phases. FR098AC3 remains retired; fresh source evaluation does not assert offered/recompiled equality. Required authority is explicit; dependency lists remain optional. Root malformed list and semantic list failures have distinct specified owners. Existing relationships and verification ownership remain intact; no new dependency relationships or domain objects require separate analyses.

```yaml
scope:
- id: FR-098
  path: spec/functional/FR-098-execute-a-replay-request.md
  role: examined
  excerpt: "QSL SHALL provide the replay executor entry, `qsl_replay::replay`, as\
    \ the\nlayer-6 `replay` facade's public API (ADR-013 O-26, C-13, TK-01; ADR-011\n\
    \xA72.1 E9, \xA76.1). It takes an FR-071 replay request. It recompiles the proved\n\
    package's source units, one or more (FR-480), through the spine (S1 to S4, the\
    \ same\nthe FR-278 composition the CLI `compile` uses, FR-027) from the\nrequest's\
    \ byte provision, against the dependency input the\npackage reference's `dependencies`\
    \ entries name (ADR-015 D-4). A changed\ndependency document is not detected:\
    \ the recompile reads the bytes the byte\nprovision carries and"
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
    list `[{"refinement": "CasTwice", "internal": ["Spec::Twice::finish"]}]` carries
    it at `/package/lock/refinement_internals` and recompiles. A well-shaped list
    naming an unknown refinement or an unresolved internal class (`Spec::Twice::zz`
    at index 0) refuses `RefinementInternals`, retaining the complete native compiler
    refusal with its original code, cause and relative locus, and separately locating
    the request at `/package/lock/refinement_internals` followed by that relative
    pointer. A root value of `{}` is instead a '
- id: FR-350
  path: spec/functional/FR-350-run-the-conformance-corpus-against-qsl.md
  role: examined
  excerpt: 'When a maintainer runs the `qualify` repository gate (FR-353) over a

    conformance corpus,

    the runner SHALL execute every vector whose applicability names a QSL

    subject through the QSL public entry point for that subject, and SHALL

    compare the typed result with the vector''s expected disposition.


    QSpec FR-336 owns the corpus schema, vector vocabulary, applicability,

    expected-value encoding and input adapters. This requirement specifies how

    QSL selects its applicable vectors, invokes its public operations and settles

    their outcomes against the corpus oracle.'
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
- id: FR-480
  path: spec/functional/FR-480-compile-and-link-a-package-from-several-source-units.md
  role: examined
  excerpt: 'QSL SHALL compile one checked package from an ordered, non-empty list
    of

    complete-V1 source units, the package''s **unit list**. QSpec FR-151 defines

    the linked-source-unit language rules; QSpec FR-322 defines the corresponding

    package representation. This requirement specifies QSL''s compile order,

    cross-unit stage behavior, source ownership and emitted package obligations.

    Each unit is read once and parsed once with the one grammar (ADR-033,

    FR-362). A unit list of one source is the one-unit compile every other

    requirement describes.'
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
- id: TC-1325
  path: spec/test-cases/TC-1325-the-runner-decodes-a-replay-request-file.md
  role: examined
  excerpt: 'Verify FR-350-AC-11 to FR-350-AC-14 and FR-350-AC-16 to FR-350-AC-18 (AC-15
    is retired) over fixture `replay-request` files. A wrong implementation this test
    catches: an adapter that executes no replay vector, one that keeps revision labels,
    one that reports a malformed corpus file as a QSL refusal.'
- id: FR-071-AC-7
  path: spec/functional/FR-071-implement-typed-replay-request.md
  role: examined
  excerpt: A request whose encoded size exceeds the configured reader bound refuses,
    and no truncated or partially-populated request is returned.
```

Changed-file inventory examined:

- Cargo.lock
- qsl-package/src/checked_v2.rs
- qsl-package/src/lib.rs
- qsl-replay/src/admitted.rs
- qsl-replay/src/call_site.rs
- qsl-replay/src/execute.rs
- qsl-replay/src/execute/tests.rs
- qsl-replay/src/execute/tests/complete_package.rs
- qsl-replay/src/execute/tests/composite.rs
- qsl-replay/src/execute/tests/value_parity_identity.rs
- qsl-replay/src/lib.rs
- qsl-replay/src/outcome.rs
- qsl-replay/src/proof_result.rs
- qsl-replay/src/proof_result/plugin_replay.rs
- qsl-replay/src/proof_result/plugin_tests.rs
- qsl-replay/src/request.rs
- qsl-replay/src/request/package.rs
- qsl-replay/src/scalar.rs
- qsl-replay/src/spine/clause/tests/frame_replay.rs
- qsl-replay/src/spine/clause/tests/state_clause_replay.rs
- qsl-replay/src/spine/clause/tests/temporal.rs
- qsl-replay/tests/call_replay_context.rs
- qsl-replay/tests/call_replay_context_root_only.rs
- qsl-replay/tests/imported_function_replay.rs
- qsl-replay/tests/imported_function_site.rs
- spec/functional/FR-098-execute-a-replay-request.md
- spec/functional/FR-350-run-the-conformance-corpus-against-qsl.md
- spec/functional/FR-480-compile-and-link-a-package-from-several-source-units.md
- spec/functional/FR-484-identify-a-function-value-parity-claim-and-retain-its-replay-inputs.md
- spec/test-cases/TC-1325-the-runner-decodes-a-replay-request-file.md
- tests/it/replay_multi_source.rs
- tests/it/value_parity_profile_selections.rs
- xtask/src/qualification.rs
- xtask/src/qualification/adapters.rs
- xtask/src/qualification/adapters/replay.rs
- xtask/tests/qualification_adapters.rs
