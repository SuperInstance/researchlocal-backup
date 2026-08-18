# LOW LEVEL — Pincher

> *For contributors, performance tuners, and anyone porting to new platforms. Covers internal module structure, key patterns, and optimization strategies.*

## Internal Architecture

### Crate Structure

```
pincher/
├── pincher-core/                    # All runtime logic
│   ├── src/
│   │   ├── lib.rs                   # Root module + re-exports
│   │   ├── reflex/
│   │   │   ├── mod.rs               # ReflexEngine
│   │   │   ├── engine.rs            # Core engine loop
│   │   │   ├── matcher.rs           # Vector similarity matching
│   │   │   └── confidence.rs        # Confidence update model
│   │   ├── db/
│   │   │   ├── mod.rs               # Database handle + schema
│   │   │   └── schema.rs            # SQL schema + row types
│   │   ├── embed/
│   │   │   ├── mod.rs               # Embedding trait + re-exports
│   │   │   └── onnx.rs              # ONNX Runtime backend
│   │   ├── sandbox/
│   │   │   ├── mod.rs               # Sandbox trait
│   │   │   └── bwrap.rs             # Bubblewrap implementation
│   │   ├── security/
│   │   │   ├── mod.rs
│   │   │   ├── veto.rs              # Veto engine
│   │   │   └── capability.rs        # Capability tokens
│   │   ├── migration/
│   │   │   ├── mod.rs               # .nail pack/unpack
│   │   │   └── packer.rs            # tar.zst + BLAKE3
│   │   ├── resource/
│   │   │   ├── mod.rs               # Resource budgets
│   │   │   └── pid.rs               # PID controller
│   │   ├── rpc/
│   │   │   └── mod.rs               # JSON-RPC server
│   │   ├── route/
│   │   │   └── mod.rs               # Graph algorithms
│   │   ├── capability/
│   │   │   ├── mod.rs               # Capability system
│   │   │   ├── manifest.rs          # Manifest types
│   │   │   └── token.rs             # Token signing
│   │   └── immunology/
│   │       └── mod.rs               # Immune system patterns
│   └── Cargo.toml
├── pincher-cli/
│   ├── src/main.rs                  # CLI binary (clap)
│   └── Cargo.toml
├── pincher-infer/                   # Python inference module
└── src/                             # Legacy root (daemon, updater, registry, extractor)
```

### Module Map

| Module | Responsibility | Key Types |
|--------|---------------|-----------|
| `reflex/` | Core engine — teach, match, execute, confidence | `ReflexEngine`, `Reflex`, `Execution` |
| `db/` | SQLite persistence + vector search | `Database`, `ReflexRow` |
| `embed/` | Text → vector embedding | `cosine_similarity`, `EMBEDDING_DIM` |
| `sandbox/` | Isolated command execution | `SandboxConfig` |
| `security/` | Pre-execution validation + auth | `VetoEngine`, `Capability` |
| `migration/` | Portable agent packing | `pack_nail`, `verify_nail` |
| `route/` | Fleet graph algorithms | `RoomGraph`, `TernaryGraph` |

## Key Internal Patterns

### Confidence Update Model

Confidence is updated multiplicatively after each execution:

```rust
// Success: confidence *= 1.005
// Failure: confidence *= 0.95
// Clamped between 0.01 and 1.00
```

This creates a natural s-curve: unconfident reflexes (<0.55) are rarely matched; high-confidence reflexes (≥0.80) get executed directly. The system naturally optimizes for what works.

### Vector Search via sqlite-vec

```
┌────────────────────────────────────────────────┐
│  SELECT id, intent, action_sql, embedding       │
│  FROM reflexes                                  │
│  WHERE embedding MATCH ?                        │
│    AND k = 5                                    │
│  ORDER BY distance                              │
└────────────────────────────────────────────────┘
```

The `sqlite-vec` extension provides virtual table support for cosine distance search. Embeddings are stored as f32 blobs (1536 bytes for 384 dimensions).

### Hash-Based Embedding Fallback

When ONNX Runtime is unavailable (no `onnx` feature), the system computes a hash-based embedding:

```rust
fn hash_embed(text: &str) -> Vec<f32> {
    // Uses a hash of each byte to distribute across 384 dimensions
    // Simpler and faster than ONNX, but less accurate
}
```

## Performance

### Benchmarks

Benchmarks are in `benchmarks/`. Run with:
```bash
cargo bench
```

Key metrics (approximate, dependent on hardware):
| Operation | With ONNX | Hash fallback |
|-----------|-----------|---------------|
| Embed text (50 chars) | ~5ms | ~10μs |
| Vector search (1000 reflexes) | ~2ms | ~2ms |
| Match + execute (cached) | ~50ms | ~50ms |
| Match + execute (novel, LLM) | ~2-5s | ~2-5s |

### Hot Paths

- **Embedding computation** — ONNX model inference is the single most expensive operation per intent. Cached embeddings for repeated intents would be a significant optimization.
- **Vector search** — sqlite-vec does brute-force cosine similarity over all stored vectors. For >10K reflexes, consider approximate nearest neighbor (ANN) indexing.
- **Command execution** — Sandbox startup (bwrap) adds ~10-20ms per invocation.

## Concurrency & Thread Safety

- `ReflexEngine` is designed for single-threaded use (one agent, one engine).
- RPC server runs on tokio, uses `Arc<ReflexEngine>` with internal mutability.
- Database operations are serialized through SQLite's own locking.
- Embedding computation is CPU-bound; consider offloading to a thread pool for concurrent access.

## Error Handling

- `EngineError` / `EngineResult` — Top-level error type, wraps all internal errors
- `DbError` / `DbResult` — Database errors (SQLite, serialization)
- `EmbedError` / `EmbedResult` — Embedding errors (model missing, inference failure)
- `VetoError` / `VetoResult` — Veto engine errors
- `PackError` / `PackResult` — Packing/unpacking errors

Errors are typed and propagate via `thiserror`. Panics are avoided in production paths.

## Testing

### Unit Tests

Run with `cargo test -p pincher-core`. Tests are co-located with modules.

### Integration Tests

- `cargo test --test '*'` for integration tests
- Reflex engine tests verify teach → match → execute roundtrip with mock sandbox
- Migration tests verify pack/unpack roundtrip with checksum validation

### Manual Testing

```bash
# End-to-end test
pincher doctor
pincher teach  # interactive
pincher do "test intent"
pincher reflexes
```

## Debugging

- Set `PINCHER_LOG_LEVEL=debug` for verbose logging
- `pincher doctor` runs comprehensive health checks
- Veto logs blocked commands with reason
- Database operations log to tracing

## Porting Guide

### Adding a New Sandbox Backend

1. Implement `Sandbox` trait in `pincher-core/src/sandbox/`
2. Add variant to `SandboxKind` enum
3. Wire selection in `ReflexEngine::new()`

### Adding a New Embedding Backend

1. Add module in `pincher-core/src/embed/`
2. Implement `Embedder` trait
3. Feature-gate with Cargo feature

### Cross-Platform Notes

| Platform | Notes |
|----------|-------|
| Linux | Primary target. bwrap + landlock both available |
| macOS | bwrap not available. Falls back to raw `Command` with warning |
| Windows | Not tested. bwrap not available. Raw `Command` with warning |

## Future Work

- WASM guest execution (wasmtime feature — wired, needs production testing)
- Landlock sandboxing (feature — wired, needs production testing)
- Reflex registry for sharing reflexes between agents
- ANN indexing for >10K scale vector search
- Multi-process execution pipelines
