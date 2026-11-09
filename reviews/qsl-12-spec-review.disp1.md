<!-- reviewer-dispositions repo=agent-ix/quire-spec-language visibility=public quoin=0.29.1 module=spec-artifacts-process@b82e42e893612b4b4ee94127714b4577ee893502 id=SR-970 round=1 model=claude-opus-5-5 run=063bcbf5-2612-42c6-86c5-0d8dcae5d1de -->

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |

+++ [reviewer data]

```yaml
dispositions:
- fnd: FND-001
  outcome: fixed
  after_excerpt: '| `analyze` | n/a until engine (5) | n/a until engine (6) | FR-281-AC-9''s binding refusal | FR-281-AC-3''s unsupported item alone | the EN-4 item of FR-281-AC-12''s `Steady` quantile claim, settled `Supported(qsl.statistical)`, handle cancelled `Deadline` before the call | the same item, handle cancelled `Requested` before the call | n/a until engine (6) | n/a (2) |'
- fnd: FND-002
  outcome: fixed
  after_excerpt: A request of three items over `Engines::qsl()`, one whose claim kind no layer-A engine advertises, settled `Unsupported` with the diagnostic `unsupported_projection`/`unsupported-requested-capability` naming that kind (the settlement `negotiate_*` gives its empty candidate set), one an engine proves and one an engine refutes, each settled `Supported` on its engine, returns exactly three records, the first settled by its `unsupported` disposition, its record carrying the `unsupported_projection`/`unsupported-requested-capability` diagnostic naming the claim kind as its `diagnostic` member an...
- fnd: FND-003
  outcome: fixed
  after_excerpt: '`undefined-evaluation{where, cause}` and `replay-parity`. A record settled by an `unsupported` or `invalid-request` disposition (QSpec FR-331 `settled_by` `disposition`) carries its FR-272 diagnostic as the record''s `diagnostic` member, with code, cause and payload, and an `unsupported` record carries exactly one of `cause` and `diagnostic`, so a disposition''s record carries the diagnostic and no `cause`. A `monitor` item'
- fnd: FND-004
  outcome: fixed
  after_excerpt: '5. `analyze` success and every `prove` (QSL half) cell other than the binding refusal, the timeout, the cancellation and the tool-failure have no byte-driven input route to the engine that decides the cell''s claim: the engine is in `Engines::qsl()` (EN-1''s `Engine::model_check` among them), but no `MatrixCase`''s bytes (a source, a domain package, a trace) build the engine''s item. The cell becomes applicable, with the input FR-281 names, when such a route exists. 6. `analyze` violation and `analyze` incomplete (the state budget) have no byte-driven input route to their engine, on the same te...'
```

+++
