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
