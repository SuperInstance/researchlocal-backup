# SCHEMAS.md — Hermes Construct Kernel Type Definitions

> **Source**: Distilled from [PLATO_BUILD_PLAN.md](./PLATO_BUILD_PLAN.md) — the 7-phase refactoring of hermes-construct into a PLATO-native tile-operating system.
>
> This file defines the **JSON schemas, type hierarchy, sandboxing model, progressive bootstrap flow, and API allowance system** that the implementation follows. All types here are the canonical reference; code generators should read from this file.

---

## 1. Core Schemas (JSON)

### Tile Schema

A **tile** is the fundamental unit of work in the system. Every operation, thought, interaction, automation, and delegation is a tile. Nothing happens outside a tile.

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "Tile",
  "description": "The fundamental unit of work. Every operation is a tile.",
  "type": "object",
  "required": [
    "tile_id", "room_id", "tile_type",
    "parent_tile", "children",
    "created_tick", "updated_tick",
    "status", "content",
    "deadband"
  ],
  "properties": {
    "tile_id": {
      "type": "string",
      "format": "uuid",
      "description": "Globally unique tile identifier"
    },
    "room_id": {
      "type": "string",
      "description": "Which room owns this tile"
    },
    "tile_type": {
      "type": "string",
      "enum": [
        "observation",
        "action",
        "thought",
        "delegation",
        "escalation",
        "artifact"
      ],
      "description": "Category of work this tile represents"
    },
    "parent_tile": {
      "type": ["string", "null"],
      "format": "uuid",
      "description": "Parent tile ID for decomposed tasks. null = root tile."
    },
    "children": {
      "type": "array",
      "items": { "type": "string", "format": "uuid" },
      "description": "Child tile IDs spawned from this tile",
      "uniqueItems": true
    },
    "created_tick": {
      "type": "integer",
      "minimum": 0,
      "description": "Monotonic tick counter at creation time"
    },
    "updated_tick": {
      "type": "integer",
      "minimum": 0,
      "description": "Monotonic tick counter at last modification"
    },
    "status": {
      "type": "string",
      "enum": [
        "active",
        "complete",
        "deadband",
        "escalated",
        "archived"
      ],
      "description": "Current lifecycle state of the tile"
    },
    "content": {
      "type": "object",
      "description": "Tile payload. Schema depends on tile_type.",
      "properties": {
        "input": { "type": "string" },
        "output": { "type": "string" },
        "tool_calls": {
          "type": "array",
          "items": { "type": "object" }
        },
        "error": { "type": "object" }
      }
    },
    "deadband": {
      "type": "object",
      "required": ["lower", "upper", "current", "trend"],
      "properties": {
        "lower": { "type": "number", "description": "Lower deadband boundary" },
        "upper": { "type": "number", "description": "Upper deadband boundary" },
        "current": { "type": "number", "description": "Current monitored value" },
        "trend": {
          "type": "string",
          "enum": ["stable", "drifting", "oscillating", "diverging"],
          "description": "Direction the monitored value is moving"
        }
      },
      "description": "Deadband circuit snapshot for automation-capable tiles"
    },
    "ensign": {
      "type": ["string", "null"],
      "description": "Ensign ID that handled this tile, if any"
    },
    "model_used": {
      "type": ["string", "null"],
      "description": "Model identifier that generated this tile's output"
    },
    "tokens_used": {
      "type": "integer",
      "minimum": 0,
      "description": "Total tokens consumed (input + output)"
    },
    "conservation_delta": {
      "type": "number",
      "description": "Energy budget delta: positive = deposit, negative = withdrawal"
    },
    "quality_score": {
      "type": "number",
      "minimum": 0.0,
      "maximum": 1.0,
      "description": "0.0-1.0 quality rating from feedback or heuristics"
    },
    "metadata": {
      "type": "object",
      "description": "Extensible metadata (tags, flags, provenance links)"
    }
  }
}
```

**Status state machine:**

```
        ┌───┐
        │   │  created with parent → ACTIVE
        └───┘
          │
          ▼
      ┌────────┐
      │ ACTIVE │ ── child completes → check children for completion
      └───┬────┘
          │
     ┌────┼──────────┐
     ▼    ▼          ▼
 ┌────────┐ ┌──────────┐ ┌───────────┐
 │COMPLETE│ │ DEADBAND │ │ ESCALATED │
 └────────┘ └──────────┘ └───────────┘
     │
     ▼
 ┌─────────┐    (after TTL / archival policy)
 │ARCHIVED │
 └─────────┘
```

---

### Room Schema

A **room** is a persistent operational context maintained by an Ensign. The Room IS the agent's context — it holds gravity, state, model parameters, and the baton for specialist handoff.

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "Room",
  "description": "Persistent operational context maintained by an Ensign. The room IS the agent's context.",
  "type": "object",
  "required": [
    "room_id", "room_type", "gravity",
    "model_params", "ensign",
    "deadband_tolerance", "complexity",
    "tile_count"
  ],
  "properties": {
    "room_id": {
      "type": "string",
      "description": "Unique room identifier (e.g. 'navigation', 'engineering')"
    },
    "room_type": {
      "type": "string",
      "enum": [
        "navigation",
        "engineering",
        "science",
        "security",
        "social",
        "custom"
      ],
      "description": "Category of room determining its operational domain"
    },
    "gravity": {
      "type": "object",
      "required": ["value", "confidence", "sample_count"],
      "properties": {
        "value": {
          "type": "number",
          "minimum": -1.0,
          "maximum": 1.0,
          "description": "JEPA gravity scalar: negative = precise, positive = creative"
        },
        "confidence": {
          "type": "number",
          "minimum": 0.0,
          "maximum": 1.0,
          "description": "Confidence in the current gravity value (low during cold start)"
        },
        "sample_count": {
          "type": "integer",
          "minimum": 0,
          "description": "Number of tiles used to determine this gravity"
        }
      },
      "description": "JEPA gravity field state for this room"
    },
    "model_params": {
      "type": "object",
      "required": ["temperature", "max_tokens", "prompt_style", "top_p"],
      "properties": {
        "temperature": {
          "type": "number",
          "minimum": 0.0,
          "maximum": 2.0,
          "description": "Model temperature derived from gravity mapping"
        },
        "max_tokens": {
          "type": "integer",
          "minimum": 100,
          "maximum": 8000,
          "description": "Maximum output tokens for this room"
        },
        "prompt_style": {
          "type": "string",
          "enum": ["precise", "balanced", "creative", "narrative"],
          "description": "System prompt style derived from gravity"
        },
        "top_p": {
          "type": "number",
          "minimum": 0.0,
          "maximum": 1.0,
          "description": "Nucleus sampling parameter"
        }
      },
      "description": "Algorithmic model parameters — output of gravity mapping"
    },
    "ensign": {
      "type": "object",
      "required": ["id", "model", "status", "alert"],
      "properties": {
        "id": { "type": "string", "description": "Ensign identifier" },
        "model": { "type": "string", "description": "Model identifier for the ensign" },
        "status": {
          "type": "string",
          "enum": ["dormant", "orienting", "green_alert", "yellow_alert", "red_alert", "standing_down", "escalated"]
        },
        "alert": {
          "type": "string",
          "enum": ["green", "yellow", "red"],
          "description": "Current alert color"
        }
      },
      "description": "The Ensign currently assigned to this room"
    },
    "deadband_tolerance": {
      "type": "number",
      "minimum": 0.0,
      "maximum": 1.0,
      "description": "Default automation deadband tolerance for this room"
    },
    "complexity": {
      "type": "string",
      "enum": ["low", "medium", "high", "variable"],
      "description": "Estimated task complexity for this room"
    },
    "tile_count": {
      "type": "integer",
      "minimum": 0,
      "description": "Total tiles created in this room"
    },
    "correlations": {
      "type": "array",
      "items": { "type": "string" },
      "description": "Room IDs this room has Penrose correlations with",
      "uniqueItems": true
    },
    "wiki": {
      "type": "object",
      "properties": {
        "pages": {
          "type": "array",
          "items": { "type": "object" },
          "description": "Wiki knowledge pages for this room"
        },
        "help_files": {
          "type": "array",
          "items": { "type": "object" },
          "description": "Help/reference files"
        },
        "controls": {
          "type": "array",
          "items": { "type": "object" },
          "description": "Control schemas and circuit definitions"
        }
      }
    },
    "baton": {
      "type": "object",
      "properties": {
        "from": { "type": "string" },
        "summary": { "type": "string" },
        "state": { "type": "object" },
        "warnings": {
          "type": "array",
          "items": { "type": "string" }
        }
      },
      "description": "Baton transfer state — present when a specialist is handing off"
    },
    "level": {
      "type": "integer",
      "minimum": 1,
      "maximum": 5,
      "description": "Progressive generation level (1 = all large model, 5 = self-operating)"
    },
    "progressive": {
      "type": "object",
      "properties": {
        "success_rate": { "type": "number", "minimum": 0.0, "maximum": 1.0 },
        "tiles_evaluated": { "type": "integer", "minimum": 0 },
        "promoted_at": { "type": "string", "format": "date-time" },
        "demoted_at": { "type": "string", "format": "date-time" },
        "escapes": {
          "type": "array",
          "items": { "type": "string" },
          "description": "Phone-a-Friend call reasons for this room"
        }
      },
      "description": "Progressive generation tracking state"
    }
  }
}
```

**Gravity → Model Params Mapping:**

| Gravity Range | Temperature | Prompt Style | Max Tokens | Top p | Frequency Penalty | Presence Penalty |
|---|---|---|---|---|---|---|
| -1.0 to -0.5 | 0.3 | precise | 500 | 0.9 | 0.3 | 0.1 |
| -0.5 to 0.0 | 0.5 | balanced | 1000 | 0.95 | 0.1 | 0.1 |
| 0.0 to 0.5 | 0.7 | creative | 2000 | 0.95 | 0.0 | 0.2 |
| 0.5 to 1.0 | 0.9 | narrative | 4000 | 0.95 | 0.0 | 0.3 |

---

### Ensign Schema

An **Ensign** is a small model (Seed-mini, GLM-flash) that maintains a room. The DJ metaphor: the ensign "reads the room," manages the energy, and plays the right set at the right alert level.

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "Ensign",
  "description": "Small-model agent that maintains a room. The DJ.",
  "type": "object",
  "required": [
    "ensign_id", "model_type",
    "status", "alert_level",
    "orientation", "story",
    "energy_budget", "deadband_monitor"
  ],
  "properties": {
    "ensign_id": {
      "type": "string",
      "description": "Unique ensign identifier (e.g. 'seed-nav-01', 'glm-sci-01')"
    },
    "model_type": {
      "type": "string",
      "enum": [
        "local_tiny",
        "local_vision",
        "local_audio",
        "local_jepa",
        "remote_light",
        "remote_vision"
      ],
      "description": "Model category indicating capability tier and locality"
    },
    "provider": {
      "type": "string",
      "description": "Provider ID (e.g. 'deepinfra', 'z.ai', 'local')"
    },
    "room": {
      "type": ["string", "null"],
      "description": "Room this ensign is assigned to. null when dormant."
    },
    "status": {
      "type": "string",
      "enum": [
        "dormant",
        "waking",
        "orienting",
        "yellow_alert",
        "red_alert",
        "standing_down",
        "escalated"
      ],
      "description": "Current lifecycle state"
    },
    "alert_level": {
      "type": "string",
      "enum": ["green", "yellow", "red"],
      "description": "Current alert color"
    },
    "orientation": {
      "type": "object",
      "required": ["room_state", "last_snapshot_ts", "affordances"],
      "properties": {
        "room_state": {
          "type": "object",
          "description": "Last known room state (JSON blob)"
        },
        "last_snapshot_ts": {
          "type": "string",
          "format": "date-time",
          "description": "When orientation was last refreshed"
        },
        "affordances": {
          "type": "array",
          "items": { "type": "object" },
          "description": "Available actions/inputs the ensign can take in this room"
        },
        "context_budget": {
          "type": "integer",
          "description": "Max context tokens for orientation (smaller models have tighter budgets)"
        }
      },
      "description": "Room context snapshot — what the ensign knows about its room"
    },
    "story": {
      "type": "object",
      "required": ["narrative", "events"],
      "properties": {
        "narrative": {
          "type": "string",
          "description": "Running narrative of what this ensign has experienced"
        },
        "events": {
          "type": "array",
          "items": {
            "type": "object",
            "properties": {
              "tick": { "type": "integer" },
              "event_type": { "type": "string" },
              "summary": { "type": "string" },
              "tile_id": { "type": "string" }
            }
          },
          "description": "Recent event log for the ensign's experience"
        },
        "max_events": {
          "type": "integer",
          "default": 50,
          "description": "Max events retained in story (FIFO)"
        }
      },
      "description": "Ensign's autobiographical memory"
    },
    "energy_budget": {
      "type": "number",
      "description": "Current energy budget remaining for this ensign's operations"
    },
    "deadband_monitor": {
      "type": "object",
      "properties": {
        "circuits": {
          "type": "array",
          "items": {
            "type": "object",
            "properties": {
              "name": { "type": "string" },
              "monitored": { "type": "string" },
              "setpoint": { "type": "number" },
              "tolerance": { "type": "number" },
              "check_interval_ticks": { "type": "integer" },
              "last_value": { "type": "number" },
              "is_breached": { "type": "boolean" }
            }
          }
        },
        "auto_remedy": {
          "type": "boolean",
          "description": "Whether this ensign can auto-apply remedies within deadband"
        }
      }
    },
    "call_reason": {
      "type": ["string", "null"],
      "description": "The OnCallReason if this ensign was woken specifically"
    },
    "metrics": {
      "type": "object",
      "properties": {
        "tiles_completed": { "type": "integer" },
        "tiles_failed": { "type": "integer" },
        "tokens_consumed": { "type": "integer" },
        "energy_consumed": { "type": "number" },
        "last_wake_ts": { "type": "string", "format": "date-time" },
        "last_sleep_ts": { "type": "string", "format": "date-time" }
      }
    }
  }
}
```

**Ensign Lifecycle (The DJ Metaphor):**

```
DORMANT ──→ WAKING ──→ ORIENTING ──→ GREEN_ALERT ──→ YELLOW_ALERT ←──→ RED_ALERT
  ↑                                                        │               │
  └──────────────────── STANDING_DOWN ←─────────────────────┘               │
                                                                             │
                                                      ESCALATED (to Hermes/Opus)
```

| State | Meaning | DJ Analogy |
|-------|---------|------------|
| DORMANT | Exists in config, not loaded | DJ is at home, gear is off |
| WAKING | Being loaded into memory | DJ is on the way to the club |
| ORIENTING | Reading room state, building context | DJ is walking the room, checking the vibe |
| GREEN_ALERT | Monitoring, fine-tuning orientation | DJ is in the booth, headphones on, setting up the next track |
| YELLOW_ALERT | Active, handling interactions | DJ is on the decks, mixing |
| RED_ALERT | Emergency, all hands | Power outage — DJ is on backup generator |
| STANDING_DOWN | Deactivating, saving orientation | DJ is packing up, last track playing |
| ESCALATED | Deferred to Opus/Hermes | DJ called the headliner to save the night |

---

### Shell Schema (THE KERNEL)

A **Shell** is the top-level isolation unit. It owns a set of rooms, ensigns, APIs, and ports. Hermes is the primary Shell. ZeroClaw and CUDAclaw instances get their own child Shells for sandboxing.

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "Shell",
  "description": "Top-level isolation unit. Owns rooms, ensigns, APIs, and ports. The Kernel.",
  "type": "object",
  "required": [
    "shell_id", "shell_type",
    "universe_path",
    "apis", "porting",
    "rooms", "ensigns",
    "correlations",
    "conservation_budget",
    "autonomy_level"
  ],
  "properties": {
    "shell_id": {
      "type": "string",
      "description": "Unique shell identifier"
    },
    "shell_type": {
      "type": "string",
      "enum": ["hermes", "zeroclaw", "cudaclaw", "ensign", "custom"],
      "description": "Type of shell determining capabilities and constraints"
    },
    "universe_path": {
      "type": "string",
      "description": "Filesystem path for this shell's writable universe (sandbox root)"
    },
    "apis": {
      "type": "array",
      "items": { "type": "string" },
      "description": "API identifiers this shell has access to (via allowances)"
    },
    "porting": {
      "type": "object",
      "required": ["outbound", "inbound"],
      "properties": {
        "outbound": {
          "type": "array",
          "items": { "$ref": "#/$defs/Port" },
          "description": "Ports this shell talks out on"
        },
        "inbound": {
          "type": "array",
          "items": { "$ref": "#/$defs/Port" },
          "description": "Ports this shell listens on"
        }
      },
      "description": "Connection ports for inter-shell communication"
    },
    "rooms": {
      "type": "object",
      "additionalProperties": { "$ref": "#/$defs/Room" },
      "description": "Map of room_id → Room owned by this shell",
      "minProperties": 0
    },
    "ensigns": {
      "type": "object",
      "additionalProperties": { "$ref": "#/$defs/Ensign" },
      "description": "Map of ensign_id → Ensign deployed in this shell",
      "minProperties": 0
    },
    "correlations": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "room_a": { "type": "string" },
          "room_b": { "type": "string" },
          "proximity": { "type": "number", "minimum": 0.0, "maximum": 1.0 },
          "transfers": {
            "type": "array",
            "items": {
              "type": "object",
              "properties": {
                "what": { "type": "string" },
                "success": { "type": "boolean" },
                "timestamp": { "type": "string", "format": "date-time" }
              }
            }
          }
        }
      },
      "description": "Penrose correlation records between rooms in this shell"
    },
    "conservation_budget": {
      "type": "number",
      "description": "Total energy budget remaining for this shell. Conservation: deposits - withdrawals = budget after every tick."
    },
    "autonomy_level": {
      "type": "integer",
      "minimum": 1,
      "maximum": 5,
      "description": "1 = all Opus, 5 = self-operating"
    },
    "parent_shell": {
      "type": ["string", "null"],
      "description": "Parent shell ID. null for root Hermes shell."
    },
    "child_shells": {
      "type": "array",
      "items": { "type": "string" },
      "description": "Child shell IDs spawned by this shell for sandboxing",
      "uniqueItems": true
    },
    "config": {
      "type": "object",
      "properties": {
        "max_rooms": { "type": "integer", "default": 10 },
        "max_ensigns": { "type": "integer", "default": 10 },
        "max_child_shells": { "type": "integer", "default": 5 },
        "allow_override": { "type": "boolean", "default": true },
        "allow_self_automation": { "type": "boolean", "default": true },
        "auto_archival_ticks": {
          "type": "integer",
          "description": "Ticks after which completed tiles are auto-archived"
        }
      }
    },
    "metadata": {
      "type": "object",
      "description": "Extensible shell metadata"
    }
  },
  "$defs": {
    "Port": {
      "type": "object",
      "required": ["port_id", "direction", "protocol", "target", "permissions", "enabled"],
      "properties": {
        "port_id": { "type": "string" },
        "direction": {
          "type": "string",
          "enum": ["inbound", "outbound", "bidirectional"]
        },
        "protocol": {
          "type": "string",
          "enum": ["telegram", "web", "websocket", "http", "mqtt", "serial", "gpio", "custom"]
        },
        "target": { "type": "string", "description": "Target endpoint (URL, path, device, etc.)" },
        "permissions": {
          "type": "array",
          "items": { "type": "string" }
        },
        "deadband": {
          "type": "object",
          "properties": {
            "lower": { "type": "number" },
            "upper": { "type": "number" },
            "trend": { "type": "string", "enum": ["stable", "drifting", "oscillating", "diverging"] }
          }
        },
        "enabled": { "type": "boolean" }
      }
    },
    "Room": { "$ref": "room-schema" },
    "Ensign": { "$ref": "ensign-schema" }
  }
}
```

---

### Port Schema (Connections Between Shells / Rooms)

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "Port",
  "description": "A connection point between shells and/or rooms. The communication primitive.",
  "type": "object",
  "required": [
    "port_id", "direction", "protocol",
    "target", "permissions", "enabled"
  ],
  "properties": {
    "port_id": {
      "type": "string",
      "description": "Unique port identifier"
    },
    "direction": {
      "type": "string",
      "enum": ["inbound", "outbound", "bidirectional"],
      "description": "Communication direction relative to the owning shell"
    },
    "protocol": {
      "type": "string",
      "enum": ["telegram", "web", "websocket", "http", "mqtt", "serial", "gpio", "custom"],
      "description": "Transport protocol for this port"
    },
    "target": {
      "type": "string",
      "description": "Target endpoint (URL, path, device path, channel ID, etc.)"
    },
    "permissions": {
      "type": "array",
      "items": { "type": "string" },
      "description": "Permission scopes granted through this port (e.g. 'read:sensors', 'write:media')",
      "uniqueItems": true
    },
    "deadband": {
      "type": "object",
      "description": "Optional deadband circuit for port-level automation",
      "properties": {
        "lower": { "type": "number" },
        "upper": { "type": "number" },
        "current": { "type": "number" },
        "trend": { "type": "string", "enum": ["stable", "drifting", "oscillating", "diverging"] }
      }
    },
    "enabled": {
      "type": "boolean",
      "description": "Whether this port is currently active"
    },
    "rate_limit": {
      "type": "integer",
      "minimum": 1,
      "description": "Max messages per minute through this port"
    },
    "backpressure_ms": {
      "type": "integer",
      "description": "Port-level backpressure in milliseconds"
    },
    "metadata": {
      "type": "object",
      "description": "Extensible port metadata"
    }
  }
}
```

---

### Deadband Circuit Schema

A **DeadbandCircuit** is self-contained automation logic that runs within a tolerance band. It monitors a quantity, checks if it's drifted outside bounds, and triggers a remediation action.

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "DeadbandCircuit",
  "description": "Self-contained automation circuit that monitors a quantity within a tolerance band.",
  "type": "object",
  "required": [
    "circuit_id", "name", "room_id",
    "monitored_quantity", "setpoint", "tolerance",
    "action", "ensign", "check_interval",
    "automation_level"
  ],
  "properties": {
    "circuit_id": { "type": "string" },
    "name": { "type": "string" },
    "room_id": { "type": "string" },
    "monitored_quantity": { "type": "string", "description": "What to watch (e.g. 'sensor.drift', 'error.rate', 'queue.depth')" },
    "setpoint": { "type": "number", "description": "Target value" },
    "tolerance": { "type": "number", "description": "Deadband width around setpoint" },
    "action": { "type": "string", "description": "Action to take when out of deadband (e.g. 'recalibrate', 'notify', 'restart')" },
    "ensign": { "type": "string", "description": "Ensign ID that executes the action" },
    "check_interval": { "type": "integer", "description": "Tick interval between checks" },
    "automation_level": {
      "type": "integer",
      "minimum": 1,
      "maximum": 4,
      "description": "1=alert only, 2=suggest fix, 3=auto-fix, 4=auto-fix+verify"
    },
    "last_checked": { "type": ["integer", "null"], "description": "Tick of last check" },
    "last_value": { "type": ["number", "null"], "description": "Last monitored value" },
    "consecutive_breaches": { "type": "integer", "default": 0 },
    "is_breached": { "type": "boolean", "default": false },
    "created_tick": { "type": "integer" },
    "updated_tick": { "type": "integer" }
  }
}
```

---

## 2. Type Hierarchy

### Full Type Tree

```
Shell (Hermes)
├── parent_shell: null
├── child_shells: [ZeroClaw-1, Ensign-...]
│
├── Room (Navigation)
│   ├── gravity: JepaGravity { value, confidence, samples }
│   ├── model_params: { temperature, prompt_style, max_tokens, top_p }
│   ├── ensign: Ensign { id, model, status, alert }
│   ├── deadband_tolerance: f64
│   ├── level: u8 (1-5)
│   ├── correlations: [Room (Engineering), Room (Science)]
│   ├── baton: { from, summary, state, warnings }
│   ├── wiki: { pages, help_files, controls }
│   └── progressive: { success_rate, tiles_evaluated, ... }
│
├── Room (Engineering)
│   ├── gravity: ...
│   ├── model_params: ...
│   ├── ensign: Ensign { ... }
│   ├── deadband_tolerance: ...
│   └── ...
│
├── Room (Science)
├── Room (Security)
├── Room (Social)
│
├── Ensign (seed-nav-01) ──→ assigned to Room(Navigation)
│   ├── model_type: remote_light
│   ├── orientation: { room_state, affordances, ... }
│   ├── story: { narrative, events }
│   ├── energy_budget: f64
│   ├── deadband_monitor: { circuits, auto_remedy }
│   └── status: yellow_alert
│
├── Ensign (glm-sci-01) ──→ assigned to Room(Science)
├── Ensign (seed-sec-01) ──→ assigned to Room(Security)
│
├── Tile (root)
│   ├── tile_type: observation | action | thought | delegation | escalation | artifact
│   ├── parent_tile: null | uuid
│   ├── children: [uuid, ...]
│   ├── deadband: { lower, upper, current, trend }
│   ├── status: active | complete | deadband | escalated | archived
│   └── conservation_delta: f64
│
├── Port (outbound to Telegram)
├── Port (inbound HTTP)
├── Port (outbound to MQTT)
│
├── DeadbandCircuit (monitor sensor-drift in Eng room)
├── DeadbandCircuit (monitor error-rate in Nav room)
│
├── Allowance (openai.com, $5/day, 100 calls/min)
└── Allowance (deepinfra.com, $2/day, 50 calls/min)
```

### Inheritance / Composition Rules

```
Shell 1──N Room         # Shell owns multiple rooms
Shell 1──N Ensign       # Shell deploys multiple ensigns
Room 1──1 Ensign        # Room has one assigned ensign
Room 1──N Tile          # Room owns many tiles
Tile 1──N Tile          # Tile can have child tiles (parent/children)
Shell 1──N Port         # Shell has many ports (inbound/outbound)
Port 0──1 Deadband      # Port may have a deadband circuit
Room 0──N DeadbandCircuit # Room may have automation circuits
Shell 0──N Allowance    # Shell has API usage allowances
Shell 0──N Shell        # Shell may have child shells (sandboxing)
Room N──N Room          # Correlations between rooms (many-to-many via Penrose)
```

### Progressive Level Progression

```
Level 1 ──→ Level 2 ──→ Level 3 ──→ Level 4 ──→ Level 5
  │            │            │            │            │
  │ All Opus   │ Ensigns    │ Ensigns    │ Ensigns    │ Self-
  │ (large     │ observe,   │ handle     │ autonomous │ operating
  │ model)     │ don't act  │ routine    │ (rare      │ (override
  │            │            │ (Opus      │ escalation)│ only)
  │            │            │ reviews)   │            │
  └────────────┴────────────┴────────────┴────────────┘
     Promotion triggers:
     • Level 1→2: After 20 tiles, ensigns built
     • Level 2→3: 85% ensign success rate over 50 tiles
     • Level 3→4: 92% success rate over 100 tiles
     • Level 4→5: 95% success rate over 500 tiles
```

### Mandelbrot Zoom (Irreducible Complexity)

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "MandelbrotZoom",
  "description": "Tracks irreducible complexity for a task type in a room. Like zooming into the Mandelbrot set — at some point you hit irreducible detail.",
  "type": "object",
  "required": ["room_id", "task_pattern", "min_tile_size", "attempts", "is_irreducible"],
  "properties": {
    "room_id": { "type": "string" },
    "task_pattern": { "type": "string", "description": "Hash/pattern of the task type" },
    "min_tile_size": { "type": "integer", "description": "Minimum tokens needed for this task in this room" },
    "attempts": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "tile_size": { "type": "integer" },
          "success": { "type": "boolean" },
          "tile_id": { "type": "string" }
        }
      }
    },
    "is_irreducible": { "type": "boolean", "description": "True if 3+ consecutive failures at the same tile size with different approaches" },
    "historical_min": { "type": "integer", "description": "Smallest tile that ever succeeded" },
    "gravity_at_last_attempt": { "type": "number" },
    "updated_tick": { "type": "integer" }
  }
}
---

## 3. Sandboxing Model

The sandboxing model ensures that child shells (ZeroClaw, CUDAclaw, Ensign instances) cannot access resources beyond their explicitly granted universe. This is enforced at the Shell level, not at the agent prompt level.

### Hard Isolation Rules

1. **Filesystem isolation**: Each Shell gets its own `universe_path` (a writable directory). The Shell CANNOT access files outside this path. The parent Shell's filesystem is invisible to child Shells.

2. **API isolation**: A Shell only sees APIs listed in its `apis` array. Each API is gated by an Allowance record. Shell cannot discover or call an API it doesn't have an explicit Allowance for.

3. **Port isolation**: A Shell only sees its own `porting.outbound` and `porting.inbound`. It cannot see parent or sibling Shells' ports.

4. **Shell visibility (asymmetric)**:
   - Hermes CAN see all child Shells and their metadata (club manager perspective)
   - Hermes CANNOT read child Shell tile/room state without explicit port permission
   - A child Shell CANNOT see its parent Shell
   - A child Shell CANNOT see its sibling Shells

5. **Conservation budget isolation**: Each Shell has its own `conservation_budget`. Budget is not shared between parent and child Shells. Parent allocates budget to child at spawn time.

### Example Sandboxes

| Shell | universe_path | apis | rooms | Can access parent? | Can access siblings? |
|---|---|---|---|---|---|
| Hermes | `/home/hermes/universe/` | all | nav, eng, sci, soc, sec | N/A (root) | N/A |
| ZeroClaw-sensors | `/home/hermes/sandbox/sensors/` | `["sensor_api"]` | navigation | No | No |
| ZeroClaw-media | `/home/hermes/sandbox/media/` | `["media_api"]` | social | No | No |
| CUDAclaw-dl | `/home/hermes/sandbox/dl/` | `["huggingface_api"]` | science | No | No |

### Sandbox Enforcement Points

- **Bootstrap**: When spawning a child Shell, the parent creates a new `universe_path`, assigns APIs via Allowance records, and provisions ports.
- **Runtime**: Every API call and filesystem operation is intercepted by the Shell's routing layer. Operations outside the Shell's scope are rejected with a permission error.
- **Conservation**: Energy accounting per-Shell. A child Shell cannot spend energy from the parent's budget.
- **Escalation**: The child ESCALATES to the parent through a port, not by directly accessing the parent's state. This creates an audit trail.

---

## 4. Progressive Bootstrap

The system starts empty and boots progressively, with each step logged as tiles. This ensures full auditability and allows rollback at any stage.

### Bootstrap Sequence

```
TICK 0:  Fresh hermes-construct clone
         └── Empty Shell
             ├── shell_type: "hermes"
             ├── rooms: {}
             ├── ensigns: {}
             ├── apis: []
             └── conservation_budget: 0

TICK 1-5:  Shell initialization
         ├── Set shell_id, universe_path
         ├── Allocate conservation_budget from environment
         ├── Load API keys from .env -> create Allowances
         ├── Set autonomy_level: 1
         └── Create initial config (max_rooms, max_ensigns, etc.)
         -> Tile(s): shell_initialized, allowances_loaded

TICK 6-10:  Room creation
         ├── Create Navigation   room (gravity: -0.3, complexity: low)
         ├── Create Engineering  room (gravity: -0.6, complexity: high)
         ├── Create Science      room (gravity: 0.0, complexity: medium)
         ├── Create Security     room (gravity: -0.8, complexity: variable)
         └── Create Social       room (gravity: 0.5, complexity: low)
         -> Tile(s): room_created (one per room, with initial gravity)

TICK 11-20:  Ensign deployment
         ├── Deploy seed-mini -> Navigation   (status: dormant)
         ├── Deploy seed-mini -> Engineering  (status: dormant)
         ├── Deploy glm-flash  -> Science      (status: dormant)
         ├── Deploy glm-flash  -> Social       (status: dormant)
         └── Deploy seed-mini -> Security     (status: dormant)
         -> Tile(s): ensign_deployed (one per ensign)

TICK 21-30:  API connection / Port setup
         ├── Connect port: outbound -> Telegram
         ├── Connect port: inbound  -> HTTP (CLI)
         ├── Configure deadband on port(s)
         └── Test each port (ping/pong)
         -> Tile(s): port_connected, port_test_passed

TICK 31-40:  Deadband configuration
         ├── Set deadbands on each room
         ├── Create initial DeadbandCircuits
         └── Enable deadband monitoring
         -> Tile(s): deadband_configured, circuits_active

TICK 41+:   First automation / Self-bootstrap complete
         ├── Hermes auto-creates tiles for automation
         ├── Auto-bootstrap tile archiving begins
         └── System ready for user interaction
         -> Tile(s): bootstrap_complete, system_ready
```

### Bootstrap as Tiles

Every bootstrap step creates one or more tiles. This means:

- **Full provenance**: You can trace every decision during boot
- **Rollback**: You can replay from any tick
- **Audit**: You can query "when did Navigation room get created?"
- **Deadband**: Even bootstrap is bounded by deadband — if a step takes too many ticks, it's flagged

### Ensign Promotion During Bootstrap

```
At deploy:     DORMANT
After orient:  ORIENTING -> GREEN_ALERT  (ensign has read the room)
After 10 successful tiles:  YELLOW_ALERT (ensign is active)
After first failure:        YELLOW_ALERT (escalates to Hermes, not Red)
After 3 consecutive failures: RED_ALERT  (phone-a-friend)
```

Ensigns must **prove themselves** at each stage. Green alert means the ensign is watching. Only after demonstrating competence does the system promote to yellow alert.

### Child Shell Spawning (Post-Bootstrap)

```
Hermes detects: "This task needs a dedicated ZeroClaw"
1. Hermes creates: Shell-ZeroClaw-1
   ├── universe_path: /home/hermes/sandbox/zeroclaw-1/
   ├── apis: ["sensor_api"]
   ├── rooms: ["navigation"]
   └── conservation_budget: 100.0 (allocated from Hermes budget)

2. Hermes deploys ZeroClaw into the new Shell
   └── ZeroClaw can only access sensor_api + navigation room
   └── ZeroClaw creates its own tiles in Sandbox
   └── ZeroClaw reports back to Hermes via port

3. Hermes monitors ZeroClaw tiles
   └── If ZeroClaw violates sandbox: kill Shell, log tile, alert Captain
   └── If ZeroClaw succeeds: merge results into Hermes tile
```

---

## 5. API Allowance System

The Allowance system gates API access for each Shell. No API call happens without a matching Allowance record. This is the **key management** layer — allowances bind provider keys to Shell scopes.

### Allowance Schema

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "Allowance",
  "description": "Grants an API to a Shell with rate limits, budget, and permissions.",
  "type": "object",
  "required": [
    "allowance_id", "shell_id", "api",
    "rate_limit", "budget", "permissions"
  ],
  "properties": {
    "allowance_id": {
      "type": "string",
      "description": "Unique allowance identifier"
    },
    "shell_id": {
      "type": "string",
      "description": "Shell granted this allowance"
    },
    "api": {
      "type": "string",
      "description": "API identifier (e.g. 'openai.com', 'deepinfra.com', 'huggingface.co')"
    },
    "key_ref": {
      "type": "string",
      "description": "Reference to the actual API key in .env (NOT the key itself). E.g. 'OPENAI_API_KEY'"
    },
    "rate_limit": {
      "type": "integer",
      "minimum": 1,
      "description": "Maximum calls per minute"
    },
    "budget": {
      "type": "number",
      "minimum": 0,
      "description": "Monetary or energy budget allocated for this allowance"
    },
    "budget_spent": {
      "type": "number",
      "minimum": 0,
      "default": 0,
      "description": "Running tally of budget consumed"
    },
    "permissions": {
      "type": "array",
      "items": { "type": "string" },
      "description": "Permission scopes (e.g. 'models:generate', 'models:embed', 'models:chat')",
      "uniqueItems": true
    },
    "expires": {
      "type": ["integer", "null"],
      "description": "Tick or unix timestamp when allowance expires. null = no expiry."
    },
    "period": {
      "type": "string",
      "enum": ["per_minute", "per_hour", "per_day", "per_request"],
      "description": "Period for the rate limit and budget"
    },
    "created_tick": {
      "type": "integer"
    },
    "updated_tick": {
      "type": "integer"
    }
  }
}
```

### Allowance Enforcement

```
Shell makes API call:
  1. Route to Shell's API router
  2. Look up Allowance for (shell_id, api)
  3. IF no Allowance -> REJECT (permission denied)
  4. IF Allowance exists:
     ├── Check rate_limit (calls this period)
     ├── Check budget (budget_spent < budget)
     ├── Check expires (not expired)
     ├── Check permissions matches request scope
     └── ALL pass -> ALLOW and deduct from budget
                    ELSE -> REJECT or ESCALATE to parent
```

### Allowance Hardening Rules

1. **Keys in .env only**: The Allowance carries a `key_ref` (e.g. `OPENAI_API_KEY`), NEVER the actual key value. Key values are loaded at bootstrap, exposed only at the HTTP transport layer.

2. **Budget is hard cap**: When `budget_spent >= budget`, the Shell cannot call that API until the budget resets or is replenished by the parent Shell.

3. **Rate limit is hard cap**: Exceeding `rate_limit` per `period` triggers a backoff. Backoff duration doubles with each successive violation.

4. **No agent-exposed keys**: The agent NEVER sees API keys in its context, logs, or tool outputs. The redaction layer intercepts any accidental exposure.

5. **Child Shell inheritance**: A child Shell's Allowances are a SUBSET of the parent's. The parent cannot grant an API it doesn't have. This ensures privilege containment.

### Example Allowances Table

| Shell | API | Rate Limit | Budget | Permissions | Expires |
|---|---|---|---|---|---|
| Hermes | openai.com | 100/min | $5/day | models:chat, models:generate | null |
| Hermes | deepinfra.com | 200/min | $2/day | models:chat, models:embed | null |
| ZeroClaw-sensors | sensor_api.local | 600/min | $0.50/day | sensors:read | null |
| ZeroClaw-media | media_api.local | 60/min | $1.00/day | media:transcode | +30 days |

---

## Appendix: Conservation Budget Allocation

Every operation has a standard energy cost. These values are used by the conservation runtime to verify:

```
total_deposits - total_withdrawals == total_budget (after every tick)
```

| Operation | Cost (units) |
|---|---|
| Tile creation | 0.1 |
| Tile completion | 0.05 |
| Tile archival | 0.01 |
| Ensign activation (dormant -> waking) | 1.0 |
| Ensign orientation | 0.5 |
| Ensign tile processing | 0.5 |
| Ensign stand-down | 0.3 |
| Gravity update | 0.01 |
| Gravity recalibrate (full recompute) | 0.1 |
| Phone-a-Friend escalation | 5.0 |
| Correlation proximity compute | 0.05 |
| Correlation knowledge transfer | 0.05 |
| Penrose re-fit (all room pairs) | 0.5 |
| Port open/close | 0.2 |
| Port message (per message) | 0.01 |
| Deadband circuit check | 0.02 |
| Deadband circuit action | 0.5 |
| Bootstrap step | 0.5 |
| Shell spawn | 5.0 |
| Shell destroy | 2.0 |
| API call (gated by Allowance) | varies by provider |

---

*This is the canonical schema reference for hermes-construct kernel implementation. All code generators and implementors should read from this file. Updated 2026-05-30.*
