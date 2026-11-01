<!-- reviewer repo=agent-ix/quire-spec-language visibility=public quoin=0.29.1 module=spec-artifacts-process@b82e42e893612b4b4ee94127714b4577ee893502 id=SR-202610310139 method=code-review lang=rust pr=quire-spec-language#2015 model=gpt-6.1-sol run=C4D1B6CE-0CA2-4072-B941-B9D83006C10E -->

| FND | Severity | Check | Summary |
| --- | --- | --- | --- |
| FND-001 | medium | untestable-ac | FR-172-AC-6 requires exact refusal code/tag/locus, but the new regression test only checks the HyperCause variants. |
| FND-002 | low | other | The existing parity-test comment still says typer unification remains future work. |

+++ [reviewer data]

```yaml
clean: false
scope:
  - {id: FR-172-AC-6, path: spec/functional/FR-172-check-hyper-and-relation-clauses-over-model-subjects.md, role: examined, excerpt: "FR-171-AC-3's `Det` with body `half(x.i) < rational(3, 4)`, for a function `half` over `x.i`'s type whose declared result is `Rational[0, 1; 1, 2]`, refuses `undefined_expression`/`unproved-range` at `rational(3, 4)`, as a value clause over a parameter `x` of a record type with one field `i` of `x.i`'s type refuses the same body. `Det` with body `rational(1, 2) < rational(3, 4)`, and `NonInterference` with that comparison in its body, each refuse `ill_typed`/`ambiguous-literal` at the literal a value clause with that body is refused at."}
  - {id: TC-597, path: spec/test-cases/TC-597-s3-checks-hyper-and-relation-clauses.md, role: examined, excerpt: "Step 6: `undefined_expression`/`unproved-range` at `rational(3, 4)`; `ill_typed`/`ambiguous-literal` for both comparison variants; each refusal equal in code and cause to its value clause's, at the corresponding expression; `ambiguous_declaration`/`ambiguous-name` at the `let` naming trace variable `a`, at the `let` naming object parameter `v`, and at the `let` naming execution variable `x` in `Det`."}
  - {id: FR-172, path: spec/functional/FR-172-check-hyper-and-relation-clauses-over-model-subjects.md, role: context_only, excerpt: "S3 SHALL type every expression of a `hyper` body or a `relation` body that is not an indexed atom, a step label or an execution read under the value typer's rules: expected types, literal typing, and the range and definedness obligations a value clause body carries."}
  - {id: qsl-semantics/tests/it/hyper_clauses.rs, path: qsl-semantics/tests/it/hyper_clauses.rs, role: examined, excerpt: "The forms a hyper body shares with the value profile are typed as the value typer types them: for each body, the relation body is admitted exactly when the same body of a Boolean function is. Where the value typer needs a proof the hyper typer does not ask for (a `Rational` result range compared with a literal), the two differ; unifying the typers is remaining work."}
findings:
  - fnd: FND-001
    severity: medium
    confidence: high
    method: code-review
    check: untestable-ac
    artifact_id: FR-172-AC-6
    related_ids: [TC-597]
    path: qsl-semantics/tests/it/hyper_clauses.rs
    lines: "1263-1285"
    excerpt: |-
      FR-171-AC-3's `Det` with body `half(x.i) < rational(3, 4)`, for a function `half` over `x.i`'s type whose declared result is `Rational[0, 1; 1, 2]`, refuses `undefined_expression`/`unproved-range` at `rational(3, 4)`, as a value clause over a parameter `x` of a record type with one field `i` of `x.i`'s type refuses the same body. `Det` with body `rational(1, 2) < rational(3, 4)`, and `NonInterference` with that comparison in its body, each refuse `ill_typed`/`ambiguous-literal` at the literal a value clause with that body is refused at.
    finding: "The new FR-172-AC-6 test checks only `BodyUnproved`/`RationalRange` and `BodyAmbiguousLiteral`; it does not assert the required `undefined_expression`/`unproved-range` and `ill_typed`/`ambiguous-literal` codes/tags or that the refusal span identifies `rational(3, 4)` / the ambiguous literal. A wrong `HyperCause::code`, `tag`, or span can therefore pass. Assert the code, tag, and exact span for the rational-range relation and both relation/atom ambiguity cases."
  - fnd: FND-002
    severity: low
    confidence: high
    method: code-review
    check: other
    artifact_id: qsl-semantics/tests/it/hyper_clauses.rs
    related_ids: []
    path: qsl-semantics/tests/it/hyper_clauses.rs
    lines: "1224-1228"
    excerpt: |-
      The forms a hyper body shares with the value profile are typed as the value typer types them: for each body, the relation body is admitted exactly when the same body of a Boolean function is. Where the value typer needs a proof the hyper typer does not ask for (a `Rational` result range compared with a literal), the two differ; unifying the typers is remaining work.
    finding: "The comment above `a_shared_value_form_is_admitted_as_the_value_typer_admits_it` says the value and hyper typers still differ and unification is remaining work, although this PR removes the rational special case and adds a test asserting unified refusals. Update the comment to describe the remaining test's actual scope."
bindings:
  - {test_id: relation_and_hyper_atom_bodies_share_value_typer_refusals, ac_id: FR-172-AC-6, trace: correct}
```

+++
