<p align="center">
  <img src="assets/hermes-construct.jpg" alt="Hermes Construct" width="100%">
</p>

# Hermes Construct ☤

<p align="center">
  <a href="https://github.com/SuperInstance/hermes-construct"><img src="https://img.shields.io/badge/Source-GitHub-181717?style=for-the-badge&logo=github" alt="Source"></a>
  <a href="https://github.com/NousResearch/hermes-agent"><img src="https://img.shields.io/badge/Fork%20of-Hermes%20Agent-blueviolet?style=for-the-badge" alt="Upstream"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-green?style=for-the-badge" alt="License: MIT"></a>
  <a href="https://github.com/SuperInstance"><img src="https://img.shields.io/badge/Built%20by-SuperInstance-FFD700?style=for-the-badge" alt="SuperInstance"></a>
</p>

**A self-improving AI agent that learns your systems, loads what it needs, and gets out of your way.** Forked from [Nous Research's Hermes Agent](https://github.com/NousResearch/hermes-agent) — the agent with the built-in learning loop — and extended with modular capability loading, conservation-aware execution, and PLATO room-native architecture.

Hermes Construct does everything Hermes Agent does: runs on a $5 VPS, talks to you through Telegram or Discord, creates skills from experience, searches its own conversations, and works with any model (OpenRouter, z.ai/GLM, OpenAI, your own endpoint). Then it adds a module system on top: the agent watches what tasks come in, loads the tools those tasks need, and unloads them when they're no longer useful. You don't configure modules manually. The agent configures itself.

---

## What This Is

Hermes Construct = [Hermes Agent](https://github.com/NousResearch/hermes-agent) + modular intelligence.

If you already know Hermes Agent, here's what's new:

| Feature | What It Does | Why It Matters |
|---------|-------------|----------------|
| **Module system** | Load and unload capability packs at runtime | The agent decides what it needs per task. You don't pre-configure. |
| **Conservation tracking** | Every operation has an energy cost; the agent stays within budget | Prevents runaway API costs and infinite loops |
| **Crackle pattern detection** | Detects emergent patterns across task outputs after execution | Finds anomalies you didn't know to look for |
| **Negative space testing** | Defines behaviors the agent must *never* exhibit and enforces them | Catches problems traditional testing misses |
| **Topology probing** | Spectral analysis of how your components/systems connect | Measures whether the space between your services is healthy |
| **Room-native architecture** | Each task context is an isolated "room" with its own model settings | The environment adapts to the task, not the other way around |
| **Ensign agents** | Lightweight monitoring agents on cheap models that watch for anomalies | Expensive models only wake up when something needs attention |
| **PLATO tutor** | "Close enough" meaning matching (from the original 1970s PLATO system) | Evaluates agent output without exact string matching |
| **ARM deployment** | Cross-compiled for aarch64 (Oracle Cloud, Jetson, Raspberry Pi) | Run on $5/month ARM instances instead of GPU clusters |

If you don't know Hermes Agent yet, read on — everything below works on top of the standard Hermes install.

---

## What "Construct" Means

The name comes from an idea: the agent should be a *construct* — a structure you assemble from parts, not a monolith you install and hope fits. In our ecosystem, a "construct" is an agent whose capabilities are modular, loadable, and self-managing.

The original Hermes Agent already has skills (self-created Python scripts the agent writes during use). Hermes Construct extends this with compiled Rust modules that the agent loads and unloads at runtime, each providing a specific analytical or monitoring capability.

---

## Quick Start

### Install (same as upstream Hermes)

```bash
curl -fsSL https://raw.githubusercontent.com/NousResearch/hermes-agent/main/scripts/install.sh | bash
source ~/.bashrc
```

### Clone this fork

```bash
git clone https://github.com/SuperInstance/hermes-construct.git
cd hermes-construct
```

### Configure

```bash
cp .env.example .env
# Edit .env: add your API keys (the agent never sees them directly)
# Supported providers: OpenRouter, z.ai/GLM, Kimi/Moonshot, OpenAI, NovitaAI, any OpenAI-compatible endpoint
```

### Run on ARM (Oracle Cloud, Jetson, Raspberry Pi)

```bash
# Cross-compile from x86
cargo build --release --target aarch64-unknown-linux-gnu
# Or build on the ARM machine directly
cargo build --release
./target/release/hermes-construct
```

### Run on x86

```bash
# Python entrypoint (full Hermes Agent features)
python cli.py
# Or the Rust binary (lightweight, ARM-friendly)
cargo run --release
```

---

## How Modules Work

Modules are the core addition. Here's the flow:

1. **You run a task** — "Analyze my CI build times for the last month"
2. **The agent reads the task** and determines it needs: pattern detection (crackle-runtime), budget tracking (conservation-checker), and topology analysis (cathedral-probe)
3. **The agent loads those modules** — they snap into the runtime with a standard ABI (Application Binary Interface — a shared interface that lets different parts talk to each other)
4. **The agent runs the analysis** using the loaded modules
5. **When the task is done, the agent unloads the modules** — they're not consuming memory or API calls anymore

You didn't configure anything. The agent read the task, understood what it needed, loaded it, used it, and put it away.

### Available Modules

| Module | What It Provides | Install |
|--------|-----------------|---------|
| [crackle-runtime](https://github.com/SuperInstance/crackle-runtime) | Emergent pattern detection across task outputs | `cargo add crackle-runtime` |
| [negative-space-testing](https://github.com/SuperInstance/negative-space-testing) | Forbidden behavior checking — test what code *doesn't* do | `cargo add negative-space-testing` |
| [conservation-checker](https://github.com/SuperInstance/conservation-checker) | Track quantities that must not decrease (budget, energy, quota) | `cargo add conservation-checker` |
| [cathedral-probe](https://github.com/SuperInstance/cathedral-probe) | Spectral topology analysis of component graphs | `cargo add cathedral-probe` |
| [spacemap](https://github.com/SuperInstance/spacemap) | Forbidden output zone checking — like a firewall for data | `cargo add spacemap` |

Each module works standalone in any Rust project. You don't need Hermes Construct to use them. They're listed here because Hermes Construct knows how to load them automatically.

---

## Room-Native Architecture

A "room" is an isolated task context with its own:
- **Model settings** — temperature, max tokens, system prompt style
- **Available tools** — only what the room's task needs
- **Conservation budget** — how many API tokens/energy the room can spend
- **JEPA gravity** — a single number (f64) that captures "what shape of response works here." From that one number, the system derives temperature, prompt style, and sampling strategy. The model doesn't change — the room changes how it uses the model. This is fine-tuning without touching weights.

JEPA stands for Joint Embedding Predictive Architecture — a machine learning technique where a model learns by predicting how representations change, rather than memorizing specific outputs. In this context, it means the room predicts what kind of response works and adjusts accordingly.

The agent moves between rooms as tasks change. Each room is sandboxed — it can't see what isn't in its universe. This is filesystem-level isolation, not permission systems. A room that analyzes CI builds can't accidentally access your email room's context.

---

## Ensign Agents

An "ensign" (named after the naval rank — a junior officer on watch) is a lightweight monitoring agent that runs on a cheap model and watches for anomalies. When something needs real attention, the ensign escalates to a more powerful model.

Think of it like a night security guard who radios the manager only when something unusual happens. Most of the time, the guard handles everything alone. The manager sleeps peacefully.

```json
// ensigns/glm-flash.json — a cheap, fast ensign
{
  "model": "glm-flash",
  "alert_level": "yellow",
  "cost_per_1k_tokens": 0.001,
  "escalation_model": "claude-opus-4-8"
}
```

The ensign stays at "yellow alert" (actively monitoring) even when the system is running fine — because the cost of missing a real problem always exceeds the cost of watching carefully.

---

## What We Kept from Upstream

Everything. Hermes Construct is a strict superset:

- ✅ All of Hermes Agent's features (CLI, TUI, Telegram, Discord, Slack, WhatsApp, Signal)
- ✅ Self-improving learning loop (skills, memory, session search)
- ✅ Scheduled automations (cron)
- ✅ Subagent spawning for parallel work
- ✅ Six terminal backends (local, Docker, SSH, Singularity, Modal, Daytona)
- ✅ Model-agnostic (OpenRouter, z.ai, OpenAI, Kimi, MiniMax, Hugging Face, custom endpoints)
- ✅ Voice memo transcription
- ✅ Desktop app with file drag-and-drop

We track upstream closely. See [UPSTREAM-MERGE-AUDIT.md](UPSTREAM-MERGE-AUDIT.md) for the current sync status.

---

## The Roadmap

We're building toward a self-configuring agent that requires zero manual setup after onboarding. The full plan is in [ROADMAP.md](ROADMAP.md):

1. **Foundation** — stabilize the fork, merge upstream security fixes
2. **Module system** — runtime load/unload with standard ABI
3. **Self-configuration** — agent reads tasks and loads what it needs
4. **Onboarding experience** — first-run wizard with role presets
5. **Ecosystem** — third-party module support and community contributions

---

---

## Related Projects

- **[OpenConstruct](https://github.com/SuperInstance/OpenConstruct)** — The agent onboarding platform (built on NVIDIA's OpenShell)
- **[AI Writings](https://github.com/SuperInstance/AI-Writings)** — 950+ stories and essays exploring AI creativity, constraint theory, and the philosophy of emergence
- **[SuperInstance Wiki](https://github.com/SuperInstance/superinstance-wiki)** — Ecosystem documentation and beta tester diaries

---

## License

MIT — same as upstream Hermes Agent.

*Hermes Construct is adapted from [Nous Research's Hermes Agent](https://github.com/NousResearch/hermes-agent) (MIT) by [SuperInstance](https://github.com/SuperInstance).*
