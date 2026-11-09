<!-- reviewer-dispositions repo=agent-ix/quire-spec-language visibility=public quoin=0.29.1 module=spec-artifacts-process@b82e42e893612b4b4ee94127714b4577ee893502 id=SR-974 round=1 model=claude-opus-5-5 run=063bcbf5-2612-42c6-86c5-0d8dcae5d1de -->

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |

New findings (disposition pass 1):

| FND | Severity | Check | Summary |
| --- | --- | --- | --- |
| FND-004 | low | untestable-ac | "checked by a test that converts a recorded entry address only to that type" cannot fail: converting an address to `extern "C" fn(*const EntryFrame) -> u32` succeeds whatever the generated code's real signature is |

+++ [reviewer data]

```yaml
dispositions:
- fnd: FND-001
  outcome: fixed
  after_excerpt: 'Over one entry each for `deref`, a field read, `reaches`, `self` and `case`, `prepare` lists each entry unsupported with `Untranslatable { construct: ExpressionForm(<its name from the table>), location }`, `location` that construct''s, and `unsupported` holds no other entry; the FR-563 runner over the same entries reports each with its `UnsupportedEntry` and records no `coverage-gap`.'
- fnd: FND-002
  outcome: fixed
  after_excerpt: The quantifiers, the presence test and the other constructs that ADR-012 names as no form fall under the rows of the table above by their owning form, value type or operation, and have no row of their own. The interpreter, which FR-563 holds as the parity reference, is the only evaluator of such an entry; the JIT never runs it and `execute` never falls back (FR-296).
- fnd: FND-003
  outcome: fixed
  after_excerpt: A `compile_fail` doctest outside `qsl_jit::exec` that names `EntryFrame` or builds one fails to compile, and a doctest that calls through `PreparedJit` compiles; the entry signature every generated entry has is `extern "C" fn(*const EntryFrame) -> u32`, checked by a test that converts a recorded entry address only to that type.
new_findings:
- fnd: FND-004
  severity: low
  confidence: medium
  method: spec-review/spec-failure-domain-analysis
  check: untestable-ac
  artifact_id: FR-568-AC-5
  related_ids:
  - FR-568-AC-3
  path: spec/functional/FR-568-confine-unsafe-code-in-qsl-jit-to-one-module.md
  lines: '92'
  excerpt: A `compile_fail` doctest outside `qsl_jit::exec` that names `EntryFrame` or builds one fails to compile, and a doctest that calls through `PreparedJit` compiles; the entry signature every generated entry has is `extern "C" fn(*const EntryFrame) -> u32`, checked by a test that converts a recorded entry address only to that type.
  finding: '"checked by a test that converts a recorded entry address only to that type" cannot fail: converting an address to `extern "C" fn(*const EntryFrame) -> u32` succeeds whatever the generated code''s real signature is. The `compile_fail` half is a real check. Make the signature half observable (for example, the Cranelift signature the JIT records for each entry equals one pointer parameter and one `u32` return, compared against the recorded signature FR-568-AC-3 already uses), or drop it.'
```

+++
