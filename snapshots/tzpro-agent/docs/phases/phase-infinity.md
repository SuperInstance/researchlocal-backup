# Phase ∞ — Platform of Platforms

> **Status:** Vision-mode, ultimate goal
> **Depends on:** Everything else stable

## Goal

The agent isn't a fishing tool anymore — it's a **moment collector**
that runs on anything that emits events.

## What runs on this platform

- TZ Pro / Nobeltec (the original flagship)
- OpenCPN (free, open-source chartplotter — opens the platform to
  sailors, cruisers, research vessels)
- ROS (Robot Operating System) for industrial robotics — autonomous
  boats, AUVs, USVs, dock automation
- SCADA systems in industrial plants
- Aquarium monitoring (yes really — same moment schema works)
- Research vessels, science fleets, ocean observatories

## What stays constant

- The `Moment` schema
- The dashboard (always 3 panels, always LAN-first)
- The provider abstraction
- The local-first, cloud-federation architecture

## What changes

The source adapters. The capture cadence. The cost model (a fleet of
AUVs has very different economics than a single fishing boat). The
deployment model (headless industrial systems have no screen, so the
dashboard becomes purely remote).

## Why this matters

The schema is the moat. Once "moment" is a stable concept, the platform
compound-grows with every new source adapter. A new feature for fishing
captains is a new feature for sailors, who don't know they want it yet.
A new analysis for industrial automation is a new analysis for science
fleets, who didn't know it was possible.

The platform doesn't need to know what it's monitoring. It just needs
to know what happened, where, when, and how strongly.

## The bar

When someone uses TZ Pro Agent on their boat, on their autonomous
underwater vehicle, and on their home aquarium, and they all sync to
the same Cloudflare account, and they all share the same dashboard
language — that's when we've arrived.
