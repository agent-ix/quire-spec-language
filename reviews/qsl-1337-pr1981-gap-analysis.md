---
id: SR-1957109
title: 'QSL-1337 PR #1981 gap-analysis: final IR pin'
type: SpecReview
analysis: gap-analysis
date: '2026-11-01'
scope: 'agent-ix/quire-spec-language; PR #1981 Cargo.lock; frozen f6a49669e9fbaad6c18feebff88be352fd12a0dd'
review_set: subset
relationships:
- target: ix://agent-ix/quire-spec-language/FR-264
  type: references
---

## Summary

Ticket: QSL-1337. Frozen head `f6a49669e9fbaad6c18feebff88be352fd12a0dd`; exact base `4bf9292c88e14a26d1f61b116a46d815fdd92355`. Scope: the two-hunk Cargo.lock pin only, excluding unrelated merged main. No applicable AssuranceProfile found in spec/. No new build or test execution, per dispatch.

The entire actual upstream delta 867cb547 to 154227dc is three additive IR-1194 Exact projection commits (#744, #746, #747). Typed-read #741 and linear Debug #740 are already ancestors of the old 867cb547 pin. Existing reader/Debug code is retained. New projection helpers are reached through the new projection method; no QSL package/replay/root caller invokes it. The new direct IR-to-Exact dependency matches the upstream crate manifest and resolves to the one existing Exact source, 82e8f9b4d9583ba4eee505550ff80a211333696f.

Evidence inspected: /home/peter/dev/worktrees/qsl-1337-final-pin-logs/run-context-after-1980.txt and clippy-after-1980.log. Command: cargo clippy --locked -p qsl-package -p qsl-replay -p quire-spec-language --all-targets --all-features -- -D warnings. Exit 0; elapsed 4:14.63; maximum RSS 1,274,736 KiB; one job; 20 GiB cap; no swap. Context records successful diff, fmt and locked no-deps metadata checks. This is compile/lint evidence, not test execution or full CI success.

## Scope

```yaml
scope:
- id: Cargo.lock:quire-contract-model
  path: Cargo.lock:1655
  role: examined
  excerpt: source = "git+https://github.com/agent-ix/quire-contract-ir?branch=main#154227dcbcda09f3abf635375777893132fc7686"
- id: Cargo.lock:IR-exact-edge
  path: Cargo.lock:1665
  role: examined
  excerpt: ' "quire-exact",'
- id: Cargo.lock:quire-exact
  path: Cargo.lock:1692
  role: examined
  excerpt: source = "git+https://github.com/agent-ix/quire-exact?branch=main#82e8f9b4d9583ba4eee505550ff80a211333696f"
- id: FR-264-AC-1
  path: spec/functional/FR-264-emit-and-read-v2-packages-at-schema-fixed-depth.md
  role: examined
  excerpt: On a thread with a 512 KiB stack, a package holding a 100,000-term sum,
    compiled under limits raised to fit it, is emitted, read back through QSL's I2
    read with `i2.*` raised to fit it, and verified at its emitted `package_id`. The
    `V2Read` clones, compares equal to its clone, formats for debug and drops on the
    same thread.
- id: a_100000_term_sum_reads_back_verifies_clones_compares_and_drops_on_a_small_stack
  path: qsl-package/src/emit/tests/deep_bodies.rs:185
  role: examined
  excerpt: '#[trace("TC-738", "FR-264-AC-1")]'
```

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

PASS for scoped dependency and acceptance-plan gaps. This subset PR review makes no repository-wide assurance claim. The delta adds no QSL requirement or behavior and no QSL source/test stub. The existing FR-264-AC-1 tagged test exercises actual emit/read/clone/equality/Debug/drop on a 512 KiB thread with supplied limits. Its presence does not prove a pass at this head.

## Coverage

Plan completion: not assessed

Semantic review: not run (not opted in); compatibility and acceptance-plan inspection are scoped checks. No formal plan supplied. Final execution evidence remains lead-owned: the actual four-package suite plus all three qsl-bench 100k checks on merged main under the 20 GiB/no-swap cap. Those checks remain necessary for ticket closure. Clippy and historical child-stack passes do not substitute. Existing aggregate/CI failures remain separately recorded, not asserted green.

Reconciliation: manual trace-tag fallback. `quire matrix --scope /home/peter/dev/worktrees/qsl-1337-final-ir-pin --format json` emitted module-discovery warnings and no JSON after several minutes; cancelled with SIGINT (exit 130). No computed coverage percentage or repository-wide pass is claimed. Inspected the FR-264-AC-1 criterion and its exact existing `#[trace("TC-738", "FR-264-AC-1")]` binding and real body. There are no changed criteria; other FR-264 tags were inspected as context, not treated as execution evidence.
