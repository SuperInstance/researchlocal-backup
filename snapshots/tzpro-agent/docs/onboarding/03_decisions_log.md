# Decisions Log — Choices Made, Alternatives Weighed

> **Purpose:** Every non-obvious architectural choice in this system,
> the alternatives that were considered, and the reasoning behind the
> chosen path. Read this **before proposing changes** to existing
> systems. Many of these decisions were made under specific constraints
> (single Windows laptop, intermittent connectivity, the captain is not
> a developer) that may not be obvious from the code alone.
>
> **Format:** Each decision has an ID (D-NNN), a one-line summary, the
> context, the alternatives considered, the choice made, the
> reasoning, and the date it was made (or last verified).

---

## D-001 — Local-first, viral later (no cloud in Phase 1)

**Summary:** All data stays on the ProArt laptop. No cloud sync, no
remote dashboard, no hosted LLM. Phase 8 adds Cloudflare federation.

**Context:** The captain works 12+ hours a day in a wheelhouse with
spotty-to-zero internet. TZ Pro is offline software. The local
experience must be so good that the captain uses it every day *without
needing* a cloud promise.

**Alternatives considered:**
- **Cloud-first (Firebase + Cloudflare + hosted LLM):** rejected
  because the captain would need internet to see his own boat's data.
  Defeats the purpose.
- **Hybrid (local capture, cloud analysis):** rejected because the
  tight feedback loop between capture and analysis (10-min cadence,
  the captain waiting for a result) demands local inference. We will
  use local Ollama in Phase 1 and add cloud models in Phase 2 only
  when the local ones prove insufficient.

**Decision:** Local-first. The cloud is a federation layer (Phase 8)
that lets *other* boats see *this* boat's insights, not a dependency
for this boat's own operation.

**Reasoning:** "Local-first, viral later" is the entire reason this
system can be adopted by captains who don't trust cloud vendors with
their proprietary fishing intel. The viral mechanic works *because*
the data stays on the boat.

---

## D-002 — Stampfile-based singleton for the capture daemon

**Summary:** The capture daemon enforces single-instance via a stampfile
(`capture_daemon.stamp.json`) containing PID + tzpro_last + child_pid,
not via Windows service or PID file.

**Context:** We need exactly one capture daemon running. If two start
(crash + manual restart + tray auto-start), they fight over
`capture_daemon.stamp.json` and the capture child becomes a zombie.

**Alternatives considered:**
- **Windows service:** rejected because installation requires admin
  elevation and the captain does not want to grant admin to the tray
  launcher. Service registration also adds a dependency on
  `pywin32-service` which we want to avoid.
- **PID file (just the PID):** rejected because PID file alone cannot
  tell you whether the process is healthy or zombied. A crashed
  process leaves a stale PID file forever unless you also stat the
  PID via the OS.
- **Stampfile (PID + last_heartbeat + child_pid + tzpro_last):**
  chosen because one JSON read tells you everything: is the daemon
  alive? is the capture child alive? is TZ Pro alive? when did
  capture last succeed?

**Decision:** Stampfile.

**Reasoning:** The stampfile is also the natural place to expose
freshness to the tray (`last_capture_at`) and the doctor
(`tzpro_last`). One file, many uses.

**Trade-off accepted:** The stampfile is a race-condition window. Two
daemons started simultaneously can both see "no stampfile" and both
proceed. Mitigation: the stampfile is written atomically with a
tempfile + rename, and the daemon re-checks after a 1-second sleep.

---

## D-003 — Capture cadence is 10 minutes on the hour boundary

**Summary:** Captures happen at HH:00, HH:10, HH:20, ..., HH:50.
Aligned to wall-clock minutes, not to TZ Pro start time.

**Context:** Sounder screens change relatively slowly. A capture every
minute is overkill (storage, review burden). A capture every hour
misses transient marks. Ten minutes is the sweet spot for salmon
trolling (which is what F/V Eileen does).

**Alternatives considered:**
- **Every 1 minute:** rejected. Storage is fine but review burden is
  unsustainable.
- **Every 30 minutes:** rejected. Misses short transient marks that
  the captain cares about.
- **Aligned to TZ Pro start:** rejected. Makes cross-day analysis
  impossible because captures drift in time.

**Decision:** Every 10 minutes, aligned to wall-clock HH:0X boundary.

**Reasoning:** Boundary-aligned captures make cross-day correlation
trivially easy (compare 09:30 across days). The capture loop computes
"next 10-minute boundary - now" and sleeps until then. Drift across
days is zero.

**Trade-off accepted:** If TZ Pro starts mid-cycle, the first capture
may be 0–9 minutes late. Acceptable.

---

## D-004 — Bridge auto-start via desktop shortcut, not Task Scheduler

**Summary:** The bridge auto-start is `fix_priority0.bat` run from a
desktop shortcut, not a Windows Task Scheduler entry.

**Context:** `install_bridge_task.bat` exists and would create a
scheduled task, but requires admin elevation. We have not yet decided
to require admin on first install.

**Alternatives considered:**
- **Windows Task Scheduler entry (the "right" way):** rejected for
  now because of the elevation requirement and the risk of breaking
  the install flow for non-admin users.
- **Run-key entry (`HKCU\...\Run`):** considered, but it auto-runs on
  every boot whether TZ Pro is on or not. Wasteful.
- **Desktop shortcut to fix script:** chosen. The captain runs it
  manually after a reboot. It's three seconds of work.

**Decision:** Desktop shortcut. Document the manual step in the
session boot doc.

**Reasoning:** Manual is fine when the manual step is short, obvious,
and reversible. We can revisit when the bridge starts failing on
reboots more than once a month.

**Future option:** Tray app could auto-start the bridge if it detects
COM6 is open but :6006 is not listening. This is the best path
forward but not yet built.

---

## D-005 — Capture daemon auto-starts on tray launch (not on boot)

**Summary:** Opening the tray icon starts the capture daemon. Tray
icon is not started on boot (manual launch by captain).

**Context:** Same reasoning as D-004. We do not want the daemon
running when the laptop is on the dock with no TZ Pro running. We
*do* want it running when the captain is in the wheelhouse with TZ
Pro on.

**Decision:** Tray-launch starts daemon. Captain launches tray after
powering up the wheelhouse.

**Trade-off accepted:** Captain must remember to launch the tray. If
he forgets, captures do not happen. Mitigation: the tray has a
prominent "Start Capture" menu item and a status tooltip.

---

## D-006 — Two-step fix script instead of one-step auto-install

**Summary:** `fix_priority0.bat` is a separate script that the captain
runs manually after reboot. It is not part of the install flow.

**Context:** The first time we tried a one-step "install everything"
script, it failed silently on permission errors and the captain lost
GPS for two days. We now separate "install" (rare, careful) from
"fix" (frequent, aggressive).

**Alternatives considered:**
- **One auto-install on first run:** rejected. Silent failure is
  worse than a manual step.
- **Separate fix script that the captain runs as needed:** chosen.
  The script is idempotent, prints clear success/failure, and is
  short enough that the captain can read it before running.

**Decision:** Separate fix script.

**Reasoning:** Idempotency + observability > elegance.

---

## D-007 — Ship P1 (capture) before P2 (analyzer)

**Summary:** We chose to harden the capture pipeline to industrial
grade before building the analyzer ensemble. Phase 2 is *blocked*
until Phase 1 has been stable for at least a week of clean captures.

**Context:** The captain needs data more than he needs analysis. An
analysis of zero captures is worthless. A capture pipeline that
silently dies means every day the computer runs is *not* a day of
valuable data. **Data collection cannot silently fail.**

**Alternatives considered:**
- **Build analyzer first, integrate later:** rejected. Would build
  intelligence on top of an unreliable foundation.
- **Build both in parallel:** rejected. Two fragile systems are worse
  than one durable one.
- **Build capture first, then analyzer:** chosen. "JSON truth, markdown
  render f(JSON)" — the schema and the data are the foundation.

**Decision:** Capture first, analyzer second.

**Reasoning:** **Every day this computer runs is a day of valuable
data, analyzed or not.** Data collection is the killer feature.
Analysis is the value-add. We get the killer feature right first.

**Status:** This is the decision that made P1 the priority. It is
also why `hermes_ensemble.py` is missing and not on fire.

---

## D-008 — Schema is tight, vision is loose

**Summary:** The `Moment` schema (`schema/moment.py`) is treated as
canonical and changed rarely. The product vision (10-phase roadmap)
is treated as aspirational and pivots freely.

**Context:** Multiple parallel ideation threads exist
(`_kimi_*.md`, `_claude_*.md`, `_mini_*.md`, `_INTELLIGENCE_*.md`,
`_SEED2_BRAINSTORM.md`, etc.). They explore wildly different futures.
We do not want every brainstorm to break the schema.

**Alternatives considered:**
- **Schema-driven vision (every roadmap change updates schema):**
  rejected. Would cause constant migrations and break old data.
- **Vision-driven schema (brainstorm free, schema follows):**
  rejected. Would cause data loss.
- **Schema tight, vision loose:** chosen. The schema is the contract;
  the vision is the dream.

**Decision:** Schema tight, vision loose.

**Reasoning:** "We hold the vision loose. We hold the schema tight.
Everything else is plumbing we can rewrite." (from ROADMAP.md)

---

## D-009 — DPAPI vault, excluded from backups

**Summary:** Secrets are encrypted with Windows DPAPI + AES-GCM + HKDF
and stored at `~/.tzpro-agent/vault.dat`. They are **not in the repo**
and **not in backups**.

**Context:** The captain needs to be able to put DeepInfra / OpenAI /
Anthropic API keys on this machine without committing them. Cross-user
and cross-machine DPAPI decryption fails by design — that is the
security guarantee. Backups of the vault would be useless (decryption
fails) and dangerous (someone with the backup file could try to crack
it offline).

**Alternatives considered:**
- **`.env` file in the repo (with `.gitignore`):** rejected. Easy to
  accidentally commit. Easy to leak via `git log -p`.
- **OS keychain (Windows Credential Manager):** rejected. Requires
  GUI interactions that the agent cannot automate reliably.
- **DPAPI vault file outside repo:** chosen. Programmatic, auditable,
  and bounded to this user + this machine.

**Decision:** DPAPI vault, excluded from backups.

**Reasoning:** See `docs/SUIT_VS_PERSON.md` for the broader
suit-vs-person boundary that this decision implements.

**Trade-off accepted:** A new machine install requires re-entering
API keys. That's the point.

---

## D-010 — LAN-reachable dashboard at :8090, no auth in Phase 1

**Summary:** The dashboard binds to `0.0.0.0:8090` and has no auth.
Anyone on the boat's Wi-Fi can read everything.

**Context:** The boat's Wi-Fi is a private LAN. The captain, the
crew, the deckhand, the fisheries observer — all need access. Adding
auth would block legitimate use.

**Alternatives considered:**
- **Auth on every request:** rejected for Phase 1. Crew ergonomics
  matter.
- **Auth only on write endpoints:** considered. Future option.
- **No auth, LAN-bound:** chosen. When we move to global access
  (Phase 8), we add Cloudflare Access.

**Decision:** No auth, LAN-bound.

**Trade-off accepted:** A crew member could read sensitive data. The
trust model is "you are on my boat, you can see my boat." When this
becomes a problem we will add auth. Not before.

---

## D-011 — pystray for the tray icon, not electron/Tauri/Qt

**Summary:** The tray app is `pystray` (a Python library wrapping the
Win32 shell API), not a full GUI toolkit.

**Context:** The tray needs to be lightweight (start in <2s, use
<50MB RAM) because it runs all day in the wheelhouse.

**Alternatives considered:**
- **Electron:** rejected. 200MB+ RAM. Overkill for a menu with 6 items.
- **Tauri:** considered. Better than Electron but still a full GUI
  runtime for a menu.
- **Qt / wxPython:** rejected. Heavy install, heavy runtime.
- **pystray:** chosen. ~5MB RAM. Uses native Win32 menus. Pure Python.

**Decision:** pystray.

**Reasoning:** The tray is a menu, not an application. Use the
smallest thing that does menus well.

---

## D-012 — JSON is truth, markdown is `f(JSON)`

**Summary:** Every persistent artifact (capture metadata, vessel state,
moment record, briefing) is JSON in the canonical store. Markdown is
*generated from* JSON, never the other way around.

**Context:** Multiple agents (Kimi, Claude, Hermes, Mini) read and
write these artifacts. If two agents disagree on whether the
markdown or the JSON is canonical, they will fight.

**Decision:** JSON canonical, markdown derived.

**Reasoning:** "Markdown is `f(JSON)`" is a rule, not a slogan. When
in doubt, regenerate the markdown from the JSON. Never edit the
markdown and expect the JSON to update. Never commit a markdown file
without regenerating it from the JSON first.

**Implementation:** `schema/moment.py` has `to_dict`, `from_dict`,
`to_json`, `markdown_render`. All four are derived from the same
dataclass. The markdown is regenerated every time the JSON changes.

---

## D-013 — Capture PNGs are written atomically via tempfile + rename

**Summary:** Every PNG capture is written to `<name>.tmp` first, then
renamed to `<name>.png`. The dashboard and downstream consumers
either see no file or a complete file, never a half-written file.

**Context:** The capture loop writes to disk while other processes
(the dashboard, the analyzer) read from disk. A partial write could
crash the dashboard's image viewer.

**Alternatives considered:**
- **Direct write:** rejected. Crash risk.
- **Database blob:** rejected. Bloats SQLite and breaks "browse
  the folder" workflow.
- **Tempfile + rename:** chosen. POSIX and Win32 both guarantee that
  `os.rename` is atomic on the same filesystem.

**Decision:** Atomic write via tempfile + rename.

**Reasoning:** Cheap, robust, idiomatic. No reason not to.

---

## D-014 — One round of file:line citations in `docs/engineer/`

**Summary:** The engineer docs cite specific files and line numbers
rather than describing components in prose.

**Context:** Future agents need to verify that docs match code. If a
doc says "`capture_v3.py` saves PNGs to `captures/v3/`" and the
actual code saves to `screenshots/`, the agent needs to discover the
discrepancy fast.

**Decision:** Cite `path/to/file.py:L123` for every claim.

**Reasoning:** A doc that can't be verified against code is a doc
that drifts. Citation forces verification.

---

## Decisions NOT yet made (open questions for next agent)

- **O-001:** Should we build a "what changed since yesterday" daily
  briefing that emails/SMSes the captain? Or is that scope creep?
- **O-002:** Should the cascade loops run as a single supervisor
  process or as independent supervised services?
- **O-003:** Should we ship a Docker image for the agent so it runs
  on Linux/Raspberry Pi too, or stay Windows-only?
- **O-004:** Should we build a real install wizard that handles the
  admin elevation + scheduled task + tray autostart in one click?
- **O-005:** Voice STT — local Whisper.cpp or cloud Whisper API?
  (Phase 5 decision, defer.)

---

## How to add a new decision

When you make a non-obvious choice, append to this file:

```markdown
## D-NNN — One-line summary

**Context:** What constraint drove this choice?
**Alternatives considered:** What else did you look at?
**Decision:** What you chose.
**Reasoning:** Why.
**Trade-off accepted:** What you gave up.
**Date:** YYYY-MM-DD.
```

If the decision supersedes an earlier one, mark the earlier as
`SUPERSEDED by D-NNN` rather than deleting it. The history of the
choice is part of the context.
