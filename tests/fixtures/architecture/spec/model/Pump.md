---
id: Pump
title: Pump
object: entity
type: FR
name: Pump
---

# Pump: Pump

## Description

The object type `Pump` of TC-197 fixture Y. It declares an identity field
and one operation, `run`, which the allocation `pump_alloc` allocates to
the part `sys_pump`.

## Properties

| Field | Type | Multiplicity | Constraints |
|-------|------|--------------|-------------|
| id | UUID | 1 | identity |

## Operations

### run

Run the pump. The operation takes no parameters, returns nothing and has an
empty frame.
