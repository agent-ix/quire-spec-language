<!-- reviewer-dispositions repo=agent-ix/quire-spec-language visibility=public quoin=0.28.3 module=spec-artifacts-process@b82e42e893612b4b4ee94127714b4577ee893502 id=SR-12082 round=1 reviewed=efd44d256f95aff855f592726ba07673b6efa2a7 model=claude-opus-5-5 run=451c303f-e9aa-41a7-aa9a-eaba9bcafec3 -->

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | still-open | The invariant test exists now, but the two cases measured to fail (a kept package's cfg cover on the host; a nominee's own host/target split) have no fixture |
| FND-002 | fixed | 607149779 |

The trace tags are honest. `a_candidate_whose_absence_changes_a_remaining_resolve_is_put_back` and `real_workspace_decisions_keep_every_package_whose_compile_changes` carry TC-915 and NFR-013-AC-4, and they assert AC-4's two clauses (put-back cases; no remaining-resolve difference).

+++ [reviewer data]

```yaml
dispositions:
  - fnd: FND-001
    outcome: still-open
    reason: "real_workspace_decisions_keep_every_package_whose_compile_changes asserts no difference, and five put-back fixtures plus a control were added. Two cases measured to fail are still untested: a kept package's cfg(windows) request that covers a host-only loss under --target all (SR-12081 FND-001), and a skipped package whose own host and target units differ between the lanes while the merged metadata matches (SR-12081 FND-002). The existing platform fixture puts the request on the helper side, the conservative direction, so it does not exercise the hole."
  - fnd: FND-002
    outcome: fixed
    fix_sha: "607149779"
    after_excerpt: |-
      | All | 2 | `test --locked --workspace --all-features --exclude <package>...` |
      ... A package is excluded only when the default lane runs it on the same compile. `cargo metadata` with and without `--all-features` nominates the packages whose own and transitive resolved features are identical in both. `cargo tree` (host and target units kept apart, every platform) then confirms that leaving the nominees out leaves every remaining package with the feature sets it has in the full `--all-features` resolve, and a nominee that fails the confirmation runs.
```

+++
