# Run a native workflow

`quire-spec run <request-file>` compiles selected model and native source files,
reads exact snapshot/invocation bytes, then validates and executes the requested
clause. No Filament, Quoin, producer process or network service is needed.

Generate the synthetic AGPL-3.0-only example files through existing Rust APIs:

```sh
nice -n 10 cargo run --locked --offline --target-dir target -j 1 --example standalone_fixtures -- /tmp/native-workflow-example
target/debug/quire-spec run /tmp/native-workflow-example/healthy/request.json
target/debug/quire-spec run /tmp/native-workflow-example/violating/request.json
target/debug/quire-spec run /tmp/native-workflow-example/operation/request.json
target/debug/quire-spec run /tmp/native-workflow-example/refused/request.json
```

The generator writes named files inside the selected directory. Build the CLI
with `cargo build --locked --offline --target-dir target -j 1 --bin quire-spec`
if it has not been built. Cases respectively return completed true (exit 0),
completed false (exit 1), completed operation true (exit 0) and a frame refusal
(exit 1). Refused/incomplete outputs have no predicate truth field.

Each request is a closed JSON envelope with `format: native-run/1` and a
`request` object. The generated request is the complete example of these fields:

| Field | Selection |
| --- | --- |
| models | Source file, native/formal identity, digest and native-rule-model/1 format |
| program | Source selection and complete authored clause bindings/points |
| snapshots / invocations | Files and the existing role-specific artifact references |
| selection | Authored owner/clause and current self object or recorded invocation |
| limits | Optional validation_work and expression_steps; omitted values use defaults |

Source digests use `sha256:` text. Runtime references retain their existing raw
64-digit lowercase digest encoding. Relative file paths resolve from the request
directory; source display paths retain the authored file string. The command
caps request/source files at 1 MiB, dependent files at 64 and aggregate dependent
reads at 8 MiB, plus the existing compiler/runtime ceilings.

Results on stdout include exact request/package/source/model/input identities,
selected clause, stage, diagnostics and actual work/events. Intake errors are
JSON on stderr. Exit 2 identifies malformed requests, identifier or I/O failures;
unknown formats and semantic refusals exit 1; budget stops exit 3. Broken pipes
end quietly with the computed status. Each run starts fresh budgets. The native
result is local execution output; portable evidence and Quire extraction remain
separately owned integration work. Contract: FR-026; actual binary tests: TC-103/104.

To export the compiled artifact for another consumer, use the generated
`compile.json`. Its native-compile/1 request contains only models and program;
compilation needs no snapshot or invocation files.

```sh
target/debug/quire-spec compile /tmp/native-workflow-example/healthy/compile.json > /tmp/native-package.json
```

Successful stdout is the exact native-linked-package/1 byte artifact, with no
wrapper or extra newline. The existing `NativePackage::read_verified` API reads
it with explicit source/model bindings and a selected digest. Static failures
leave stdout empty and use the same command-error JSON/exit convention as run.
An output I/O failure can leave a partial prefix; exit 2 and digest verification
distinguish that from a complete artifact. Contract: FR-027; binary tests: TC-105.
