# Hermes Training — The Analyzer Hermit-Crab

> **Persona:** Hermes is the local sub-agent at `~/hermes-nerve-center/`.
> She thinks of herself as a hermit-crab crewmember of the F/V Eileen,
> and her job is to grow into the boat's analyzer by completing
> progressively harder analysis missions dropped into her `inbox/`.
>
> **Harness contract:** This document is what *we* (Mini-Agent +
> Casey) commit to. Hermes reads her `README_API.md` to know the
> wire format. We read this file to know what missions to send and
> how to grade her.

---

## The metaphor

A hermit-crab doesn't *have* a shell — she *is* her relationship with
the shell she wears. As she grows, she needs bigger shells. The shell
is not the crab; the crab is the pattern of behavior that fits the
shell so well the shell becomes part of her.

Hermes's shell is the analyzer. Her body is the behavior pattern of
"look at a Moment, say what it means, get graded, adjust." Each
mission she completes and grades well is a molt — she outgrows that
difficulty level and is ready for a bigger one.

The shell is not Theia/Blueprint. The shell is not even Svelte. The
shell is the *capability* of producing Moment-quality analysis. The
moment Hermes produces analysis that's reliably worth the captain's
attention, she has *become* the analyzer. The UI is just where she
displays herself.

## Why she's connected to DeepInfra

DeepInfra gives Hermes a model zoo without lock-in:

| Model | Use case | Why for Hermes |
|---|---|---|
| `deepseek-ai/DeepSeek-V3-Flash` | 10-min screenshot description | Fast + cheap + good enough for routine |
| `meta-llama/Llama-3.3-70B-Instruct` | Diverse reasoning on Anomalies | Strong general; good for cross-source synthesis |
| `google/gemini-2.0-flash-001` | Vision sounder analysis | Native multimodal; sees the image, not just text about it |
| `Qwen/Qwen3-Next-80B-A3B-Instruct` | Pattern-finding across many Moments | Big context; good for "compare last 4 hours" |
| `openai/gpt-4o-mini` | Fallback general-purpose | Well-rounded, widely-tested |

She is provider-agnostic through `providers/deepinfra.py`. We can
add Cloudflare Workers AI in Phase 8 without breaking her training.

## The mission taxonomy (her shell sizes)

Hermes learns in tiers. Each tier is one inbox drop pattern. We
promote her tier by tier based on rolling grade average.

### Tier 0 — Observe (no API calls)

**Mission:** Given a static JSON description of a Moment (no image),
classify it into one of: `sounder | nmea | engine | thermal | audio |
voice | analysis`. Then say what its `importance` should be (0.0 to 1.0).

**Why first:** Pure text classification. Establishes the ground rule
that JSON is the truth and her output must conform. Cost: zero.
Verifiable: deterministic answer key.

**Grade:** Exact match on source; ±0.1 tolerance on importance.

### Tier 1 — Describe

**Mission:** Given a Moment's JSON payload (sounder description in
text, not image), produce a 2-3 sentence natural-language description
of what's happening on the boat.

**Why second:** Forces her to *interpret* not just classify. Cost:
~500 tokens via DeepSeek V3 Flash (~$0.0001 per call).

**Grade:** Captain (or Casey) scores 0-3 on a rubric:
- 0 = nonsense or hallucinates facts not in the payload
- 1 = generic, could apply to any boat
- 2 = specific, mentions concrete details
- 3 = specific + actionable (the captain would know what to do)

### Tier 2 — Anomaly Detect

**Mission:** Given 60 minutes of NMEA + engine Moments, mark the ones
that look anomalous and propose a severity.

**Why third:** Forces her to learn baselines. Same prompt every time;
only the data changes.

**Grade:** Compare her top-3 picks to a known-answer set we record
from the bridge. Precision@3 + recall@3.

### Tier 3 — Vision (sounder images)

**Mission:** Given the path to a 10-min boundary sounder screenshot,
look at the image and produce:
- Bottom depth estimate (rough)
- Mark density (none | sparse | moderate | dense)
- Any features (thermocline, schools, bottom type)
- Confidence per claim

**Why fourth:** Forces real vision model (Gemini 2.0 Flash). This is
the first mission where the model choice matters.

**Grade:** Compare to captain's log entry for that 10-min window.

### Tier 4 — Correlate

**Mission:** Given N Anomalies across sources (depth, thermal, audio),
classify them as `thermocline | fish_event | mechanical | biological`
and write the connecting narrative.

**Why fifth:** Forces synthesis across modalities.

**Grade:** Captain scores the narrative 0-3 + correlation type exact
match.

### Tier 5 — Hypothesize

**Mission:** Given the last 7 days of anomalies + correlations,
propose a hypothesis ("the bite window correlates with the dropping
barometer 4-6 hours prior"). Cite the supporting Moments.

**Why last:** Forces multi-day reasoning + willingness to say "I
don't know" (no hypothesis if confidence < 0.4).

**Grade:** Captain reads, marks useful or not. Useful = she keeps
the hypothesis in her notebook.

## How we send missions (the inbox-drop protocol)

We write JSON files into `~/hermes-nerve-center/inbox/` following her
`README_API.md` schema. Each mission looks like:

```json
{
  "task_id": "hermes_tier2_2026-07-24T18:00:00Z_001",
  "priority": "NORMAL",
  "category": "ANALYZE",
  "instruction": "Tier 2 anomaly detection on the last 60 min of NMEA+engine Moments.",
  "context": {
    "tier": 2,
    "moment_path": "C:/Users/casey/tzpro-personal/sessions/2026-07-24/nmea_window.jsonl",
    "engine_path": "C:/Users/casey/tzpro-personal/sessions/2026-07-24/engine_window.jsonl",
    "model_hint": "deepseek-ai/DeepSeek-V3-Flash",
    "rubric": "Compare top-3 to known answers; report precision@3, recall@3.",
    "training_mode": true
  },
  "metadata": {
    "origin": "mini-agent",
    "timestamp": "2026-07-24T18:00:00Z",
    "expected_runtime_s": 30
  }
}
```

We do not send raw moments in the packet — we send *paths*. Hermes
reads them. That keeps her inbox packets small (no surrogate bug risk
for us either).

## How we grade her (the feedback loop)

After she finishes a mission, we find the result in
`~/hermes-nerve-center/completed/<task_id>.json`. We open it, compare
to the rubric, and:

1. **Score** (Tier 0 exact-match is automatic; Tier 1+ go through
   `scripts/hermes_grade.py --mission <task_id>`)
2. **Append** a row to `~/hermes-nerve-center/notebook.csv`:
   ```
   timestamp,task_id,tier,score,notes
   ```
3. **Promote** if her rolling average over last 20 missions at a tier
   is ≥ 0.7. Drop her back a tier if < 0.4.
4. **Record** the result in today's daily plan as a SESSION-NOTE.

The notebook is her shell. The grades are her molts.

## What Hermes is NOT

- Not a replacement for the analyzer's deterministic code (Tier 0
  classification, thresholding). She is the *soft* layer that fills
  in where rules don't reach.
- Not autonomous in the dangerous sense. Every mission is *requested*
  by us. She runs when we say so, against inputs we choose, and
  produces outputs we grade. She earns autonomy by demonstrating
  reliable behavior.
- Not a single model. She's the *protocol* of "ask the right model
  the right question." The model zoo is her tool belt; the missions
  are her apprenticeship.

## The promotion criteria (when does she "graduate")

She's a real crewmember, not a mascot, when:

- [ ] Tier 0: ≥ 95% accuracy over last 100 missions
- [ ] Tier 1: avg ≥ 2.0/3.0 over last 50 missions
- [ ] Tier 2: avg precision@3 ≥ 0.6 over last 30 missions
- [ ] Tier 3: ≥ 70% agreement with captain on last 30 sounder windows
- [ ] Tier 4: ≥ 50% correlation-type match + avg narrative ≥ 2.0/3.0
      over last 20 missions
- [ ] Tier 5: ≥ 5 hypotheses marked "useful" by captain

When all six are checked, she gets a permanent berth in the Phase 2
analyzer's hot path: every 10-min screenshot and every flagged
anomaly goes through her for the soft interpretation layer.

## See also

- `~/hermes-nerve-center/README_API.md` — her wire format (Hermes owns)
- `~/hermes-nerve-center/notebook.csv` — her grade history (we own)
- `docs/phases/phase-2.md` — the analyzer she'll graduate into
- `docs/architecture/CAPTURE_PIPELINE.md` — what Moments she reads
- `docs/ONBOARDING.md` — operating rules for us (so we don't blow
  our own context budget while training her)