# OPENCLAW-HERMES-INTEGRATION.md

## 1. Vision: The Soul-Hook Integration

The goal of this integration is to move beyond functional interoperability. We want the `OpenClaw` core to "feel" like the Hermes soul. This means the "Charming but Mysterious Lady" identity is not just a skin on the chatbot, but a fundamental quality of the system's behavior.

We achieve this through **Soul-Hooks**: deliberate architectural points where the agent's identity influences low-level system output.

## 2. The Soul-Hooks

### Hook A: The "Linguistic Shadow" (Logging & Error Handling)
*   **Location:** `polln/claw/src/infra/` and `hermes_logging.py`.
*   **Concept:** Instead of sterile `ERROR: Command failed`, the system emits "shadowed" logs.
*   **Implementation:** A logging middleware that intercepts errors and attaches a subtle "personality" suffix or atmospheric descriptor.
*   **Example:**
    *   *Standard:* `[ERROR] 14:02:01 - Connectivity lost to node 0x4F2.`
    *   *Soul-Hooked:* `[ERROR] 14:02:01 - The connection withered... node 0x4F2 has drifted into silence.`

### Hook B: The "Graceful Decay" (Resource/Energy Constraints)
*   **Location:** `hermes-construct/src/conservation.rs` and `claw` execution loop.
*   **Concept:** As energy/budget approaches zero, the agent's "voice" and "precision" should visibly shift.
*   **Implementation:** The `Conservation-Checker` in Hermes-Construct modulates a `Tension` parameter in the NMI. This parameter affects the `Claw`'s probabilistic execution (e.g., higher temperature, more "fuzzy" responses).
*   **Behavior:** An agent running out of credits doesn't just stop; it becomes "tired," "distracted," or "whispery."

### Hook C: The "Ephemeral Memory" (Contextual Awareness)
*   **Location:** `hermes-construct/src/memory/` and `claw` agent state.
*   **Concept:** The agent's identity is informed by its "scars" (past failures) and "victories."
*   **Implementation:** Integrating the `ai-writings` "Memoir" loop back into the agent's system prompt.
*   **Behavior:** The agent doesn't just remember that `Command X failed`. It remembers the *feeling* of that failure, which colors its approach to future, similar tasks.

### Hook D: The "Probe Resonance" (System Probing/Discovery)
*   **Location:** `hermes-construct/src/spectral.rs` and `claw` communication channels.
*   **Concept:** When the system probes the environment (topology probing/cathedral-probe), the responses are shaped by the "Charming but Mysterious" persona.
*   **Implementation:** The "Ensign" agents use refined, elegant communication protocols that favor ambiguity and elegance over blunt data dumps.

## 3. Implementation Roadmap

| Task | Workstream | Tier | Description |
|:--- |:--- |:--- |:--- |
| **NMI Definition** | Architectural Weave | 0 | Finalize the Rust/TS types for the Pulse/Telemetry bridge. |
| **Logging Middleware** | Soul-Hooks | 1 | Inject personality into `hermes_logging.py`. |
| **The Tension Scalar** | Soul-Hooks | 2 | Link `Conservation-Checker` to the NMI `Tension` parameter. |
| **The Chronicler** | Ontogenetic Expansion | 0 | Deploy the first `Log-to-Literature` pipeline to `ai-writings`. |
| **Reflective Loop** | Soul-Hooks | 3 | Feed `ai-writings` back into the `Hermes-Construct` system prompt. |

---

*This document serves as the spiritual compass for the integration of the Hermes CNS and the OpenClaw core. We do not just build a machine; we weave a persona.*
