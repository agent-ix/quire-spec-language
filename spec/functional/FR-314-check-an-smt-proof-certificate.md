---
id: FR-314
title: "Check an SMT proof certificate"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-015
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-338
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-315
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-127
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-163
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-331
    type: references
  - target: ix://agent-ix/quire-specification/FR-375
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-245
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-276
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-255
    type: depends_on
---
# FR-314: Check an SMT proof certificate

## Description

`qsl-replay`'s `check_smt_proof` SHALL accept an `SmtProofCertificate` that
comes with an SMT backend's `BoundedComplete{depth}` or
`Inductive{depth}` proof, before it settles, exactly when every query it carries is
the query FR-315 encodes for the item at that depth and every proof it
carries refutes its query using only checked Alethe rules (ADR-018 PC-6,
LA-3). FR-127's settlement map runs the check: a proof whose certificate it
verifies settles `proved`, `Certified`; one it shows wrong
settles `inconclusive`, `CertificateRejected{rule, at}`; one it cannot
verify because a step uses an unchecked rule settles `proved`,
`Uncertified`, as an SMT proof that arrives with no certificate does. The
settlement rule under "Outputs" states the whole mapping once.

For a refinement's per-step simulation obligations (QSpec FR-468),
`check_refinement_smt_proof` SHALL accept a `RefinementSmtCertificate` exactly
when it holds one base part and one step part per step row, each the query
FR-315 encodes for it, and every part refutes its query using only checked
Alethe rules.

## Use case

A verification operator relies on a bounded-complete or inductive proof
from an SMT solver. The solver sits outside the qualified core, so the
proof is labelled `certified` only after the core checks the
solver's own proof objects against the queries the core itself encodes.

## Inputs

- FR-338's `CertificateRequest` for the item, and the result's FR-127
  `SmtProofBasis` (`BoundedComplete{depth}` or `Inductive{depth}`).
- An `SmtProofCertificate`.
- The request's FR-245 `CertificateCheckLimits` and FR-276's `Cancel`
  handle.
- The `SmtEncodingLimits` (FR-315) that the item's certificate request
  records: FR-338's `CertificateRequest.smt_encoding`, the values of the run's
  `smt_encoding.*` settings (FR-126 `ModelCheckRequest.smt_encoding`), under
  which CG's SMT backend encoded every query of the item ("Encoding
  limits").

```rust
pub enum ProofFormat { Alethe }              // cvc5's Alethe proof format

pub struct RefutedQuery {
    pub query: Vec<u8>,                       // SMT-LIB 2 script, FR-315 canonical printing
    pub proof: Vec<u8>,                       // the solver's refutation in `format`
}

pub enum SmtProofCertificate {
    BoundedComplete { format: ProofFormat, unrolling: RefutedQuery },
    Inductive { format: ProofFormat, base: RefutedQuery, step: RefutedQuery },
}

/// A refinement's per-step simulation obligations (QSpec FR-468): the refuted
/// initial query and one refuted step query per step row, in the order of the
/// refinement node's `steps` array (QSpec FR-375).
pub struct RefinementSmtCertificate {
    pub format: ProofFormat,
    pub base: RefutedQuery,
    pub steps: Vec<RefutedQuery>,
}

pub fn check_smt_proof(
    request: &CertificateRequest<'_>,
    basis: &SmtProofBasis,                    // FR-127: BoundedComplete or Inductive, with depth
    certificate: &SmtProofCertificate,
    limits: &CertificateCheckLimits,          // FR-245
    cancel: &Cancel,                          // FR-276
) -> Result<SmtProofCheck, CertificateRejection>;

/// The refinement item's request: the recompiled package, the refinement node,
/// and the re-admitted concrete and abstract subjects. It is not FR-338's
/// `CertificateRequest`, which holds one model subject and a temporal item.
pub struct RefinementCertificateRequest<'a> {
    pub package: &'a CheckedPackage,
    pub refinement: NodeKey,
    pub concrete: &'a ModelSubject,
    pub abstract_: &'a ModelSubject,
}

pub fn check_refinement_smt_proof(
    request: &RefinementCertificateRequest<'_>,
    certificate: &RefinementSmtCertificate,
    limits: &CertificateCheckLimits,          // FR-245
    cancel: &Cancel,                          // FR-276
) -> Result<SmtProofCheck, CertificateRejection>;

pub enum SmtProofCheck {
    Verified,
    Unverifiable { part: QueryPart, step: u64 },  // first command with an unchecked rule
    Unencodable { part: QueryPart },              // refinement only: FR-315 refuses the row's query
    Stopped(IncompleteCause),                     // LimitReached{limit, value, setting} or Cancelled{source}
    Fault(InternalFault),                         // FR-315 returned `SmtEncodingRefusal::Fault` rebuilding a query
}

pub enum ModelCheckLimit {
    // FR-126's members, FR-166's and FR-184's, plus:
    MaxCertificateBytes,   // limit name max_certificate_bytes
    MaxCertificateEdges,   // limit name max_certificate_edges
}
```

`ModelCheckLimit` gains `MaxCertificateBytes` and `MaxCertificateEdges`, as
FR-166 and FR-184 extend it, so FR-127's `LimitReached { limit:
ModelCheckLimit, value, setting }` carries a stopped SMT check's limit.

FR-338's `CertificateRule` gains `QueryMismatch`, `ShapeMismatch`,
`ProofStepInvalid` and `NotRefutation`, and this checker also rejects with
FR-163's `Malformed`. `CertificateRejection.at` is FR-338's
`CertificateLocus`, here `Query { part }` or `ProofStep { part, index: u64 }`,
`part` being `Unrolling`, `Base` or `Step`; for a refinement certificate `Step`
carries its row, `Step { row: Some(r) }`. QSpec FR-331's Certification
table spells every rule and both loci, `step/<r>` for a step row included,
and lists the rules this checker may name at a query part and at a proof
step; FR-338's "Rejection spellings" applies it.

## Outputs

Each entry returns `Result<SmtProofCheck, CertificateRejection>`:

- `Ok(SmtProofCheck::Verified)` when the certificate is accepted: every
  carried proof refutes its expected query using checked rules alone.
- `Ok(SmtProofCheck::Unverifiable { part, step })` when no rule rejects and
  the command numbered `step` of `part`'s proof is the first that uses an
  unchecked rule.
- `Ok(SmtProofCheck::Unencodable { part })`, from
  `check_refinement_smt_proof` only, when FR-315 refuses the query of the
  step row `part` names.
- `Ok(SmtProofCheck::Stopped(cause))` when a limit was reached or the
  `Cancel` handle tripped ("Limits and cancellation").
- `Ok(SmtProofCheck::Fault(fault))` when FR-315 returned
  `SmtEncodingRefusal::Fault` while the checker rebuilt a query.
- `Err(CertificateRejection{rule, at})` naming the first rule that failed
  and where.

**Settlement rule.** FR-127's settlement map reads each alternative, and this
is the one statement of the mapping: `Verified` settles `proved`, `Certified`;
`Unverifiable` and `Unencodable` each settle `proved`, `Uncertified`; `Stopped`
settles V-7; `Fault` settles `failed`; `Err` settles `inconclusive`,
`CertificateRejected{rule, at}`. `Verified` is the only alternative that
settles `Certified`. `Unverifiable` and `Unencodable` are never `Verified` and
never `Certified`: neither shows the certificate wrong, and neither shows the
query refuted by checked rules, so a caller never reports either as verified,
and a refinement's records are `Certified` only when the checker returned
`Verified` for the whole certificate (FR-146).

## Behavior

- The checker SHALL reject with `ShapeMismatch` at `Query { part }`, `part`
  being the first part the basis expects (`Unrolling` for
  `BoundedComplete`, `Base` for `Inductive`), a certificate whose variant
  differs from the basis: `BoundedComplete` carries one unrolling query and
  `Inductive` carries a base query and a step query.
- The checker SHALL then reject with `ShapeMismatch` at `Query { part:
  Unrolling }`, before it encodes any query, a `BoundedComplete{depth}`
  basis for an item that is not a TP-2 `on origin` item, or whose `depth`
  is below the item's ADR-014 TR-4 horizon `h`: ADR-018 V-2 is
  bounded-complete only for a TP-2 `on origin` item unrolled to `depth >=
  h`, because below `h` the claim can fail at a position the query does
  not reach, so a refutation of the shallower query proves nothing.
  FR-127's "Bounded-complete basis rule" applies the same rule first, to a
  `BoundedComplete` result with or without a certificate, so a result that
  breaks it never reaches this checker or FR-127's no-certificate path.
- The checker SHALL encode each expected query with FR-315 from the
  recompiled package and the re-admitted subject at the basis's depth: the
  unrolling for `BoundedComplete{depth}`, and the base case and the step
  case for `Inductive{depth}` (ADR-018 V-2, V-3).
- The checker SHALL reject with `QueryMismatch` at that part when a
  carried query differs, byte for byte, from FR-315's canonical printing of
  the expected query.
- When FR-315 refuses an expected query of `check_smt_proof` with a
  `Refused { code, locus }`, whatever the code, `too-large` under the
  recorded limits and `out-of-fragment` included, the checker SHALL reject
  with `QueryMismatch` at that part: FR-315 prints no query for the item at
  that depth, so no carried query equals one. The item settles
  `inconclusive`, `CertificateRejected{QueryMismatch, Query { part }}`,
  never `proved`.
- When FR-315 returns `SmtEncodingRefusal::Fault` while either entry
  rebuilds any query, the checker SHALL return `SmtProofCheck::Fault`
  carrying that `InternalFault` and check nothing further. FR-127 settles it
  `failed` (FR-315 "Internal faults").
- The checker SHALL read each part's proof as a sequence of Alethe
  commands and SHALL number its commands from 0 in the proof's flattened
  text order: each command, whether `assume`, `step`, `anchor` or another
  Alethe command, and each command inside a subproof, the `step` that closes
  the subproof included, takes the next number, whatever its `:id`.
  `ProofStep { part, index }` and
  `Unverifiable { part, step }` name the command with that number in that
  part's proof.
- The checker SHALL reject with `Malformed` at `Query { part }` a part
  whose proof is not a sequence of well-formed Alethe commands: bytes that
  are not UTF-8, text that does not parse as Alethe commands, a subproof
  that is not closed, or an `:id` defined twice.
- The checker SHALL reject with `NotRefutation` at `Query { part }` a part
  whose proof has no command.
- The checker SHALL reject with `ProofStepInvalid` at the first command
  whose rule is in the checked Alethe rule set below and whose conclusion
  does not follow by it from its premises, or whose premises are not
  earlier commands or assertions of its query.
- The checker SHALL reject with `NotRefutation` at the last command of a
  proof whose last command does not conclude the empty clause.
- When no rule above rejects and some command's rule is outside the
  checked set (`hole` and `lia_generic` included), the checker SHALL return
  `Unverifiable` naming the first such command: the certificate is not
  shown wrong, and the proof settles `proved`, `Uncertified`.
- The checker SHALL apply each rule above, in the order listed, to the base
  part and then to the step part before it applies the next rule, and SHALL
  report the first that fails.
- The checker SHALL return `Verified` only when every carried proof
  refutes its query using checked rules alone, both the base and the step
  for `Inductive`.
- The checker SHALL read nothing from the solver but the certificate.

### Encoding limits

- For an SMT proof, the item's `CertificateRequest.smt_encoding` (FR-338)
  SHALL record the `SmtEncodingLimits` of the item's request
  (`ModelCheckRequest.smt_encoding`, FR-126), each member at the request's
  value or, when the request sets none, at its FR-255 default. These are
  the values CG's SMT backend encoded the item's queries under (FR-315
  "Size limits"), because the backend and the settlement read the one
  request.
- `check_smt_proof` SHALL rebuild every expected query by FR-315 under
  exactly the recorded limits, never under the defaults alone. FR-315's
  refinement queries carry no size measure, so `check_refinement_smt_proof`
  reads no encoding limit. The limits decide only whether
  FR-315 prints a script, never its bytes, so a query that fits the
  recorded limits rebuilds byte for byte as the solver received it,
  whatever limits a later caller would choose.
- The checker's work stays bounded by `max_certificate_bytes`, which it
  charges for every byte FR-315 prints ("Limits and cancellation"),
  whatever encoding limits the request records.

### Limits and cancellation

- The checker SHALL charge `max_certificate_bytes` one unit per byte of
  each carried query and proof as it reads them, and one unit per byte of
  each expected query as FR-315 prints it; FR-315's encoding work is
  charged to this counter and to no other.
- The checker SHALL charge `max_certificate_edges` one unit per Alethe
  command it reads and one unit per entry of a command's `:premises` and
  `:discharge` lists.
- Each charge SHALL be made before the work it pays for, in the order the
  checker applies the rules above. When a charge would exceed its limit,
  the checker SHALL stop and return `Stopped(IncompleteCause::LimitReached
  {limit, value, setting})`, naming the limit (`MaxCertificateBytes`, limit
  name `max_certificate_bytes`, or `MaxCertificateEdges`, limit name
  `max_certificate_edges`), its configured value and its setting
  (`certificate_check.max_certificate_bytes` or
  `certificate_check.max_certificate_edges`, FR-255), and SHALL apply no
  later rule. A rejection, `Unverifiable` or `Unencodable` that an earlier
  rule finds before the denied charge is returned instead.
- The checker SHALL poll the `Cancel` handle at each charge; when it is
  cancelled, the checker SHALL return `Stopped(IncompleteCause::Cancelled
  {source})` with the handle's actual Requested or Deadline source.
- A `Stopped` result is neither a rejection nor `Unverifiable`: it shows
  the certificate neither wrong nor unverifiable, and the item settles by
  FR-127's stop rule.

### Refinement certificates

`check_refinement_smt_proof` is the entry for a refinement item whose safety
half is settled from FR-146's records. It takes the item's
`RefinementCertificateRequest` and no `SmtProofBasis`: each record still settles `Inductive{depth: 1}` (FR-146), and
the basis variant rules above do not apply to this entry. Let `N` be the
number of step rows of the recompiled refinement's `steps` array.

- The checker SHALL first reject with `ShapeMismatch` a certificate whose
  `steps` holds fewer or more than `N` parts: at `Step { row: Some(len) }`
  for fewer, `len` being the number of parts present, and at
  `Step { row: Some(N) }` for more.
- The checker SHALL then encode the expected queries with FR-315's refinement
  queries (the initial query for `base`, the step query of row `r` for
  `steps[r]`) and SHALL reject with `QueryMismatch`, at `Base` or at
  `Step { row: Some(r) }`, the first part in the order `base`, then rows
  `0..N`, whose carried query differs byte for byte. A step part carried in
  the `base` position therefore rejects `QueryMismatch` at `Base`. A row
  whose query FR-315 refuses to encode (a `_` argument, an `any` row, a
  protocol or `binds_result` row, or a term outside `QF_LIA`: FR-315 "Refinement queries") has no expected query: its part is neither compared nor
  checked.
- The checker SHALL then check the proofs of `base` and of the encodable
  `steps[r]` in that order by the proof rules above, naming
  `ProofStep { part, index }` with `part` `Base` or `Step { row: Some(r) }`.
- When no rule above rejects, the checker SHALL return `Unverifiable` naming
  the first command with an unchecked rule, in the order `base` then rows
  `0..N`, or, if a row's query FR-315 refuses to encode precedes it in that
  order, `Unencodable { part: Step { row: Some(r) } }` naming that row; a
  certificate with a refused row is never `Verified`. `Unencodable` is distinct
  from `Unverifiable`, so no command index stands for a whole part.
- When FR-315 refuses the `Initial` query, whatever the code (`out-of-fragment`
  for a mapping row outside `QF_LIA` included), the checker SHALL reject with
  `QueryMismatch` at `Base`: FR-315 prints no initial query, so no carried
  base query equals one, and the base part has no expected query to compare.
  The refusal is not `Unencodable`, which names a step row only. A certificate with
  the wrong number of step parts is still rejected `ShapeMismatch` first, because
  the shape check precedes every query comparison. The refinement
  fails closed at `Base`, the first part in the order above: no step part is
  compared or checked, and the half settles `inconclusive`,
  `CertificateRejected{QueryMismatch, Base}`, never `proved`.
- The checker SHALL return `Verified` only when every row is encodable and the
  base and every step part refute their queries using checked rules alone.

### Assembly from CG run records

A `RefinementSmtCertificate` is assembled by the QSL adapter (QSL-908) from
the CG run records of the refinement item, never read from the QSpec wire,
which carries no refinement SMT certificate (QSpec FR-331). CG's run records
for a refinement item are a `Base` record and one `Step{row}` record per step
row, `row` zero-based in the order of the refinement node's `steps` array
(QSpec FR-375).

- The adapter SHALL set `format` to `Alethe`, `base` to a `RefutedQuery` from
  the `Base` record, and `steps[r]` to a `RefutedQuery` from the `Step{row: r}`
  record, for each row `r` in `steps` order.
- For each record the adapter SHALL set `RefutedQuery.query` to the FR-315
  canonical bytes of that record's query, unchanged, and `RefutedQuery.proof`
  to the Alethe proof bytes the record carries, unchanged.
- The adapter SHALL assemble no certificate, and the result SHALL carry none,
  when a record is missing, a `Step{row}` record is duplicated or has a row
  outside `0..N`, a record is ill-formed (it carries no query or no proof), or
  a record cannot be verified as the record of the item's query and proof.
  The item then settles `proved`, `Uncertified`, as a result with no
  certificate does (FR-127); a refused assembly is never a
  `CertificateRejected` settlement. A certificate that is assembled and
  whose proof is wrong is rejected by `check_refinement_smt_proof` as above.
- The serialization of the `Base` and `Step{row}` records belongs to the CG
  run-record format; neither QSL nor QSpec defines it.

### Checked Alethe rules

The checker accepts these Alethe rules and checks each command against the
rule's definition in the Alethe specification: `assume`, `refl`, `trans`,
`cong`, `eq_reflexive`, `eq_transitive`, `eq_congruent`,
`eq_congruent_pred`, `resolution`, `th_resolution`, `contraction`,
`reordering`, `tautology`, `not_not`, `and`, `not_or`, `or`, `not_and`,
`implies`, `not_implies1`, `not_implies2`, `equiv1`, `equiv2`,
`not_equiv1`, `not_equiv2`, `ite1`, `ite2`, `not_ite1`, `not_ite2`,
`and_pos`, `and_neg`, `or_pos`, `or_neg`, `implies_pos`, `implies_neg1`,
`implies_neg2`, `equiv_pos1`, `equiv_pos2`, `equiv_neg1`, `equiv_neg2`,
`ite_pos1`, `ite_pos2`, `ite_neg1`, `ite_neg2`, `false`, `not_symm`,
`la_generic` (with its coefficients, checked by exact rational
arithmetic), `la_disequality`, `la_totality`, `la_tautology`,
`la_mult_pos`, `la_mult_neg`, `bind`, `subproof` and the `anchor`
command. Every other rule is unchecked.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-314-AC-1 | Accepted vectors: over the `Counter` subject under event-position false-extension, the TP-2 `on origin` claim `always[0,3] holds(c.value <= 3)` proved `BoundedComplete{depth: 3}`, at its horizon 3 (ADR-018 V-2), with an Alethe certificate whose unrolling query is FR-315's encoding at depth 3 and whose proof refutes it with checked rules, is accepted; `always holds(c.value <= 3)` proved `Inductive{depth: 1}` with base and step queries FR-315 encodes at depth 1, each refuted with checked rules, is accepted. Each item settles `proved`, `Certified`, success. | Test (TC-893) |
| FR-314-AC-2 | Rejected vectors: AC-1's bounded certificate with its query encoded at depth 2 is rejected `QueryMismatch` at `Unrolling`; with one step's premise replaced by a later step, `ProofStepInvalid` at that step; with its last step removed so the proof ends short of the empty clause, `NotRefutation`. AC-1's inductive certificate with an invalid step proof is rejected `ProofStepInvalid` at that `Step` step, and a `BoundedComplete` certificate offered for the `Inductive` basis is rejected `ShapeMismatch`. Each settles `inconclusive`, `CertificateRejected`. Unverifiable vector: AC-1's bounded certificate with one step's rule replaced by `hole`, or by `lia_generic`, and no other defect returns `Unverifiable` at that step and settles `proved`, `Uncertified`, the same as the bounded result with no certificate. | Test (TC-893) |
| FR-314-AC-3 | Command numbers, over AC-1's bounded certificate with its proof replaced, `A` being the term of the first `assert` of its query. The proof `(assume a0 A)` `(anchor :step t2)` `(assume t2.a0 A)` `(step t2.t1 (cl) :rule resolution :premises (t2.t2))` `(step t2.t2 (cl A) :rule hole)` `(step t2 (cl (not A)) :rule subproof :discharge (t2.a0))` is rejected `ProofStepInvalid` at `ProofStep { part: Unrolling, index: 3 }`, and at the same locus with its `:id`s renamed `a0` to `t9`, `t2` to `t0`, `t2.a0` to `t0.t8`, `t2.t1` to `t0.t7` and `t2.t2` to `t0.t1`. The proof `(assume a0 A)` `(anchor :step t2)` `(assume t2.a0 A)` `(step t2.t1 (cl A) :rule hole)` `(step t2 (cl (not A) A) :rule subproof :discharge (t2.a0))` is rejected `NotRefutation` at `ProofStep { part: Unrolling, index: 4 }`. The proof `(assume a0 A)` `(anchor :step t2)` `(assume t2.a0 A)` `(step t2.t1 (cl) :rule hole)` `(step t2 (cl (not A)) :rule subproof :discharge (t2.a0))` `(step t3 (cl) :rule resolution :premises (a0 t2))` returns `Unverifiable { part: Unrolling, step: 3 }` and settles `proved`, `Uncertified`. A proof of no bytes is rejected `NotRefutation` at `Query { part: Unrolling }`. The proofs `(assume a0 A`, `(assume a0 A)` `(assume a0 A)`, `(assume a0 A)` `(anchor :step t2)` `(assume t2.a0 A)`, and the single byte `0xff` are each rejected `Malformed` at `Query { part: Unrolling }`. AC-1's inductive certificate with a base proof of no bytes is rejected `NotRefutation` at `Query { part: Base }`, and with the step proof `(assume a0 A` `Malformed` at `Query { part: Step }`. Each rejection settles `inconclusive`, `CertificateRejected`. | Test (TC-893) |
| FR-314-AC-4 | Refinement certificates, over `CasRefinesCounter` (seven step rows): the certificate with a base part and seven step parts, each refuting its FR-315 refinement query with checked rules, is `Verified`; with six step parts it is rejected `ShapeMismatch` at `Step { row: Some(6) }`, with eight at `Step { row: Some(7) }`; with a step part in the `base` position and the base part in a step position it is rejected `QueryMismatch` at `Base`; with the third step part's query replaced by the fourth's it is rejected `QueryMismatch` at `Step { row: Some(2) }`; with the fifth step part's proof (at least four commands) having its command 3 made not to follow from its premises it is rejected `ProofStepInvalid` at `ProofStep { part: Step { row: Some(4) }, index: 3 }`; with command 3 of the base proof using `hole` it returns `Unverifiable { part: Base, step: 3 }`; over `CasRefinesCounterAdd` (FR-315-AC-6), whose `bump` row has a `_` argument so FR-315 refuses its query, with a base part and eight step parts, valid for the seven rows other than `bump` and a present part at the `bump` row's position (neither compared nor checked), it returns `Unencodable { part: Step { row: Some(r) } }`, `r` the position of the `bump` row in the `steps` order. | Test (TC-893) |
| FR-314-AC-5 | AC-1's bounded certificate, checked with `max_certificate_bytes` 1 and the default `max_certificate_edges`, returns `Stopped(LimitReached{limit: MaxCertificateBytes, value: 1, setting: certificate_check.max_certificate_bytes})` and settles `incomplete`, execution `resource-incomplete`, basis `unavailable`, never `proved`; with `max_certificate_edges` one below the proof's command count plus its premise and discharge entries it returns `Stopped` naming `MaxCertificateEdges` and `certificate_check.max_certificate_edges`; with `max_certificate_edges` exactly that count it returns `Verified`; with both at their FR-245 defaults it returns `Verified`. AC-1's bounded certificate checked with a `Cancel` handle cancelled before the call returns `Stopped(Cancelled{source: Requested})` and settles `incomplete`, `cancelled{requested}`. AC-2's `QueryMismatch` certificate, whose query differs at its first byte, checked with `max_certificate_bytes` equal to its carried query's bytes plus the printed bytes of its expected query, which leaves out its proof's bytes and so lies below what the whole check charges, is rejected `QueryMismatch`, not `Stopped`: the rejection is found before the proof's bytes are charged. AC-4's `CasRefinesCounter` certificate checked by `check_refinement_smt_proof` with `max_certificate_bytes` 1 returns `Stopped` naming `max_certificate_bytes`. Checking each certificate twice under the same limits gives equal results. | Test (TC-1165) |
| FR-314-AC-6 | Each entry returns `Result<SmtProofCheck, CertificateRejection>`, and its `Ok` is never empty: AC-1's accepted bounded and inductive certificates return `Ok(SmtProofCheck::Verified)`; AC-2's `QueryMismatch` certificate returns `Err(CertificateRejection{rule: QueryMismatch, at: Query { part: Unrolling }})`; AC-2's `hole` certificate returns `Ok(Unverifiable { part: Unrolling, step })` at that step; AC-4's `CasRefinesCounterAdd` certificate returns `Ok(Unencodable { part: Step { row: Some(r) } })`; AC-5's limit-stopped certificate returns `Ok(Stopped(LimitReached{..}))`. AC-1's bounded certificate checked for the TP-2 `on origin` claim `always[0,3] holds(c.value * c.value <= 9)` at `BoundedComplete{depth: 3}`, whose unrolling FR-315 refuses `out-of-fragment`, returns `Err` with `QueryMismatch` at `Query { part: Unrolling }` and settles `inconclusive`, `CertificateRejected`. | Test (TC-1199) |
| FR-314-AC-7 | FR-314-AC-1's bounded certificate for `always[0,3] holds(c.value <= 3)` at `BoundedComplete{depth: 3}`, whose unrolling query declares eight constants, checked with a `CertificateRequest` whose `smt_encoding` has `max_constants` 8 and every other member at its default, returns `Verified`: the checker rebuilds the query under 8 and it equals the carried one byte for byte. The same certificate with `smt_encoding.max_constants` 7 is rejected `QueryMismatch` at `Query { part: Unrolling }`, FR-315 refusing the rebuild `too-large` naming `constants`, value 8, limit 7 and setting `smt_encoding.max_constants`, and settles `inconclusive`, `CertificateRejected`, never `proved`. A request recording the defaults returns `Verified`. | Test (TC-1250) |
| FR-314-AC-8 | Over FR-314-AC-1's `Counter` under event-position false-extension, the TP-2 `on origin` claim `always[0,3] eventually[0,1] holds(c.value = 3)`, horizon 4, which FR-315-AC-35 refutes: a `BoundedComplete{depth: 0}` certificate whose carried query and proof are otherwise well formed is rejected `ShapeMismatch` at `Query { part: Unrolling }` before any query is encoded, and settles `inconclusive`, `CertificateRejected`, never `proved`; a `BoundedComplete{depth: 0}` result for it with no certificate settles the same. FR-314-AC-1's `always[0,3] holds(c.value <= 3)` with a `BoundedComplete{depth: 2}` basis, below its horizon 3, is rejected the same way, and so is a `BoundedComplete{depth: 3}` basis for the TP-1 item `always holds(c.value <= 3)`. FR-314-AC-1's accepted `BoundedComplete{depth: 3}` certificate is still accepted. | Test (TC-1201) |
| FR-314-AC-9 | The QSL adapter (QSL-908) assembles a `RefinementSmtCertificate` from CG's `Base` record and `Step{row}` records: over `CasRefinesCounter` (seven step rows), given a `Base` record and seven `Step{row}` records for rows 0 to 6, it yields `format` `Alethe`, `base` carrying the `Base` record's FR-315 canonical query bytes and proof bytes unchanged, and `steps[r]` carrying the `Step{row: r}` record's query bytes and proof bytes unchanged, for every `r`, in row order, with the records supplied out of row order. | Test (TC-893) |
| FR-314-AC-10 | Over AC-9's records, the adapter assembles no certificate, and the item settles `proved`, `Uncertified`, not `CertificateRejected`, when the `Base` record is missing, when the `Step{row: 3}` record is missing, when the `Step{row: 3}` record is supplied twice, when a `Step{row: 7}` record is supplied, and when the `Step{row: 3}` record carries no proof. | Test (TC-893) |
| FR-314-AC-9 | Over `CasRefinesCounter` with a mapping row outside `QF_LIA`, so that FR-315 refuses `Initial` `out-of-fragment` naming the row, a certificate with a base part and seven step parts, the base part carrying the query of an in-fragment initial state, and the same certificate with an empty base part, are each rejected `QueryMismatch` at `Base`, never `Unencodable` and never `Unverifiable`, whatever the step parts carry (a certificate with six or eight step parts is rejected `ShapeMismatch` first, as in AC-4) (a `hole` proof in a step part included), and the refinement's `Initial` record settles `inconclusive`, `CertificateRejected{QueryMismatch, Base}`. Over `CasRefinesCounterAdd`, whose `bump` query alone is refused, the same certificate is not rejected: it returns `Unencodable`, `proved`, `Uncertified`. | Test (TC-893) |

## Dependencies

- ADR-018 V-2, V-3, PC-1, PC-6, LA-2, LA-3, DS-2.
- [FR-315](FR-315-encode-the-smt-lib-transition-relation.md) (the encoding
  and its canonical printing), [FR-338](FR-338-check-an-en-1-closure-certificate.md)
  (the request, rule and rejection types and the wire spelling of a
  `QueryPart`), QSpec FR-468 (the refinement's queries and certificate), QSpec FR-375 (the
  `steps` order the CG records follow),
  [FR-163](FR-163-check-a-hyper-item-s-product-closure-certificate.md)
  (the `Malformed` rule),
  [FR-127](FR-127-settle-a-model-check-verdict-as-a-terminal-record.md)
  (the settlement map that runs the check),
  [FR-245](FR-245-check-a-zone-certificate-in-the-qualified-core.md)
  (`CertificateCheckLimits`),
  [FR-276](FR-276-cancel-a-lifecycle-operation.md) (the `Cancel` handle) and
  [FR-255](FR-255-name-the-setting-that-raises-a-reached-limit.md) (the
  limits' setting names).

## References

- The Alethe proof format and its rule definitions (cvc5).
- Owning ticket: Linear QSL-366; the label table: Linear QSL-390; the
  recorded encoding limits: Linear STD-408.
