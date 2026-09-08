// SPDX-License-Identifier: AGPL-3.0-only
// Newly authored finite model for native compiler review fixtures.
using AgentIx.Semantic.Decorators;
namespace AgentIx.Semantic;

@minValue(0)
@maxValue(1000)
scalar VersionNumber extends int64;

@role("agent-ix:entity")
model ConfigVersion {
  versionNumber: VersionNumber;
  parent?: ConfigVersion;
}

@operations("ConfigVersion")
interface ConfigVersionOperations {
  attemptUpdate(): boolean;
}
