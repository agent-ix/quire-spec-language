---
id: SR-1090
title: "QSL-394 spec review of FR-350 to FR-354, FR-003 and US-035"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@46c958aefd7f25e22a52768e8289c946e1f74e0b; spec/functional/FR-003, FR-350..FR-354; spec/test-cases/TC-016, TC-875..TC-885; spec/usecase/US-035; spec/spec.md; spec/tests.md; spec/native-readiness/tests.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-350
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-351
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-352
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-353
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-354
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-003
    type: reviews
  - target: ix://agent-ix/quire-spec-language/US-035
    type: reviews
---

## Summary

Ticket: QSL-394. PR agent-ix/quire-spec-language#581, commits 44b6ab67 and
46c958ae over origin/main. QSpec counterparts read on origin/main and on the
open QSpec PR #174 head (spec/wave-b-q9-ceremony): FR-311, FR-336,
interface_019, FR-132, FR-133. ADR-012 §4 and §5 read on QSL main.

Checked against the owner rulings:
- qualification tests behaviour only: no ledger, waiver, pin, manifest,
  evidence chain or version binding in the diff. The report names the QSL
  version that ran, which is allowed. Met.
- coverage measured from executed vectors: FR-351 settles each capability
  from vectors executed in the run; FR-352 coverage is passed over in-scope,
  `uncovered` counted against it. Met.
- the corpus's own `results` decide nothing: FR-351 behaviour, AC-3 and
  TC-878 step 1. Met.
- no fixed caps: FR-003 drops the 1 MiB clamp and makes the ceiling
  caller-raisable, the refusal naming the ceiling, value and API; FR-350
  limits are caller-raisable with published defaults. Met.
- no compat paths; ticket ids only in References; every AC has a behaviour
  TC (TC-875 to TC-885 cover all 22 ACs). Met.
- `quire validate` on the 22 changed files exits 0 and
  `tools/check-index-completeness.sh` passes.

FR-354 keyword-first extension choice: confirmed as sound. Core declarations
all begin with a core keyword, so a new leading keyword at declaration
position cannot reinterpret any source that already parses, collisions are
decidable from the catalog alone, and it fits ADR-012 §5.1 S2's leading-token
parser entry table. FND-007 asks for one sentence that keeps the keyword
contextual. FND-001 is the ADR conflict, which is separate from the keyword
choice.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-354 admits declaration forms and node kinds from catalog data at run time, but ADR-012 §5.1 makes the parser leading-token enum (S2) and the checked node enum (S3) closed Rust enums whose new variants must fail the build at every seam, and §4.2 requires a builder with an explicit arm per (state, clause) pair. A schema-defined node kind cannot be a variant of a closed enum, and IR/CG cannot match it exhaustively. FR-354 depends on ADR-012 but neither amends it nor states the closed shape it fits (for example one extension variant carrying definition, node kind and subnodes, checked by a schema-driven builder). | spec/functional/FR-354-admit-extensions-from-declarative-grammar-and-typed-node-schemas.md:27-31,56-66 |
| FND-002 | high | FR-350 makes any vector that reaches a resource limit `incomplete`, whatever its expected disposition. QSpec FR-336's classes include `incomplete` and boundary vectors, whose expected disposition is the limit refusal (for example FR-003's `resource_exhausted` at a ceiling). Such a vector can never be `passed`, so its capability never passes and `complete-v1: qualified` is unreachable whenever the corpus holds one. | spec/functional/FR-350-run-the-conformance-corpus-against-qsl.md:73-75 |
| FND-003 | medium | FR-350 gives no precedence between Comparison and Other outcomes. A refusal vector expecting `ill_typed` that gets `unknown_required_feature` is `failed` under Comparison (lines 61-64) and `unsupported` under line 70; a value vector that hits a limit is `failed` and `incomplete`. Two implementers would settle these differently. | spec/functional/FR-350-run-the-conformance-corpus-against-qsl.md:57-79 |
| FND-004 | medium | FR-350-AC-3 drops the behaviour's condition "for a vector that expects a success or a different refusal". As written, a vector that expects `unknown_required_feature`/`unknown-feature` (QSpec FR-132-AC-3, FR-133-AC-3, and FR-354's own reader refusal) is `unsupported` rather than `passed`. TC-876 step 1 does not state the vector's expected disposition either. | spec/functional/FR-350-run-the-conformance-corpus-against-qsl.md:94 |
| FND-005 | medium | The runner selects by "applicability names a QSL subject", and the QSL subjects are compiler, runtime, replay and formatter. QSpec FR-336 applicability is "required consumer types and supported semantic profiles", and FR-311 profiles are language, runtime, monitor, analyzer/backend, tool and consuming-system. No mapping is given, FR-351-AC-2 uses "the language subject", which is not one of FR-350's subjects, and capability-level applicability (FR-351) versus vector-level applicability (FR-350) is undefined. | spec/functional/FR-350-run-the-conformance-corpus-against-qsl.md:20,50-54; spec/functional/FR-351-settle-each-capability-from-its-executed-vectors.md:27,60 |
| FND-006 | medium | FR-354 defines the extension definition's members (a grammar schema of keyword-led productions and a typed-node schema of node kind and clause types). That is part of the definition format, which QSpec owns; QSpec FR-133 and FR-131 say only "exact definition, dependency edges and typed package node", and the ticket says a QSpec FR is needed. FR-111's `Definition` holds bytes, role, edges and capabilities, with no schema members, and is not amended. The schema format needs a QSpec FR, or an STD ticket, that FR-354 cites. | spec/functional/FR-354-admit-extensions-from-declarative-grammar-and-typed-node-schemas.md:36-45 |
| FND-007 | medium | FR-354 does not say the extension's leading keyword is recognised only at declaration-start position. If it is reserved everywhere in a selecting unit, selecting the extension changes the meaning of an existing identifier named `retention`, which QSpec FR-133 forbids ("cannot steal syntax"). | spec/functional/FR-354-admit-extensions-from-declarative-grammar-and-typed-node-schemas.md:39-40,72 |
| FND-008 | low | The unselected-form refusal requires S1 to consult extensions the unit did not select. A non-selecting unit's refusal then varies with the catalog: `unsupported_construct`/`declaration-form` when the catalog holds the extension, and some other parse refusal when it does not. State the refusal for an unrecognised leading keyword independently of the catalog, or state that the lookup covers unselected entries. | spec/functional/FR-354-admit-extensions-from-declarative-grammar-and-typed-node-schemas.md:75 |
| FND-009 | low | A missing clause refuses `ill_typed`/`type-mismatch` "naming the expected and the actual type", but a missing clause has no actual type. FR-354 already uses `missing-member` for absent members. | spec/functional/FR-354-admit-extensions-from-declarative-grammar-and-typed-node-schemas.md:76 |
| FND-010 | low | QSpec FR-133-AC-3 requires a consumer that refuses an unknown required extension to retain its source and exact identity. FR-354's reader refusal and TC-884 step 3 check only the code and cause. | spec/functional/FR-354-admit-extensions-from-declarative-grammar-and-typed-node-schemas.md:77,94 |
| FND-011 | low | No requirement covers a corpus that reads but fails QSpec FR-336 validation (missing required property, duplicate key, out-of-domain value). FR-353 gives exit 2 only for an unreadable corpus and an unknown scope id, and no AC tests exit 2 for the unknown-scope refusal. | spec/functional/FR-353-settle-the-complete-v1-qualification-verdict.md:42-44 |

## Verdict

The runner, settlement, report and verdict FRs meet every owner ruling: they
test behaviour only, they measure coverage from executed vectors, and the
corpus's `results` decide nothing. Not mergeable as it stands, because of two
high findings. FR-354 conflicts with ADR-012's closed seams (FND-001), and
the `incomplete` rule makes QSpec's own incomplete-class vectors impossible
to pass (FND-002). FND-003 to FND-007 are gaps that a reader would trip on.
The keyword-first extension choice itself is confirmed.
