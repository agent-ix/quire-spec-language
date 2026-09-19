---
type: master-requirements
name: architecture
org: agent-ix
title: "Pump System Architecture Specification"
---
# Pump System Architecture Specification

The architecture fixture bundle (filament-core-data#173). It declares QSpec
TC-197 fixture Y: the value type `Count`, the object types `Sys`, `Pump` and
`Tank`, the interfaces `Flow` and `Flow2`, the parts `sys_pump` and
`sys_tank`, the ports `pump_out` and `tank_in`, the connection `pipe` and the
allocation `pump_alloc` of `Pump/run`. Every declaration is its own artifact
and its artifact id is its declaration key (QSpec FR-152, FR-154).
