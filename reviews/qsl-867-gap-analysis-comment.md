<!-- reviewer repo=agent-ix/quire-spec-language visibility=public quoin=0.29.1 module=spec-artifacts-process@b82e42e893612b4b4ee94127714b4577ee893502 id=SR-202610310140 method=gap-analysis lang=rust pr=quire-spec-language#2015 model=gpt-6.1-sol run=C4D1B6CE-0CA2-4072-B941-B9D83006C10E -->

| FND | Severity | Check | Summary |
| --- | --- | --- | --- |
| FND-001 | medium | coverage | FR-172-AC-6's typed-refusal test is tagged but does not check the required code, tag, or exact locus. |

+++ [reviewer data]

```yaml
clean: false
scope:
  - {id: FR-172-AC-6, path: spec/functional/FR-172-check-hyper-and-relation-clauses-over-model-subjects.md, role: examined, excerpt: "FR-171-AC-3's `Det` with body `half(x.i) < rational(3, 4)`, for a function `half` over `x.i`'s type whose declared result is `Rational[0, 1; 1, 2]`, refuses `undefined_expression`/`unproved-range` at `rational(3, 4)`, as a value clause over a parameter `x` of a record type with one field `i` of `x.i`'s type refuses the same body. `Det` with body `rational(1, 2) < rational(3, 4)`, and `NonInterference` with that comparison in its body, each refuse `ill_typed`/`ambiguous-literal` at the literal a value clause with that body is refused at."}
  - {id: TC-597, path: spec/test-cases/TC-597-s3-checks-hyper-and-relation-clauses.md, role: examined, excerpt: "Step 6: `undefined_expression`/`unproved-range` at `rational(3, 4)`; `ill_typed`/`ambiguous-literal` for both comparison variants; each refusal equal in code and cause to its value clause's, at the corresponding expression; `ambiguous_declaration`/`ambiguous-name` at the `let` naming trace variable `a`, at the `let` naming object parameter `v`, and at the `let` naming execution variable `x` in `Det`."}
  - {id: qsl-semantics/tests/it/hyper_clauses.rs, path: qsl-semantics/tests/it/hyper_clauses.rs, role: examined, excerpt: "#[trace(\"TC-597\", \"FR-172-AC-6\")] fn relation_and_hyper_atom_bodies_share_value_typer_refusals exercises rational range, ambiguous literals, and root shadowing."}
findings:
  - fnd: FND-001
    severity: medium
    confidence: high
    method: gap-analysis
    check: coverage
    artifact_id: FR-172-AC-6
    related_ids: [TC-597]
    path: qsl-semantics/tests/it/hyper_clauses.rs
    lines: "1263-1285"
    excerpt: |-
      FR-171-AC-3's `Det` with body `half(x.i) < rational(3, 4)`, for a function `half` over `x.i`'s type whose declared result is `Rational[0, 1; 1, 2]`, refuses `undefined_expression`/`unproved-range` at `rational(3, 4)`, as a value clause over a parameter `x` of a record type with one field `i` of `x.i`'s type refuses the same body. `Det` with body `rational(1, 2) < rational(3, 4)`, and `NonInterference` with that comparison in its body, each refuse `ill_typed`/`ambiguous-literal` at the literal a value clause with that body is refused at.
    finding: "The AC-6 parity test is tagged, but it does not assert the required refusal codes, tags, or exact loci for `RationalRange` and `AmbiguousLiteral`; TC-597 step 6 requires those outputs."
bindings:
  - {test_id: relation_and_hyper_atom_bodies_share_value_typer_refusals, ac_id: FR-172-AC-6, trace: correct}
```

+++
