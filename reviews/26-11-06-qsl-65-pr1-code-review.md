---
id: SR-20261105101073
title: QSL-65 qualification adapters code review
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; QSL-65; PR2462; qualification public-operation adapters and replay result writer"
review_set: subset
---

## Summary

Review of frozen production fc7861b92 and test-only correction a1a4c548b in the existing ticket worktree. Rust review is folded into this code-review artifact. No source edits, builds, extra worktrees, commits or pushes were performed. Exact runtime model metadata was unavailable; no model identifier is inferred.

The change replaces qualification placeholders with public compile, run and function-replay calls and an owner-side typed result writer. It consumes the merged nullable-model QSpec contract without fabricating a model identity. The expressly open four-child-process determinism protocol and positive refinement-internals route receive no completion credit here.

## Examined Scope

| Unit | Role | Excerpt |
| --- | --- | --- |
| FR-350 | examined | The runner SHALL execute every vector whose applicability names a QSL subject through the QSL public entry point for that subject, and SHALL compare the typed result with the vector's expected disposition. |
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
| TC-1325 | examined | Replay adapter mapping, malformed-input and native result controls. |

Production scope: xtask qualification dispatcher, adapters and replay decoder; qsl-replay function result wire writer and strict witness-value reader exposure; qsl-semantics ObjectEnvironment reference accessors. Cargo dependencies and all changed tests/spec paragraphs were examined. No workflow or CI gate change, unsafe code, local dependency pin or source vendoring was added.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Population reconstruction discards every admitted member's field values before admit_binding. The owner's binding.subset-value loop therefore sees empty subsetting/subsetted values and accepts a snapshot whose reference-valued subsetting field violates its subset constraint, allowing a runtime vector to be falsely qualified. Preserve actual admitted field facts through an owner conversion and add a genuine invalid-subset control. | xtask/src/qualification/adapters.rs:575-580; qsl-semantics/src/model/population.rs:1027-1149 |
| FND-002 | medium | Schema-valid deeply nested replay values encounter serde_json's hidden default recursion ceiling in parse_unique_json before public replay admission. No caller setting can raise this ceiling; the adapter reports corpus-decode tool-failure for a conforming nested option/record request instead of passing it to its owner. The recursive witness_json conversion also needs depth-safe handling rather than merely disabling the parser guard. | xtask/src/qualification/adapters.rs:199-216; xtask/src/qualification.rs:910-914; xtask/src/qualification/adapters/replay.rs:518-605 |

## Verdict

FAIL; not mergeable until both findings are resolved. The population omission is a semantic admission bypass. The deep-input issue is a narrower adapter completeness failure.

## Coverage

Static source and call-site review only; no duplicate build was started. Historical coder evidence reports 13 adapter tests, five CLI tests, and the supplied-cause codec unit control passing. Those are not independent reruns. The single combined-tree final gate remains the lead's responsibility after fixes. Population owner docs and implementation independently establish that subset-value validation reads PopulationMember.field_values, while observation snapshot admission contains no replacement subsetting check.

No applicable AssuranceProfile was found. Optional semantic gap review was not requested or run; ordinary code/spec faithfulness review was performed.
