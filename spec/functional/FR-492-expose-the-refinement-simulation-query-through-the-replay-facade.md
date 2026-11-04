---
id: FR-492
title: "Expose the refinement simulation query through the replay facade"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-314
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-315
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-060
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: traces_to
---
# FR-492: Expose the refinement simulation query through the replay facade

## Description

A backend that checks the per-step simulation obligations of a refinement
needs FR-315's `Initial` and `Step { row }` queries. The refinement node's
`steps` array holds the rows in declaration order, so a query's `row` is the
row's position among the rows as written.
`encode_refinement_query` and its request and kind types are `qsl-eval`'s;
ADR-011 FB-05 and T-12 rule (a) (FR-060 T12-A) forbid CG to name `qsl_eval` or
`qsl_replay::spine`. This requirement exposes them at the `qsl_replay` root.

## Outputs

The `qsl_replay` root exports replay-owned wrapper types and functions; it
re-exports no `qsl_eval` item (FR-100-AC-8). A consumer builds the request from
root items alone:

- `RefinementQueryRequest::new(closure, refinement, concrete, abstract_)`: the
  request of a refinement query (FR-314's request of a refinement item). It
  takes the `CheckedClosureHandle` that `acquire_checked_closure` retains, the
  refinement's `NodeKey` and two subjects.
- `CheckedClosureHandle::refinement_node_key(name) -> Result<NodeKey,
  RefinementKeyRefusal>`: the node key of the refinement declared as `name`,
  resolved from the retained closure (no digest; it does not depend on how
  documents are keyed). `RefinementKeyRefusal` is `UnknownName`,
  `NotARefinement` (declared as a state clause) or `NotEmitted`.
- `RefinementUniverse::for_population(package, population, keys)` and
  `RefinementUniverse::new(population, keys)`: a universe keyed by the declared
  names. `RefinementUniverse` and `RefinementQuerySubject` are
  `#[non_exhaustive]`; `RefinementQuerySubject::new(..)` builds a subject with
  default normalization and observation limits and no proof bounds.
- `RefinementQuerySubject`: one subject (the concrete or the abstract model):
  the domain package and snapshot provisions, the initial `DocumentRef`s, the
  `RefinementUniverse`s (a population's `DeclarationKey` and its object keys),
  the per-expansion candidate bound, the normalization, observation and meter
  limits and the proof bounds. The wrapper converts it into the `qsl_eval`
  subject inside the entry.
- `RefinementQueryKind`: `Initial` or `Step { row }`, converted to the
  `qsl_eval` kind inside the entry.
- `encode_refinement_query(&RefinementQueryRequest, RefinementQueryKind)`.

## Behavior

- The query encoding shall be FR-315's, unchanged: the wrapper converts its
  inputs and calls the existing entry.
- The wrappers shall hold the checked closure and the subject members unchanged.
- The wrappers shall compute no digest, hash, revision, seal, lock or pin; a
  digest-keyed field of the wrapped request is passed through until the digest
  removal program changes the request (QSL-1282).

### Public refinement certificate adapter

The request SHALL expose `refinement_key() -> &NodeKey`, returning its held
selected key unchanged, and `step_count() -> Result<usize, SmtEncodingRefusal>`,
reading the same retained-refinement join as assembly and checking. An
unresolvable selection is the existing encoding invariant fault, never a
caller-supplied count, probe-until-refusal loop or zero default.

The replay-owned request SHALL expose `assemble_smt_certificate(handoff,
limits, cancel)` and `check_smt_proof(certificate, limits, cancel)`. Assembly
SHALL consume the existing CG FR-045 run-record byte format: a little-endian
unsigned 64-bit count; a Base tag `0`, or Step tag `1` followed by its
little-endian unsigned 64-bit row; then separate query and proof fields,
each a little-endian unsigned 64-bit length followed by those bytes. It
SHALL consume the whole input and reject unknown tags, truncated fields,
nonrepresentable lengths and trailing bytes. This adapter owns no second
run-record format and adds no QSpec wire member.

The request's retained selected refinement SHALL determine the required
step count. Exactly one Base and one Step for every selected declaration-order
row are required; missing, duplicate or out-of-range parts refuse assembly.
Assembly SHALL validate the complete framing and part set before copying
query/proof fields into the existing `RefinementSmtCertificate`, preserving
all bytes, including empty and non-UTF8 proofs. A present invalid proof is
not an absent certificate. CG's format has no encoding for an absent field:
CG refuses to emit such a handoff; a truncated stream is malformed framing.
The private assembly boundary SHALL also reject absent query/proof fields.

`RefinementHandoffRefusal` SHALL distinguish `Malformed`, `Shape`,
`Stopped(IncompleteCause)` and `Fault(InternalFault)`. No failed assembly
returns a partial certificate. Assembly SHALL use a fresh stage budget from
`CertificateCheckLimits`: every supplied byte counts once against
`max_certificate_bytes`, including framing, and each record counts once
against `max_certificate_edges`. It SHALL poll cancellation before charging
and while visiting records; allocation reservation failure stops with
`ResourceExhausted`. It SHALL validate the count against the retained
refinement before allocating record slots, use checked length arithmetic,
and impose no additional unconfigured cap. An unresolvable selected
refinement is an invariant fault, not a zero-row refinement.

`check_smt_proof` SHALL construct the internal request only inside the
existing private request bridge and delegate the existing FR-314 checker.
It SHALL expose the native `SmtProofCheck`/`CertificateRejection` result
unchanged. Checking has its existing fresh checker budget; assembly and
checking do not claim to share a mutable budget. No public signature or
re-export of these adapter additions SHALL name a `qsl_eval` request type.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-492-AC-1 | Over a request built from `qsl_replay` root items alone (the closure from `acquire_checked_closure`), `CasRefinesCounter`'s `Initial` query is a `QF_LIA` script over one state copy, its `Step` query of a row is over two copies, queries of different rows differ, and a query is deterministic. | Test (TC-1275) |
| FR-492-AC-2 | **RETIRED (steps are in declaration order, so there is no position mapping).** | Test, retired |
| FR-492-AC-3 | **RETIRED (steps are in declaration order, so there is no position mapping).** | Test, retired |
| FR-492-AC-4 | The `qsl_replay` root re-exports no `qsl_eval` item: `arch-lint api-surface --qsl . --qsl-only` reports no finding. | Test (arch-lint-api-surface-qsl) |
| FR-492-AC-5 | `refinement_node_key("CasRefinesCounter")` over the retained closure returns the node key of that refinement, and a second call returns the same key. | Test (TC-1275) |
| FR-492-AC-6 | `refinement_node_key` over a name the closure does not declare returns `UnknownName`, and over a name declared as a state clause returns `NotARefinement`. | Test (TC-1275) |
| FR-492-AC-7 | `RefinementUniverse::for_population(package, population, keys)` equals the universe of the `DeclarationKey` of that population declared in that package. | Test (TC-1275) |
| FR-492-AC-8 | Over a replay-root request for the seven-row refinement, the CG-format Base and Step handoff assembles in row order in either input order, with exact query/proof bytes preserved; the zero-row refinement accepts Base only. No caller-provided count defines the selected refinement. | Test (TC-1275) |
| FR-492-AC-9 | Missing or duplicate Base or Step, out-of-range Step, a handoff for a different selected row count, unknown tag, truncated header/row/field, impossible length, mismatched count and trailing bytes each refuse with the specified typed shape or malformed result and no partial certificate. The private assembler also refuses absent query or proof fields. | Test (TC-1275) |
| FR-492-AC-10 | A present empty proof and arbitrary non-UTF8 bytes survive assembly. The request-owned checker reports the native NotRefutation or Malformed rejection for those proofs and QueryMismatch for a different query; assembly itself claims no proof acceptance. A checked-rule refutation through the facade yields the same native result as FR-314. | Test (TC-1275) |
| FR-492-AC-11 | Assembly succeeds at its exact byte/record budgets, stops at one less or zero with the corresponding existing limit and setting, and a cancelled handle stops with its native cause. Limits, cancellation and reservation failure expose no partial certificate. Checking retains its independent FR-314 budget. | Test (TC-1275) |
| FR-492-AC-12 | The two new request methods are usable from replay-root types alone; no adapter public signature exposes the internal certificate request, and callers cannot invoke the private with_certificate_request bridge. | Test (TC-1275), Inspection |
| FR-492-AC-13 | The public refinement_key accessor returns the exact key selected from the retained closure, and step_count returns the authoritative declaration-order row count for seven-row and zero-row selections; an unrelated key faults rather than giving a count. The driver needs no private request type or caller row-count authority. | Test (TC-1275) |

## Dependencies

FR-314, FR-315, FR-060 (T12), ADR-011.
