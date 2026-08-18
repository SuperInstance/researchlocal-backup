# Council Mini — Captain-Experience Lens

> **Date:** 2026-07-20
> **Lens:** What would a non-technical fisherman FEEL is more
> intelligent in week one?
> **Scope:** Top 3–4 picks from recent SuperInstance pushes, judged
> strictly by what the captain notices — voice, scrubber, debrief,
> wind watcher — and what shows up as clutter.
> **Anchor:** docs/26 — the captain's loop is event-driven and physical;
> the agent's loop is clock-driven and retrospective; the shell meets on
> four surfaces (voice, scrubber, escalation card, dial). "Never a fifth
> channel."

---

## How I scored

For each repo I asked four questions:

1. **What does he see/hear differently?** — surface change visible from
   the captain's chair.
2. **What's the mechanism?** — the *minimum* of what has to be true
   underneath.
3. **Adoption cost** — what the captain (or the engineer on his behalf)
   has to pay.
4. **Risk it feels like clutter** — where it would violate docs/26's
   "no fifth channel" rule, or interrupt when the rules say silence.

I gave weight to anything that **changes a moment in the day** the
captain already lives — not features he'd have to discover. The wind
watcher that stays quiet unless something changes is the bar; if a
repo can't beat "silence + one well-timed sentence," it isn't a
pick.

---

## Candidates considered (and why most don't make it)

| Repo | Verdict | Reason |
|---|---|---|
| **spectro** | out | Pure dev tool — sends one prompt to N models. No fishing surface. Infrastructure for *us*, not for him. |
| **chart-room** | out | Same problem in a fancier wrapper. "Four navigator perspectives" is a research-aid metaphor; the captain doesn't ask "what would the fisherman/sailor/tourist/native think of this depth reading?" — he asks "should I move." |
| **a2ui** | out | Renders intent → HTML/Markdown/JSON. The captain isn't filling out admin forms. The mechanic might use it, but he never sees it. |
| **A2A-native-notebookLM** | out | A whole research/ingest stack (FastAPI + Next.js + SurrealDB + LangGraph). Powerful for the *fleet*, invisible to him. Wrong altitude. |
| **PersonalLog** | out | Per its repo description, "personal logging and tracking application for the Cocapn fleet." It's a tool for *us* writing logs — not a captain-facing surface. CI-only README confirms there's no captain surface here yet. |
| **agent-loop** | out | Local autonomous agent for laptops (goals.md → tick loop). It's a developer harness; running it onboard the boat means a 14-model flock on his wheelhouse hardware. Wrong shape. |
| **whistle** | out | DSL for compiling intent → PLATO/fence/flux configs. Pure infra glue. The captain never types a whistle program. |
| **edge-weight** | out | 404 on readme lookup — can't evaluate what doesn't ship. |
| **vetcheck** | consider | He wouldn't see it directly, but it removes the failure mode where his morning digest is wrong because the local model drifted overnight. The voice quality is the same; the *trust* it carries is higher. Strong support pick, weak direct pick. |
| **shepherds-console** | out | Ops dashboard for *us*. Pastures/fences/kennel are shepherd metaphors; the captain's metaphor is *water*. |
| **SmartCRDT** | out | Distributed state backbone with merge semantics. Right answer for the engine room, wrong answer for the wheelhouse. |
| **othismos-reef** | out | Citation DAG with erosion + reefquakes. Beautiful engineering. Invisible to him unless we repackage it as "your sounder records that nothing referred to get quietly dropped" — and even then, that's a week-two refinement, not a week-one feeling. |

---

## TOP 4 — ordered by what he'd notice first

### #1 — vetcheck (the quiet one that makes the rest believable)

**What he sees/hears differently:** Nothing new on screen. Everything he
already hears — the morning digest, the well-timed observation at 10:00,
the anomaly card at 13:40 — feels *steadier*. The line that used to be
slightly off once a month stops being off at all. The "last three times
I called this, two held" track record that docs/26 already commits us
to starts being right, because the model behind it is being quietly
examined.

**Mechanism underneath:** Three operations — **physical exam** (full
regression suite on the model that runs his day-loop), **weight check**
(statistical drift on output embeddings compared to a baseline), and
**quarantine** (auto-isolate a model that's failing critically). The
overnight shift (02:00, docs/26) runs the daily weight check; Monday
02:00 runs the full physical; the captain's day never sees the work.

**Adoption cost:** One-time baseline snapshot when we set up the
onboard model. A health-certificate check before each deployment of a
new breed. **Zero captain time.** The engineer gets a one-page report
we read; he gets silence he can lean on.

**Risk it feels like clutter:** Almost none. This is the purest
expression of docs/26's "silence by default" rule — the captain's
loop is unchanged; the agent's loop just stops failing silently.
*This is the only pick where the absence of a new surface is the
feature.* The trap to avoid: surfacing vetcheck results in the morning
digest. The captain doesn't want to hear "the model passed its
checkup." He wants to *not* notice. That's the win condition.

---

### #2 — PersonalLog (when it gets a real README — for now, watch)

**What he sees/hears differently:** *Potential:* a local-first logging
companion that lives on his phone/tablet and treats every catch
event, voice note, and M10 record as a personal artifact — searchable
on his own time, never dependent on cloud. The dock scene (docs/26
"the dock — where the catch log pays off") gets a searchable history
he can scrub from his phone at dinner without asking the system to
generate a view.

**Mechanism underneath:** Per the repo description, a logging and
tracking application — WASM-compiled Rust core + Next.js UI. Local
first means his data lives on his devices, with sync as a courtesy
not a dependency. (The current README is CI-only, so I'm judging
this on intent and stack, not on shipped surface — flag this.)

**Adoption cost:** Medium-high for *us*; near zero for him if we ship
it right. He already has the phone; we ship a paired app; he never
opens it during the day, opens it at the dock.

**Risk it feels like clutter:** **Real.** If PersonalLog becomes a
*fifth* channel — its own notifications, its own "you haven't logged
in 3 days" pings, its own feed — it directly violates docs/26. The
only safe version is: *it never speaks; it only listens, and it
shows up when he opens it.* That constraint has to be enforced at
the architecture level (no notification permissions, no badges,
ambient only) or this is the pick that betrays the rest.

**Status:** hold-and-watch. The repo needs a real README and a
documented captain-surface story before it becomes a pick. Today
this is a *maybe* I'm flagging for the council, not a yes.

---

### #3 — vetcheck + othismos-reef, **composed**: the trust-erosion fix

**What he sees/hears differently:** Same silence as vetcheck alone, but
the debrief at 19:00 starts to *forget the right things*. His
"yesterday" doesn't keep surfacing a single weird sounder blip from
three Tuesdays ago as if it were evidence; ancient noise that nothing
references anymore has quietly dropped off. The catch log feels
*cleaner* without him knowing why.

**Mechanism underneath:** **othismos-reef** is a citation DAG with
three gates (structural, connective, pressure) and **automatic
erosion** of unreferenced entries after a configurable age. Layer
promotion (surface → consolidation → foundation) is automatic. So
when paired with the catch-log's actual citation structure (which
catches referenced which marks referenced which bottom readings), the
unreferenced blips dissolve naturally; the load-bearing patterns
sink to "foundation." Vetcheck makes sure the model that *queries* the
reef hasn't drifted.

**Adoption cost:** Engineering work, not captain work. We define the
reef schema for his catch log, set erosion_age to something forgiving
(e.g. 180 ticks), pin the genuinely load-bearing entries
(seasonal-bottom, north-edge-thermocline), and let the system age.
**Zero captain time.**

**Risk it feels like clutter:** Real and specific — *the wrong
erosion is invisible damage.* If a piece of knowledge erodes that he
later needed ("the rock that fishes in a SE swell only"), he won't
know what he forgot; he'll just feel the system is wrong once and
lose trust forever. Mitigation: **pin aggressively by default**, log
every erosion to an audit trail, and the morning digest gets *one*
line ("3 notes consolidated, 1 archived") so the curator work is
visible without being noisy. This is a week-two-or-three refinement,
not a week-one pick on its own.

**Why I'm listing it anyway:** Because vetcheck alone is a trust
maintainer, and vetcheck + reef is a trust *compounder*, and the
captain will *notice the difference* between week one and week four
even if he can't name it.

---

### #4 — chart-room / spectro **behind one specific feature**: the disagreement report

**What he sees/hears differently:** *Nothing new in his loop.* But
the agent that talks to him is now running its high-stakes calls
(escalation card at 13:40, the well-timed 10:00 observation, the
weather turn) through a disagreement filter. He doesn't see four
panels; he sees *better one-sentence outputs* — because when the
fleet's models converge, the captain hears confident speech; when
they diverge, the agent stays silent unless its confidence crosses
the bar from docs/26.

**Mechanism underneath:** **chart-room** runs the same prompt against
four "navigators" (fisherman / sailor / tourist / native — bottom
detail, systemic flow, surface signal, negative space) and surfaces
consensus vs. divergence. **spectro** is the more general N-model
version of the same pattern. The trick is: we don't render the four
panels to the captain. We render *only the convergence score and the
single best observation* — and we use divergence as a *confidence
gate* before speaking.

**Adoption cost:** All engineering. The four-model ensemble runs
behind the agent's voice surface. The captain has nothing to learn;
the agent gets better at *when to shut up* — which is the entire
docs/26 contract.

**Risk it feels like clutter:** Low if we keep the panels internal;
**catastrophic** if we ever expose the "four navigator" UI to him. A
non-technical fisherman does not need to see "what the sailor
perspective thinks of the depth reading." He needs the system to
already have done that thinking and only speak when it's earned it.
Any time someone says "but wouldn't it be cool to show him the
disagreement?" — that's a fifth channel and the answer is no.

---

## The picks, ranked

1. **vetcheck** — directly strengthens every surface the captain
   already trusts, costs him nothing, *is* the absence of clutter by
   construction. This is the highest-leverage thing on the list.
2. **PersonalLog** (watch) — high potential, currently underbaked;
   becomes a real pick the day it ships a captain-surface story
   that respects docs/26's "never a fifth channel."
3. **vetcheck + othismos-reef (composed)** — week-two compound that
   makes the catch log self-curating. Powerful, but needs design
   care on pinning/erosion defaults.
4. **chart-room/spectro behind the scenes** — improves the agent's
   judgment by improving its silence; never expose the panels.

---

## #1 pick

**vetcheck.**

It is the only candidate that improves the captain's week one
*without adding a single new surface, screen, sound, or moment*. It
removes the silent failure mode — the model that drifted, the
embedding that shifted, the overnight regression he never noticed
until Tuesday's "bait at 30 fathoms" was wrong. Docs/26 says the
agent owes the human *silence by default, mistakes first in every
debrief, an honest "I don't know."* Vetcheck is what makes those
three promises actually true instead of aspirational. The captain
will not be able to name what changed. He will simply feel, by
Friday, that the boat is *smarter than it was on Monday.* That is
exactly the feeling docs/26 is built to produce.
