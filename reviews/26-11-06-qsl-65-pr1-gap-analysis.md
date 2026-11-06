---
id: SR-20261105101074
title: QSL-65 qualification adapters gap analysis
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; QSL-65; PR2462; scoped adapter and function result writer traceability"
review_set: subset
---

## Summary

Scoped requirement/test/source gap review at fc7861b92 production plus test-only a1a4c548b. The computed repository matrix was attempted with a 15-second bound and exited 124 without a matrix. Manual scoped trace mapping is therefore used; no repository-wide matrix PASS is claimed.

## Examined Scope

| Unit | Role | Excerpt |
| --- | --- | --- |
| FR-350-AC-1 | examined | Over a fixture corpus holding one positive compile vector, one positive `run` vector, one `replay` vector and one `format` vector, each with the expected result QSL returns, each vector is `passed` and each ran through its subject's public entry point. |
| FR-350-AC-3 | examined | A vector that expects a success, which QSL answers with `unknown_required_feature`/`unknown-feature`, is `unsupported`, holding that pair; the same answer to a vector that expects that pair is `passed`. A vector that expects a success, run with an evaluation budget of zero, is `incomplete`, retaining its budget kind, bound, actual count and the subject's setting or request field that raises it, and with the default budget is `passed`. A boundary vector that expects the zero-budget limit refusal, run with a budget of zero, is `passed`. A vector expecting `ill_typed`/`type-mismatch` that QSL ans |
| FR-350-AC-4 | examined | A vector naming a source file absent from the corpus is `tool-failure` holding the I/O error, and the vectors before and after it in the corpus keep the outcomes they have when it is removed. |
| FR-350-AC-5 | examined | Two runs over the same fixture corpus with the same build and limits give equal per-vector outcomes. Two identical successful vectors, each requiring the full configured budget, both pass in either corpus order. The corpus and its named input bytes are unchanged after execution, including its recorded results and coverage. |
| FR-350-AC-8 | examined | For each in-scope `run` vector, QSL adapts the corpus input to the existing FR-100 `Call` and invokes the public runtime with the effective evaluation and supplied-value limits. Arguments retain their authored order and bind to the selected function's declared parameters; a value that cannot be adapted is refused under the corpus adapter or FR-100 admission, never silently dropped. |
| FR-350-AC-9 | examined | For an in-scope `run` vector with object context, QSL admits the selected model and object environment through FR-084/FR-089/FR-106 and resolves any population argument to the binding established by that admission before S6a. Native-run/1 does not gain a population argument wire from qualification. |
| FR-350-AC-10 | examined | For each in-scope `replay` vector, QSL adapts the corpus input to the existing FR-071 `ReplayRequestWire`, supplies its bytes to FR-098, and settles the FR-072 typed result. Byte-path resolution follows QSpec FR-336-AC-8; malformed request data retains the owning refusal and an unreadable named file is a per-vector `tool-failure`. |
| FR-350-AC-11 | examined | The `replay-request` adapter of the runner decodes a vector's `replay-request` file, a closed document of QSpec FR-336 `$defs.ReplayRequest`, into `ReplayRequestWire` member by member as FR-336's mapping states, and gives the result to `qsl_replay::replay`; a corpus whose replay vectors the runner could not decode before this adapter now executes them. |
| FR-350-AC-12 | examined | The adapter maps each `sources` entry, of the package and of each `dependencies` entry, to its authority and identity as the file states them, with no revision label, and maps the `byte_provision` rows by reading each sibling `bytes_path` file as raw bytes. |
| FR-350-AC-13 | examined | The adapter maps `object_environment` to the closed `{authority, identity}` identity and refuses a file whose `object_environment` identity carries any other member. |
| FR-350-AC-14 | examined | A `replay-request` file that does not match `$defs.ReplayRequest` (an unknown member, a missing required member such as a `sources` entry's `identity`, a wrong JSON kind) is that vector's `tool-failure` with cause `corpus-decode` naming the JSON pointer of the defect, and not a QSL refusal. |
| FR-350-AC-16 | examined | A `replay-request` file whose `package` is the checked-package document named `orders` with `contract_version` `quire.checked-package/v2`, whose `lock.sources` holds the one `{authority, identity}` entry `("repo", "orders.qsl")`, and whose `dependencies` holds `{identity: "lib/money", sources: [("repo", "money.qsl")]}`, decodes to a request with package name `orders`, that one source and one dependency entry with `identity` `lib/money` and its one source, and no version member; the same file with `package` removed, with `package` set to the string `"orders"`, with `contract_version` set to `qu |
| FR-350-AC-17 | examined | A `byte_provision` row whose `bytes_path` is `missing.bin` (absent from the vector directory), `unlisted.bin` (present but not listed as `kind: bytes`), `/etc/hostname` (absolute) or `../x.bin` (parent-traversing) is a `tool-failure` with cause `corpus-decode` and pointer `/byte_provision/0/bytes_path`, and no request is constructed; a listed `kind: bytes` path is read as raw bytes. |
| FR-350-AC-18 | examined | A `stage_limits` map holding the key `depth` (no FR-255 setting of that name) is a `tool-failure` with cause `corpus-decode` and pointer `/stage_limits/depth`; the key `s1.input_bytes` with the value `"1024"` decodes to the entry `("s1.input_bytes", 1024)`. |
| FR-350-AC-19 | examined | A compile vector whose source selects its supplied `model` document passes and retains the public compiler's exact output bytes; removing that model gives the compiler's typed refusal rather than an adapter success. The runner exposes passed operation bytes separately from the FR-352 JSON report. |
| FR-350-AC-6 | context_only | One test starts four child `cargo test` processes over the determinism sample of the published QSpec corpus. Two run with `RUST_TEST_THREADS=1`. Two run with `RUST_TEST_THREADS=2`, and in each of those a second test runs the runner over the same sample at the same time, both starting at one barrier. Each child writes its FR-352 JSON report, and the output bytes of each `passed` outcome, to its own output directory. The parent finds the four reports byte-equal, and for each vector `passed` in all four children, finds its four output byte strings byte-equal. |

TC-1325 native request/result controls were examined. Existing RecordWire/CauseWire codecs are reused. Positive refinement-internals support and exact AC-6 four-process qualification remain explicitly open.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The new complete function-result writer has no genuine positive Witness replay control. All new end-to-end replay fixtures select ReplaySource::Input, and the codec unit constructs an Input result with pre-call stops. Dropping the witnessed record, disagreement or resolved-region output from the new writer can escape these tests. Add a public witness replay with a decisive record and meaningful source/disagreement facts and assert those values through qualification. | qsl-replay/src/result/wire/replay.rs:44-102; xtask/tests/qualification_adapters.rs:247-339 |

## Verdict

CONDITIONAL; FND-001 must be addressed before merge. The admission and deep-decoder defects are recorded once in the companion code review rather than duplicated here.

## Coverage

Plan completion: not assessed.

Computed matrix unavailable: timeout exit 124, artifacts matrix.json (empty) and matrix.stderr in this scratch directory. Manual scoped test/tag/source mapping was completed. No assurance claim is made for the rest of QSL-65, editor/catalog work, extension pipeline, or the explicitly unqualified AC-6/refinement routes. No optional semantic gap analysis, builds or source edits were performed.

## Clarification

The original FND-001 request for a genuine function replay with a nonempty decisive record is too strong. ADR-031 SW-7 and `qsl-replay/src/execute.rs:1393-1411` establish that a function result has no decisive occurrence and settles with `WitnessCheck::Agrees(None)`. Reject that nonempty-record expectation; do not fabricate one or reroute the corpus through a different state-clause envelope. The concrete coverage gap is the reachable public Witness replay arm, its absent record, actual resolved source regions and a real disagreement path. A control must exercise those facts so removing them from the new writer fails. The finding remains conditional for this reachable coverage scope. Original finding text is retained for auditability.

## Dispositions

Round 1 at b7bb37969.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | A genuine public function Witness replay now asserts the witnessed arm and false value, explicit record:null with key presence, a true-result verdict disagreement with exact code-owned values, and a genuine undefined operation's source regions/no-value disagreement. Incorrect oracle record, region and disagreement members fail qualification. The original demand for Some(record) is rejected per the Clarification: ADR-031 SW-7 makes it unreachable for function replay. No fabricated record or state-envelope rerouting was added. |

Current verdict: PASS for the original scoped coverage finding. The new corpus deep-boundary defect is recorded once as code-review FND-003. Historical focused fix tests were compiling at disposition; no independent build or full gate was run.
