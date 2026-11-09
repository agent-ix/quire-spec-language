---
id: SR-20261109172912000017
title: "spec-review/scope-boundary of QSL-1826 certificate profile"
type: SpecReview
analysis: scope-boundary
scope: "agent-ix/quire-spec-language; QSL-1826; spec/functional/FR-201-check-a-probability-certificate.md, spec/functional/FR-282-check-analyze-certificates-in-the-qualified-core.md, spec/decisions/ADR-028-exact-probabilistic-engine.md"
review_set: subset
---

## Summary

Frozen head 08f30253e772cdba31f9ddcd705a44f55e6b688e; diff origin/main...HEAD. Ticket QSL-1826. Only FR-201, FR-282 and ADR-028 changed. No implementation exists for the new codec/facade obligations; this is a pre-implementation specification review, not acceptance of those obligations. No active AssuranceProfile document was found in spec/; no enforced review run is required by a profile. AGENTS authorizes selecting applicable analyses without another interview. No source edits, cargo gate or whole-repository matrix were run.

## Verdict

**PASS** — no concrete introduced defect found; this specification slice is mergeable. Implementation and full acceptance evidence remain future work.

## Analysis

FR-201 assigns closed-profile writer/strict-reader semantics and independent checking to the QSL probability certificate path (core). FR-282 assigns bounded source admission, qualified S1-S3 compilation and fresh-process composition to the qualified facade (core), with caller limits/cancellation cross-cutting. QSpec owns the consumed identity/value/key transport contract; compatibility is guaranteed by the referenced schema and planned round-trip tests, not an independently restated upstream schema. QSL owns the objective specialization and complete key algebra. Solver/provider output is untrusted evidence only. A fresh executable must have no provider/qsl-analyze dependency; the source path cannot depend on S4 emission or fabricated replay wire. Existing FR-282-AC-1 binds bytes to analyze production emission, and AC-11 extends that acceptance path to S4-refused random/workload packages. No producer dependency or new product approval flow is introduced.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Examined scope

```yaml
scope:
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: 'For the untimed, acyclic, unbounded `eventually` reachability slice, the

    document codec SHALL implement the following QSL-owned profile of QSpec

    FR-412''s open, nonempty `objective` object.'
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: 'This profile does not narrow the

    other objective obligations above and below; unsupported forms SHALL remain

    explicitly unsupported rather than be encoded as this profile.'
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "The objective SHALL have exactly `profile`, `targets`, `optimization`\
    \ and\n  `fairness`."
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "`profile` SHALL be\n  `\"quire.probability-objective/reachability/v1\"\
    `; `targets` SHALL be the\n  complete set of reachable target product keys, transported\
    \ using QSpec\n  FR-331's `ProbabilityProductKey`; `optimization` SHALL be `\"\
    minimum\"`,\n  `\"maximum\"` or `\"workload\"`; `fairness` SHALL be the empty\
    \ array."
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "An empty\n  target set is valid."
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "Targets are extensional evidence, not an expression\n  encoding or a trusted\
    \ qualitative set: the checker SHALL independently\n  evaluate the checked claim's\
    \ target over its own reachable product and\n  compare the exact set before acceptance."
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "It SHALL rederive optimization\n  from the checked claim, not from certificate\
    \ values.\n- Admission SHALL require one subject initial state, at most one admitted\n\
    \  claim binding, no clocks, reward, authored fairness, horizon or other\n  temporal\
    \ form, and an acyclic continuation graph apart from implicit\n  terminal stutter."
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "Workload admission SHALL require a single resolved\n  distribution at\
    \ each live state."
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "Unsupported shape, profile version,\n  optimization or certificate kind\
    \ SHALL give an explicit unsupported\n  result with no accepted certificate.\n\
    - The document SHALL have exactly `identity`, `objective`, `kind`, `values`\n\
    \  and `ranking`, with `kind` equal to `\"exact\"`."
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "Identity SHALL use the\n  existing structural `{node_id, model, ordinal}`\
    \ contract."
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "Values SHALL\n  use QSpec's existing `{state, value}` rows and `Rational`\
    \ representation\n  `{numerator, denominator}`, both canonical decimal integer\
    \ strings with\n  a positive denominator (zero is `\"0\"/\"1\"`);\n  ranking SHALL\
    \ be required and use its existing `{state, rank}` rows and\n  `Count` (JSON integer\
    \ 0 through 9007199254740991)."
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "No new top-level\n  discriminator, package identity or field is introduced."
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "Lower, upper,\n  long-run, policy and component documents are outside\
    \ this codec profile.\n- The writer SHALL use RFC 8785 UTF-8 canonical JSON."
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "The reader SHALL\n  reject malformed UTF-8/JSON, duplicate or unknown\
    \ members, missing\n  members, noncanonical bytes, nonreduced rationals, nonpositive\n\
    \  denominators and noncanonical integer strings rather than normalize\n  them\
    \ into accepted evidence."
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "It SHALL use QSpec's existing unpadded\n  canonical base64url transport\
    \ for state keys."
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "`targets`, `values` and\n  `ranking` SHALL each be strictly ascending\
    \ by decoded complete canonical\n  key octets, with no duplicate keys."
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "Values and ranking SHALL cover\n  exactly the checker-rebuilt reachable\
    \ states, including terminal states;\n  probabilities SHALL lie in [0,1].\n- Keys\
    \ SHALL use FR-196's authoritative complete six-component basis,\n  including\
    \ the actual `binding`, through the shared\n  `ProbabilityProductKey::for_model_state`\
    \ helper for this slice."
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "The stale\n  five-component illustrative tuple in QSpec FR-412/schema\
    \ documentation\n  SHALL NOT cause binding to be dropped."
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "Actual absent monitor,\n  accumulator and pending components remain null."
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "The reader SHALL admit\n  keys against the checked subject and compare\
    \ complete canonical octets;\n  a model-only key, foreign binding or altered component\
    \ is not an alias.\n- The checker SHALL check Bellman equality for the independently\
    \ selected\n  operator at every live state, target value 1 and non-target terminal\n\
    \  value 0, and CE-3's positive-value rank progress as well as threshold\n  satisfaction."
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "It MAY emit checker-verified acyclic height ranks, with\n  terminal height\
    \ 0 and live height one plus the maximum successor height;\n  target ranks can\
    \ be 0."
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: Rank overflow SHALL stop under a named limit.
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: Equality alone SHALL NOT discharge the required ranking condition.
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "Every-scheduler `AtLeast` selects minimum and `AtMost` selects maximum;\n\
    \  workload uses its resolved average."
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "Exact evidence in these proof cases\n  needs no crossing-bound policy."
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "A case needing an upper-on-minimum or\n  lower-on-maximum policy SHALL\
    \ NOT fabricate or silently omit that policy.\n- Identity mismatch SHALL refuse\
    \ as conflicting identity before product\n  enumeration."
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "Objective mismatch SHALL refuse as invalid runtime input\n  before acceptance;\
    \ an internally consistent certificate for another\n  target or optimization SHALL\
    \ never establish this claim."
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "Graph,\n  arithmetic, parsing and source admission SHALL obey caller limits\
    \ and\n  `&Cancel`, preserving stopped/cancelled outcomes without truth values."
- id: FR-201-AC-9
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: For an admitted acyclic eventual claim, writing and reading the closed
    profile preserves identity, exact rational values, complete keys, targets and
    required ranks byte for byte. Duplicate/out-of-order rows, unreduced 2/4, denominator
    0, missing ranking, extra fields, padded base64url and a model-only key are rejected.
- id: FR-201-AC-10
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: A valid document offered for another identity refuses before enumeration;
    replacing its targets or optimization while retaining valid Bellman rows refuses
    objective mismatch. A same-model key with a different binding is rejected rather
    than merged. Altering a value fails equality; removing or breaking positive-value
    rank progress fails ranking even when equalities hold.
- id: FR-201-AC-11
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: Minimum AtLeast, maximum AtMost and resolved-workload exact proofs pass
    the profile checker at an attained threshold; a threshold strictly beyond the
    certified initial value fails. Cyclic, timed, bounded, reward, fair, multi-initial
    and multi-binding cases and non-exact kinds return explicit unsupported with no
    accepted certificate. Pre-cancelled input and exhausted byte/state/edge/arithmetic
    budgets stop without a truth value.
- id: FR-282
  path: spec/functional/FR-282-check-analyze-certificates-in-the-qualified-core.md
  role: examined
  excerpt: "For FR-201's closed reachability profile, the fresh-process facade SHALL\n\
    \  admit the bounded source `ByteProvision`, re-check S1 to S3 into a\n  `CheckedPackage`,\
    \ admit the `ModelSystem` and selected claim binding,\n  read the certificate\
    \ document through that profile's strict reader and\n  invoke the independent\
    \ core checker with the caller's limits and\n  `&Cancel`."
- id: FR-282
  path: spec/functional/FR-282-check-analyze-certificates-in-the-qualified-core.md
  role: examined
  excerpt: "It SHALL extract/reuse the source completeness and provision\n  identity\
    \ admission in `qsl-replay/src/request.rs`, together with existing\n  content-hash\
    \ validation wherever required by the provision's source\n  contracts; it SHALL\
    \ NOT duplicate those semantics or invent a new hash\n  identity."
- id: FR-282
  path: spec/functional/FR-282-check-analyze-certificates-in-the-qualified-core.md
  role: examined
  excerpt: "Source and document byte ceilings SHALL be\n  enforced before unbounded\
    \ allocation or source lookup."
- id: FR-282
  path: spec/functional/FR-282-check-analyze-certificates-in-the-qualified-core.md
  role: examined
  excerpt: "The facade SHALL\n  observe cancellation throughout admission, parsing\
    \ and checking.\n- That facade SHALL expose the checked-source path independently\
    \ of S4\n  emission."
- id: FR-282
  path: spec/functional/FR-282-check-analyze-certificates-in-the-qualified-core.md
  role: examined
  excerpt: "`compile_package` runs S4 and therefore SHALL NOT be required\n  to obtain\
    \ this package: random/workload packages refused by FR-185 still\n  reach the\
    \ checker through qualified S1 to S3."
- id: FR-282
  path: spec/functional/FR-282-check-analyze-certificates-in-the-qualified-core.md
  role: examined
  excerpt: "The facade SHALL NOT require\n  a fake emitted package or fabricated `ReplayRequestWire`\
    \ for this path."
- id: FR-282
  path: spec/functional/FR-282-check-analyze-certificates-in-the-qualified-core.md
  role: examined
  excerpt: "The existing\n  replay-wire entry retains its own decode/recompile obligations.\n\
    - The fresh checker executable SHALL link no provider or `qsl-analyze`\n  dependency\
    \ and SHALL use only the certificate document, provision,\n  checked subject/claim\
    \ binding and caller controls as evidence."
- id: FR-282
  path: spec/functional/FR-282-check-analyze-certificates-in-the-qualified-core.md
  role: examined
  excerpt: "It SHALL\n  enforce FR-201's identity and independently rederived objective\
    \ comparison\n  before accepting the document."
- id: FR-282-AC-11
  path: spec/functional/FR-282-check-analyze-certificates-in-the-qualified-core.md
  role: examined
  excerpt: An exact acyclic eventual certificate from a random/workload source package
    that S4 refuses is written, then accepted in a fresh executable linking only qualified-core
    crates after bounded source admission and S1 to S3 checking. No emitted-package
    bytes or fabricated replay request are supplied. The same executable rejects changed
    values, refuses mismatched identity/objective, and preserves typed source/hash
    failures.
- id: FR-282-AC-12
  path: spec/functional/FR-282-check-analyze-certificates-in-the-qualified-core.md
  role: examined
  excerpt: With a source provision or certificate document one byte above its caller
    ceiling, the fresh checker stops/refuses before reading beyond that ceiling; pre-cancellation
    and cancellation during graph expansion stop without acceptance or a truth value.
    Exhausted state, edge and arithmetic budgets retain the actual named setting and
    consumption.
- id: ADR-028
  path: spec/decisions/ADR-028-exact-probabilistic-engine.md
  role: examined
  excerpt: 'FR-201''s closed reachability document profile owns the QSL specialization

    inside QSpec FR-412''s existing open `objective` object.'
- id: ADR-028
  path: spec/decisions/ADR-028-exact-probabilistic-engine.md
  role: examined
  excerpt: 'Its versioned

    `profile` selects extensional complete target keys, independently checked

    against the claim, the existing minimum/maximum/workload optimization and

    an explicitly empty fairness array.'
- id: ADR-028
  path: spec/decisions/ADR-028-exact-probabilistic-engine.md
  role: examined
  excerpt: 'Extensional targets avoid inventing a

    second semantic expression language.'
- id: ADR-028
  path: spec/decisions/ADR-028-exact-probabilistic-engine.md
  role: examined
  excerpt: 'The discriminator versions this closed

    member contract without adding a top-level certificate field.'
- id: ADR-028
  path: spec/decisions/ADR-028-exact-probabilistic-engine.md
  role: examined
  excerpt: 'FR-196''s

    complete binding-bearing key basis remains authoritative despite QSpec''s

    older five-component illustration.'
- id: ADR-028
  path: spec/decisions/ADR-028-exact-probabilistic-engine.md
  role: examined
  excerpt: 'Required exact ranks and Bellman

    equalities both remain CE-3 obligations.'
- id: ADR-028
  path: spec/decisions/ADR-028-exact-probabilistic-engine.md
  role: examined
  excerpt: 'This first codec slice is untimed,

    acyclic, unbounded eventual reachability with one initial state and at most

    one binding; unsupported forms retain all broader CE obligations and are

    not coerced into this profile.'
- id: ADR-028
  path: spec/decisions/ADR-028-exact-probabilistic-engine.md
  role: examined
  excerpt: 'FR-282 defines the independent checked-source

    S1 to S3 facade so S4''s emission refusal cannot prevent fresh checking.'
- id: FR-196
  path: spec/functional/FR-196-build-the-probabilistic-product.md
  role: context_only
  excerpt: '["quire.probability-product-key/v1",binding,model,monitor,accumulator,pending]'
- id: ADR-028-CE-3
  path: spec/decisions/ADR-028-exact-probabilistic-engine.md
  role: context_only
  excerpt: '**Exact**: both, with equality, so `x` is the value.'
- id: QSpec-FR-412
  path: agent-ix/quire-specification:spec/objects/interfaces/FR-412-probability-certificate.md
  role: context_only
  excerpt: 'The checker accepts a certificate that meets its conditions and puts the

    initial state''s value on the claim''s side of the threshold'
- id: request.rs
  path: qsl-replay/src/request.rs
  role: context_only
  excerpt: "// Completeness: every named source, the proved package's and each\n \
    \       // dependency's, must have a matching entry (FR-071-AC-5, AC-9)."
```
