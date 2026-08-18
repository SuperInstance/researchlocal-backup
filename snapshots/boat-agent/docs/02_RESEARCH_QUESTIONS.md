# Boat Agent System - Research Questions & Insights

> **Target Audience:** Research agents investigating system improvements
> **Purpose:** Document open questions, investigation areas, and insights
> **Status:** Active Research Log

---

## Priority Framework

- 🔴 **P0 - Critical:** Safety or core functionality impact
- 🟡 **P1 - High:** Significant user experience or performance impact
- 🟢 **P2 - Medium:** Feature enhancement or optimization opportunity
- ⚪ **P3 - Low:** Nice-to-have or exploratory

---

## 🔴 P0 - Critical Research Questions

### RQ-001: Time-Synchronization Precision in Multi-Modal Learning

**Question:** What is the actual precision of word-level timestamps from Whisper API when aligned with telemetry data?

**Context:** The system aligns captain voice transcripts with telemetry snapshots to learn vessel behaviors. If timestamps are misaligned by >500ms, the AI may associate explanations with wrong vessel states.

**Investigation Needed:**
1. Measure Whisper timestamp error vs ground truth (controlled test)
2. Characterize error distribution (mean, std, outliers)
3. Determine minimum alignment confidence threshold

**Potential Impact:**
- Worst case: Garbage training data → confused AI → unsafe behavior
- Best case: Training quality improved by 40%+

**Approach:**
- Create test dataset with known alignment
- Run through Whisper API with `timestamp=word`
- Measure offset from expected timestamps
- Plot error distribution

**Success Criteria:**
- 95% of words aligned within ±250ms
- Clear threshold for discarding ambiguous alignments

---

### RQ-002: Sonar Vision Model Failure Modes

**Question:** What are the failure modes of generic vision models (LLaVA) on marine echogram displays?

**Context:** Generic models trained on internet images may misinterpret:
- Gain changes (screen adjustments) as biomass
- Surface clutter as fish schools
- Bottom hardness as interference

**Investigation Needed:**
1. Collect dataset of real echogram images with expert labels
2. Test LLaVA on this dataset
3. Categorize error types
4. Quantify false positive/negative rates

**Potential Impact:**
- Worst case: False fish leads → wasted fuel/captain distrust
- Best case: Fine-tuned model improves accuracy 3x

**Approach:**
- Partner with 3-4 vessels to collect labeled data
- Build confusion matrix of model predictions vs expert labels
- Identify systematic errors

**Success Criteria:**
- Identify 5+ systematic error categories
- False positive rate < 15% on test set
- Clear path to fine-tuning or calibration

---

### RQ-003: PID Stability Across Sea States

**Question:** Can a single set of PID parameters provide stable control across all sea states?

**Context:** Hydrodynamic response varies dramatically between calm seas and 6-foot swells. Fixed PID may oscillate in rough conditions.

**Investigation Needed:**
1. Characterize vessel response in different sea states
2. Measure PID stability margins (Bode plots)
3. Test gain scheduling approaches

**Potential Impact:**
- Worst case: Oscillation → gear wear → passenger discomfort
- Best case: Adaptive control improves smoothness 50%

**Approach:**
- Sea trial data collection (calm vs moderate vs rough)
- System identification per condition
- Stability analysis per condition

**Success Criteria:**
- Quantify stability margin degradation
- Recommend adaptive approach if margins vary > 30%

---

### RQ-004: Sensor Fusion Algorithm Selection

**Question:** What is the optimal Kalman filter topology for fusing GPS, compass, and rudder position data?

**Context:** Multiple sensors with different noise characteristics need fusion for state estimation.

**Investigation Needed:**
1. Compare EKF vs UKF vs Particle Filter
2. Test with real vessel telemetry
3. Measure estimation error vs computational cost

**Potential Impact:**
- Worst case: Poor fusion → jerky control → mechanical wear
- Best case: Smooth control → extended component life

**Approach:**
- Implement all three filter types
- Run on logged sea trial data
- Compare RMSE and runtime

**Success Criteria:**
- Select filter with best RMSE/runtime tradeoff
- Document tuning procedure

---

## 🟡 P1 - High Priority Research Questions

### RQ-005: Vocabulary Update Convergence

**Question:** How many reinforcement learning iterations are needed for the vocabulary system to stabilize?

**Context:** The /v1/memory/train endpoint updates word mappings. Need to understand convergence rate.

**Investigation Needed:**
1. Simulate vessel operations with varying update frequencies
2. Measure vocabulary size over time
3. Identify when vocabulary stabilizes

**Potential Impact:**
- Worst case: Unbounded vocabulary → degraded performance
- Best case: Know when to lock vocabulary

**Approach:**
- Build simulation with synthetic captain feedback
- Run for 100+ iterations
- Plot vocabulary growth

**Success Criteria:**
- Identify convergence pattern
- Recommend lock threshold

---

### RQ-006: Compression Quality Threshold

**Question:** What is the minimum image quality that preserves sonar feature detectability?

**Context:** We compress to <15KB for satellite links. Need to find optimal compression.

**Investigation Needed:**
1. Test compression levels (PNG quality 1-100)
2. Measure file size vs feature preservation
3. Find sweet spot

**Potential Impact:**
- Worst case: Over-compression → lost features → missed fish
- Best case: Optimize for 50% bandwidth reduction

**Approach:**
- Collect reference sonar images
- Compress at various levels
- Run vision model on each
- Compare detections

**Success Criteria:**
- Find minimum quality with <5% detection loss
- Document bandwidth savings

---

### RQ-007: Code Generation Safety Verification

**Question:** How can we prove generated Python scripts are safe before execution?

**Context:** AI-generated code could have infinite loops, overflow, or unsafe values.

**Investigation Needed:**
1. Research static analysis tools for Python
2. Define safety assertion language
3. Build verification pipeline

**Potential Impact:**
- Worst case: Unsafe code executes → vessel damage
- Best case: Formal verification of safety properties

**Approach:**
- Research Python AST analysis
- Design assertion DSL
- Implement pre-execution checker

**Success Criteria:**
- Catch 90% of unsafe patterns
- Minimal false positives

---

### RQ-008: Compass Deviation Calibration Frequency

**Question:** How often should compass deviation calibration be repeated?

**Context:** Magnetic deviation changes with equipment additions, cargo, heading.

**Investigation Needed:**
1. Measure deviation change over time
2. Identify significant factors
3. Recommend calibration schedule

**Potential Impact:**
- Worst case: Uncorrected deviation → navigation errors
- Best case: Optimal calibration interval

**Approach:**
- Track deviation over season
- Correlate with equipment changes
- Model degradation

**Success Criteria:**
- Quantify degradation rate
- Recommend calibration interval

---

## 🟢 P2 - Medium Priority Research Questions

### RQ-009: Human Override Detection Latency

**Question:** What is the minimum detectable human input latency?

**Context:** Need to distinguish intentional override from vibration/jitter.

**Investigation Needed:**
1. Measure natural joystick vibration at sea
2. Characterize human input patterns
3. Set detection threshold

**Potential Impact:**
- Improved UX, fewer false overrides

---

### RQ-010: Black Box Storage Efficiency

**Question:** What compression ratio can we achieve for black box logs without losing verifiability?

**Context:** Cryptographic logs grow indefinitely. Need efficient storage.

**Investigation Needed:**
1. Test compression algorithms
2. Verify hash chain integrity post-compression
3. Measure space savings

**Potential Impact:**
- Extended log retention, reduced storage cost

---

### RQ-011: Multi-Vessel Fleet Learning

**Question:** Can vessel-specific rules be generalized across a fleet?

**Context:** Each vessel learns from its captain. Can we share patterns?

**Investigation Needed:**
1. Identify common patterns across vessels
2. Test transfer learning
3. Quantify benefit

**Potential Impact:**
- Faster onboarding for new vessels

---

### RQ-012: Offline Model Performance

**Question:** How does local Ollama (llama3.2:3b) compare to cloud DeepSeek for sonar interpretation?

**Context:** Need to understand offline capability tradeoffs.

**Investigation Needed:**
1. Benchmark both models on same prompts
2. Compare accuracy, latency, quality
3. Document tradeoffs

**Potential Impact:**
- Informed offline mode design decisions

---

## ⚪ P3 - Low Priority / Exploratory

### RQ-013: Captain Voice Recognition

**Question:** Can we identify individual captains from voice patterns?

**Potential Use:** Personalized rules per captain

---

### RQ-014: Fish Species Prediction

**Question:** Can sonar features predict likely fish species?

**Potential Use:** Target species recommendations

---

### RQ-015: Weather Route Optimization

**Question:** Can the system suggest routes based on weather forecasts?

**Potential Use:** Fuel efficiency, safety

---

## Insights From Development

### Insight-001: Hardware Watchdog is Non-Negotiable

**Observation:** During development, USB disconnections caused laptop freeze detection to fail.

**Implication:** 800ms heartbeat timeout is too long for reliable safety. Consider 300ms.

**Status:** ⚠️ Requires testing

---

### Insight-002: NMEA Sentences Must Include Checksum

**Observation:** Autopilot rejects sentences without valid XOR checksum.

**Implication:** Every NMEA generator must implement checksum calculation.

**Status:** ✅ Documented in implementation guide

---

### Insight-003: Captain Prefers "Coaching" Over "Autopilot"

**Observation:** Test captains want to coach the AI, not fully delegate.

**Implication:** UI should emphasize active supervision mode over full autonomy.

**Status:** ⚠️ Needs user testing

---

### Insight-004: Vocabulary Training is Critical for Trust

**Observation:** Captains quickly distrust system that uses generic terms ("biomass") vs their terms ("feed layer").

**Implication:** Vocabulary training should be first-time-onboarding flow.

**Status:** ⚠️ Needs UI implementation

---

### Insight-005: Serial Port Lock is #1 Support Issue

**Observation:** Navigation software locking COM port causes most setup failures.

**Implication:** GPS bridge wizard needs clearer instructions and auto-retry.

**Status:** ⚠️ Needs UX improvement

---

## Experiment Proposals

### EXP-001: Controlled Sea Trial for Alignment

**Objective:** Measure time synchronization accuracy

**Setup:**
1. Generate synthetic telemetry + known voice timestamps
2. Run through full pipeline
3. Measure final alignment error

**Resources:** 1 day, test vessel or simulator

---

### EXP-002: Vision Model A/B Testing

**Objective:** Compare LLaVA vs fine-tuned model

**Setup:**
1. Collect 100 labeled sonar images
2. Test both models
3. Compare accuracy

**Resources:** 1 week, partnership with fishing vessel

---

### EXP-003: PID Tuning Sensitivity Analysis

**Objective:** Map PID stability vs parameters

**Setup:**
1. System ID on test vessel
2. Vary PID parameters
3. Measure response in different conditions

**Resources:** 3 days, calm + rough water testing

---

## Literature Review

### Recommended Reading

1. **Marine Autopilot Control**
   - "Robust Adaptive Control for Marine Surface Vessels" (IEEE TMC)
   - Focus: Wave disturbance rejection

2. **Multi-Modal Learning**
   - "ImageBind" (Meta Research)
   - Focus: Cross-modal alignment techniques

3. **Behavioral Cloning**
   - "Learning from Human Preferences" (OpenAI)
   - Focus: RLHF for control systems

4. **Sensor Fusion**
   - "Kalman Filtering for Marine Navigation" (Springer)
   - Focus: Practical implementation

---

## Open Decisions Requiring Research

| Decision | Options | Research Needed | Target Date |
|----------|---------|------------------|-------------|
| Vision model | LLaVA vs custom | RQ-002, EXP-002 | Q2 |
| PID approach | Fixed vs adaptive | RQ-003, EXP-003 | Q2 |
| Sensor fusion | EKF vs UKF | RQ-004 | Q2 |
| Alignment threshold | ±100ms vs ±250ms | RQ-001, EXP-001 | Q1 |
| Calibration interval | Monthly vs quarterly | RQ-008 | Q3 |

---

## Research Log

**2024-01-15:** Initiated research document. Identified 15 research questions.

**TODO:**
- [ ] Schedule EXP-001 (alignment testing)
- [ ] Reach out to vessel operators for EXP-002 (vision data)
- [ ] Set up simulator for EXP-003 (PID tuning)

---

**Next:** See `00_ARCHITECTURE_OVERVIEW.md` for system context
