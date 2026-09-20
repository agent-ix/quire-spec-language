---
id: Flow
title: Flow
type: interface
object: interface
---

# [Flow] Flow

## Description

The flow interface of TC-197 fixture Y: one feature, the field `Flow/rate`
typed `Count` `{1,1}`. FR-070's generic `## Properties` extraction, not a
manifest locator, is what supplies the field name the `## Features` table
below checks against.

## Properties

| Field | Type | Multiplicity | Constraints |
|-------|------|--------------|-------------|
| rate | Count | 1 | |

## Contract

```yaml
name: Flow
fields:
  - name: rate
    type: Count
    multiplicity: 1..1
operations: []
featureOrder: [rate]
```

## Features

The interface's features in declaration order: one field, no operations.

| Feature | Kind |
|---|---|
| rate | field |
