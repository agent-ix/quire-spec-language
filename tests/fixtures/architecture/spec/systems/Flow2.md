---
id: Flow2
title: Flow2
type: interface
object: interface
relationships:
  - target: Flow
    type: specializes
---

# [Flow2] Flow2

## Description

The flow interface `Flow2` of TC-197 fixture Y: `Flow2.supertypes = [Flow]`.
It conforms to `Flow` (the inherited field `Flow/rate`, typed `Count`
`{1,1}`) and declares one feature of its own, the operation `reset`, which
the manifest's required `## Features` table (quire-rs#448) needs at least
one row of its own declaration.

## Contract

```yaml
name: Flow2
supertypes: [Flow]
fields: []
operations:
  - name: reset
    inputs: []
    output: null
featureOrder: [reset]
```

## Operations

### reset

Reset the flow to its zero rate. The operation takes no parameters, returns
nothing and has an empty frame.

## Features

The interface's own features in declaration order: no fields, one
operation. `Flow2` inherits `Flow/rate` through `specializes`; inherited
features are not repeated here.

| Feature | Kind |
|---|---|
| reset | operation |
