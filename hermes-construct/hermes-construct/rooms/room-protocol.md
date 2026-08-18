# Room Protocol

## What Is a Room?

A room is an isolated task context — a bounded workspace where a specific type
of work happens. Each room has its own model settings, budget, modules, and
behavioral parameters. Rooms don't share state unless explicitly connected.

Think of them as departments on a starship: Engineering builds, Science explores,
Navigation routes, Social communicates. Each has its own culture and constraints.

## Gravity

Gravity is the single f64 that drives a room's personality. It maps to a
position on a spectrum from rigid/precise (negative) to creative/exploratory
(positive):

- **Negative gravity** (-1.0 → 0.0): Precise, deterministic, low-temperature
  work. Engineering, debugging, monitoring.
- **Zero gravity** (0.0): Balanced, scientific, exploratory. Analysis, research.
- **Positive gravity** (0.0 → 1.0): Creative, warm, high-temperature. Social,
  brainstorming, writing.

From gravity, the system derives defaults for:
- **Temperature**: `0.1 + (gravity + 1.0) * 0.4` (range 0.1–0.9)
- **Prompt style**: negative → precise/analytical, zero → exploratory, positive → warm/expansive
- **Sampling**: higher gravity → more diverse outputs; lower → more deterministic

These derived defaults can be overridden by explicit values in the room config.

## Deadband

`deadband_tolerance` defines how much gravity can drift before the system
re-adjusts room parameters. Small deadband = tight control. Large deadband =
more adaptive flexibility.

If gravity drifts within the deadband, no action is taken. If it drifts beyond,
the system recalculates temperature, prompt style, and sampling from the new
gravity value.

Example: engineering has deadband 0.10 and gravity -0.6. Gravity can drift to
-0.5 or -0.7 without triggering re-adjustment. Beyond that, parameters update.

## Room Isolation Rules

1. **No shared state by default.** Each room maintains its own context, memory,
   and budget. Rooms communicate only through explicit message passing.

2. **Budget is per-room.** `conservation_budget` is the total token-cost budget
   allocated to the room. Once exceeded, the room must escalate or pause.

3. **Modules are scoped.** `allowed_modules` restricts which subsystems a room
   can invoke. A room cannot load a module not in its list.

4. **Concurrency is bounded.** `max_concurrent_tiles` limits parallel operations.
   A room cannot exceed this without escalation.

5. **Timeouts are hard.** `timeout_seconds` is the maximum wall-clock time for a
   single operation. Exceeding it triggers an ensign alert.

6. **Escalation is escape-valved.** Each room declares an `escalation_model` —
   the expensive model that takes over when the room's default model can't handle
   a situation. Escalation resets the room's context to a clean state.
