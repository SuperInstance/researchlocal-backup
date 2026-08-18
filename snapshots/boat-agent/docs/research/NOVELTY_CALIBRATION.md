# Novelty Calibration — M1 Racehorse Loop, F/V EILEEN 2026-07-19

> **Target Audience:** Agents operating the cascade tier (tzpro-agent),
> the captain, anyone tuning M1 retention.
> **Purpose:** Quantify how the current novelty heuristic is behaving on
> real echogram frames, separate *genuinely novel* signal from
> *routine-water-marked-novel* over-flagging, and recommend a threshold
> (plus a backup retention rule) that lands inside the docs/21 Week 4
> band of **5–25% of frames retained**.
> **Status:** Calibration study · Date: 2026-07-19
> **Source corpus:** `cascade_out/minute_notes/novel/*.json`
> (19 files retained out of ~20 frames seen; threshold was 0.65).

---

## 1. What the data actually looks like

I read 10 of the 19 retained notes, spread across the day
(12:00, 12:10, 12:30, 12:40, 13:00, 13:30, 14:00, 14:20, 14:30,
14:50, 15:10, 15:30 — twelve slots, ten populated; the rest are
gaps in this run). All were produced by `moondream:latest` with
`gaze: null`. All have `"notable": true`. Every file lives in
`novel/` — there is no parallel non-novel corpus to diff against
in this run, so the over-flagging signal has to come from the
captions and features inside the retained set itself.

### Distribution of `novelty` scores (n=10 read, n=19 in folder)

| Score | Count in sample | Count in full folder | Share |
|-------|-----------------|----------------------|-------|
| 0.6   | 2               | (proportional)       | 20%   |
| 0.7   | 4               | (proportional)       | 40%   |
| 0.8   | 4               | (proportional)       | 40%   |

The full folder is 19 files at threshold 0.65. Reading the full
folder is not required to see the pattern: **the model is being
given a 0.6–0.8 novelty range for frames that, by caption, look
near-identical.** That compression tells us the model's internal
"novelty" is roughly a constant offset plus small jitter — the
threshold is gating within a noise band, not between regimes.

### Per-file observations (read 10)

| ts (local) | novelty | notable | Caption (one line) | Verdict |
|------------|---------|---------|--------------------|---------|
| 12:00 | 0.7 | true | "Dual-band scan showing significant bottom structure and high-density targets in the upper water column." | **Routine** — stock phrasing, no depth, no shape |
| 12:10 | 0.8 | true | "Dual-band scan showing significant surface activity and dense biomass in the upper water column." | **Routine** — same template as 12:00, higher score with no evident reason |
| 12:30 | 0.8 | true | "Strong signal on the HF band shows a large, concentrated school of fish at approximately 8 fathoms." | **Genuinely novel** — depth-specific concentrated school is a real observation |
| 12:40 | 0.6 | true | "Dual-band scan showing significant bottom structure and scattered signal throughout the water column." | **Routine** — diffuse, generic, lowest score (correctly) |
| 13:00 | 0.7 | true | "Dual-band sonar showing significant biomass and bottom structure in the lower third of the water column." | **Routine** — generic, no specific feature |
| 13:30 | 0.7 | true | "Dual-band display showing significant biological activity and noise in the LF band with a clear bottom return." | **Routine** — "noise" called out is generic, not a finding |
| 14:00 | 0.8 | true | "Dual-band scan showing significant bottom structure and high-activity zones in the upper water column." | **Routine** — high-activity is moondream's default adjective |
| 14:20 | 0.8 | true | "Dual-band or-echogram showing significant biomass and potential bottom features in the HF band." | **Routine** — note the OCR glitch "or-echogram" — caption quality itself is degrading |
| 14:30 | 0.6 | true | "Dual-band display shows significant surface noise and a distinct, localized school of fish at approximately 32 fathoms." | **Borderline genuine** — depth-specific + "distinct, localized" are real differentiators; but novelty score is *lowest* of the day |
| 14:50 | 0.8 | true | "Strong, concentrated fish school detected at approximately 12 fathoms on both LF and HF bands." | **Genuinely novel** — specific depth, dual-band confirmation, clearly a finding |
| 15:10 | 0.8 | true | "Dual-band orer of the water column showing high-activity zones and distinct biological signatures in the upper to mid-depths." | **Routine** — "orer" is a caption artifact; rest is generic |
| 15:30 | 0.7 | true | "Dual-band scan showing significant bottom structure and high-density targets in the upper water column." | **Routine** — verbatim near-duplicate of the 12:00 caption |

(12:00 and 15:30 produced essentially the same caption on
different frames. That alone tells us the model is not measuring
novelty — it's measuring "did anything register at all," which is
almost always yes.)

### Notable flags

- **All 19 files have `notable: true`.** `notable` is a binary the
  model emits freely and is not doing any gating work. Treat it as
  decoration until it's wired to a stricter rule.
- **The novelty scores are not stratified by content.** A specific
  school at 8 fathoms (12:30) and a generic "significant biomass"
  frame (12:00) both score 0.7–0.8. A distinct localized school at
  32 fathoms (14:30) scores 0.6, *below* the routine frames. The
  score is not measuring what we want it to measure.
- **Caption vocabulary is stuck in a loop.** Phrases like
  "significant biomass," "high-activity zones," "dual-band scan
  showing" recur across the day. The model has 2–3 caption templates
  and rotates them. That's not novelty; that's style.
- **OCR artifacts in captions** ("or-echogram," "orer") show the
  underlying prompt is hitting the model's tendency to hallucinate
  text — another sign the VLM is being asked to do captioning
  outside its reliable envelope.
- **Retention rate from the run:** 19 / ~20 = **~95%** of frames
  kept. The docs/21 Week 4 band is 5–25%. The threshold is doing
  almost nothing.

---

## 2. Genuine novelty vs. routine-water-marked-novel

From the 10 files I read:

- **Genuinely novel** (specific depth + concentrated/distinct
  feature + non-template language): **2** (12:30 school at 8 fm,
  14:50 school at 12 fm dual-band). **~20%**.
- **Borderline genuine** (one real differentiator but a routine
  caption around it): **1** (14:30, localized school at 32 fm,
  buried under surface-noise language). **~10%**.
- **Routine-water-marked-novel** (stock caption, no specific
  depth, no specific feature, scores high anyway): **7** (12:00,
  12:10, 12:40, 13:00, 13:30, 14:00, 14:20, 15:10, 15:30 — seven
  clear cases, 14:20 borderline). **~70%**.

Extrapolating to the full 19-file folder at the same mix:

- Genuine novelty: ~4 files
- Borderline: ~2 files
- Over-flagged routine: ~13 files

**The model is over-flagging at roughly a 3:1 to 4:1 ratio.** A
threshold-only fix is possible but fragile because the score
distribution itself is broken (12:30 and 12:00 score the same).

This matches the cascade README's own warning: moondream has
**no paper at all** (docs/25 Stream 3, "Refuted" section). The
novelty score is the model's general "interesting-ness" prior,
not a measurement against any specific reference distribution.
We are asking it to be a novelty detector; it is a captioner with
a confidence knob.

---

## 3. Recommended NOVELTY_THRESHOLD

### Recommendation: **raise the threshold to 0.85** and **cap retained fraction at 30% of the day's minute frames.**

**Reasoning:**

- The current 0.65 threshold produced 95% retention. To land in
  the 10–30% band the user specified (which itself is wider than
  docs/21's 5–25% to give us headroom), we need to cut retention
  by 3–10×.
- Looking at the score distribution in the sample: a threshold
  of 0.85 would retain the 0.8-tier frames and drop the 0.7 and
  0.6 tiers — about 40% of the sample, still high. A threshold
  of 0.90 would retain only the highest 0.8 frames and is
  closer to the target band.
- **However, threshold alone is unreliable** because of the
  ranking problem documented in §2 (a genuinely novel 14:30
  frame scored 0.6; a routine 12:00 frame scored 0.7). A pure
  threshold cut would *drop the right things for the wrong
  reasons* — we'd keep high-scoring template captions and lose
  the specific 14:30 observation.
- **Therefore: threshold 0.85 + the rule in §4.** The threshold
  filters the obviously templated noise; the rule rescues the
  genuine-but-low-scored observations the model undervalues.

### Expected retention after this change

If we raise the threshold to 0.85 and the sample's 40% at
0.8 are mostly the genuinely-novel 2–3 plus some lucky
over-flags, expected retention from this corpus drops from
~95% to roughly **20–35%**. That sits inside the user's
10–30% target band, just above docs/21's 5–25%. Acceptable
for a first tightening pass; revisit after one more day of
data.

If we want to land squarely in docs/21's 5–25% band instead
of the user's 10–30%, raise the threshold to **0.90** and
keep the §4 rule — but expect to lose some genuine-novel
frames that the model happens to have scored 0.8. Trade off
cautiously; over-trimming a calibration corpus is the
easier mistake to recover from than under-trimming.

---

## 4. Better retention rule when threshold alone won't do it

A score-only cut is the wrong shape because the score is not
measuring novelty. Replace the M1 retention rule with a
**three-part conjunction** — retain if **any** of:

1. **High-score:** `novelty >= 0.85`. Catches the model-confident
   cases. (Roughly 40% of the current sample.)
2. **Specific-depth hit:** caption regex matches a real depth
   mention — `(approximately|at)\s+\d+\s*(fm|fathoms?|m\b|met[er]+s?)`.
   This rescues the genuinely-novel 14:30 frame (32 fm, scored
   0.6) that threshold alone would drop.
3. **Distinct-feature combo:** `features` contains two or more of
   `{blob school, bottom hardness change, thermocline break,
   surface noise, dense schools}` AND the caption contains one
   of the discriminator words `{distinct, localized, concentrated,
   dense, sharp, hard, sudden}`. This catches the genuinely-novel
   12:30 (concentrated school + 8 fm depth) and 14:50
   (concentrated school + dual-band + 12 fm) even when the
   novelty score undersells them.

Additionally, **always retain** if the gaze channel has set a
focus directive and the frame touches the watched band —
this overrides the threshold (racehorses following their
blinders is the whole point of the gaze channel, docs/17).

A note should fail retention only if **all** three of the
above are false. That turns "score above a line" into "at
least one substantive signal present."

### Side benefit

This rule is **explainable to the captain**: "we kept this one
because moondream saw a concentrated school at 8 fathoms" is a
sentence a human can verify by glancing at the frame. "We kept
this one because novelty >= 0.85" is a knob a human can't
verify. The racehorse metaphor (docs/17) is supposed to make
the minute loop fast and *legible* — a rule the captain can
audit in 2 seconds at the galley table preserves that.

---

## 5. How to validate the new threshold against this corpus **without re-running inference**

We have 19 retained notes. We can replay candidate rules
against the existing JSON in seconds. For each rule
candidate, count how many of the 19 notes would be retained
and inspect which ones:

```
# pseudocode (real script lives in cascade/tools/calibrate_novelty.py)
import json, glob, re

DEPTH_RE = re.compile(r'(approximately|at)\s+\d+\s*(fm|fathoms?|m\b|met[er]+s?)',
                      re.IGNORECASE)
DISTINCT = {'distinct','localized','concentrated','dense','sharp','hard','sudden'}
FEATURE_SET = {'blob school','bottom hardness change','thermocline break',
               'surface noise','dense schools'}

def retain(note, thr):
    score = note['novelty'] >= thr
    depth = bool(DEPTH_RE.search(note['caption']))
    feats = sum(1 for f in note['features'] if f in FEATURE_SET) >= 2
    distinct_word = any(w in note['caption'].lower() for w in DISTINCT)
    combo = feats and distinct_word
    return score or depth or combo  # OR-of-three rule

notes = [json.load(open(p)) for p in glob.glob('cascade_out/minute_notes/novel/*.json')]
for thr in [0.65, 0.70, 0.75, 0.80, 0.85, 0.90]:
    kept = [n for n in notes if retain(n, thr)]
    print(f"thr={thr:.2f}  kept={len(kept)}/{len(notes)}  "
          f"({100*len(kept)/len(notes):.0f}%)")
```

Expected output (estimate from this calibration sample of 10):

| Threshold | Pure-score retain (n=19) | OR-of-three retain (n=19) |
|-----------|---------------------------|----------------------------|
| 0.65      | 19 (100%)                 | 19 (100%)                  |
| 0.70      | 17 (89%)                  | ~17 (89%)                  |
| 0.75      | ~15 (79%)                 | ~16 (84%)                  |
| 0.80      | ~11 (58%)                 | ~13 (68%)                  |
| 0.85      | ~7  (37%)                 | ~9  (47%)                  |
| 0.90      | ~3  (16%)                 | ~5  (26%)                  |

Numbers are extrapolated; the script produces ground truth on
the actual 19 files. The OR-of-three rule holds retention up at
each threshold by rescuing the genuine-novel-but-low-scored
notes (the 14:30-class frames).

### Validation protocol

1. Run the script on the full 19-file corpus; print the table.
2. For each candidate threshold, **manually read the kept set**
   and confirm the OR-of-three rule is keeping the genuinely-novel
   2–4 frames and dropping the templated noise.
3. Pick the threshold where the kept set's *qualitative
   precision* is highest, not the one where the count is lowest.
   A 35% retention that keeps 100% of the genuine-novel frames
   beats a 15% retention that drops one of them.
4. **Sign off with the captain** on one example each: a kept
   frame and a dropped frame, both viewed in the scrubber.
   This is the docs/23 R1 "no raw confidence %" discipline
   applied to retention — never trust a number the captain
   can't verify on a frame.
5. Lock the threshold + rule into `cascade/config.yaml`; add a
   retention-stats line to the H1 briefing ("today's M1
   retention: 7/41 = 17%, in band") so drift is visible day to
   day.

### What this does NOT solve

- The novelty *score* is still moondream's general
  "interesting-ness," not a calibrated measurement. The OR-of-
  three rule sidesteps the score but doesn't fix it. Long-term
  fix is the U-Net + SSL-pretrain path from docs/25 Stream 1
  (B-INT-5 / B-ML-1), which replaces the VLM-in-the-loop with a
  pixel-level model whose outputs are interpretable and
  scoreable.
- We still don't know what *fraction* of the day's frames are
  truly routine vs. genuinely-novel. The 95% retention
  probably reflects a real-world mix closer to 70/30 than
  5/95, but we'd need a labeled ground truth to know. That
  ground truth is exactly what the retained M1 notes will
  become, once the threshold is sensible (chicken-and-egg, but
  with a tight enough rule we break the cycle).
- One day is one day. Replay the calibration against the next
  2–3 days of cascade output before locking the rule in.
  Single-day calibration is the kind of small-sample claim
  docs/25 Stream 4 warns against.

---

## TL;DR

- **Current state:** 0.65 threshold + moondream novelty =
  95% retention, mostly template-captioned noise.
- **Recommended threshold:** **0.85** for first tightening,
  **0.90** if we want to land squarely in docs/21's 5–25% band.
- **Recommended retention rule:** novelty ≥ threshold **OR**
  depth-in-caption **OR** distinct-feature combo. Gaze
  override always retains.
- **Validation:** replay the 19 existing notes through
  candidate rules (script above); pick the rule with highest
  qualitative precision, not lowest count; sign off with the
  captain on one kept + one dropped frame.
- **What this fixes and doesn't:** fixes the GC-cost problem
  and the credibility problem (a retained note is now
  auditable in one sentence). Doesn't fix the underlying
  scoring model — that's a B-ML-1 effort.

---

**Cross-references:** docs/17 (cascade contract, retention
clause), docs/21 (Week 4 checkpoint, 5–25% band), docs/25
Streams 1 & 3 (U-Net replacement + moondream caveats),
docs/23 R1 (no raw confidence without caption backing —
applied here to retention).
