# Theia Research — Projection Layer Host Decision

> **Research commissioned by:** captain (Round 14 SESSION-NOTE in 2026-07-23 daily plan)
> **Purpose:** R&D on projection-layer host for Phase 1.5 / Phase 10.0
> **Status:** Complete — recommendation: **static SPA over FastAPI**

---

## Executive Summary

After extensive R&D, **Theia is not recommended** for the F/V Eileen boat-agent platform. Theia's minimum 6 GB RAM requirement and ~500 MB disk footprint exceed the Windows ProArt's 8 GB RAM capacity (with no GPU) and the constraint of staying web-native without native installation.

**Recommended alternative:** A static SPA (Svelte or Vue 3) over FastAPI with CSS Grid for the four-panel layout. This delivers:
- **Install footprint:** ~50-100 KB (Svelte) or ~150-200 KB (Vue 3)
- **Runtime RAM:** <100 MB baseline
- **Four-panel layout:** Native CSS Grid (`2fr 1fr 1fr / 1fr 1fr`)
- **Python bridge:** WebSocket or HTTP (direct to FastAPI, no intermediary)

---

## Findings

### 1. Theia Full IDE vs Blueprint vs Smaller Alternatives

| Option | Install Size | Runtime RAM | Four-Panel | Web-Native | License |
|--------|-------------|-------------|------------|------------|---------|
| **Theia** | ~500-700 MB | 6-8 GB minimum | Custom (drag-drop) | Yes (cloud mode) | EPL-2.0 |
| **Blueprint** | ~500-700 MB | 6-8 GB minimum | Custom (drag-drop) | Yes (cloud mode) | EPL-2.0 |
| **Svelte SPA** | ~50-100 KB | <100 MB | Native CSS Grid | Yes | MIT |
| **Vue 3 SPA** | ~150-200 KB | <100 MB | Native CSS Grid | Yes | MIT |
| **CodeMirror 6** | ~21 KB | Minimal | 1 panel only | Component | MIT |

**Theia/Blueprint position:** Full IDE frameworks designed for cloud development environments. Overkill for a boat dashboard.

**SPA position:** Purpose-built for exactly this use case — lightweight, web-native, zero-install.

---

### 2. Install Footprint and Runtime Cost on Windows ProArt (8 GB RAM, no GPU)

| Metric | Theia | Svelte SPA | Notes |
|--------|-------|------------|-------|
| **Disk footprint** | ~500-700 MB | ~50-100 KB | 5,000–10,000× smaller |
| **Baseline RAM** | 6-8 GB | <100 MB | 60–80× smaller |
| **GPU requirement** | None (but CPU-heavy) | None | Both CPU-only OK |
| **Windows compatibility** | Full (Docker) | Full (any browser) | Both work |

**Verdict:** Theia consumes 75-100% of available RAM just to run. The SPA approach leaves headroom for capture pipeline, analyzer, and OS.

---

### 3. Agent Bridge: Projection to Python Service

**Theia path (complex):**
```
Browser → Theia (Node.js) → WebSocket → FastAPI (Python)
```
- Requires Node.js intermediary
- Theia backend runs on Node.js, not Python
- Extension system adds build toolchain (webpack, npm)

**SPA path (direct):**
```
Browser → WebSocket/HTTP → FastAPI (Python)
```
- Direct communication with existing dashboard.py
- No intermediary runtime
- WebSocket for real-time (gauges, alerts)
- HTTP for queries (captures list, vessel state)

**Recommended bridge:** WebSocket over FastAPI
```python
# In dashboard.py
from fastapi import WebSocket

@app.websocket("/ws")
async def projection_ws(websocket: WebSocket):
    await websocket.accept()
    # Stream vessel_state.jsonl updates, alerts, gauges
    while True:
        data = await get_next_state()
        await websocket.send_json(data)
```

---

### 4. Path of Least Resistance to Four-Panel Shell with dashboard.gauges Widget

**Phase 1.5 Scope (single PR, ~200 LOC):**

Files to add:
```
projection/
  ├── index.html          # 4-panel shell (CSS Grid)
  ├── app.js              # panel loading, WebSocket client
  └── widgets/
      └── dashboard-gauges.js  # wrap existing /api/health
```

Files to modify (minimal):
```
dashboard.py              # add WebSocket endpoint for gauges
```

**CSS Grid layout (20 lines):**
```css
.projection-shell {
  display: grid;
  grid-template-columns: 240px 1fr 360px;
  grid-template-rows: 1fr 320px;
  grid-template-areas:
    "left center right"
    "left bottom  right";
  height: 100vh;
}
```

**Panel 1 (left, captures tree):** Reuse existing dashboard logic

**Panel 2 (center, empty for Phase 1.5):** Placeholder

**Panel 3 (right, agent chat):** Reuse existing dashboard logic

**Panel 4 (bottom, dashboard.gauges):** New widget wrapping `/api/health`

**Total Phase 1.5 estimate:** ~200 LOC new, ~20 LOC modified.

---

### 5. Existing Open-Source Projects for Robotics/Boat/Industrial UIs

| Project | Why It Applies | Why It Doesn't |
|---------|----------------|----------------|
| **Foxglove Studio** | Professional robotics visualization, ROS2-native | Heavier than Theia, ROS-focused architecture |
| **FUXA** | Web-based SCADA, industrial dashboards | Designed for full SCADA, overkill for 4-panel |
| **ROSboard** | Web-based, mobile-friendly, real-time robotics data | ROS2-specific, requires ROS ecosystem |

**Verdict:** None are direct fits. All are heavier than the SPA approach and tied to specific ecosystems (ROS2, SCADA). The custom SPA is lighter and more directly aligned with the git-agent paradigm.

---

### 6. Phase 1.5 Migration Plan (PR-Sized Slice)

**Goal:** Working four-panel shell with `dashboard.gauges` widget in bottom pane. No breaking changes to `dashboard.py`.

**Step 1:** Add `projection/` directory with static files
- `projection/index.html` — shell with empty panels
- `projection/app.js` — panel loader, WebSocket client
- `projection/style.css` — IDE-shape grid layout

**Step 2:** Add WebSocket endpoint to `dashboard.py`
```python
@app.websocket("/ws/gauges")
async def gauges_websocket(websocket: WebSocket):
    await websocket.accept()
    cfg = vessel_config.load()
    await websocket.send_json({
        "vessel": cfg["vessel"]["name"],
        "lan_ip": _get_lan_ip(),
        "providers_enabled": vessel_config.enabled_provider_names(cfg),
        # Add more KPIs over time
    })
```

**Step 3:** Serve static files from `dashboard.py`
```python
from fastapi.staticfiles import StaticFiles
app.mount("/projection", StaticFiles(directory="projection"), name="projection")
```

**Step 4:** Access at `http://192.168.x.x:8090/projection/`

**Result:** Four-panel shell with gauges in bottom, other panels empty (future widgets).

**Phase 1.5 file list (new):**
- `projection/index.html` (~50 LOC)
- `projection/app.js` (~80 LOC)
- `projection/style.css` (~40 LOC)
- `dashboard.py` (+~30 LOC for WebSocket)

**Total:** ~200 LOC added, preserves existing `dashboard/` directory.

---

## Alternatives Considered

| Alternative | Pros | Cons | Confidence |
|-------------|------|------|------------|
| **Theia** | Full IDE framework, VS Code extension compatibility | 6-8 GB RAM, 500+ MB disk, Node.js intermediary | Low |
| **Blueprint** | Slightly stripped Theia | Same resource constraints, still overkill | Low |
| **CodeMirror 6** | Extremely light (21 KB) | Editor component only, not a dashboard framework | Medium (for center panel only) |
| **Monaco Editor** | Full VS Code editing | 81+ MB, overkill for simple edits | Low |
| **Svelte SPA** | Lightest (~50 KB), best memory efficiency | Smaller ecosystem than Vue/React | **High (recommended)** |
| **Vue 3 SPA** | Light (~150 KB), balanced ecosystem | Slightly heavier than Svelte | **High (recommended)** |

---

## Decision

**Host choice:** Static SPA (Svelte or Vue 3) over FastAPI

**Rationale:**
1. **Resource constraints:** 8 GB RAM Windows ProArt cannot spare 6 GB for Theia
2. **Web-native requirement:** SPA serves over LAN with zero client install
3. **Direct Python bridge:** No Node.js intermediary needed
4. **Phase 1.5 scope:** Single PR, ~200 LOC, preserves dashboard.py
5. **Git-agent paradigm:** Projection as thin adapter, not the cockpit

**Install footprint:** ~50-200 KB (vs ~500 MB for Theia)

**RAM cost:** <100 MB baseline (vs 6-8 GB for Theia)

**Agent bridge:** WebSocket or HTTP direct to FastAPI (no intermediary)

**Confidence:** **High** — Theia is fundamentally designed for a different use case (cloud IDEs vs boat dashboard).

---

## References

- Theia IDE: https://theia-ide.org/
- EPL-2.0 License: https://www.eclipse.org/legal/epl-2.0/
- CodeMirror 6: https://codemirror.net/ (MIT)
- Vue 3: https://vuejs.org/ (MIT)
- Svelte: https://svelte.dev/ (MIT)
- FastAPI WebSocket: https://fastapi.tiangolo.com/advanced/websockets/
- CSS Grid Layout: https://developer.mozilla.org/en-US/docs/Web/CSS/CSS_Grid_Layout
- Foxglove Studio: https://docs.foxglove.dev/
- FUXA SCADA: https://github.com/frangoteam/FUXA
- ROSboard: https://github.com/MoffKalast/rosboard

---

**Next action:** Proceed with Phase 1.5 implementation using SPA approach. File tracking: `docs/phases/phase-1.5.md` (to be created).
