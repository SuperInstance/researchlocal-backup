# Puzzle 1: The Message Routing Problem

## Solution: `route_message` Algorithm

### Overview

When a Telegram message arrives, it must be routed to the correct room. The routing considers:
1. **Gravity matching** — which room's gravity is closest to the message's semantic "vibe"
2. **Room availability** — rooms with ensigns at yellow/green alert are preferred
3. **Ensign readiness** — the ensign must be in a handleable state
4. **Deadband state** — rooms in stable deadband get routing preference
5. **Fallback chain** — if the primary room can't handle it, we cascade

### Types

```rust
/// Gravity signal extracted from a user message.
/// This is a low-dimensional embedding produced by a lightweight
/// classification heuristic (v0.1) or a JEPA embedding (v0.2+).
#[derive(Debug, Clone)]
pub struct GravitySignal {
    /// The semantic gravity of the message, [-1.0, +1.0].
    /// Negative = precise/technical, Positive = creative/social.
    pub value: f64,
    /// Confidence in the signal extraction.
    pub confidence: f64,
    /// Which room types the message explicitly references (if any).
    pub explicit_room_hints: Vec<String>,
}

/// Routing decision returned by the algorithm.
#[derive(Debug, Clone)]
pub struct RoutingDecision {
    /// The primary room chosen.
    pub room_id: String,
    /// How the decision was made.
    pub method: RoutingMethod,
    /// Score [0, 1] indicating confidence in this routing.
    pub confidence: f64,
    /// Fallback chain: ordered list of rooms to try if primary fails.
    pub fallback_chain: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RoutingMethod {
    /// Message explicitly matched a room by keywords.
    KeywordMatch,
    /// Gravity signal was closest to this room's gravity.
    GravityMatch,
    /// Room has a Penrose correlation with the best-gravity room.
    CorrelationRedirect,
    /// Default social room chosen because nothing else matched.
    DefaultFallback,
    /// Emergency: all rooms at red alert, picked least-bad option.
    EmergencyFallback,
}

/// Room's suitability for routing (computed per routing decision).
#[derive(Debug, Clone)]
struct RoomSuitability {
    room_id: String,
    gravity_distance: f64,
    ensign_can_handle: bool,
    alert_weight: f64,
    deadband_stable: bool,
    overall_score: f64,
}
```

### Step 1: Extract Gravity Signal from Message

```rust
/// Extract a gravity signal from a user message.
///
/// This uses a lightweight keyword + pattern approach in v0.1.
/// In v0.2+, this would use a JEPA embedding model.
///
/// The algorithm:
/// 1. Tokenize the message
/// 2. Score each token against room-type keyword lists
/// 3. Compute a weighted gravity based on matched keywords
/// 4. Derive confidence from match density
fn extract_gravity_signal(message: &str) -> GravitySignal {
    let lower = message.to_lowercase();
    let words: Vec<&str> = lower.split_whitespace().collect();

    if words.is_empty() {
        return GravitySignal {
            value: 0.0,
            confidence: 0.0,
            explicit_room_hints: vec![],
        };
    }

    // Room-type keyword maps: (keyword_list, gravity_value, room_type)
    let room_keywords: &[(&[&str], f64, &str)] = &[
        // Navigation: precise, directional
        (&["navigate", "direction", "where", "location", "map", "route",
           "go to", "find", "path", "way"], -0.3, "navigation"),
        // Engineering: very precise, technical
        (&["build", "code", "fix", "debug", "implement", "compile",
           "deploy", "server", "config", "error", "bug", "git",
           "test", "refactor", "api", "function", "module"], -0.6, "engineering"),
        // Science: balanced, analytical
        (&["research", "analyze", "science", "study", "data", "experiment",
           "hypothesis", "correlate", "pattern", "predict", "model"], 0.0, "science"),
        // Security: very precise, protective
        (&["security", "safe", "protect", "vulnerability", "threat",
           "audit", "encrypt", "password", "access", "permission"], -0.8, "security"),
        // Social: creative, narrative
        (&["hello", "hi", "hey", "chat", "story", "tell", "fun",
           "joke", "music", "movie", "game", "play", "friend"], 0.5, "social"),
    ];

    let mut matched_gravity: f64 = 0.0;
    let mut match_count: usize = 0;
    let mut explicit_hints: Vec<String> = Vec::new();

    for (keywords, gravity, room_type) in room_keywords {
        for keyword in *keywords {
            // Multi-word keywords need special handling
            if keyword.contains(' ') {
                if lower.contains(keyword) {
                    matched_gravity += gravity;
                    match_count += 1;
                    if !explicit_hints.contains(&room_type.to_string()) {
                        explicit_hints.push(room_type.to_string());
                    }
                }
            } else {
                for word in &words {
                    if word == keyword || word.starts_with(keyword) {
                        matched_gravity += gravity;
                        match_count += 1;
                        if !explicit_hints.contains(&room_type.to_string()) {
                            explicit_hints.push(room_type.to_string());
                        }
                    }
                }
            }
        }
    }

    if match_count == 0 {
        // No keywords matched. Default to neutral gravity.
        GravitySignal {
            value: 0.0,
            confidence: 0.1, // low confidence
            explicit_room_hints: vec![],
        }
    } else {
        // Average the matched gravities
        let avg_gravity = matched_gravity / match_count as f64;
        // Confidence scales with match density (capped at 1.0)
        let confidence = (match_count as f64 / words.len() as f64).min(1.0);

        GravitySignal {
            value: avg_gravity.clamp(-1.0, 1.0),
            confidence,
            explicit_room_hints: explicit_hints,
        }
    }
}
```

### Step 2: Compute Room Suitability Scores

```rust
/// Compute a suitability score for each room given a gravity signal.
///
/// Scoring formula:
///   overall_score = gravity_affinity * 0.5
///                 + ensign_readiness * 0.25
///                 + deadband_bonus * 0.15
///                 + confidence_weight * 0.10
///
/// where:
///   gravity_affinity    = 1.0 - |room.gravity - signal.value| / 2.0
///   ensign_readiness    = 1.0 if yellow_alert, 0.7 if green_alert, 0.3 otherwise
///   deadband_bonus      = 0.2 if room's deadband circuits are stable
///   confidence_weight   = room.gravity_confidence
fn compute_suitability(
    rooms: &[Room],
    signal: &GravitySignal,
    ensigns: &HashMap<String, Ensign>,
    deadband_circuits: &[DeadbandCircuit],
) -> Vec<RoomSuitability> {
    rooms.iter().map(|room| {
        // 1. Gravity affinity: how close is this room's gravity to the signal?
        let gravity_distance = (room.gravity - signal.value).abs();
        let gravity_affinity = 1.0 - gravity_distance / 2.0; // [0, 1]

        // 2. Ensign readiness
        let ensign = room.ensign_id.as_ref()
            .and_then(|eid| ensigns.get(eid));
        let (ensign_can_handle, alert_weight) = match ensign {
            Some(e) if e.can_handle() && e.alert_level == AlertLevel::Yellow =>
                (true, 1.0),
            Some(e) if e.can_handle() && e.alert_level == AlertLevel::Green =>
                (true, 0.7),
            Some(e) if e.status == EnsignStatus::Escalated =>
                (false, 0.1), // escalated ensigns can't take new work
            Some(_) =>
                (false, 0.3), // dormant/orienting/red
            None =>
                (false, 0.0), // no ensign assigned
        };

        // 3. Deadband stability
        let room_circuits: Vec<_> = deadband_circuits.iter()
            .filter(|c| c.room_id == room.id)
            .collect();
        let deadband_stable = room_circuits.is_empty()
            || room_circuits.iter().all(|c| !c.is_breached);
        let deadband_bonus = if deadband_stable { 0.2 } else { 0.0 };

        // 4. Room gravity confidence
        let confidence_weight = room.gravity_confidence;

        // 5. Bonus for explicit room hints
        let hint_bonus = if signal.explicit_room_hints.contains(&room.room_type.as_str().to_string()) {
            0.3 // strong signal that this is the right room
        } else {
            0.0
        };

        let overall_score = gravity_affinity * 0.4
            + alert_weight * 0.25
            + deadband_bonus * 0.1
            + confidence_weight * 0.05
            + hint_bonus * 0.2;

        RoomSuitability {
            room_id: room.id.clone(),
            gravity_distance,
            ensign_can_handle,
            alert_weight,
            deadband_stable,
            overall_score: overall_score.clamp(0.0, 1.0),
        }
    }).collect()
}
```

### Step 3: The Full `route_message` Function

```rust
/// Route an incoming message to the best room.
///
/// Algorithm:
/// 1. Extract gravity signal from the message
/// 2. Compute suitability for all rooms
/// 3. Try explicit room hints first (high-confidence direct match)
/// 4. If no explicit match, pick highest suitability score
/// 5. Check ensign can handle — if not, walk fallback chain
/// 6. If all rooms are unavailable, use emergency fallback
pub fn route_message(
    db: &Connection,
    message: &str,
    correlations: &[Correlation],
) -> Result<RoutingDecision, String> {
    let rooms = room::get_all_rooms(db)
        .map_err(|e| format!("route_message: get rooms: {}", e))?;
    let all_ensigns = ensign::get_all_ensigns(db)
        .map_err(|e| format!("route_message: get ensigns: {}", e))?;
    let ensign_map: HashMap<String, Ensign> = all_ensigns
        .into_iter()
        .filter_map(|e| e.room_id.clone().map(|rid| (rid, e)))
        .collect();
    let circuits = deadband::run_checks(db, conservation::current_tick())
        .unwrap_or_default();

    // Edge case: no rooms at all
    if rooms.is_empty() {
        return Err("No rooms available for routing".to_string());
    }

    // Step 1: Extract gravity signal
    let signal = extract_gravity_signal(message);

    // Step 2: Compute suitability for all rooms
    let mut suitability = compute_suitability(
        &rooms, &signal, &ensign_map, &circuits
    );

    // Sort by overall_score descending
    suitability.sort_by(|a, b| {
        b.overall_score.partial_cmp(&a.overall_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Step 3: Try explicit room hints first
    if !signal.explicit_room_hints.is_empty() && signal.confidence > 0.3 {
        for hint in &signal.explicit_room_hints {
            if let Some(s) = suitability.iter().find(|s| {
                rooms.iter().any(|r| {
                    r.id == s.room_id && r.room_type.as_str() == hint.as_str()
                })
            }) {
                if s.ensign_can_handle {
                    let fallback = build_fallback_chain(
                        &suitability, &s.room_id
                    );
                    return Ok(RoutingDecision {
                        room_id: s.room_id.clone(),
                        method: RoutingMethod::KeywordMatch,
                        confidence: signal.confidence,
                        fallback_chain: fallback,
                    });
                }
            }
        }
    }

    // Step 4: Pick highest suitability with a handleable ensign
    for candidate in &suitability {
        if candidate.ensign_can_handle {
            let method = if signal.explicit_room_hints.is_empty()
                && signal.confidence < 0.3 {
                RoutingMethod::GravityMatch
            } else {
                RoutingMethod::KeywordMatch
            };

            let fallback = build_fallback_chain(
                &suitability, &candidate.room_id
            );

            return Ok(RoutingDecision {
                room_id: candidate.room_id.clone(),
                method,
                confidence: candidate.overall_score,
                fallback_chain: fallback,
            });
        }
    }

    // Step 5: Check correlation redirects
    // If the best room's ensign can't handle, check correlated rooms
    if let Some(best_unhandled) = suitability.first() {
        let correlated = find_correlated_room(
            &best_unhandled.room_id, correlations, &suitability
        );
        if let Some(corr) = correlated {
            if corr.ensign_can_handle {
                let fallback = build_fallback_chain(
                    &suitability, &corr.room_id
                );
                return Ok(RoutingDecision {
                    room_id: corr.room_id.clone(),
                    method: RoutingMethod::CorrelationRedirect,
                    confidence: 0.5, // moderate confidence
                    fallback_chain: fallback,
                });
            }
        }
    }

    // Step 6: Default fallback — social room or first available
    let social = suitability.iter()
        .find(|s| {
            rooms.iter().any(|r|
                r.id == s.room_id && r.room_type == RoomType::Social
            )
        });
    let fallback_target = social
        .or_else(|| suitability.first())
        .ok_or("No rooms available at all")?;

    let fallback = build_fallback_chain(
        &suitability, &fallback_target.room_id
    );

    // Determine if this is a true emergency
    let all_red = rooms.iter().all(|r| {
        r.ensign_id.as_ref()
            .and_then(|eid| ensign_map.get(&r.id))
            .map(|e| e.alert_level == AlertLevel::Red)
            .unwrap_or(true)
    });

    Ok(RoutingDecision {
        room_id: fallback_target.room_id.clone(),
        method: if all_red {
            RoutingMethod::EmergencyFallback
        } else {
            RoutingMethod::DefaultFallback
        },
        confidence: 0.2,
        fallback_chain: fallback,
    })
}

/// Build an ordered fallback chain from the suitability list.
fn build_fallback_chain(
    suitability: &[RoomSuitability],
    exclude_room: &str,
) -> Vec<String> {
    suitability.iter()
        .filter(|s| s.room_id != exclude_room)
        .take(3) // max 3 fallback rooms
        .map(|s| s.room_id.clone())
        .collect()
}

/// Find a correlated room that can handle the message.
fn find_correlated_room(
    room_id: &str,
    correlations: &[Correlation],
    suitability: &[RoomSuitability],
) -> Option<&RoomSuitability> {
    correlations.iter()
        .filter(|c| c.room_a == room_id || c.room_b == room_id)
        .filter(|c| c.correlation.abs() > 0.5)
        .filter_map(|c| {
            let other_id = if c.room_a == room_id { &c.room_b } else { &c.room_a };
            suitability.iter().find(|s| s.room_id == *other_id)
        })
        .max_by(|a, b| a.overall_score.partial_cmp(&b.overall_score)
            .unwrap_or(std::cmp::Ordering::Equal))
}
```

### Step 4: Integration with Kernel's `process_message`

The existing `kernel.rs::process_message` should be updated to use `route_message`:

```rust
// In kernel.rs, replace the current routing logic:

pub async fn process_message(&self, msg: &PortMessage) -> Result<(), String> {
    let tick = conservation::advance_tick();

    // 1. Create observation tile
    let mut obs_tile = Tile::new(TileType::Observation, &msg.text, tick);
    obs_tile.conservation_delta = costs::TILE_CREATE;
    {
        let mut cons = self.conservation.lock().await;
        cons.spend(costs::TILE_CREATE)?;
    }

    // 2. Route to room using the full algorithm
    let db = self.db.lock().await;
    let correlations = penrose::get_all_correlations(&db)
        .unwrap_or_default();
    let routing = route_message(&db, &msg.text, &correlations)?;
    drop(db);

    log::info!(
        "Routed message to room '{}' via {:?} (confidence: {:.2})",
        routing.room_id, routing.method, routing.confidence
    );

    obs_tile.room_id = Some(routing.room_id.clone());

    // 3. Get ensign for room, try fallback chain if primary fails
    let (ensign_info, room_id) = {
        let db = self.db.lock().await;
        let mut result = ensign::get_ensign_for_room(&db, &routing.room_id)
            .map_err(|e| format!("{}", e))?;

        let mut chosen_room = routing.room_id.clone();

        // If primary ensign can't handle, try fallback chain
        if result.as_ref().map(|e| !e.can_handle()).unwrap_or(true) {
            for fallback_id in &routing.fallback_chain {
                if let Some(e) = ensign::get_ensign_for_room(&db, fallback_id)
                    .map_err(|e| format!("{}", e))?
                {
                    if e.can_handle() {
                        result = Some(e);
                        chosen_room = fallback_id.clone();
                        break;
                    }
                }
            }
        }

        (result, chosen_room)
    };

    // ... rest of process_message unchanged, using room_id ...
    // ... (model params from room gravity, API call, create action tile, etc.)
}
```

### Step 5: Integration with lau-jepa-gravity

The `GravitySignal` extraction currently uses keyword matching. When JEPA embeddings are available:

```rust
/// Future: JEPA-based gravity extraction.
///
/// The jepa-predict crate provides a `Jepa<D>` engine with
/// `db_in` (perceptions) and `db_out` (predictions + surprise).
///
/// To use it for gravity:
/// 1. Each room maintains a Jepa<8> (8-dim embedding space)
/// 2. Every tile in the room is embedded via the JEPA model
/// 3. The room's gravity is derived from the JEPA's prediction
///
/// For message routing:
/// 1. Embed the incoming message using the same model
/// 2. Compare the embedding against each room's JEPA prediction
/// 3. Route to the room with lowest surprise
///
/// Type mapping:
///   jepa_predict::Entry<8>.data  →  GravitySignal embedding
///   jepa_predict::Jepa<8>.predict()  →  Room's expected next gravity
///   jepa_predict::Jepa<8>.surprise() →  Routing distance metric

#[cfg(feature = "jepa-routing")]
fn extract_gravity_signal_jepa(
    message: &str,
    room_jepas: &HashMap<String, jepa_predict::Jepa<8>>,
) -> GravitySignal {
    let embedding = embed_message(message); // external embedding model

    let mut best_room: Option<String> = None;
    let mut min_surprise: f64 = f64::MAX;

    for (room_id, jepa) in room_jepas {
        if let Some(prediction) = jepa.predict() {
            let surprise = jepa_predict::Jepa::<8>::surprise(
                prediction.predicted, embedding
            );
            if surprise < min_surprise {
                min_surprise = surprise;
                best_room = Some(room_id.clone());
            }
        }
    }

    match best_room {
        Some(room_id) => GravitySignal {
            value: 0.0, // derived from JEPA, not needed for keyword matching
            confidence: 1.0 - min_surprise,
            explicit_room_hints: vec![room_id],
        },
        None => GravitySignal {
            value: 0.0,
            confidence: 0.0,
            explicit_room_hints: vec![],
        },
    }
}
```

### Edge Cases

| Scenario | Behavior |
|----------|----------|
| **No rooms exist** | Return error. Kernel should have created default rooms during bootstrap. |
| **All rooms at red alert** | Route to least-bad room via `EmergencyFallback`. Log a system tile. The ensign will escalate to phone-a-friend (Opus). |
| **No rooms match gravity** | Route to social room (default) or first room. Low confidence flag. |
| **No ensign assigned** | Route to the room anyway. The kernel will use a default provider/model. |
| **Ensign at red alert but correlation exists** | Route to correlated room via `CorrelationRedirect`. |
| **Message is empty** | Return signal with value=0.0, confidence=0.0. Routes to default. |
| **Message matches multiple rooms** | Highest suitability score wins. Explicit hints break ties. |
| **All ensigns escalated** | Emergency fallback. Create an escalation tile. Use default provider directly. |
| **SQLite locked during routing** | Return error. Kernel retries on next tick. Messages queue in port buffer. |
| **Penrose correlation points to dead room** | `find_correlated_room` checks `ensign_can_handle` — skips dead rooms. |

### Fallback Chain Visualization

```
Message arrives
    │
    ▼
Extract gravity signal
    │
    ▼
Keyword match? ──Yes──→ Route to matched room ──ensign OK?──→ Done ✓
    │                                              │
    No                                          No → next fallback
    │                                              │
    ▼                                          (try 3 rooms)
Gravity match: highest suitability?
    │                                              │
    ensign OK? ──Yes──→ Done ✓                     ▼
    │                                        All failed?
    No → correlation redirect?
    │                                              │
    ▼                                              ▼
Try correlated room ──ensign OK?──→ Done ✓    Emergency fallback
    │                                    (route to social/default)
    No
    │
    ▼
Default fallback (social room)
    │
    ▼
Emergency fallback (any room, lowest red alert)
```
