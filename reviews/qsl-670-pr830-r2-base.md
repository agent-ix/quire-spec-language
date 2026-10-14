---
id: SR-9113
title: "Spec review of QSL-670 chain-link identity reconciled to QSpec #252 (PR 830)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@d743c882ce048292f3669a589199a6c9fccfc043; spec/functional/FR-081, spec/functional/FR-084 (AC-7), spec/non-functional/NFR-012, spec/test-cases/TC-1022..TC-1025, TC-410, TC-434, spec/tests.md, spec/decisions/ADR-013, ADR-017"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-081
    type: reviews
  - target: ix://agent-ix/quire-spec-language/NFR-012
    type: reviews
---
# Spec review of QSL-670 chain-link identity reconciled to QSpec #252 (PR 830)

## Summary

Ticket: QSL-670. Base spec review of PR #830 at d743c882c against main and the
merged QSpec #252 (merge 1e6d75a45904de67cd26b389959d2ac3e5696566; read from
quire-specification origin/main: TC-195 N01/N02/N12/N13/N15/N19,
value-accounting.md normalize.* rows, model-complete.md, FR-150, FR-201, the
effective-declaration vectors). Private detail stays in this file; anything
posted cites QSpec by id only.

Independent recomputation (script: JCS of the #252 preimage shape, checked
first against N01's exact lengths 366/666/491/695, universe 335, view 3101):

- N19 chain T0..T10000: 10,001 types, 20,001 facts, 15,104,845 hashed bytes,
  60,006 work units. Matches FR-081-AC-12, TC-1025 and NFR-012.
- N13: chain 149,894 B / 199 / 600; ladder 209,400 / 296 / 794 (universe 461 =
  335 + 126); dense 108,988 / 165 / 392. All match TC-1024.
- View header 264 B; frame 154 B; per type exactly 559 + 4k, per edge 436 + 2k,
  constant 264 + 335 - 1. TC-1024 step 3's prose formula reproduces N19 exactly.
  ("about 902 per type" is 903 at k = 86; approximate wording, not a defect.)
- N12 = 76: 8 records, 24 facts, 4 cycle-checks, 13 declarations, 15 hashes =
  64; redefinition-check B/x2 m=2 r=0, D/x4 m=4 r=1 = 7; conflict-check on
  (D, A/x), c=2, f(B)+f(D) = 2+3 = 5. Matches #252 N12 and value-accounting.
- NFR-012: chain at the record ceiling 151,633,328 B (exact); 50-parent shape
  over 10,001 types 312,988,887 B (~313 MB, refused); edges over 10,001 types
  ~425,000-426,600 (about 426,000, fine); effective-declaration bound 1,786
  (exact: n=1786 gives 1,599,365, n=1787 gives 1,601,154). The 527 bound does
  not reproduce (FND-003).

Domain moves agree with #252 model-complete.md: only effective-declaration,
effective-view, object-universe and the inherit/subset/redefine rule ids go to
/v2; complete/v1, complete.rules/v1 and accounting/v1 stay. Fact shape
{ordinal, rule, inputs, parent} with parent only on inherit/v2 agrees with #252.
The phase-3 type-hash exception and the 524,288-byte bound are stated
explicitly as QSL-only, but the bound is not achievable (FND-002).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-081 and NFR-012 give every effective declaration a qualify fact ("one qualify fact, one inherit fact per direct parent, and the phase-4 facts"). Under QSpec #252 (model-complete member rule: qualify only when T is the declaring type; TC-195 N01/N02/N15) an inherited member has no qualify fact. Consequences: TC-1022 step 4 expects (D, f) to hold "one qualify fact and two inherit facts" (correct: two inherit facts only), and TC-434 expected 6 states 5,061 retained facts with 4,820 for members for a 121-type chain whose root declares 20 fields (correct: 121 x 20 = 2,420 member facts, 2,661 total). | FR-081:124-126; NFR-012:97-99; TC-1022:36-38; TC-434:61-62 |
| FND-002 | high | FR-081-AC-12 and TC-1025 step 1 require the 10,001-type chain's canonical package encoding to be at most 524,288 bytes (52.4 B per type) and to fail setup rather than shrink. One type entry needs at least its identity and its supertype reference: {"identity":"ix://test/orders/T5000","supertypes":["ix://test/orders/T4999"]} alone is 77 B, and a field-less type in the repo's own semantic-IR fixture shape (tests/fixtures/spine-model.semantic-ir.json, "Gadget") is 389 B, about 3.9 MB for 10,001 types. The bound is unachievable, so TC-1025 always fails setup and AC-12/AC-13 cannot pass. | FR-081:262; TC-1025:18-19,28-29 |
| FND-003 | medium | NFR-012 says the field-per-type chain "is refused beyond 527 edges" on hashed_bytes. Recomputed with the #252 preimage shape (N01-validated lengths; field keys T<i>/f), the chain is admitted to 584 edges (n=584: 268,295,952 B; n=585: 269,212,521 B); 579-584 for other plausible field names. 527 reproduces (525) only when every inherited member also carries a qualify fact, i.e. FND-001's error. | NFR-012:110-113 |
| FND-004 | medium | NFR-012's work bullet says each record, fact, declaration, hash and direct-edge cycle check charges one unit and "the work ceiling does not bind before the fact ceiling for an acyclic package". FR-081 (this PR) and #252 value-accounting also charge normalize.redefinition-check m + r and normalize.conflict-check sum (c-1) f(o), which are quadratic. Counterexample: A with N=2,600 fields, B < A with 2,600 fields each redefining one of A's: about 13,000 retained facts (far under 1,600,000) but redefinition-check work N x 2N + N(N-1)/2 = 16,898,700 > 16,777,216, so the work ceiling binds first. | NFR-012:117-119; FR-081:231-237 |
| FND-005 | medium | The /v1 -> /v2 rename in ADR-013 is incomplete. Lines 197, 215 and 1227 move to object-universe/v2 and effective-declaration/v2, but the O-05 Conversions row (line 199: "UniverseId carries the quire.model.object-universe/v1 digest"), OQ-E (line 1229: "FR-201's quire.model.object-universe/v1 row"; #252's FR-201 row is now /v2) and QC-22 (line 1162) still say /v1. ADR-013 contradicts itself and the merged QSpec FR-201. | ADR-013:199,1162,1229 |

Recorded (not blocking, one line each): "about 902 bytes per type" is 903 at
k = 86 (approximate); TC-434 step 4 "2^18 paths" for an 18-level lattice is
2^17 per bottom type (pre-existing text); TC-1024's "every type has 50 parents"
holds only after the first 50 types.

## Verdict

Not mergeable as written: five blocking findings (two high). Clean:
N19/N13/N12 figures and the byte formula, the /v2 domain set and fact shape vs
#252, AC-9..AC-13 each bound to an exercising TC, TC renumbering complete
(TC-1022..TC-1025 in front matter, tests.md, FR-081 AC rows and dependencies,
NFR-012; main's TC-1021 intact as the protocol-reference case), ADR-017 line,
FR-084-AC-7 and TC-410 renames, test-parsed FR-092/FR-255 untouched, and
quire validate on the changed files clean (module warnings only), consistent
with the coder's make check-spec-validation rc=0 log.
