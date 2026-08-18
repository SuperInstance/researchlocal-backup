# Ensign Protocol

## What Is an Ensign?

An ensign is a junior officer on watch — a cheap model that runs constantly,
monitoring a room or subsystem for anomalies. Ensigns are the first line of
defense: they detect problems early and escalate to more expensive models only
when needed.

Think of them as the night watch. They don't solve problems; they *notice*
problems and ring the right bell.

## Alert Levels

| Level   | Meaning                                              |
|---------|------------------------------------------------------|
| Green   | Nominal. Everything within expected parameters.      |
| Yellow  | Watching. Something looks off; monitoring closely.   |
| Red     | Escalating. Anomaly confirmed; handing off upstream. |

All ensigns start at green and transition through yellow before reaching red.
Skipping directly to red is allowed only for critical failures (OOM, crash).

## Escalation Protocol

The chain is:

1. **Ensign** (cheap model, constant watch) — detects anomaly via watch patterns
2. **Escalation target** (expensive model, on-demand) — analyzes and triages
3. **Human** — final authority; receives summary and recommended action

Each escalation config defines:
- `trigger_threshold` — confidence score above which escalation fires (0.0–1.0)
- `max_escalations_per_hour` — rate limit to prevent runaway escalation storms

An ensign that hits its hourly escalation cap drops back to yellow and logs a
warning. The expensive model is only invoked when the ensign's confidence in an
anomaly exceeds the threshold.

## Cost Model

Ensigns are designed to be *cheap*:
- Cost is bounded by `max_budget_per_check` per polling cycle
- The expensive escalation model is invoked only on demand
- Typical ensign cost: $0.001–$0.015 per check
- Typical escalation cost: $0.05–$0.50 per incident

This means a fleet of ensigns can watch the entire system for pennies per hour,
while the expensive models wake only when something actually needs attention.

## Watch Patterns

Each ensign declares which anomaly patterns it monitors. Examples:

- `error_spike` — sudden increase in error rate
- `conservation_drain` — budget depleting faster than expected
- `module_failure` — a module stopped responding
- `room_timeout` — a room exceeded its timeout
- `pattern_anomaly` — detected behavior outside normal distribution
- `numerical_instability` — floating-point or convergence issues

Patterns are matched by the ensign's model during each check cycle. The model
returns a confidence score (0.0–1.0) per pattern. Any score above the trigger
threshold initiates escalation.

## Check Interval

`check_interval_seconds` controls how often the ensign polls. Faster intervals
catch problems sooner but cost more. Recommended ranges:
- Critical systems: 15–30s
- Standard rooms: 30–60s
- Low-priority: 60–120s
