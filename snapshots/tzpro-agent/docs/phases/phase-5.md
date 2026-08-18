# Phase 5 — Voice STT/TTS

> **Status:** Planned
> **Depends on:** Phase 1 (session model), Phase 4 (more sources)

## Goal

The crew can talk to their dashboard from anywhere on the boat, hands
free, while doing other work. The captain can ask the system questions
out loud while steering.

## Architecture

- **STT (speech-to-text)** — local Whisper via faster-whisper on the
  ProArt. Push-to-talk or always-listening with wake-word ("hey agent").
- **TTS (text-to-speech)** — DeepInfra or ElevenLabs, runs on the
  selected heavy model with streaming output
- **Audio transport** — WebRTC from phone browser to dashboard server
  (or native app wrapper if WebRTC latency is too high)

## Voice capture becomes a moment

When crew member says *"24 chum, 3 pinks near the top, chum mostly at
30-45 fm"*, that utterance becomes:

```json
{
  "id": "voice_2026-07-23T15:42:11Z",
  "timestamp_utc": "2026-07-23T15:42:11Z",
  "source": "human_voice",
  "lat": 55.78, "lon": -131.68,
  "session_id": "crew-sam",
  "payload": {
    "audio_path": "/captures/voice/2026-07-23/154211.webm",
    "transcript": "24 chum, 3 pinks near the top, chum mostly at 30-45 fm",
    "duration_s": 7.2
  },
  "tags": ["chum", "pinks", "catch_report"],
  "embeddings": {"quick": [...], "deep": [...]}
}
```

## Why this matters for the analyzer

These voice moments become **training signal** for the echogram analyzer.
When the analyzer sees a screenshot and asks "what was caught here?",
the answer is now a structured moment it can correlate against. Over
time, the analyzer learns to recognize the visual signatures of high
chum-density troll windows, pink migrations near the surface, etc.

## What's new in Phase 5

- `voice/` package — STT worker, TTS worker, audio bridge
- Dashboard gets a microphone button + audio output
- Phone PWA with media stream permissions
- Optional wake-word detection on always-listening mode
