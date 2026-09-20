---
id: pipe
title: pipe
type: connection
object: connection
---

# [pipe] pipe

## Description

The connection `pipe` of TC-197 fixture Y: from the port `pump_out` to the
port `tank_in`, both ends `{1,1}`. Both ends' owners are parts of `Sys`, so
`pipe` is an effective member of `Sys`.

## Connection

| Source | Source Multiplicity | Target | Target Multiplicity | Direction |
|---|---|---|---|---|
| pump_out | 1..1 | tank_in | 1..1 | source-to-target |
