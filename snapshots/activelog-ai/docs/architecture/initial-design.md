# log-origin — Design Document

> White-label, Cloudflare-native AI gateway. Clone → configure → deploy.

## 1. Tech Stack

| Layer | Choice | Why |
|-------|--------|-----|
| Language | TypeScript 5.x (strict) | Type safety, Workers-native |
| Runtime | Cloudflare Workers (workerd) | Edge-first, free tier generous |
| HTTP Framework | **Hono** | Best Workers DX, typed routes, middleware |
| Database | D1 (SQLite) | First-class Workers binding |
| Cache | KV | Session tokens, rate limits, ephemeral state |
| Blob Store | R2 | Training data exports, file uploads |
| ORM | **Drizzle ORM** (D1 adapter) | Type-safe queries, migrations, zero runtime overhead |
| Testing | Vitest + @cloudflare/vitest-pool-workers | Workers-compatible test runner |
| Deployment | Wrangler 3.x | Official CF tooling |
| Web UI | **Preact** + HTM (no build) | Lightweight, ships as static assets in Worker |
| Auth | Passphrase (HMAC-SHA256) | Phase 1; upgrade path to OAuth/OIDC |
| Linting | Biome | Fast, TS-native |

### package.json dependencies

```json
{
  "dependencies": {
    "hono": "^4.7",
    "drizzle-orm": "^0.39",
    "@logai/core": "workspace:*"
  },
  "devDependencies": {
    "typescript": "^5.7",
    "wrangler": "^4.1",
    "vitest": "^3.1",
    "@cloudflare/vitest-pool-workers": "^0.8",
    "drizzle-kit": "^0.31",
    "biome": "^1.10",
    "@cloudflare/workers-types": "^4.20250314",
    "preact": "^10.26",
    "htm": "^3.1"
  }
}
```

---

## 2. Repo Structure

```
log-origin/
├── package.json                    # monorepo root
├── tsconfig.json                   # base TS config
├── vitest.config.ts                # Vitest config (workers pool)
├── wrangler.toml                   # Worker + D1/R2/KV bindings
├── drizzle.config.ts               # Drizzle Kit config
├── biome.json                      # Linting/formatting
├── .env.example                    # Template for secrets
├── LICENSE                         # MIT
├── README.md                       # Quickstart + architecture overview
│
├── src/                            # @logai/core — the library
│   ├── index.ts                    # Public exports
│   ├── types.ts                    # All shared interfaces & types
│   ├── config.ts                   # LogOriginConfig parsing/validation
│   ├── origin.ts                   # LogOrigin class — main entry point
│   │
│   ├── routing/
│   │   ├── index.ts
│   │   ├── router.ts               # Router class — intent classification
│   │   ├── rules.ts                # Static classification rules (ported from Python)
│   │   └── types.ts                # RoutingAction, Classification, etc.
│   │
│   ├── pii/
│   │   ├── index.ts
│   │   ├── engine.ts               # PIIEngine — detect, dehydrate, rehydrate
│   │   ├── patterns.ts             # Regex patterns (ported from Python Dehydrator)
│   │   └── types.ts                # PIIEntity, PIIClassification
│   │
│   ├── providers/
│   │   ├── index.ts
│   │   ├── base.ts                 # Abstract Provider class
│   │   ├── openai-compatible.ts    # OpenAI-compatible API (DeepSeek, OpenRouter, etc.)
│   │   ├── workers-ai.ts           # Cloudflare Workers AI binding
│   │   └── types.ts                # ProviderResponse, ProviderConfig
│   │
│   ├── agents/
│   │   ├── index.ts
│   │   ├── agent.ts                # Agent class — identity, system prompt, capabilities
│   │   ├── registry.ts             # AgentRegistry — lookup by name/capability
│   │   └── types.ts                # AgentConfig, AgentCapabilities
│   │
│   ├── feedback/
│   │   ├── index.ts
│   │   ├── store.ts                # FeedbackStore — rankings, preferences
│   │   └── types.ts                # Feedback, Preference
│   │
│   ├── crypto/
│   │   ├── index.ts
│   │   └── vault.ts                # CryptoVault — encrypt/decrypt with user key
│   │
│   ├── draft/
│   │   ├── index.ts
│   │   ├── round.ts                # DraftRound — multi-provider comparison
│   │   └── types.ts                # Draft, DraftResult
│   │
│   ├── storage/
│   │   ├── index.ts
│   │   ├── sessions.ts             # Session CRUD via D1
│   │   ├── interactions.ts         # Interaction logging via D1
│   │   ├── pii-store.ts            # PII entity persistence via D1
│   │   └── preferences.ts         # User preferences via D1
│   │
│   └── middleware/
│       ├── index.ts
│       ├── auth.ts                 # Passphrase authentication
│       ├── rate-limit.ts           # KV-based rate limiting
│       └── error-handler.ts        # Consistent error responses
│
├── worker/                         # Cloudflare Worker — the deployed gateway
│   ├── index.ts                    # Hono app, bindings, routes
│   ├── routes/
│   │   ├── chat.ts                 # POST /api/chat — main chat endpoint
│   │   ├── session.ts              # GET/POST /api/sessions — session management
│   │   ├── feedback.ts             # POST /api/feedback — thumbs up/down
│   │   ├── admin.ts                # GET /api/admin/* — stats, preferences
│   │   └── tunnel.ts               # WS /api/tunnel — local instance bridge (Phase 2)
│   ├── handlers/
│   │   ├── chat-handler.ts         # Orchestrates PII → Route → Provider → Respond
│   │   └── session-handler.ts      # Session lifecycle
│   └── bindings.ts                 # CloudflareEnv type definition
│
├── web/                            # Preact UI — served as static assets
│   ├── index.html                  # Entry point
│   ├── app.tsx                     # Root component
│   ├── components/
│   │   ├── chat.tsx                # Main chat interface
│   │   ├── message.tsx             # Single message bubble
│   │   ├── session-list.tsx        # Session sidebar
│   │   ├── settings.tsx            # Preferences panel
│   │   └── login.tsx               # Passphrase entry
│   ├── state.ts                    # State management (preact-signals)
│   ├── api.ts                      # Fetch wrapper for Worker API
│   └── style.css                   # Minimal CSS (no framework)
│
├── templates/                      # Personality templates for themed forks
│   ├── default.md                  # Generic helpful assistant
│   ├── coder.md                    # Software engineering specialist
│   ├── creative.md                 # Creative writing
│   └── README.md                   # How to create custom templates
│
├── db/
│   ├── schema.ts                   # Drizzle schema definition
│   ├── migrations/                 # Generated by drizzle-kit
│   │   ├── 0001_initial.sql
│   │   └── meta/
│   │       ├── _journal.json
│   │       └── 0001_snapshot.json
│   └── seed.sql                    # Default preferences seed
│
├── tests/
│   ├── setup.ts                    # Vitest setup (mock bindings)
│   ├── routing/
│   │   ├── rules.test.ts           # Static classification rules
│   │   └── router.test.ts          # Router integration
│   ├── pii/
│   │   ├── patterns.test.ts        # PII detection patterns
│   │   ├── engine.test.ts          # Dehydrate/rehydrate cycles
│   │   └── edge-cases.test.ts      # Unicode, Chinese, Russian names
│   ├── providers/
│   │   └── openai-compatible.test.ts
│   ├── feedback/
│   │   └── store.test.ts
│   ├── storage/
│   │   └── sessions.test.ts
│   ├── worker/
│   │   ├── chat.test.ts            # E2E chat flow
│   │   └── auth.test.ts            # Auth middleware
│   └── fixtures/
│       ├── messages.ts             # Sample messages for tests
│       └── pii-samples.ts          # PII test cases
│
├── docs/
│   ├── architecture.md             # System architecture overview
│   ├── configuration.md            # All config options
│   ├── providers.md                # Setting up AI providers
│   ├── forking-guide.md            # How to create a themed fork
│   ├── api-reference.md            # HTTP API docs
│   └── embedding.md                # Using @logai/core in your own project
│
└── scripts/
    ├── setup.ts                    # Interactive first-run setup
    └── seed-db.ts                  # Seed D1 with initial data
```

---

## 3. Core TypeScript Interfaces

### `src/types.ts`

```typescript
// ─── Common ───────────────────────────────────────────

export interface LogOriginConfig {
  /** Auth passphrase (hashed with HMAC-SHA256) */
  authPassphraseHash: string;
  /** Default agent system prompt (from template) */
  systemPrompt: string;
  /** Provider configurations */
  providers: ProviderConfig[];
  /** Routing configuration */
  routing: RoutingConfig;
  /** PII configuration */
  pii: PIIConfig;
  /** User preferences */
  preferences: Record<string, string>;
}

// ─── Routing ──────────────────────────────────────────

export type RoutingAction = 'cheap' | 'escalation' | 'compare' | 'draft' | 'local' | 'manual';

export interface Classification {
  action: RoutingAction;
  confidence: number;
  reason: string;
}

export interface RoutingRule {
  name: string;
  pattern: string;
  action: RoutingAction;
  confidence: number;
}

export interface RoutingConfig {
  cheapModel: string;
  escalationModel: string;
  staticRules: RoutingRule[];
  /** Enable dynamic routing (learned from feedback) */
  dynamicEnabled: boolean;
}

// ─── PII ──────────────────────────────────────────────

export type PIIType = 'person' | 'email' | 'phone' | 'address' | 'ssn' | 'credit_card' | 'api_key' | 'passport';

export interface PIIEntity {
  entityId: string;      // e.g., "PERSON_A", "EMAIL_B"
  entityType: PIIType;
  realValue: string;
  createdAt?: string;
  lastUsed?: string;
}

export interface DehydrationResult {
  text: string;                // Dehydrated text
  entities: PIIEntity[];       // What was found
  preamble: string;            // Coherence preamble for LLM
}

export interface PIIConfig {
  enabled: boolean;
  /** Additional patterns beyond built-in defaults */
  extraPatterns?: Array<{ type: PIIType; pattern: string }>;
}

// ─── Providers ────────────────────────────────────────

export interface ProviderConfig {
  id: string;
  type: 'openai-compatible' | 'workers-ai';
  name: string;
  /** Base URL for openai-compatible */
  baseUrl?: string;
  /** API key (stored in env/secrets, not config) */
  apiKeyEnvVar?: string;
  /** Model identifier */
  model: string;
  /** Role: cheap, escalation, or both */
  roles: Array<'cheap' | 'escalation'>;
  /** Max tokens for responses */
  maxTokens?: number;
  /** Temperature */
  temperature?: number;
}

export interface ProviderMessage {
  role: 'system' | 'user' | 'assistant';
  content: string;
}

export interface ProviderResponse {
  content: string;
  model: string;
  usage: {
    promptTokens: number;
    completionTokens: number;
  };
  latencyMs: number;
}

// ─── Agents ───────────────────────────────────────────

export interface AgentConfig {
  id: string;
  name: string;
  systemPrompt: string;
  capabilities: AgentCapabilities;
}

export interface AgentCapabilities {
  routing: boolean;      // Can classify messages
  pii: boolean;          // Can detect PII
  feedback: boolean;     // Can learn from feedback
  local: boolean;        // Can run locally
  multiProvider: boolean; // Can compare providers
}

// ─── Feedback ─────────────────────────────────────────

export type FeedbackRating = 'up' | 'down';

export interface Feedback {
  interactionId: number;
  rating: FeedbackRating;
  critique?: string;
  createdAt: string;
}

// ─── Draft ────────────────────────────────────────────

export interface DraftResult {
  providerId: string;
  modelName: string;
  content: string;
  latencyMs: number;
  tokens: number;
}

// ─── Cloudflare Bindings ─────────────────────────────

export interface CloudflareEnv {
  DB: D1Database;
  KV: KVNamespace;
  R2: R2Bucket;
  AUTH_HASH: string;
  /** Provider API keys stored as secrets */
  DEEPSEEK_API_KEY?: string;
  OPENROUTER_API_KEY?: string;
  OPENAI_API_KEY?: string;
}

export type Bindings = CloudflareEnv;
```

### `src/origin.ts` — LogOrigin class

```typescript
import type { Bindings, Classification, DehydrationResult, ProviderResponse, LogOriginConfig } from './types';

export class LogOrigin {
  private config: LogOriginConfig;
  private router: Router;
  private pii: PIIEngine;
  private feedback: FeedbackStore;
  private providers: Map<string, Provider>;

  constructor(config: LogOriginConfig) { /* ... */ }

  /** Main entry: message in, response out */
  async chat(
    message: string,
    sessionId: string,
    history: ProviderMessage[],
    env: Bindings,
  ): Promise<{ response: ProviderResponse; classification: Classification; dehydration: DehydrationResult }> {
    const dehydration = this.pii.dehydrate(message, env);
    const classification = this.router.classify(message);
    const provider = this.resolveProvider(classification.action);
    const response = await provider.chat(
      dehydration.text,
      dehydration.preamble,
      history,
      env,
    );
    await this.logInteraction(sessionId, message, dehydration, classification, response, env);
    return { response, classification, dehydration };
  }

  /** Record user feedback, update routing weights */
  async recordFeedback(interactionId: number, rating: FeedbackRating, critique?: string, env: Bindings): Promise<void> { /* ... */ }
}
```

### `src/routing/router.ts`

```typescript
export class Router {
  constructor(config: RoutingConfig) { /* ... */ }

  /** Classify a message. Static rules always checked first; dynamic rules if enabled. */
  classify(message: string): Classification {
    // 1. Check command prefixes (draft, local, manual) — always win
    // 2. Check static heuristic rules
    // 3. If dynamicEnabled, check learned rules from FeedbackStore
    // Return highest-confidence match
  }

  /** Map a Classification.action to a specific provider */
  resolveProvider(action: RoutingAction): string {
    // action → provider config id
  }
}
```

### `src/pii/engine.ts`

```typescript
export class PIIEngine {
  private patterns: Map<PIIType, RegExp>;

  constructor(config?: PIIConfig) { /* ... */ }

  /** Find all PII entities in text */
  detect(text: string): Array<{ type: PIIType; value: string; start: number; end: number }> { /* ... */ }

  /** Replace PII with LOG_IDs, persist mappings to D1 */
  async dehydrate(text: string, env: Bindings): Promise<DehydrationResult> { /* ... */ }

  /** Replace LOG_IDs back with real values from D1 */
  async rehydrate(text: string, env: Bindings): Promise<string> { /* ... */ }
}
```

### `src/providers/base.ts`

```typescript
export abstract class Provider {
  abstract readonly id: string;
  abstract readonly type: string;

  /** Send a chat completion request */
  abstract chat(
    messages: ProviderMessage[],
    systemPrompt: string,
    env: Bindings,
    options?: { maxTokens?: number; temperature?: number },
  ): Promise<ProviderResponse>;

  /** Stream a chat completion (for SSE) */
  abstract chatStream?(
    messages: ProviderMessage[],
    systemPrompt: string,
    env: Bindings,
    options?: { maxTokens?: number; temperature?: number },
  ): AsyncIterable<ProviderResponse>;
}
```

### `src/providers/openai-compatible.ts`

```typescript
export class OpenAICompatibleProvider extends Provider {
  constructor(private config: ProviderConfig) { super(); }

  async chat(messages: ProviderMessage[], systemPrompt: string, env: Bindings, opts?): Promise<ProviderResponse> {
    const apiKey = env[this.config.apiKeyEnvVar!];
    const start = Date.now();
    const res = await fetch(`${this.config.baseUrl}/v1/chat/completions`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${apiKey}`,
      },
      body: JSON.stringify({
        model: this.config.model,
        messages: [{ role: 'system', content: systemPrompt }, ...messages],
        max_tokens: opts?.maxTokens ?? this.config.maxTokens ?? 4096,
        temperature: opts?.temperature ?? this.config.temperature ?? 0.7,
      }),
    });
    // parse, return ProviderResponse
  }
}
```

---

## 4. D1 Schema

### `db/schema.ts` (Drizzle)

```typescript
import { sqliteTable, text, integer, real } from 'drizzle-orm/sqlite-core';

export const sessions = sqliteTable('sessions', {
  id: text('id').primaryKey(),
  timestamp: text('timestamp').notNull(),
  summary: text('summary').notNull(),
  metadata: text('metadata').default('{}'),
});

export const messages = sqliteTable('messages', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  sessionId: text('session_id').notNull().references(() => sessions.id, { onDelete: 'cascade' }),
  role: text('role').notNull(),          // user | assistant | system
  content: text('content').notNull(),
  timestamp: text('timestamp').notNull(),
});

export const piiMap = sqliteTable('pii_map', {
  entityId: text('entity_id').primaryKey(),
  entityType: text('entity_type').notNull(),
  realValue: text('real_value').notNull(),
  createdAt: text('created_at').default("(datetime('now'))"),
  lastUsed: text('last_used').default("(datetime('now'))"),
});

export const interactions = sqliteTable('interactions', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  sessionId: text('session_id').notNull(),
  userInput: text('user_input').notNull(),
  rewrittenInput: text('rewritten_input'),
  routeAction: text('route_action').notNull(),
  routeReason: text('route_reason'),
  targetModel: text('target_model').notNull(),
  response: text('response').notNull(),
  escalationResponse: text('escalation_response'),
  responseLatencyMs: integer('response_latency_ms'),
  escalationLatencyMs: integer('escalation_latency_ms'),
  feedback: text('feedback'),              // up | down | null
  critique: text('critique'),
  createdAt: text('created_at').default("(datetime('now'))"),
});

export const userPreferences = sqliteTable('user_preferences', {
  key: text('key').primaryKey(),
  value: text('value').notNull(),
  updatedAt: text('updated_at').default("(datetime('now'))"),
});

export const routingRules = sqliteTable('routing_rules', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  name: text('name').notNull(),
  pattern: text('pattern').notNull(),
  action: text('action').notNull(),
  confidence: real('confidence').notNull(),
  source: text('source').default('static'),  // static | learned
  hitCount: integer('hit_count').default(0),
  createdAt: text('created_at').default("(datetime('now'))"),
});
```

### `migrations/0001_initial.sql`

```sql
CREATE TABLE IF NOT EXISTS sessions (
  id TEXT PRIMARY KEY,
  timestamp TEXT NOT NULL,
  summary TEXT NOT NULL,
  metadata TEXT DEFAULT '{}'
);

CREATE TABLE IF NOT EXISTS messages (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
  role TEXT NOT NULL,
  content TEXT NOT NULL,
  timestamp TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS pii_map (
  entity_id TEXT PRIMARY KEY,
  entity_type TEXT NOT NULL,
  real_value TEXT NOT NULL,
  created_at TEXT DEFAULT (datetime('now')),
  last_used TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS interactions (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  session_id TEXT NOT NULL,
  user_input TEXT NOT NULL,
  rewritten_input TEXT,
  route_action TEXT NOT NULL,
  route_reason TEXT,
  target_model TEXT NOT NULL,
  response TEXT NOT NULL,
  escalation_response TEXT,
  response_latency_ms INTEGER,
  escalation_latency_ms INTEGER,
  feedback TEXT,
  critique TEXT,
  created_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS user_preferences (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL,
  updated_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS routing_rules (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL,
  pattern TEXT NOT NULL,
  action TEXT NOT NULL,
  confidence REAL NOT NULL,
  source TEXT DEFAULT 'static',
  hit_count INTEGER DEFAULT 0,
  created_at TEXT DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_messages_session ON messages(session_id);
CREATE INDEX IF NOT EXISTS idx_pii_type ON pii_map(entity_type);
CREATE INDEX IF NOT EXISTS idx_interactions_route ON interactions(route_action);
CREATE INDEX IF NOT EXISTS idx_interactions_feedback ON interactions(feedback);
CREATE INDEX IF NOT EXISTS idx_routing_rules_source ON routing_rules(source);
```

---

## 5. Worker API

### Endpoints

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | `/api/auth` | No | Validate passphrase, return session token |
| POST | `/api/chat` | Yes | Send message, get AI response (JSON) |
| POST | `/api/chat/stream` | Yes | SSE streaming response |
| GET | `/api/sessions` | Yes | List sessions |
| GET | `/api/sessions/:id` | Yes | Get session with messages |
| DELETE | `/api/sessions/:id` | Yes | Delete session |
| POST | `/api/feedback` | Yes | Submit thumbs up/down on interaction |
| GET | `/api/admin/stats` | Yes | Usage stats (interactions, accuracy) |
| GET | `/api/admin/preferences` | Yes | Get user preferences |
| PUT | `/api/admin/preferences` | Yes | Update preferences |
| GET | `/api/admin/pii` | Yes | List PII entities |
| DELETE | `/api/admin/pii/:id` | Yes | Delete a PII entity |
| GET | `/*` | No | Serve web/ static assets |

### `worker/bindings.ts`

```typescript
export interface CloudflareEnv {
  DB: D1Database;
  KV: KVNamespace;
  R2: R2Bucket;
  AUTH_HASH: string;
  // Provider API keys (set via `wrangler secret put`)
  DEEPSEEK_API_KEY?: string;
  OPENROUTER_API_KEY?: string;
  OPENAI_API_KEY?: string;
}

export interface HonoVariables {
  sessionId: string;
  userId: string;
}
```

### `worker/index.ts` (simplified)

```typescript
import { Hono } from 'hono';
import type { CloudflareEnv, HonoVariables } from './bindings';
import { auth } from '../src/middleware/auth';
import { cors } from 'hono/cors';
import { chatRoutes } from './routes/chat';
import { sessionRoutes } from './routes/session';
import { feedbackRoutes } from './routes/feedback';
import { adminRoutes } from './routes/admin';
import { serveStatic } from './routes/static';

const app = new Hono<{ Bindings: CloudflareEnv; Variables: HonoVariables }>();

app.use('*', cors());
app.use('/api/*', auth);

app.route('/api', chatRoutes);
app.route('/api', sessionRoutes);
app.route('/api', feedbackRoutes);
app.route('/api', adminRoutes);
app.route('*', serveStatic);  // serves web/ assets

export default app;
```

---

## 6. Phase 1 Scope — MVP

### What's IN (working demo in < 2 weeks)

1. **Single provider** — OpenAI-compatible (DeepSeek). One provider, two roles (cheap/escalation can be same model initially).
2. **PII dehydration** — All regex patterns from Python Dehydrator ported. Entity persistence in D1. Rehydration of responses.
3. **Static routing** — All 14 rules from Python `routing_script.py` ported. No dynamic learning yet.
4. **Auth** — Passphrase auth with HMAC-SHA256 hash. Session tokens in KV with TTL.
5. **Chat API** — `POST /api/chat` (JSON) and `POST /api/chat/stream` (SSE).
6. **Session management** — Create, list, get, delete sessions. History sent as context.
7. **Feedback** — Thumbs up/down on responses. Stored in D1 (no routing impact yet).
8. **Web UI** — Single-page chat with Preact+HTM. Login, chat, session list, feedback buttons. No build step.
9. **Deploy** — `wrangler deploy` works. D1 migration runs. Everything on free tier.

### What's OUT (Phase 2+)

- Multi-provider comparison (draft mode)
- Dynamic routing (learning from feedback)
- Local model tunneling
- Workers AI provider
- R2 file uploads
- Semantic caching
- Training data export
- OAuth/OIDC
- Rate limiting (KV-based)
- User preferences system prompt injection
- Themed personality templates

### Phase 1 File Count

| Directory | Files |
|-----------|-------|
| `src/` | 14 |
| `worker/` | 7 |
| `web/` | 8 |
| `db/` | 3 |
| `tests/` | 8 |
| Root configs | 6 |
| **Total** | **~46 files** |

---

## 7. Migration Strategy from LOG-mcp

### Direct ports (logic → TypeScript)

| Python Source | TypeScript Target | Notes |
|--------------|-------------------|-------|
| `vault/routing_script.py::_STATIC_RULES` | `src/routing/rules.ts` | All 14 rules, exact same patterns |
| `vault/routing_script.py::classify_static()` | `Router.classify()` | Same regex matching, same priority ordering |
| `vault/routing_script.py::resolve_action()` | `Router.resolveProvider()` | Action → provider mapping |
| `vault/core.py::Dehydrator._COMPILED_PATTERNS` | `src/pii/patterns.ts` | All regex patterns (email, phone, SSN, CC, API key) |
| `vault/core.py::Dehydrator.detect_entities()` | `PIIEngine.detect()` | Including name detection, Chinese/Russian support |
| `vault/core.py::Dehydrator.dehydrate()` | `PIIEngine.dehydrate()` | Same ID generation (PERSON_A, EMAIL_B, etc.) |
| `vault/core.py::Dehydrator.build_preamble()` | `PIIEngine` preamble | Same coherence preamble text |
| `vault/core.py::RealLog._init_db()` schema | `db/schema.ts` + `migrations/` | Near-identical SQL schema |
| `vault/core.py::RealLog.add_interaction()` | `src/storage/interactions.ts` | Same columns, same logic |
| `vault/core.py::RealLog.update_feedback()` | `src/feedback/store.ts` | Same up/down + critique |
| `vault/core.py::_seed_default_preferences()` | `db/seed.sql` | Same defaults (concise, casual, no_disclaimers) |

### Test ports

| Python Test | TypeScript Test | What to port |
|------------|----------------|-------------|
| `tests/test_routing_script.py` | `tests/routing/rules.test.ts` | All classification test cases |
| PII detection tests (in core.py tests) | `tests/pii/patterns.test.ts` | Dehydrate/rehydrate cycles |

### NOT ported

- `vault/routing_optimizer.py` — Phase 2 (dynamic routing)
- `vault/model_comparator.py` — Phase 2 (draft mode)
- `vault/local_inference.py` — Phase 2 (tunnel)
- `vault/semantic_cache.py` — Phase 3
- `vault/training_pipeline.py` — Phase 3
- `vault/model_*.py` — All model management (not relevant to Workers)
- `gateway/` — Rewritten from scratch for Hono

---

## 8. Key Design Decisions

### Why Preact + HTM (no build step)?
The web UI ships as static assets served by the Worker. Preact is 4KB. HTM lets us write JSX-like templates as tagged template literals — no transpile step. This keeps the build pipeline simple: `tsc` for the library/worker, copy `web/` as-is.

### Why Drizzle over raw SQL?
Drizzle generates types from your schema. When you do `db.select().from(sessions)`, you get `Session[]` — not `any[]`. For a codebase that's meant to be forked and extended by strangers, type safety in data access is worth the dependency.

### Why Hono over itty-router?
Hono has better middleware support (auth, CORS, rate limiting), typed bindings for Cloudflare, and a larger ecosystem. itty-router is lighter but Hono isn't significantly heavier on Workers.

### Why passphrase auth for Phase 1?
Self-hosted gateway. Users set a passphrase during `scripts/setup.ts`. We hash it with HMAC-SHA256 (using a random server key stored in KV). Session tokens are random UUIDs stored in KV with 24h TTL. Simple, secure enough for personal use, upgradable.

### Entity ID generation (ported from Python)
Same algorithm as `RealLog.next_log_id()`: try single letters A-Z first, then AA-ZZ, then timestamp fallback. Maintains compatibility with the Python codebase's entity IDs if someone migrates.

---

## 9. Build & Deploy

```bash
# Install
npm install

# Dev (local D1 + Worker)
npx wrangler dev

# Run tests
npx vitest

# Type check
npx tsc --noEmit

# Generate D1 migration from schema changes
npx drizzle-kit generate

# Deploy
npx wrangler deploy
```

### `wrangler.toml`

```toml
name = "log-origin"
main = "worker/index.ts"
compatibility_date = "2025-01-01"

[[d1_databases]]
binding = "DB"
database_name = "log-origin-db"
database_id = "<generated-on-create>"

[[kv_namespaces]]
binding = "KV"
id = "<generated-on-create>"

[[r2_buckets]]
binding = "R2"
bucket_name = "log-origin-storage"

[vars]
# AUTH_HASH set via: npx wrangler secret put AUTH_HASH

[site]
bucket = "./web"

# Static assets (web/) are served by the Worker, not CF Pages
# This keeps everything in one deploy target
```

---

## 10. Implementation Order (Phase 1)

1. **Day 1-2**: Project scaffolding — `package.json`, `tsconfig`, `wrangler.toml`, `db/schema.ts`, Drizzle setup, first migration
2. **Day 2-3**: `src/types.ts` — All interfaces. `src/pii/patterns.ts` + `engine.ts` — Port PII from Python
3. **Day 3-4**: `src/routing/rules.ts` + `router.ts` — Port routing from Python
4. **Day 4-5**: `src/providers/base.ts` + `openai-compatible.ts` — Single provider
5. **Day 5-6**: `worker/index.ts` + routes — Chat endpoint, auth middleware
6. **Day 6-7**: `web/` — Preact UI (login, chat, feedback)
7. **Day 7-8**: Tests — Routing, PII, chat E2E
8. **Day 8-9**: `src/storage/` — Sessions, interactions, PII store
9. **Day 9-10**: Polish — Error handling, README, deploy to Cloudflare

---

*This document is the blueprint. Start building from Section 4 (schema), then Section 3 (interfaces), then Section 2 (repo), then Section 7 (port Python logic).*
