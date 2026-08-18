# The Suit and the Person — The Privacy Charter

> **The single most important document in this repository.** If you
> only read one file before contributing, read this one.
>
> This is what makes the boat-agent system *viral*. It is also what
> keeps it private. Privacy and virality are not opposites here — they
> are the same boundary, named correctly.

---

## The TL;DR (one paragraph)

A git-agent is two things — the **suit** (the repo, the agent's body) and
the **person** (the data the agent carries in the suit). The suit is the
public artifact: schemas, code, doctrine, harness, README. The person is
the private artifact: actual NMEA captures from a real boat, the captain's
voice notes, API keys, the SQLite database of a specific vessel's
moments, the trained LoRA weights for a specific fishing operation. The
suit is what other people clone, fork, and operate. The person is what
the suit wears for one specific operation. The suit is the PowerArmor;
the person is the pilot inside it. **Never commingle the two.**

---

## Why this matters

A git-agent system that commingles suit and person is useless to anyone
except its creator. The moment you push `.env` to the public repo, the
moment you commit a real boat's NMEA log, the moment you check the
captured thermal image of someone's engine bay into git — the repo is
now toxic. It cannot be shared. It cannot be made public. It cannot be
forked. The viral mechanic is dead.

A git-agent system that separates suit and person is the opposite: the
suit is a clean, descriptive, shareable artifact. Anyone can clone it.
Anyone can fork it. Anyone can read it and understand how to operate
*their own* agent in *their own* operation. The person they bring to
the suit — their data, their boat, their voice, their keys — lives
outside the repo, attached via configuration, never committed.

This is how `tzpro-agent/` becomes a template that other captains can
use without seeing Casey's boat. And how Casey's boat stays a private
instance of a public template.

---

## The charter (the seven rules)

### 1. The repo is the suit. The data is the person.

The boundary is sharp. The repo contains:

- **Source code** (agents, harnesses, dashboards, APIs).
- **Schemas** (the shape of Moments, Captures, Alerts — without the data).
- **Doctrine** (this file, BOOTCAMP, AGENT_OPERATING_MODEL, etc).
- **Sample/fixture data** that's clearly synthetic and isolated in `tests/` or `fixtures/`.
- **Documentation** about how to operate, but never examples that reveal a real operation.

The repo does **not** contain:

- Real NMEA captures from a real boat.
- Real voice notes, fish counts, or captain's observations.
- API keys, OAuth tokens, or any credentials.
- The live SQLite database of a specific vessel's moments.
- Trained LoRA weights for a specific operation.
- Personal notes, names, phone numbers, locations of personal significance.
- Logs of a real session (the BATON_PASS docs are *templates*, not personal logs).

### 2. Personal data lives in a sibling folder, not in the repo.

The convention is:

```
~/
├── tzpro-agent/                  # the suit (this repo, public-clean)
│   ├── docs/
│   ├── delegation/
│   ├── capture_daemon.py
│   ├── doctor.py
│   └── ...
│
├── tzpro-personal/               # the person (NOT a repo, or a private repo)
│   ├── vessel_state.jsonl        # your boat's actual NMEA stream
│   ├── captures/                 # your boat's actual screenshots
│   ├── vault.dpapi               # your API keys (DPAPI-encrypted)
│   ├── lora/                     # your trained LoRA weights
│   ├── daily_log/                # your captain's daily notes
│   ├── sessions/                 # your session exports
│   └── vessel_config.local.json  # your boat's specific config overrides
│
└── tzpro-agent-private.git/      # optional: a *private* fork if you want
    └── ... (mirror of tzpro-agent with your changes, never public)
```

The suit reads the person via **explicit paths** configured in
`vessel_config.py` or environment variables. The suit never assumes
the person is in the repo. The suit can be operated without any
person (pure schema + harness + tests) — that's the demo mode.

### 3. Schemas are the contract. Data is the payload.

Schemas live in the repo. They describe the shape of a Moment, a
Capture, an Alert. They are version-controlled, public, and stable.
A schema is like a Class definition in OOP — it belongs to the
type, not the instance.

Data that flows through a schema is the payload. That payload lives
in the person's storage. The suit defines `class Moment: ...` and
the person fills `Moment(timestamp=..., sounder_image=...)`.

This means someone can read the repo and understand *how* to
represent their own boat's data without seeing Casey's boat's data.

### 4. Configuration is connection, not commitment.

The suit has a `vessel_config.py` that defines a boat's
configuration. The default values in `vessel_config.py` are
**demonstrative placeholders** — not real boat settings. Real
settings live in a `vessel_config.local.json` that's loaded at
runtime if present, and ignored otherwise.

```python
# vessel_config.py (in repo, generic)
DEFAULT_VESSEL_NAME = "ExampleBoat"
DEFAULT_VESSEL_MMSI = 0  # zero = "not configured"
```

```json
// vessel_config.local.json (in person, example)
{
    "vessel_name": "F/V Eileen",
    "vessel_mmsi": 366999999,
    "nmea_port": "COM6",
    "thermal_camera_index": 1,
    "underwater_camera_index": 0
}
```

The local file is `.gitignore`'d. The suit can run with or without
it. The same suit works for every boat.

### 5. Logs are private. Templates are public.

A `BATON_PASS_<date>.md` document is a *template artifact*. It
describes the format, the structure, the lessons. The actual *content*
of someone's session handoff — what they were working on, what their
data showed, what they discovered — is private.

The repo can contain:

- A `docs/BATON_PASS_TEMPLATE.md` showing the structure.
- A `docs/BATON_PASS_<date>.md` that is a **synthetic example**
  using fake boat data, for educational purposes.

The repo cannot contain:

- A real session's handoff with real boat data, real decisions, real
  observations.

If you want to commit a real session's handoff, put it in the
private fork (`tzpro-agent-private.git/`) or in the personal folder
under `sessions/`. The public repo stays clean.

### 6. API keys, secrets, and credentials are vaulted, never committed.

The repo includes a `vault.py` module (in the suit) that provides
the *mechanism* for DPAPI-encrypted secret storage. The vault file
itself (`vault.dpapi`) is in the person. The vault module is
generic; the secrets inside it are personal.

`vault.py` is committed. `vault.dpapi` is `.gitignore`'d. Anyone
running the suit creates their own vault on first run.

This rule covers every credential:

- LLM provider API keys (Anthropic, OpenAI, OpenRouter, DeepInfra).
- Cloudflare API tokens.
- TzPro or marine software license keys.
- Analysis service credentials (Workers AI, Vectorize).
- Push notification tokens.
- Any other secret.

### 7. The "viral mechanic" is the fork-and-private pattern.

The way a suit becomes viral:

1. Developer (Casey) builds the suit in a public repo.
2. Other captains and developers find the suit useful.
3. They fork the repo (their fork is private by default).
4. They customize their fork for their own operation — adding
   features, tuning prompts, training local LoRAs for their
   fishery, etc.
5. They use their private fork as their own git-agent. Their
   boat's data lives in their personal folder, accessed via
   their customized suit.
6. **Optional:** they contribute back improvements to the
   public suit via PR. The improvements are generic (schema
   additions, new capture sources, better algorithms) — never
   specific to their operation.

This works because the suit is *complete enough to be useful,
generic enough to be safe to share*. The completeness comes
from the suit solving real problems (capture, analysis, alarms,
dashboard). The genericness comes from the suit/data boundary.

---

## The "what stays in the repo" checklist

Before committing, ask:

- [ ] Does this file contain real data from a real operation?
- [ ] Does this file contain credentials, tokens, or keys?
- [ ] Does this file contain personal observations, names, or locations?
- [ ] Could this file embarrass the captain, the boat owner, or a crew member?
- [ ] Could a competitor use this file to gain an unfair advantage?
- [ ] Does this file describe a specific vessel's hardware configuration in a way that should be private?

If any answer is yes, the file goes in the personal folder, not the
repo. Add a `.gitignore` entry if needed.

The exception: **synthetic fixtures**. `tests/fixtures/*.json` with
clearly fake data (vessel name "TESTBOAT", MMSI 0, coordinates
"00.0000°N 00.0000°W") can be committed. They document the schema
and are useful for testing.

---

## The "what stays in the personal folder" boundary

```
tzpro-personal/
├── *.jsonl, *.db, *.sqlite      # actual data
├── captures/                    # actual images
├── voice/                       # actual voice notes
├── logs/                        # actual session logs
├── vault.dpapi                  # encrypted credentials
├── lora/                        # trained model weights
├── vessel_config.local.json     # local overrides
├── sessions/                    # session exports, BATON_PASS archives
└── exports/                     # anything exported for sharing
```

Anything you would not want published in a fishing magazine goes here.

---

## The "what goes in a private fork" boundary

Sometimes the person wants to track changes to the suit itself
(the code, the schema, the harness) but customized for their
operation. That's a private fork:

```
tzpro-agent-private.git/         # GitHub: your-username/tzpro-agent (private)
├── (mirror of tzpro-agent with your customizations)
├── experiments/                 # your experimental features
├── custom-analyzers/            # your fishery-specific code
└── private-docs/                # your personal notes about your instance
```

The private fork is a normal git repo, just not public. You can
push to it, branch it, and tag it. You can pull updates from the
public suit and rebase. The diff between your fork and the public
suit is your own work-product.

PRs from the private fork to the public suit should always
**remove** personal paths before submitting.

---

## The "how to onboard a new captain" flow

A new captain wants to use the boat-agent platform. Their flow:

1. They clone `tzpro-agent/` (the suit) into their own machine.
2. They create `tzpro-personal/` (their data folder) wherever they
   want. Convention: as a sibling of `tzpro-agent/`.
3. They run `python setup.py init` which scaffolds `vessel_config.local.json`
   in `tzpro-personal/` and a `vault.dpapi` skeleton.
4. They point their suite at the personal folder via env vars
   (`TZPRO_PERSONAL_DIR=...`).
5. They begin capturing. Their data lands in `tzpro-personal/`.
6. The suit runs, the analysis runs, the dashboard works. None of
   their data is in the repo. They can `git status` in `tzpro-agent/`
   and see "nothing to commit, working tree clean" — even after
   hours of operation.

If they later want to customize the suit (their own analyzer, their
own capture cadence), they fork it private. Their customization
goes in their fork. Their data still lives in `tzpro-personal/`.

---

## The "what if I commit something by mistake" mitigation

- `.gitignore` should be **aggressive**. Anything that looks like
  data, secrets, or personal notes should be ignored by default.
  Whitelist the few things the repo actually wants to track.
- `doctor.py check` should include a privacy check: scan the
  working tree for forbidden patterns (API keys, NMEA coordinates,
  vessel names, etc.) and refuse to commit if found.
- Pre-commit hooks (when available) should run the privacy check.
- The first line of defense is the discipline of the agent. The
  second is `.gitignore`. The third is the doctor. The fourth is
  review.

A useful rule: **if the file's content would surprise the captain
when shown on a public screen, it doesn't go in the repo.**

---

## The "what lives in the suit's schema" boundary

Schemas are public. They describe shapes. They include:

- Field names (e.g., `timestamp`, `lat`, `lon`, `sounder_image_path`).
- Field types (e.g., `str`, `float`, `Path`).
- Validation rules (e.g., `lat in [-90, 90]`, `lon in [-180, 180]`).
- Enumerations (e.g., `state_class ∈ {docked, trolling, slow_cruise, ...}`).
- Example synthetic values in comments.

Schemas do **not** include:

- Sample real data. (Use `tests/fixtures/` for that, with synthetic data.)
- Implementation details that leak operation specifics (e.g., the
  exact MMSI of any vessel).
- Hard-coded paths to the personal folder.

The schema is the contract. The contract is public. The data
fulfilling the contract is private.

---

## The "what this means for the agent's design" principle

When the agent designs a new feature, it must ask:

1. **What is generic?** (Suit) — the schema, the code, the
   interface, the docker-compose, the docs.
2. **What is specific?** (Person) — the actual data, the
   specific config, the trained model, the daily log.

A new feature lands in the suit as a generic mechanism. The
user (captain) configures it with their specific values in
their personal folder.

**Example:** A new "alert on engine temperature" feature.

- **Suit** (committed): the analyzer code, the alert schema, the
  dashboard widget, the threshold configuration interface.
- **Person** (private): `vessel_config.local.json` has
  `"engine_temp_threshold_f": 195`. The thresholds are tuned
  for the specific engine (Detroit 6-71 vs. Cummins QSB).

A new captain adopting the suit sets their own thresholds in
their own personal config. The suit doesn't know or care.

---

## Connects to other docs

- `docs/AGENT_OPERATING_MODEL.md` — doctrine. The "who you are"
  adds the "and what you wear" via this document.
- `docs/BOOTCAMP.md` — operating rules. Add a rule here:
  **"Suit and person stay separate. Personal data lives in
  `~/tzpro-personal/`, not in the repo."**
- `docs/NAVIGATION.md` — cockpit map. The personal folder is its
  own "cockpit" in the sense that it's a folder the agent operates
  in, but not a version-controlled repo.
- `docs/ROADMAP.md` — phase index. The "viral mechanic" is the
  Phase-∞ vision. The privacy charter is what makes it work.
- `docs/BATON_PASS_<date>.md` — template, not personal. Add a
  `BATON_PASS_TEMPLATE.md` that this document references.

---

## What this means for *this* boat

Casey's F/V Eileen sits in `~/tzpro-personal/`. The repo
`tzpro-agent/` is the suit. They are separate. The suit is
evolving toward being shareable. The person is private.

Right now, **the suit is far from clean**. There are still
`vessel_state.jsonl` files, captures, logs, and `.consensus_state.json`
files in the repo root that should be in the personal folder.

The migration plan:

1. Create `~/tzpro-personal/` (sibling folder).
2. Move runtime data, captures, logs, vault, local config from
   `tzpro-agent/` to `~/tzpro-personal/`.
3. Update `.gitignore` to exclude the runtime files (already done
   in part — needs review).
4. Update `vessel_config.py` to load `vessel_config.local.json`
   from `~/tzpro-personal/` when present.
5. Run `git status` — should be clean of runtime data.
6. Commit the doctrinal changes (this document + the migration).
7. **First public push** — the suit is now safe to share.

The migration is a task. It is not done yet. It is the next
major piece of work after the capture-system spec lands.

---

## See also

- `docs/AGENT_OPERATING_MODEL.md` — the doctrine
- `docs/BOOTCAMP.md` — the operating rules (add the suit/person rule)
- `docs/NAVIGATION.md` — the cockpit map (add `~/tzpro-personal/` as a non-repo folder)
- `docs/architecture/PRIVACY_BOUNDARY.md` — the operational schema
- `docs/architecture/CAPTURE_PIPELINE.md` — the multi-modal capture pipeline
- `docs/ROADMAP.md` — the phase index (Phase-∞ is the viral mechanic)
