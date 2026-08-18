# Onboarding — The Next Agent's Field Manual

> **Audience:** A new agent (human or AI) taking over operation of this
> system. This is the **starting point** — read this, then follow the
> reading order. Every other onboarding doc in this directory assumes you
> have already read the doctrine at `docs/SUIT_VS_PERSON.md` and
> `docs/BOOTCAMP.md`.

---

## What this directory is

This is **the handoff**. It supersedes — but does not replace — the
older session-scoped onboarding docs (`ONBOARDING.md` at repo root and
the versioned BATON_PASS files in `docs/`). Those were written *for* a
specific Mini-Agent session picking up mid-flight. This directory is
written for a cold-boot agent who has never seen this repo before.

The difference matters: the older docs say "here is where we are" and
trust you to find the doctrine later. These docs say **"here is the
doctrine, here is where we are, here is what to do first, here is why we
made the choices we made."** Skip the older docs on first read; come
back to them if you need session archaeology.

---

## Reading order (cold boot)

1. **`docs/SUIT_VS_PERSON.md`** — the privacy charter. 30 minutes. Not
   optional. If you skip this you will eventually leak data or commit
   secrets and the viral mechanic will die.
2. **`docs/BOOTCAMP.md`** — the 14 operating rules. 20 minutes. This is
   *how* you operate; the charter is *what* you protect.
3. **`01_session_boot.md`** *(this directory)* — the 5-minute cold-boot
   ritual that brings the system up from a fresh clone.
4. **`02_state_of_the_system.md`** — what is currently working, what is
   half-built, what is broken. Verified numbers, not aspirations.
5. **`03_decisions_log.md`** — the choices made, the alternatives
   weighed, the trade-offs accepted. Read this *before* proposing
   changes to existing systems.
6. **`04_paradigm_and_principles.md`** — the deeper philosophy. Why
   "local-first, viral later." Why "schema tight, vision loose." Why we
   ship incomplete systems that work completely.
7. **`05_forward_roadmap.md`** — where we are going. What the next
   agent should focus on and what to defer.
8. **`06_bootstrap_checklist.md`** — the actionable checklist.
   Print this. Tape it next to the laptop.

---

## What this directory is NOT

- It is not a replacement for `docs/engineer/` (the subsystem reference
  set). Those docs describe *how each piece works*. This directory
  describes *why the system exists and what to do next*.
- It is not a session log. Session logs go in `docs/PLANS/daily/`.
- It is not a static artifact. If you change the architecture, update
  `03_decisions_log.md` with the new decision and reasoning. If you
  complete a phase, update `05_forward_roadmap.md`. **The handoff is
  alive.**

---

## The one-paragraph summary

The `tzpro-agent` is a local-first capture-and-analysis system that
watches a TZ Pro / Nobeltec sounder screen on a fishing vessel, captures
frames on a 10-minute boundary cadence, indexes them by time and
position, and serves them through a LAN-reachable dashboard. It runs on
a single Windows laptop (an ASUS ProArt) and survives reboots via a
tray icon, a stampfile-managed capture daemon, and a doctor that
auto-repairs the most common failure modes. Today it is delivering a
**durable industrial-grade capture pipeline** with verified health
(9/9 doctor checks, capture verdict HEALTHY, last capture 153s old at
write time). Tomorrow it grows toward analyzer wiring (Phase 2), then
voice (Phase 5), then cloud federation (Phase 8), then the IDE-shaped
projection layer (Phase 10). The schema is the spine. The local app is
the killer app. The cloud is the federation layer, not the foundation.

---

## Where to get help when stuck

- **Subsystem reference:** `docs/engineer/` — every major component has
  a doc with file:line citations.
- **Symptom lookup:** `docs/engineer/13_troubleshooting.md` — symptom →
  fix cookbook with diagnostic one-liners.
- **Quick commands:** `docs/QUICK_REFERENCE.md` (older, partial).
- **Operational cadence:** `docs/DAILY_WORKFLOW.md` (older, still
  largely accurate for Phase 1).
- **The Captain (Casey):** works the boat 12+ hours a day. He is not a
  developer. He is the domain expert. **His word wins over the docs.**
  The docs win over your assumptions. The code wins over the docs.
  Always verify in the actual codebase before trusting any doc.

---

## The handoff itself

This directory was created by Mini-Agent Session #50+1 on 2026-07-24,
immediately after shipping:
- `adfa9da` — P0 fix: restore GPS relay (COM6 → TCP:6006)
- `b64c2ef` — P1 fix: industrial-grade capture pipeline
- `84fce2e` — comprehensive `docs/engineer/` reference set
- this commit — handoff documentation

The agent's first duty was restoring the two P0/P1 systems that had
broken and were silently failing to deliver data. The agent's second
duty was documenting the system well enough that the next agent could
operate it without re-deriving context. This is the artifact of the
second duty.

Read the docs. Verify the state. Make it your own. Ship.
