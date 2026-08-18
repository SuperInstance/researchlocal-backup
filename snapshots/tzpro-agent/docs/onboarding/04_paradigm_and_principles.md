# Paradigm and Principles — Why We Build This Way

> **Audience:** An agent or human who wants to understand the *why*
> behind the system. This is not actionable — it is conceptual. Read
> this once, then keep it in the back of your mind as you make
> decisions.

---

## The one-sentence paradigm

**Local-first, schema-tight, vision-loose, captain-trusting, data-protecting, viral-by-being-good.**

---

## The longer version

This system is a **git-agent**. A git-agent is an AI that lives in a
git repository, treats the repo as its long-term memory, and uses
doctrine (markdown docs) to bridge the gap between sessions where its
working memory dies.

The git-agent pattern works because:

1. **Git is durable.** Every commit is a snapshot of understanding.
   When the session collapses (planned reboot, AI crash, prompt
   truncation, whatever), the repo survives.
2. **Git is diffable.** Two agents can disagree about a design and
   resolve it via `git diff`. The conversation is the commit message.
3. **Git is portable.** A new agent clones the repo and has the full
   context. No proprietary context-store required.
4. **Git is shareable.** Other boats, other captains, other fishing
   operations can clone the suit and bring their own person to it.

The git-agent pattern *requires* that the repo be clean — meaning the
**suit** (code, schemas, doctrine, harness) is the only thing in the
repo. The **person** (real boat data, API keys, captain's voice
recordings) lives outside the repo. See `docs/SUIT_VS_PERSON.md`.

---

## The seven principles

### Principle 1: The data is the product

**Every day this computer runs is a day of valuable data, analyzed or
not.**

We do not let analysis gaps block data collection. We do not let
model failures block data collection. We do not let feature
incompleteness block data collection. The capture pipeline must be
working, durable, and observable at all times when TZ Pro is on.

This is the principle that made P1 (capture) ship before P2 (analyzer).
Analysis is a value-add on top of data. Data is the killer feature.

### Principle 2: The captain is the domain expert

The captain works the boat 12+ hours a day. He knows the fishing
grounds, the sounder, the tide, the school behavior, the weather.
We (the agents) know code, models, schemas, and pipelines.

**The captain's word wins over the docs. The docs win over our
assumptions. The code wins over the docs.** Always verify in the
actual codebase before trusting any doc. But if the captain tells
you the sounder looks wrong on the east side of Cape Decision, you
fix the sounder orientation — you do not explain why your doc says
it should be correct.

### Principle 3: Local-first, viral later

The captain must be able to use this system *today* on a boat with no
internet. The dashboard must show yesterday's marks even if the
internet is down. The chatbot must answer questions about specific
hauls without calling a cloud API.

Cloud is a federation layer (Phase 8), not a dependency. Other boats
can see this boat's insights via cloud sync, but this boat does not
*depend* on the cloud to function.

This is the principle that made us choose local Ollama over hosted
GPT-4 for the first model integrations.

### Principle 4: Schema tight, vision loose

The `Moment` schema is canonical and changes rarely. New sources,
new payloads, new tags — yes. New top-level shape — almost never.

The 10-phase roadmap is aspirational and pivots freely. We are not
committed to Phase 10 looking exactly as written. We *are* committed
to every data artifact fitting into `Moment`.

This is the principle that lets us explore without breaking what is
built.

### Principle 5: The system is the captain's tool, not ours

We do not add features for elegance. We do not add abstractions for
"future flexibility." We do not refactor working code because it
"could be cleaner."

We add features the captain has asked for. We add abstractions when
the third use case appears, not the first. We refactor when the
code is preventing a fix the captain needs.

The YAGNI principle is operational, not rhetorical.

### Principle 6: The capture pipeline is industrial

The capture pipeline is the killer feature. It must be:
- **Durable:** survives reboots, crashes, TZ Pro restarts.
- **Observable:** the captain can see at a glance that it is working.
- **Verifiable:** has a `verify` subcommand that returns HEALTHY/STALE/UNHEALTHY.
- **Recoverable:** auto-recovers on tray launch; manual-recoverable
  via a desktop shortcut.
- **Auditable:** every capture has a JSON metadata sidecar with
  timestamp, position, fix quality, satellite count.

If any of these properties is missing for even one capture, we have
failed the principle.

### Principle 7: JSON truth, markdown render `f(JSON)`

Every persistent artifact is JSON in the canonical store. Markdown is
*generated from* JSON. When in doubt, regenerate the markdown from
the JSON. Never edit the markdown and expect the JSON to update.
Never commit a markdown file without regenerating it from the JSON
first.

This is the principle that lets multiple agents (Kimi, Claude,
Hermes, Mini) read and write the same artifacts without fighting.

---

## The four pillars of the architecture

### Pillar 1: Capture (sensors in)

`nmea_bridge.py` reads COM6 (GPS). `capture_v3.py` screenshots DISPLAY6
(sounder). Both write to disk in standardized formats. Both are
supervised by stampfile-aware daemons. Both are verified by `doctor.py`.

The capture pillar must work before anything else matters.

### Pillar 2: Schema (truth in the middle)

`schema/moment.py` is the canonical shape of every "thing that
happened." GPS fixes, sounder captures, voice notes, AIS contacts,
engine gauges, model analyses — all fit into `Moment`.

The schema is the spine. Everything else is muscle.

### Pillar 3: Analysis (intelligence in)

The analyzer reads moments and produces analyses. Currently the
analyzer is partially built (`hermes_worker.py` exists but
`hermes_ensemble.py` is missing). When complete, it will run on a
10-min cadence and escalate from local Ollama to DeepInfra based on
confidence.

The analysis pillar must not block the capture pillar. If the
analyzer is broken, the captain still gets captures.

### Pillar 4: Presentation (value out)

The dashboard (`dashboard.py`) and the projection layer (Phase 10)
turn moments and analyses into something the captain can act on.
The tray (`tray_app.py`) is the front door.

The presentation pillar is where the "killer app" experience lives.
But it cannot exist without the data pillar.

---

## The git-agent doctrine (extended)

A git-agent has a duty to the repository that goes beyond normal
software engineering:

1. **The repo is your long-term memory.** Commit early, commit often.
   A session that ends without committing is a session that didn't
   happen. The work is lost. The lessons are lost.

2. **The repo is your successor's first impression.** When you write
   code, write it for the agent who will read it next (which might
   be you with amnesia). Comments are not optional. Docstrings are
   not optional. Commit messages are not optional.

3. **The repo is your shared brain with the captain.** The captain
   reads the docs. The captain reads the commit messages. The
   captain does not read the code (he is not a developer). Write
   docs for the captain first, code for the next agent second.

4. **The repo is your doctrine, not your biography.** We do not
   commit personal opinions, hot takes, or session-by-session
   narrative. We commit decisions, reasoning, and verified state.
   The history of *what we thought* is preserved in `docs/PLANS/daily/`
   if needed, but the repo is *what we know is true*.

5. **The repo is public-by-default, private-by-exception.** The
   suit/person boundary is the default. If you are about to commit
   something that is not code/schema/doctrine/sample-firmware,
   stop and ask: is this suit or person?

---

## What we are NOT

We are not building:
- A cloud-first SaaS product.
- A general-purpose AI agent framework.
- A sounder-replacement product.
- A fisheries-research product.
- A multi-tenant system.

We are building:
- A local-first tool for one captain on one boat.
- That other captains can adopt by cloning the suit and bringing
  their own person.
- That eventually federates across boats via Phase 8 cloud sync.
- That is honest about its scope.

---

## The deepest principle

If you take only one thing from this document, take this:

> **The captain's job is to fish. Our job is to make sure the data
> flows, the GPS works, and the tools don't break. Everything else
> is decoration.**

When in doubt, ask: "Does this make the captain's fishing day better
or worse?" If better, do it. If worse, don't. If unclear, ask him.
