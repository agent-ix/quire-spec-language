---
id: SR-20261109172912000029
title: "code-review \u2014 QSL-1826 certificate codec"
type: SpecReview
analysis: code-review
scope: agent-ix/quire-spec-language; Cargo.lock; Cargo.toml; README.md; qsl-analyze/src/analyze/exact.rs;
  qsl-analyze/tests/it/exact_probabilistic_item.rs; qsl-analyze/tests/it/provision_contract.rs;
  qsl-probability-check/Cargo.toml; qsl-probability-check/README.md; qsl-probability-check/src/main.rs;
  qsl-probability-check/tests/fixtures/link.native; qsl-probability-check/tests/fixtures/link.semantic-ir.json;
  qsl-probability-check/tests/fixtures/link.snapshot.json; qsl-probability-check/tests/it/main.rs;
  qsl-probability-check/tests/it/roundtrip.rs; qsl-replay/src/lib.rs; qsl-replay/src/model_check/limits.rs;
  qsl-replay/src/probability_certificate.rs; qsl-replay/src/probability_certificate/document.rs;
  qsl-replay/src/probability_certificate/source.rs; qsl-replay/src/probability_certificate/witness.rs;
  qsl-replay/src/proof_result.rs; qsl-replay/src/request.rs; spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md;
  qsl-replay/src/probability_certificate/document/writer.rs; qsl-foundation/src/node_wire.rs;
  qsl-foundation/Cargo.toml; FR-196 canonical product representation
review_set: subset
---

## Summary

Reviewed all 23 changed files, public acceptance ownership, actual source S1–S3 admission, independent operator/target/workload derivation, full-key equality, canonical transport, value/rank math, error precedence and normal dependency direction. One resource-admission defect blocks merge.

Frozen head b8e90511601149793afae8ddcfb626f3f6a056b3, base 91c6158a171f335ba51dbc129f936e5b7a787096. Ticket: QSL-1826. Review scope is untimed, unbounded acyclic exact eventual reachability only; bounded monitor implementation, broader certificates, malformed replay-wire forms and refutation/witness work remain open.

## Verdict

**FAIL — not mergeable until FND-001 is fixed.** The accepted token owns exactly the production writer bytes, and terminal/provider projection retains them. No evidence is produced on rejected or stopped paths. Independent equality and rank checks remain separate; workload uses the checked production weighted-step distribution, including the 391/400 positive case and altered-weight rejection. Full identity, ordinal/model and binding comparisons prevent aliases. No new package pins, unsafe code, ambient I/O in core, runtime Cargo or analyzer normal dependency was found. Existing peer provider APIs are preserved.

FND-001 reproduction: the scratch Rust probe constructs one complete 8,000,000-byte model key before measurement and calls the public writer with bytes=0, storage_bytes=0 and states=0. VmHWM rises from 35388 KiB to 76648 KiB before returning Bytes(bound=0, actual=1); the extra allocation is the three base64 row strings built at lines 572–607. `canonical()` also constructs JSON bytes and an unbounded canonical tree before its final bounded reader checks storage. The current zero-budget analyzer test checks only the eventual limit result, not admission ordering. Probe source/result are adjacent in this scratch directory.

Validation: inspected captured all-target Clippy success and focused core 14, analyzer 11, fresh process 9 and provider 1 passing tests, plus format and normal-dependency tree logs `/tmp/qsl1826-provider-*`. These were not redundantly rerun. The new targeted Rust probe ran against this target directory's built production rlib and exited 0. No aggregate/make ci/Kani/full matrix run; cargo-deny and broad boundary/string-edge lanes remain unverified in this pass.

## Assurance Context

No active AssuranceProfile applies: repository search found mentions in historical review artifacts, not a current profile declaration. No active exception was used. No enforced review workflow was required. Full computed matrix and aggregate architecture/string-edge/deny gates were not run under the explicit scoped-review brief; no full assurance completion is claimed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The writer materializes all encoded wire rows before byte/storage admission: even bytes=0 and storage_bytes=0 allocate document-sized base64 and rational strings before returning a limit. Large solver evidence can exhaust memory before the promised bounded stop; stream/count and reserve before retaining wire rows and canonical trees. | qsl-replay/src/probability_certificate/document.rs:555 |

## Coverage

Examined units (clean units included):

```yaml
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
    certified initial value fails. Unbounded cyclic, timed, reward, fair, multi-initial
    and multi-binding cases, bounded forms outside the admitted subset and non-exact
    kinds return explicit unsupported with no accepted certificate. Pre-cancelled
    input and exhausted byte/state/edge/arithmetic budgets stop without a truth value.
- id: FR-282-AC-1
  path: spec/functional/FR-282-check-analyze-certificates-in-the-qualified-core.md
  role: examined
  excerpt: 'The EN-5 certificate from FR-281-AC-1, written to bytes by `analyze` and
    passed with the byte provision and the subject binding to the EN-5 checker in
    a fresh process that links only qualified-core crates, is accepted: the process
    re-checks the provision''s sources into a `CheckedPackage`, admits the `ModelSystem`
    and runs the checked-subject form.'
- id: FR-282-AC-2
  path: spec/functional/FR-282-check-analyze-certificates-in-the-qualified-core.md
  role: examined
  excerpt: The same certificate with one probability bound changed is rejected with
    a cause naming the failing obligation.
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
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "Graph,\n  arithmetic, parsing and source admission SHALL obey caller limits\
    \ and\n  `&Cancel`, preserving stopped/cancelled outcomes without truth values."
- id: FR-255
  path: spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md
  role: examined
  excerpt: '| `certificate_check.max_certificate_bytes` | `max_certificate_bytes`
    | certificate checkers | `CertificateCheckLimits` max_certificate_bytes; EN-5
    `ExactProbLimits.document.bytes` | 1073741824 bytes | FR-245, FR-201 |'
- id: FR-255
  path: spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md
  role: examined
  excerpt: '| `certificate_check.max_working_bytes` | `max_working_bytes` | probability
    document writer/reader | `ExactProbLimits.document.storage_bytes`; source checker
    document storage ceiling | 1073741824 working bytes | FR-201, FR-282 |'
- id: FR-255
  path: spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md
  role: examined
  excerpt: '| `certificate_check.max_rank` | `max_rank` | exact reachability document
    writer | closed-profile Count representability ceiling; a stop requires a different
    supported representation, not a larger Count | 9007199254740991 rank units | FR-201
    |'
```


## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | high | The shared helper and strict reader use a base64 string as the model component, contradicting FR-196's parsed ["quire.simulation.state-key/v1",model_form] array. Every produced unbounded document therefore has nonconforming keys, while a conforming document is refused. Embed the parsed owner model form with its required wrapper and reject the former nested transport without fallback. | qsl-replay/src/probability_certificate/document.rs:321, qsl-replay/src/probability_certificate.rs:516 |

Additional examined scope:

```yaml
- id: FR-196
  path: spec/functional/FR-196-build-the-probabilistic-product.md
  role: examined
  excerpt: '`model` is `["quire.simulation.state-key/v1",model_form]`, with

    `model_form` the full FR-101/QSpec FR-181 typed simulation-state form,

    including semantic state, control, queues, roles, observations, bounds

    and memory. Exploration compares its full octets, not a hash. Traces

    name a model state by this same state key; a certificate''s complete product

    key contains the full canonical model form.'
```

The current origin/main contract was checked directly, not inferred from a draft. FR-196's worked vectors also explicitly prohibit an additional JSON/base64 string layer. ModelSystem::state_key at qsl-eval/src/simulation/model.rs:932 returns the owner encoding from key::state_key_of over encode::model_key(...).0; the latter is the full simulation-state tagged form at model/encode.rs:1443. The helper must retain that owner form as parsed JSON inside the FR-196 model wrapper, without reconstructing its typed semantics. The outer QSpec base64url transport remains required and unchanged. A representation assertion grounded directly in FR-196 is needed: writer and reader roundtripping the same nonconforming helper cannot detect this error. Non-null formula monitors remain outside the delivered unbounded capability; future shared structural admission must use ['formula', parsed complete FR-338 key] and leave actual context/product authority with the bounded owner.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Frozen da445108 replaces retained wire vectors/tree with borrowed canonical events; key and bounded decimal transport reserve bytes/storage before allocation, and structural node traversal reserves before stack growth. Output sink growth is also admitted. Original finding text is preserved. |
| FND-002 | still-open | Confirmed at frozen da445108: component 2 is encoded_model String and decoder requires String; the specified parsed model array is rejected. Coder is correcting this shared representation in the same PR. |

## Disposition pass 1 verdict

**FAIL — not mergeable while FND-002 remains open.** FND-001 is fixed; its original finding is unchanged. Round inspected the writer/source diff, including bounded foundation node encoder and normal dependency delta. Inspected passing confined/final fix logs: core 16, analyzer 11, fresh process 9 and provider 1, all-target Clippy, format and normal dependency tree. No tests/matrix/aggregate gate were rerun. Reused the coder's standalone production-linked budget probe: bytes=0 and storage=0 each stop before row-text construction, with roughly 0.1–0.3 MiB measurement overhead rather than the original ~40 MiB; positive insufficient byte/storage controls stop before the large key transport. Review confirms reserve-before-growth calls and explicit cancellation independently of the memory observation. Broader assurance limits remain as originally recorded.
