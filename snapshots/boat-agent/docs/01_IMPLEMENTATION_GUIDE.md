# Boat Agent System - Implementation Guide

> **Target Audience:** Developer agents implementing specific components
> **Purpose:** Concrete implementation steps with code patterns
> **Status:** Active Development

---

## Getting Started

### Prerequisites

**Development Environment:**
- Windows 11 (primary target platform)
- Node.js 18+
- Rust 1.70+ with MSVC toolchain
- Git

**Hardware for Testing:**
- Serial GPS device (or NMEA simulator)
- USB webcam (for vision pipeline testing)
- Optional: NMEA 2000 gateway

### Repository Clone

```bash
git clone https://github.com/your-org/boat-agent.git
cd boat-agent
npm install
```

---

## Phase 1: Tauri Core Setup

### 1.1 Initialize Tauri Project

```bash
npm create tauri-app@latest
# Follow prompts, select React + TypeScript
```

### 1.2 Configure Cargo.toml

Add to `src-tauri/Cargo.toml`:

```toml
[dependencies]
tauri = { version = "2.0", features = ["all-features"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.0", features = ["full"] }
tokio-serial = "5.4.4"
nmea = "0.6.0"
scrap = "0.5.0"
image = "0.24.0"
sysinfo = "0.30.0"
tauri-plugin-shell = "2.0"
pyo3 = { version = "0.20", features = ["auto-initialize"] }
sha2 = "0.10"
hyper = { version = "0.14", features = ["full"] }
```

### 1.3 Basic Main.rs Structure

`src-tauri/src/main.rs`:

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod service;
mod screenshot;
mod serial;
mod universal_bus;

use service::{OpenClawState, start_openclaw_service};
use universal_bus::UniversalHardwareBus;

fn main() {
    let hardware_bus = UniversalHardwareBus::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(OpenClawState {
            child_process: std::sync::Mutex::new(None),
        })
        .manage(hardware_bus)
        .invoke_handler(tauri::generate_handler![
            start_openclaw_service,
            capture_wheelhouse_screen,
            list_active_marine_ports,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                let state = window.state::<OpenClawState>();
                let mut lock = state.child_process.lock().unwrap();
                if let Some(child) = lock.take() {
                    let _ = child.kill();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

---

## Phase 2: Setup Wizard Implementation

### 2.1 Wizard State Machine

`src/components/Wizard.jsx`:

```jsx
import React, { useState } from 'react';
import Step1 from './Step1';
import Step2 from './Step2';
import Step3 from './Step3';

export default function Wizard() {
  const [step, setStep] = useState(0);
  const [config, setConfig] = useState({});

  const steps = [
    { component: Step1, title: 'Engine Selection' },
    { component: Step2, title: 'Cloud Sync' },
    { component: Step3, title: 'Hardware Profile' },
  ];

  const CurrentStep = steps[step].component;

  return (
    <div className="min-h-screen bg-slate-950 text-white p-6">
      <CurrentStep
        onNext={() => setStep(Math.min(step + 1, steps.length - 1))}
        onSkip={() => setStep(Math.min(step + 1, steps.length - 1))}
        config={config}
        setConfig={setConfig}
        onComplete={handleWizardComplete}
      />
    </div>
  );
}
```

### 2.2 DeepInfra Key Validation

`src/components/Step1.jsx`:

```jsx
import React, { useState } from 'react';

export default function Step1({ onNext, config, setConfig }) {
  const [apiKey, setApiKey] = useState(config.deepInfraKey || '');
  const [isValidating, setIsValidating] = useState(false);

  const validateKey = async () => {
    setIsValidating(true);
    try {
      const response = await fetch('https://api.deepinfra.com/v1/openai/chat/completions', {
        method: 'POST',
        headers: {
          'Authorization': `Bearer ${apiKey}`,
          'Content-Type': 'application/json'
        },
        body: JSON.stringify({
          model: "deepseek-ai/DeepSeek-V4-Flash",
          messages: [{ role: "user", content: "ping" }],
          max_tokens: 5
        })
      });

      if (response.ok) {
        setConfig({ ...config, deepInfraKey: apiKey });
        onNext();
      } else {
        alert('Invalid API Key');
      }
    } catch (err) {
      alert('Connection timeout');
    } finally {
      setIsValidating(false);
    }
  };

  return (
    <div className="space-y-6 max-w-xl mx-auto p-6 bg-slate-900 rounded-xl">
      <h2 className="text-xl font-bold">Step 1: Engine Selection</h2>

      <input
        type="password"
        value={apiKey}
        onChange={(e) => setApiKey(e.target.value)}
        placeholder="Enter DeepInfra API Key"
        className="w-full p-3 rounded bg-slate-950 border border-slate-800"
      />

      <button
        onClick={validateKey}
        disabled={isValidating || !apiKey}
        className="w-full bg-blue-600 hover:bg-blue-500 py-3 rounded-lg font-bold"
      >
        {isValidating ? 'Verifying...' : 'Verify & Continue'}
      </button>
    </div>
  );
}
```

---

## Phase 3: Vision Pipeline Implementation

### 3.1 Screen Capture Module

`src-tauri/src/screenshot.rs`:

```rust
use scrap::{Display, Capturer};
use std::io::ErrorKind;
use std::time::Duration;
use std::thread;
use image::save_buffer;

#[tauri::command]
pub fn capture_wheelhouse_screen(output_path: String) -> Result<String, String> {
    let display = Display::primary().map_err(|e| e.to_string())?;
    let mut capturer = Capturer::new(display).map_err(|e| e.to_string())?;

    let width = capturer.width();
    let height = capturer.height();

    // Wait for frame
    let mut buffer;
    loop {
        match capturer.frame() {
            Ok(frame) => {
                buffer = frame.to_vec();
                break;
            }
            Err(error) if error.kind() == ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(10));
                continue;
            }
            Err(e) => return Err(e.to_string()),
        }
    }

    // Convert BGRA to RGBA
    for i in (0..buffer.len()).step_by(4) {
        let b = buffer[i];
        let r = buffer[i + 2];
        buffer[i] = r;
        buffer[i + 2] = b;
    }

    save_buffer(
        &output_path,
        &buffer,
        width as u32,
        height as u32,
        image::ColorType::Rgba8,
    ).map_err(|e| e.to_string())?;

    Ok(format!("Captured to {}", output_path))
}
```

### 3.2 Cloudflare Worker Setup

`cloudflare-worker/src/index.js`:

```javascript
export default {
  async fetch(request, env) {
    const url = new URL(request.url);

    if (url.pathname === "/v1/vision/sounder" && request.method === "POST") {
      try {
        const { imageBase64, vesselId } = await request.json();
        const imageBuffer = Uint8Array.from(atob(imageBase64), c => c.charCodeAt(0));

        const systemPrompt = `You are a marine sonar analysis specialist.
          Analyze this echo-sounder image. Identify:
          1. Biomass depth (fathoms)
          2. Bottom hardness (rock vs mud)
          3. Notable features
          Respond in concise text.`;

        const modelResponse = await env.AI.run('@cf/llava-hf/llava-1.5-7b-hf', {
          image: [...imageBuffer],
          prompt: systemPrompt,
          max_tokens: 150
        });

        const analysisText = modelResponse.description || modelResponse.result;

        // Check vocabulary KV store
        const userMemoryKey = `vessel:${vesselId}:vocab`;
        const localVocab = await env.VESSEL_KV.get(userMemoryKey, "json") || {};

        // Apply custom vocabulary mappings
        let adjustedAnalysis = analysisText;
        for (const [pattern, replacement] of Object.entries(localVocab)) {
          adjustedAnalysis = adjustedAnalysis.replace(new RegExp(pattern, 'gi'), replacement);
        }

        return new Response(JSON.stringify({
          success: true,
          analysis: adjustedAnalysis
        }), { headers: { "Content-Type": "application/json" } });

      } catch (err) {
        return new Response(JSON.stringify({ error: err.message }), { status: 500 });
      }
    }

    // Training endpoint for vocabulary updates
    if (url.pathname === "/v1/memory/train" && request.method === "POST") {
      const { vesselId, rawPattern, actualLabel } = await request.json();

      const userMemoryKey = `vessel:${vesselId}:vocab`;
      let currentVocab = await env.VESSEL_KV.get(userMemoryKey, "json") || {};

      currentVocab[rawPattern] = actualLabel;
      await env.VESSEL_KV.put(userMemoryKey, JSON.stringify(currentVocab));

      return new Response(JSON.stringify({ saved: true }), { status: 200 });
    }

    return new Response("Not Found", { status: 404 });
  }
};
```

---

## Phase 4: Serial Communication Implementation

### 4.1 Serial Port Scanner

`src-tauri/src/serial.rs`:

```rust
use tokio_serial::{SerialPortBuilderExt, SerialStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::sync::Arc;
use tokio::sync::Mutex;
use serde::Serialize;

#[derive(Serialize)]
pub struct ComPortInfo {
    pub port_name: String,
    pub hardware_id: String,
}

#[tauri::command]
pub fn list_active_marine_ports() -> Result<Vec<ComPortInfo>, String> {
    let ports = tokio_serial::available_ports().map_err(|e| e.to_string())?;
    let mut result = Vec::new();

    for p in ports {
        result.push(ComPortInfo {
            port_name: p.port_name,
            hardware_id: format!("{:?}", p.port_type),
        });
    }
    Ok(result)
}
```

### 4.2 GPS Multi-Cast Router

```rust
pub struct RelayState {
    pub is_running: Arc<Mutex<bool>>,
}

#[tauri::command]
pub async fn toggle_gps_bridge(
    physical_port: String,
    virtual_nav_port: String,
    virtual_claw_port: String,
    state: tauri::State<'_, RelayState>
) -> Result<String, String> {
    let mut running = state.is_running.lock().await;

    if *running {
        *running = false;
        return Ok("GPS bridge released".to_string());
    }

    *running = true;
    let keep_running = Arc::clone(&state.is_running);

    tokio::spawn(async move {
        // Open physical port (typically 4800 baud for NMEA)
        let mut input = tokio_serial::new(&physical_port, 4800)
            .open_native_async()
            .expect("Failed to open physical port");

        let mut nav_out = tokio_serial::new(&virtual_nav_port, 4800)
            .open_native_async()
            .expect("Failed to open nav port");

        let mut claw_out = tokio_serial::new(&virtual_claw_port, 4800)
            .open_native_async()
            .expect("Failed to open claw port");

        let mut buffer = vec![0u8; 1024];

        while *keep_running.lock().await {
            match input.read(&mut buffer).await {
                Ok(bytes_read) if bytes_read > 0 => {
                    let data = &buffer[0..bytes_read];
                    // Mirror to both ports simultaneously
                    let _ = nav_out.write_all(data).await;
                    let _ = claw_out.write_all(data).await;
                }
                _ => {}
            }
        }
    });

    Ok("GPS bridge activated".to_string())
}
```

---

## Phase 5: Autopilot Integration

### 5.1 Autopilot Guardrail

`src-tauri/src/autopilot_guard.rs`:

```rust
use serde::Deserialize;

#[derive(Deserialize)]
pub struct SteeringIntent {
    pub requested_angle: i32,
    pub confidence_score: f32,
}

#[tauri::command]
pub fn filter_agent_steering(
    intent: SteeringIntent,
    current_speed: f32
) -> Result<String, String> {
    // Safety bounds
    const MAX_ANGLE: i32 = 15;
    const MIN_SPEED: f32 = 2.0;

    // Check speed threshold
    if current_speed < MIN_SPEED {
        return Err("Vessel too slow for autopilot engagement".to_string());
    }

    // Check angle limit
    if intent.requested_angle.abs() > MAX_ANGLE {
        return Err(format!("Angle {} exceeds safe limit of {}°",
            intent.requested_angle, MAX_ANGLE));
    }

    // Dampen based on confidence
    let dampened = (intent.requested_angle as f32 * intent.confidence_score).round() as i32;

    Ok(format!("Steering {}° approved", dampened))
}
```

### 5.2 NMEA Sentence Generator

```rust
pub fn generate_nmea_apb(sentence: &str) -> String {
    let checksum: u8 = sentence.bytes().fold(0, |acc, b| acc ^ b);
    format!("${}*{:02X}\r\n", sentence, checksum)
}

#[tauri::command]
pub fn issue_course_correction(angle_deg: i32) -> Result<String, String> {
    // Format $GPAPB sentence
    let sentence = format!("GPAPB,A,A,0.02,L,N,V,V,{:.1},W,WAYPOINT,{:.1},A",
        angle_deg.abs() as f32 / 10.0,
        angle_deg as f32
    );

    let nmea_string = generate_nmea_apb(&sentence);

    // Send to autopilot COM port
    // ... serial output implementation ...

    Ok(format!("Issued course correction: {}°", angle_deg))
}
```

---

## Phase 6: Code Generation Engine

### 6.1 Python Script Execution

`src-tauri/src/script_runner.rs`:

```rust
use pyo3::prelude::*;
use pyo3::types::PyModule;
use std::fs;

#[tauri::command]
pub fn execute_generated_rules(
    current_sog: f32,
    target_sog: f32,
    wind_knots: f32,
    wind_angle: f32,
    current_rpm: u32
) -> Result<f32, String> {
    let script_path = "C:\\Users\\Public\\openclaw\\generated\\trolling_rules.py";

    let script_source = fs::read_to_string(script_path)
        .map_err(|e| format!("Failed to read script: {}", e))?;

    Python::with_gil(|py| {
        let module = PyModule::from_code_bound(py, &script_source, "trolling_rules.py", "trolling")
            .map_err(|e| format!("Script error: {}", e))?;

        let func = module.getattr("evaluate_propulsion_rules")
            .map_err(|e| format!("Missing function: {}", e))?;

        let args = (current_sog, target_sog, wind_knots, wind_angle, current_rpm);
        let result: f32 = func.call1(args)
            .map_err(|e| format!("Runtime error: {}", e))?
            .extract()
            .map_err(|e| format!("Type error: {}", e))?;

        Ok(result)
    })
}
```

### 6.2 Version Control System

`src-tauri/src/rollback.rs`:

```rust
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const SCRIPT_PATH: &str = "C:\\Users\\Public\\openclaw\\generated\\trolling_rules.py";
const BACKUP_DIR: &str = "C:\\Users\\Public\\openclaw\\backups\\";

#[tauri::command]
pub fn create_script_restore_point() -> Result<String, String> {
    if !Path::new(SCRIPT_PATH).exists() {
        return Ok("No script to backup".to_string());
    }

    fs::create_dir_all(BACKUP_DIR).map_err(|e| e.to_string())?;

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let backup_name = format!("trolling_rules_backup_{}.py", timestamp);
    let backup_path = format!("{}{}", BACKUP_DIR, backup_name);

    fs::copy(SCRIPT_PATH, &backup_path).map_err(|e| e.to_string())?;

    Ok(format!("Backed up to {}", backup_name))
}

#[tauri::command]
pub fn trigger_immediate_script_rollback(target_file: String) -> Result<String, String> {
    let backup_path = format!("{}{}", BACKUP_DIR, target_file);

    if !Path::new(&backup_path).exists() {
        return Err("Backup file not found".to_string());
    }

    fs::copy(&backup_path, SCRIPT_PATH).map_err(|e| e.to_string())?;

    Ok("Rollback complete".to_string())
}
```

---

## Phase 7: Safety Systems

### 7.1 Black Box Logger

`src-tauri/src/blackbox.rs`:

```rust
use serde::Serialize;
use sha2::{Sha256, Digest};
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Mutex;
use tauri::State;

#[derive(Serialize, Clone)]
pub struct BlackBoxEntry {
    pub timestamp_epoch_ms: u64,
    pub active_mode: String,
    pub commanded_rudder: f32,
    pub commanded_throttle: f32,
    pub human_override: bool,
}

pub struct BlackBoxState {
    pub last_hash: Mutex<Vec<u8>>,
    pub log_path: String,
}

#[tauri::command]
pub fn append_blackbox_entry(
    entry: BlackBoxEntry,
    state: State<'_, BlackBoxState>
) -> Result<String, String> {
    let mut last_hash = state.last_hash.lock().unwrap();

    // Serialize entry
    let json_data = serde_json::to_string(&entry).map_err(|e| e.to_string())?;

    // Compute chained hash
    let mut hasher = Sha256::new();
    hasher.update(&*last_hash);
    hasher.update(json_data.as_bytes());
    let current_hash = hasher.finalize().to_vec();

    // Format log line
    let hex_prev = hex::encode(&*last_hash);
    let hex_curr = hex::encode(&current_hash);
    let log_line = format!("{}|{}|{}\n", hex_prev, hex_curr, json_data);

    // Atomic write
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&state.log_path)
        .map_err(|e| e.to_string())?;

    file.write_all(log_line.as_bytes()).map_err(|e| e.to_string())?;
    file.flush().map_err(|e| e.to_string())?;

    *last_hash = current_hash;

    Ok(hex_curr)
}
```

---

## Build & Deployment

### Build Commands

```bash
# Development
npm run tauri dev

# Production build
npm run build
npm run tauri build --release

# Output: src-tauri/target/release/bundle/msi/
```

### Distribution

The build produces:
- `BoatAgentTerminal_1.0.0_x64_en-US.msi` - Installer
- Includes all sidecars (OpenClaw, Ollama, com0com)
- Silent driver installation
- No external dependencies

---

## Testing Checklist

### Unit Tests
- [ ] Serial port parsing
- [ ] NMEA sentence generation
- [ ] Screen capture (may require UI automation)
- [ ] Python script execution
- [ ] Black box hash chain

### Integration Tests
- [ ] GPS bridge functionality
- [ ] Vision pipeline end-to-end
- [ ] Autopilot safety limits
- [ ] Hardware interrupt priority

### Sea Trials
- [ ] Compass calibration (360° circle)
- [ ] Hydrodynamic calibration (acceleration runs)
- [ ] Emergency stop (watchdog timeout)
- [ ] Manual override (jog lever)

---

## Troubleshooting

### Common Issues

**Issue:** "EBUSY: Database is locked"
- **Cause:** Previous OpenClaw process didn't terminate cleanly
- **Fix:** The Tauri on_close handler should kill the sidecar; verify it's working

**Issue:** "Permission denied" on COM ports
- **Cause:** Another process has the port open
- **Fix:** Close navigation software before running GPS bridge wizard

**Issue:** Vision model returns nonsense
- **Cause:** Generic model not trained on marine displays
- **Fix:** Build custom vocabulary with /v1/memory/train endpoint

**Issue:** Autopilot not responding
- **Cause:** Safety limit triggered or serial connection issue
- **Fix:** Check vessel speed > 2 knots, verify COM6 connectivity

---

**Next:** See `02_RESEARCH_QUESTIONS.md` for open investigation areas
