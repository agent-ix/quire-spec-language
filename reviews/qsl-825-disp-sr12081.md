<!-- reviewer-dispositions repo=agent-ix/quire-spec-language visibility=public quoin=0.28.3 module=spec-artifacts-process@b82e42e893612b4b4ee94127714b4577ee893502 id=SR-12081 round=1 reviewed=efd44d256f95aff855f592726ba07673b6efa2a7 model=claude-opus-5-5 run=451c303f-e9aa-41a7-aa9a-eaba9bcafec3 -->

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | still-open | `cargo tree --target all` is the union over every platform, so a kept package's `cfg(windows)` request still covers a loss on the host (measured on a fixture) |
| FND-002 | still-open | A nominee's own default-lane vs all-features compile is still proved only by merged metadata; a host/target split of a dependency hides it (measured on a fixture) |
| FND-001 | fixed (round 3) | cc2792031: read_tree_features runs cargo tree on the host (no --target all); fixture another_platforms_request_of_a_kept_package_does_not_cover_a_host_loss |
| FND-002 | fixed (round 3) | cc2792031: a nominee runs unless its compile units in cargo tree --workspace equal its units under --all-features (compiles_same_as); fixture a_candidate_compiled_differently_by_the_default_lane_is_not_skipped |

Verified fixed in 607149779: the remaining-package check catches the dev-dependency chain, transitive, build-dependency and `default-features` cases, and qsl-attrs is put back on the real workspace. `settle` terminates (each round removes one candidate or returns), it is deterministic (sorted input, first index wins a tie), and in the worst case it runs the whole workspace. A failing `cargo tree` is an `Err` that propagates out of the lane, never a skip. The production paths have no unwrap or panic. Comparing each package's set of feature sets is sound for the packages that stay, because excluding packages can only remove features from a unit. So a unit that is both host and target with the same features cannot hide a difference, and the risk the author flagged is not real. Each `cargo tree` run takes about 0.27 s on this workspace (real run timed: 0.283 s with `-e features`, 0.258 s with the lane's format). The real workspace needs about 12 runs (~3.3 s), and the worst case for 9 nominees is about 55 runs (~15 s).

+++ [reviewer data]

```yaml
dispositions:
  - fnd: FND-001
    outcome: still-open
    reason: "read_tree_features runs `cargo tree --target all`, the union over all platforms, while the lane compiles for the host. Fixture: kept depends on shared and has [target.'cfg(windows)'.dependencies] shared = { features = [\"alloc\"] }; helper depends on shared with alloc. With --target all the tree without helper equals the full one (shared|alloc), so helper stays skipped. On the host the tree without helper has shared| (no alloc), a feature set the full all-features run never compiles. This is the platform-conditional cover case FND-001 named. Fix: verify on the host platform (drop --target all, or check both and treat any difference as RUN)."
  - fnd: FND-002
    outcome: still-open
    reason: "The cargo tree check compares only the packages that stay. A nominee's own default-lane compile against its all-features compile is still proved only by the merged cargo metadata sets. Fixture: members p (depends on d), m (extra = [\"d/f\"], depends on d), q (build-dependency on d with f). Metadata gives d [\"f\"] in both lanes, so p is nominated. The tree without p equals the full tree, so p is skipped. The default host tree has d| and d|f, so p's default-lane test builds against d without f, and p is never tested against d with f. This breaks NFR-013's 'excluded only when the default lane runs it on the same compile'. Fix: per nominee, compare the per-unit feature sets of its subtree in cargo tree --workspace (default) against --all-features on the host, and run it on any difference."
```

+++
