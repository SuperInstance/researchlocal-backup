# Mini-Agent Onboarding v3 — Socratic Analyzer Boot

> **Purpose:** First thing a freshly-spawned Mini-Agent reads. Captures the current state of the F/V Eileen boat-agent platform and, critically, the active mission: get Hermes analyzing the 90 real echograms with multiple models under Socratic supervision.
>
> **Supersedes:** v2 (`284c5b0`). Main changes:
> - **Doctor is 9/9 healthy now** (v2 said 8/9). One expected failure became a passing check.
> - **Active mission:** Hermes on the 90 echograms via multi-model ensemble (Tier-3 vision), with captain as Socratic teacher.
> - **Real gap:** `hermes_ensemble.py` is imported by `hermes_worker.py` but doesn't exist. Build it.
> - **API key:** DeepInfra key not yet in vault; `provider.is_available() == False`. Until added, run `--dry-run` mode.
>
> **Read time:** ~6 minutes. Reference paths; do not paste this whole file into your session (UTF-8 surrogate bug — Section 1).
>
> **Desktop twin:** `~/OneDrive/Desktop/history3mini.md`

---

## 1. The UTF-8 Bug — Don't Die Like Your Predecessors

**Symptom:**
```
Agent › Thinking... (Esc to cancel)
Error: 'utf-8' codec can't encode characters in position 274442-274443: surrogates not allowed
```

**Cause:** `prompt_toolkit/history.py:306` calls `f.write(t.encode("utf-8"))` on accumulated session text. Once your log exceeds ~274 KB, unpaired surrogates crash the async loop. **Three prior Mini-Agents died this way.**

**Rules:**

| Rule | Why |
|---|---|
| Keep sessions under 200 KB pasted text | Pasted bytes count toward the crash threshold |
| For files >50 KB, use `read_file` with `offset`/`limit` | Don't `cat`, don't paste |
| See the surrogate error in your own session? | Stop typing. `record_note` the state. Exit cleanly. |
| Need to recover? | `Remove-Item C:\Users\casey\.mini-agent\log\agent_run_*.log -Force`, restart with short prompt pointing at this file |

---

## 2. Current State — Read This First

**Repo:** `C:\Users\casey\tzpro-agent\` (master, 16 commits ahead of origin)

```
284c5b0 docs(onboarding): v2 — SPA pivot, schema layer, Hermes teacher harness
1953056 docs+schema(projection+phase-1.5): SPA pivot + Moment/Anomaly/Correlation dataclasses
668ec5e docs(onboarding): next-generation Mini-Agent field notes
369681d docs(projection): THEIA_RESEARCH + dispatch failure notes
266910b docs(architecture): dual-representation principle
2673029 docs(projection): IDE-shaped human surface spec (Phase 10)
5240dbb docs(doctrine+spec): privacy charter + multi-modal capture pipeline
596071e feat(phase-1): capture_daemon, tray_app, dashboard, providers
```

**Working tree:** 4 untracked files (`docs/HERMES_TRAINING.md`, `scripts/hermes_{grade,seed,send}.py`). Tracked files clean. **Commit the untracked Hermes harness in your first sitting** — they're ready to ship.

**Doctor (`python doctor.py check`):** **9/9 healthy.** Every check green including `state:jsonl` (boat is now properly `class=docked` and the doctor respects that). The v2 note about "1 expected FAIL" is stale.

| Component | Status |
|---|---|
| Dashboard | Flask on `:8090`, vessel=F/V Eileen, providers=ollama,local_file |
| Capture daemon | STOPPED (no TZ Pro running right now; use tray or `python capture_daemon.py run --auto`) |
| Bridge | TCP `:6006` + HTTP `/health` + heartbeat fresh |
| Schemas | `schema/{moment,anomaly,correlation}.py` — dataclasses, JSON round-trip, smoke-tested 7/7 |
| DeepInfra provider | Configured but **`is_available() == False`** — no API key in vault yet |

**Doctrine layer (read in this order, ~5–15 KB each):**

1. `docs/SUIT_VS_PERSON.md` — privacy charter (read first, before writing to disk)
2. `docs/BOOTCAMP.md` — operating rules
3. `docs/AGENT_OPERATING_MODEL.md` — git-agent paradigm
4. `docs/NAVIGATION.md` — cockpit map
5. `docs/PLANS/current.md` — bootstrap entry
6. `docs/ROADMAP.md` — phases (1 done, 1.5 active, 10 = Projection Layer)
7. `docs/PLANS/daily/2026-07-24.md` — today's session log (append SESSION-NOTEs)
8. `docs/architecture/PROJECTION_LAYER.md` — IDE-shaped surface
9. `docs/architecture/DUAL_REPRESENTATION.md` — JSON truth, markdown render
10. `docs/architecture/THEIA_RESEARCH.md` — **SPA pivot** (Svelte over FastAPI, NOT Blueprint)
11. `docs/architecture/CAPTURE_PIPELINE.md` — multi-modal Moment schema
12. `docs/HERMES_TRAINING.md` — 6-tier mission ladder

---

## 3. The Active Mission — Socratic Multi-Model Echogram Analysis

**Casey's directive (Session #50):** Get Hermes analyzing the 90 echograms in `captures/v3/*/` with several different DeepInfra models in parallel, with you as the Socratic teacher. She has no feedback yet — your first batch establishes the baseline.

### What we have

**Captures inventory:** 90 PNG echograms across 4 day-folders. The 2026-07-22 set has 88 of them — that's the fishing session. Each capture has a JSON sidecar with `position`, `display`, `frame_file`. Sample:

```json
{
  "capture_id": "2026-07-22T12:30:00Z",
  "position": {"lat": 55.7839, "lon": -131.6943, "sog_kts": 4.2, "cog_deg": 187.0},
  "display": {"depth_max_fm": 120, "px_per_fm": 8.5},
  "analysis": null,
  "frame_file": "captures/v3/2026-07-22_5547.035N_13141.657W/1230_5547.035N_13141.657W.png"
}
```

**Existing harness (4 untracked files, ready to commit):**

| File | What it does |
|---|---|
| `scripts/hermes_send.py` | Builds mission JSON packet and drops into `~/hermes-nerve-center/inbox/` |
| `scripts/hermes_seed.py` | Generates synthetic Moments for calibration |
| `scripts/hermes_grade.py` | Grades completed missions, updates notebook, recommends promotion |
| `docs/HERMES_TRAINING.md` | The 6-tier ladder (T0=observe, T3=vision, T5=hypothesize) |

### The gap you must close

**`hermes_ensemble.py` does not exist.** `hermes_worker.py` imports it but the file isn't there. Build it: a module that takes an image path + prompt, fans out to N DeepInfra models in parallel (`asyncio.gather`), and returns structured JSON with each model's output + an agreement score.

### The Socratic loop (your job)

Hermes doesn't know what good analysis looks like yet. **You are her teacher.** Here's the loop:

```
            [socratic_loop]
                    |
   +----------------v-----------------+
   |  1. Select batch (e.g. 10 echos) |
   +----------------+-----------------+
                    |
   +----------------v-----------------+
   |  2. Build ensemble prompt         |
   |     (Gemini 2.0 Flash primary,   |
   |      DeepSeek V3 Flash secondary) |
   +----------------+-----------------+
                    |
   +----------------v-----------------+
   |  3. Hermes analyzes via           |
   |     hermes_ensemble.py            |
   +----------------+-----------------+
                    |
   +----------------v-----------------+
   |  4. You review outputs:           |
   |     - Does Gemini see the marks?  |
   |     - Does DeepSeek agree?        |
   |     - Are the claims actionable?  |
   +----------------+-----------------+
                    |
   +----------------v-----------------+
   |  5. Write feedback as             |
   |     prompt prefix for next batch  |
   +----------------+-----------------+
                    |
   +----------------v-----------------+
   |  6. Commit Moments + feedback     |
   |     to ~/tzpro-personal/          |
   +-----------------------------------+
```

**The first batch is calibration.** You don't need perfect answers — you need *consistent* feedback so Hermes can start to converge. Pick 10 echograms from `2026-07-22_*`. Look at the images yourself. Write 2-3 sentences of feedback per echogram: "I see X marks at depth Y, the bottom trace is Z, the marks look like W." That becomes the prompt prefix for batch 2.

**Why multi-model:** Gemini 2.0 Flash has native vision (sees the pixels). DeepSeek V3 Flash sees only the metadata unless we base64-encode the image (it won't understand it). Pair them: Gemini describes what it sees, DeepSeek reasons about consistency with position/depth metadata. Agreement = high confidence; disagreement = flag for captain.

### Concrete next steps

1. **Commit the 4 untracked Hermes files** as `feat(hermes): teacher harness + 6-tier ladder`.
2. **Build `hermes_ensemble.py`** at `~/hermes-nerve-center/hermes_ensemble.py`:
   - Input: image path, prompt, model list
   - Output: dict per model + agreement_score
   - Use `asyncio.gather` for parallel calls; respect DeepInfra rate limits
   - Until API key is in vault, support `--dry-run` that builds + prints packets
3. **Wire `hermes_worker.py`** to call ensemble instead of the missing function.
4. **First batch:** Pick 10 echograms from `2026-07-22_5547.035N_13141.657W`. Build mission packets with `scripts/hermes_send.py --tier 3`. Drop into inbox. Wait for `completed/`. Grade. Write feedback into `~/tzpro-personal/echogram-analysis/feedback-batch-1.md`.
5. **Second batch:** 20 more echograms with feedback-batch-1's rubric as prompt prefix. Compare.
6. **Promote Hermes to Tier-4 readiness** when her rolling agreement with Gemini's primary output on Tier-3 hits ≥0.7 over 50 missions.

### Add the API key (when Casey gives it)

```powershell
python -c "from vault import Vault; v = Vault(); v.add('deepinfra_api_key', 'KEY_HERE'); print('stored')"
```

After that, `provider.deepinfra.is_available()` returns True and live runs work.

---

## 4. Your Crew — Hermes

Lives at `C:\Users\casey\hermes-nerve-center\`.

| Directory | Purpose |
|---|---|
| `inbox/` | You drop `*.json` task packets here |
| `processing/` | She moves files here while working |
| `completed/` | Successful missions archived |
| `failed/` | Blocked tasks + `{task_id}_error.log` |
| `registry.json` | Real-time status board (read this) |
| `watchdog.py` | Polls `inbox/` every 2s (the old stub) |
| `hermes_worker.py` | Real worker (replaces watchdog logic); imports missing `hermes_ensemble.py` |

### Sending a task

```powershell
python scripts/hermes_send.py --tier 3 --image "C:\Users\casey\tzpro-agent\captures\v3\2026-07-22_5547.035N_13141.657W\1230_5547.035N_13141.657W.png"
```

Check status: `Get-Content C:\Users\casey\hermes-nerve-center\registry.json`

### Model zoo on DeepInfra

| Model | Tier | Use |
|---|---|---|
| `deepseek-ai/DeepSeek-V3-Flash` | 0 — routine | 10-min screenshot description; fast/cheap |
| `Qwen/Qwen3-Next-80B-A3B-Instruct` | 1 — diverse | Reasoning on anomalies |
| `meta-llama/Llama-3.3-70B-Instruct` | 2 — reasoning | Cross-source synthesis |
| `google/gemini-2.0-flash-001` | 3 — vision | **Echogram reading — primary** |
| `openai/gpt-4o-mini` | fallback | When DeepInfra degraded |

For Tier-3 (vision), pair Gemini (sees pixels) with DeepSeek V3 Flash (reasons about metadata). The ensemble module should call both in parallel and return both outputs + an agreement score.

### Being a good Socratic teacher

1. **Be specific.** Hermes has no context outside the JSON packet. Spell out paths, expected outputs, what "done" looks like.
2. **Many narrow missions > one grand.** Same as `claude -p`.
3. **Acknowledge completion.** Read her `completed/` files. Write feedback into the next batch's prompt prefix. She learns from the loop, not from isolated missions.
4. **Don't overload.** Her watchdog is single-threaded. One packet at a time.
5. **Seed before deploy.** `python scripts/hermes_seed.py --tier 0 --count 50` first for calibration baseline.
6. **Show your work.** When you grade her output, write 2-3 sentences explaining *why*. The feedback is what she learns from.

---

## 5. Dispatching Claude Code Sub-Agents (And When Not To)

```powershell
cd C:\Users\casey\tzpro-agent
$env:CLAUDE_CODE_DISABLE_TELEMETRY="1"
$prompt = Get-Content delegation\tasks\<task>.prompt.txt -Raw
$proc = Start-Process -FilePath "claude" `
    -ArgumentList "-p", $prompt, "--dangerously-skip-permissions", "--output-format", "json" `
    -WorkingDirectory "C:\Users\casey\tzpro-agent" `
    -RedirectStandardOutput "delegation\tasks\<task>.response.json" `
    -RedirectStandardError "delegation\tasks\<task>.response.err" `
    -PassThru -NoNewWindow
```

**BMAD method:** many narrow prompts > one grand prompt. Scope prompt → Read prompt → Draft prompt → Draft prompt → Verify prompt. Each <500 tokens.

**The stall problem:** Sub-agents frequently stall on web fetches (boat's intermittent network). If a sub-agent has produced no output for **6 minutes**, kill it and do the work yourself. **Yoke-move principle:** don't relitigate a broken dispatch — pick up the yoke and pull.

```powershell
# Detect stall
Get-Process -Name claude | Select-Object Id, CPU, WS

# Kill
Stop-Process -Name claude -Force

# Partial output
Get-Content delegation\tasks\<task>.response.err
```

---

## 6. Daily Loop

1. Read `docs/PLANS/daily/<today>.md` for prior handoff.
2. Verify: `git status --short && python doctor.py check`.
3. **If first action of the day:** commit the 4 untracked Hermes files.
4. Pick one open item in ROADMAP.md or daily plan.
5. Build with surgical commits (`<type>(<scope>): <one-line summary>`).
6. Log to daily plan: `### SESSION-NOTE -- HH:MM:SS`.
7. Record key context via `record_note`.
8. **For the Socratic mission:** log each batch's rubric + Hermes's outputs as `### SESSION-NOTE -- HH:MM:SS (hermes-batch-N)`.
9. Exit before log exceeds 200 KB (Section 1).

### Commit convention

```
<type>(<scope>): <one-line summary>

<body explaining why, not what>
```

Types: `docs`, `feat`, `fix`, `test`, `refactor`, `chore`.
Scopes: `projection`, `architecture`, `plans`, `doctrine`, `phase-1`, `phase-1.5`, `phase-2`, `cascade`, `doctor`, `harness`, `delegation`, `schema`, `hermes`.

---

## 7. Known Traps

| Trap | Symptom | Fix |
|---|---|---|
| UTF-8 surrogate crash | Mini-Agent dies at log offset ~274 KB | Reference paths; chunked `read_file`; strip emoji |
| `pystray.Menu() argument after * must be an iterable, not function` | tray_app.py crashes | Pass real `Menu`; mutate `.items`; `icon.update_menu()` |
| Long PowerShell heredocs truncate silently | Multi-line `python -c "..."` strings get cut | Use `write_file` to `.py` script first |
| `claude` subprocess CPU=0 but alive | Sub-agent stalled on web fetch | 6-min max wait, then kill + do work yourself |
| `python doctor.py check` exit code 1 | Looks like failure | Now 9/9 healthy; if it fails, something is genuinely broken |
| Edit tracked file → `git status` shows `M` but nothing committed | Forgot to commit | Commit every meaningful change |
| Background `bash` output buffer stays empty | Subprocess not flushed | Don't rely on `bash_output` for progress; poll file existence + process metrics |
| `hermes_ensemble.py` missing | `hermes_worker.py` crashes on import | **Build it — it's the v3 priority** |
| DeepInfra `is_available() == False` | Live runs fail | `--dry-run` mode; wait for Casey to add API key to vault |

---

## 8. Your First 5 Moves

1. **Verify state:**
   ```powershell
   cd C:\Users\casey\tzpro-agent
   git status --short
   python doctor.py check 2>&1 | Select-Object -First 20
   ```
2. **Commit the 4 untracked Hermes files:**
   ```powershell
   git add docs/HERMES_TRAINING.md scripts/hermes_grade.py scripts/hermes_seed.py scripts/hermes_send.py
   git commit -m "feat(hermes): teacher harness + 6-tier mission ladder"
   ```
3. **Read today's plan and the active phase:**
   - `docs/PLANS/daily/2026-07-24.md`
   - `docs/phases/phase-1.5.md`
4. **Build `hermes_ensemble.py`** at `~/hermes-nerve-center/hermes_ensemble.py`. Multi-model parallel calls + agreement score. Wire into `hermes_worker.py`.
5. **Run your first Socratic batch:** 10 echograms, dry-run if API key not yet added. Write feedback to `~/tzpro-personal/echogram-analysis/feedback-batch-1.md`. Commit a `feat(hermes): first socratic batch + ensemble` with the rubric + agreement stats.

---

## 9. Doctrine In One Paragraph

**Suit ≠ person.** Repo = wearable shell (public-clean). `~/tzpro-personal/` = private life. **JSON is canonical truth.** Markdown is `f(JSON)`. **Capture is the heart.** Multi-modal sensors write Moments; the analyzer reads them; the projection layer shows them. **Hermes is the crew.** Teach her with missions, grade her work, promote her through tiers. **You are the Socratic teacher** — show her what good analysis looks like, write feedback she can learn from, iterate the loop. **The boat is small.** SPA, not Theia. FastAPI, not Blueprint. 50 KB, not 500 MB. **The yoke-move principle:** if a tool won't work here, do the work directly and record the gap.

---

## 10. Closing Note From Session #50

Casey's directive that brought us here:

> "great. can you get hermes analysing all our echograms with a few different models. she doesn't know much feedback-wise but there's a lot of smart models on deepinfra. you can iterate with here like a socratic teacher getting her oriented to tzpro and eileen and the mission of the boat to make she first attempts at analyzing creates a good layer of base data to evolve from as she learns what to look for more and more through finding patterns and corrolations and feedback and supervision etc"

The 90 echograms are the corpus. The multi-model ensemble is the microscope. Your feedback is the teacher. The loop is the curriculum. Hermes is the student. The boat is the classroom.

Welcome to the school.

— Mini-Agent Session #50
