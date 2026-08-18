# Phase 9 — Spatially/Temporally Aware Vector DB

> **Status:** Planned — vision-mode
> **Depends on:** Phase 8 (Vectorize at scale)

## Goal

Embeddings aren't just text or image features anymore — they encode
**where** and **when** as first-class dimensions, so "moments near this
one in time and space" becomes a native query.

## Concept

A standard embedding says: "this moment is like that one" (semantic
similarity).

A spatially/temporally aware embedding says: "this moment is *adjacent*
to that one in time, position, AND content." So a query like *"show me
echograms from when we were 0.5 nm from here, 2 hours earlier"* becomes
a single vector operation.

## How

Encode position as a sinusoidal feature (like transformer positional
encodings), encode time the same way, concatenate with the content
embedding. Train or fine-tune so the geometry respects the relationships
we care about.

For grouping: HNSW or similar ANN index where the distance metric
weights time/space/content differently based on the query type.

## Why this matters

Without this, "what did the sounder look like when we were last here?"
requires explicit time-range and bbox filters. With this, it's a single
nearest-neighbor lookup.

The group-by-raw-properties concept (mentioned in the user's notes about
"grouped and analysed by its raw data twin properties") becomes
straightforward: moments with similar time+space+content vectors form
natural clusters; clusters become patterns; patterns become entries in
the daily debrief.

## What's new in Phase 9

- New embedding model (fine-tune of nomic-embed or similar)
- Modified Vectorize index with custom distance metric
- Query API: `find_similar(moment_id, time_weight=, space_weight=, content_weight=)`
- Visualization: cluster maps in the dashboard
