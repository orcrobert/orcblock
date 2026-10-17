# NNNN: <feature name>

Status: draft
Issue: #n (optional)

## Problem
Who has it, what happens today, why it matters.

## Behaviour
What the user sees and does, step by step, including empty, error and "helper not installed" states.

## Acceptance criteria
- [ ] A testable statement

## Design
Crates and modules touched, core types and commands, wire protocol changes, UI screens and components, files the daemon writes, risks.

## Bypass and safety
How this could end a block early, spend or refill potions, or let another local process misuse the daemon, and what prevents it. "None" only if the feature cannot touch the daemon.

## Slices
1. The smallest shippable piece, under ~300 changed lines

## Test plan
Unit, property, integration and UI tests; manual checks on a real Mac.

## Out of scope

## Open questions
