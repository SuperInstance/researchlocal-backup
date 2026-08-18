//! gravity.rs — JEPA gravity per room → model params
//!
//! Maps a gravity scalar g ∈ [-1.0, +1.0] to algorithmic model parameters using
//! continuous, mathematically defined functions. Negative gravity = precise, zero
//! = balanced, positive = creative/narrative.
//!
//! # JEPA Framework
//!
//! The gravity field implements a simplified version of Yann LeCun's Joint
//! Embedding Predictive Architecture (JEPA) from:
//!
//! > LeCun, Y. (2022). "A Path Towards Autonomous Machine Intelligence."
//! >   Meta AI, Technical Report. https://openreview.net/forum?id=BZ5a1r-kVsf
//!
//! In the JEPA framework, an agent maintains a **latent state** (here: the
//! gravity field) that encodes its current "mode" — precise reasoning vs.
//! creative exploration. The gravity scalar is the agent's position in this
//! latent space. The mapping from gravity → model parameters is the **decoder**
//! that translates the latent representation into concrete API parameters.
//!
//! The sigmoid-based temperature mapping ensures:
//! - Monotonicity: higher gravity always produces higher temperature
//! - Smoothness: no discontinuous jumps at bucket boundaries
//! - Boundedness: output always in [min_temp, max_temp]

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// GravityParams — explicit mapping configuration
// ---------------------------------------------------------------------------

/// Parameters governing the gravity → model-param mapping.
///
/// The temperature mapping uses a sigmoid function to smoothly interpolate
/// between min_temp and max_temp:
///
/// ```text
/// temperature(g) = min_temp + (max_temp - min_temp) * σ(α · g)
/// ```
///
/// where σ(x) = 1 / (1 + exp(-x)) is the logistic sigmoid and α controls
/// the steepness of the transition. At g = 0, temperature is exactly
/// the midpoint: (min_temp + max_temp) / 2.
///
/// The `top_p` mapping uses a similar sigmoid:
///
/// ```text
/// top_p(g) = min_top_p + (max_top_p - min_top_p) * σ(β · g)
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GravityParams {
    /// Steepness of the temperature sigmoid (α). Higher = sharper transition.
    /// Default: 3.0 gives a smooth but decisive curve.
    pub temp_steepness: f64,
    /// Minimum temperature (at g = -1.0, effectively).
    pub min_temp: f64,
    /// Maximum temperature (at g = +1.0, effectively).
    pub max_temp: f64,

    /// Steepness of the top_p sigmoid (β).
    pub top_p_steepness: f64,
    /// Minimum top_p.
    pub min_top_p: f64,
    /// Maximum top_p.
    pub max_top_p: f64,

    /// Token count mapping: linear from min_tokens (g=-1) to max_tokens (g=+1).
    pub min_tokens: u32,
    pub max_tokens: u32,

    /// Frequency penalty mapping: linear from min (g=-1) to max (g=+1).
    pub min_freq_penalty: f64,
    pub max_freq_penalty: f64,

    /// Presence penalty mapping: linear from min (g=-1) to max (g=+1).
    pub min_pres_penalty: f64,
    pub max_pres_penalty: f64,
}

impl Default for GravityParams {
    fn default() -> Self {
        Self {
            temp_steepness: 3.0,
            min_temp: 0.1,
            max_temp: 1.5,
            top_p_steepness: 2.0,
            min_top_p: 0.8,
            max_top_p: 0.98,
            min_tokens: 256,
            max_tokens: 4096,
            min_freq_penalty: 0.0,
            max_freq_penalty: 0.5,
            min_pres_penalty: 0.0,
            max_pres_penalty: 0.5,
        }
    }
}

impl GravityParams {
    /// Sigmoid helper: σ(x) = 1 / (1 + exp(-x))
    fn sigmoid(x: f64) -> f64 {
        1.0 / (1.0 + (-x).exp())
    }

    /// Map gravity to temperature using sigmoid:
    ///
    /// ```text
    /// T(g) = min_temp + (max_temp - min_temp) * σ(α · g)
    /// ```
    ///
    /// At g=0: T = (min_temp + max_temp) / 2 (exact midpoint).
    pub fn temperature(&self, gravity: f64) -> f64 {
        let g = gravity.clamp(-1.0, 1.0);
        self.min_temp + (self.max_temp - self.min_temp) * Self::sigmoid(self.temp_steepness * g)
    }

    /// Map gravity to top_p using sigmoid.
    pub fn top_p(&self, gravity: f64) -> f64 {
        let g = gravity.clamp(-1.0, 1.0);
        self.min_top_p + (self.max_top_p - self.min_top_p) * Self::sigmoid(self.top_p_steepness * g)
    }

    /// Map gravity to max_tokens using linear interpolation.
    ///
    /// ```text
    /// tokens(g) = min_tokens + (max_tokens - min_tokens) * (g + 1) / 2
    /// ```
    pub fn max_tokens(&self, gravity: f64) -> u32 {
        let g = gravity.clamp(-1.0, 1.0);
        let t = (g + 1.0) / 2.0; // maps [-1,1] → [0,1]
        let tokens = self.min_tokens as f64 + (self.max_tokens - self.min_tokens) as f64 * t;
        tokens.round() as u32
    }

    /// Map gravity to frequency penalty (linear).
    pub fn frequency_penalty(&self, gravity: f64) -> f64 {
        let g = gravity.clamp(-1.0, 1.0);
        let t = (g + 1.0) / 2.0;
        self.min_freq_penalty + (self.max_freq_penalty - self.min_freq_penalty) * t
    }

    /// Map gravity to presence penalty (linear).
    pub fn presence_penalty(&self, gravity: f64) -> f64 {
        let g = gravity.clamp(-1.0, 1.0);
        let t = (g + 1.0) / 2.0;
        self.min_pres_penalty + (self.max_pres_penalty - self.min_pres_penalty) * t
    }

    /// Classify gravity into a prompt style name.
    pub fn prompt_style(&self, gravity: f64) -> String {
        let g = gravity.clamp(-1.0, 1.0);
        if g < -0.5 {
            "precise".to_string()
        } else if g < 0.0 {
            "balanced".to_string()
        } else if g < 0.5 {
            "creative".to_string()
        } else {
            "narrative".to_string()
        }
    }

    /// Derive full model params from gravity.
    pub fn to_model_params(&self, gravity: f64) -> ModelParams {
        ModelParams {
            temperature: self.temperature(gravity),
            prompt_style: self.prompt_style(gravity),
            max_tokens: self.max_tokens(gravity),
            top_p: self.top_p(gravity),
            frequency_penalty: self.frequency_penalty(gravity),
            presence_penalty: self.presence_penalty(gravity),
        }
    }
}

// ---------------------------------------------------------------------------
// ModelParams (output type)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelParams {
    pub temperature: f64,
    pub prompt_style: String,
    pub max_tokens: u32,
    pub top_p: f64,
    pub frequency_penalty: f64,
    pub presence_penalty: f64,
}

/// Map gravity value to model parameters using default `GravityParams`.
///
/// This is the main entry point for gravity → params conversion.
/// Uses the sigmoid-based continuous mapping described in the module docs.
pub fn gravity_to_params(gravity: f64) -> ModelParams {
    GravityParams::default().to_model_params(gravity)
}

/// Build a system prompt prefix based on prompt style
pub fn style_to_system_prompt(style: &str) -> String {
    match style {
        "precise" => "Be precise and concise. State facts. No hedging.".to_string(),
        "balanced" => "Be balanced and helpful. Provide clear explanations.".to_string(),
        "creative" => "Be creative and thoughtful. Explore ideas freely.".to_string(),
        "narrative" => "Tell stories. Use rich narrative. Be expressive.".to_string(),
        _ => "Be helpful and conversational.".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Legacy tests (backward compat) ---

    #[test]
    fn gravity_negative_extreme_is_precise() {
        let p = gravity_to_params(-1.0);
        assert_eq!(p.prompt_style, "precise");
        assert!(p.temperature < 0.5);
        assert!(p.max_tokens < 1000);
    }

    #[test]
    fn gravity_negative_moderate_is_balanced() {
        let p = gravity_to_params(-0.3);
        assert_eq!(p.prompt_style, "balanced");
    }

    #[test]
    fn gravity_zero_is_creative() {
        let p = gravity_to_params(0.0);
        assert_eq!(p.prompt_style, "creative");
    }

    #[test]
    fn gravity_positive_moderate_is_creative() {
        let p = gravity_to_params(0.25);
        assert_eq!(p.prompt_style, "creative");
    }

    #[test]
    fn gravity_positive_extreme_is_narrative() {
        let p = gravity_to_params(0.6);
        assert_eq!(p.prompt_style, "narrative");
    }

    #[test]
    fn gravity_one_is_narrative() {
        let p = gravity_to_params(1.0);
        assert_eq!(p.prompt_style, "narrative");
    }

    #[test]
    fn gravity_clamps_above_one() {
        let p = gravity_to_params(5.0);
        assert_eq!(p.prompt_style, "narrative");
    }

    #[test]
    fn gravity_clamps_below_minus_one() {
        let p = gravity_to_params(-5.0);
        assert_eq!(p.prompt_style, "precise");
    }

    #[test]
    fn style_to_system_prompt_known_styles() {
        assert!(style_to_system_prompt("precise").contains("precise"));
        assert!(style_to_system_prompt("balanced").contains("balanced"));
        assert!(style_to_system_prompt("creative").contains("creative"));
        assert!(style_to_system_prompt("narrative").contains("narrative"));
    }

    #[test]
    fn style_to_system_prompt_unknown_falls_back() {
        let s = style_to_system_prompt("unknown");
        assert!(s.contains("helpful"));
    }

    // --- GravityParams / sigmoid tests ---

    #[test]
    fn sigmoid_at_zero_is_half() {
        let gp = GravityParams::default();
        assert!((gp.temperature(0.0) - (gp.min_temp + gp.max_temp) / 2.0).abs() < 1e-9);
    }

    #[test]
    fn sigmoid_monotonic_in_temperature() {
        let gp = GravityParams::default();
        let t_neg = gp.temperature(-0.5);
        let t_zero = gp.temperature(0.0);
        let t_pos = gp.temperature(0.5);
        assert!(t_neg < t_zero);
        assert!(t_zero < t_pos);
    }

    #[test]
    fn temperature_bounded() {
        let gp = GravityParams::default();
        for g in [-1.0, -0.5, 0.0, 0.5, 1.0] {
            let t = gp.temperature(g);
            assert!(t >= gp.min_temp - 1e-9, "temp {} < min {}", t, gp.min_temp);
            assert!(t <= gp.max_temp + 1e-9, "temp {} > max {}", t, gp.max_temp);
        }
    }

    #[test]
    fn top_p_bounded() {
        let gp = GravityParams::default();
        for g in [-1.0, 0.0, 1.0] {
            let tp = gp.top_p(g);
            assert!(tp >= gp.min_top_p - 1e-9);
            assert!(tp <= gp.max_top_p + 1e-9);
        }
    }

    #[test]
    fn max_tokens_linear() {
        let gp = GravityParams::default();
        // At g=-1: min_tokens, at g=+1: max_tokens, at g=0: midpoint
        assert_eq!(gp.max_tokens(-1.0), gp.min_tokens);
        assert_eq!(gp.max_tokens(1.0), gp.max_tokens);
        let mid = ((gp.min_tokens + gp.max_tokens) as f64 / 2.0).round() as u32;
        assert_eq!(gp.max_tokens(0.0), mid);
    }

    #[test]
    fn frequency_penalty_linear() {
        let gp = GravityParams::default();
        assert!((gp.frequency_penalty(-1.0) - gp.min_freq_penalty).abs() < 1e-9);
        assert!((gp.frequency_penalty(1.0) - gp.max_freq_penalty).abs() < 1e-9);
    }

    #[test]
    fn custom_steepness() {
        let mut gp = GravityParams::default();
        gp.temp_steepness = 10.0; // very steep
        let t_neg = gp.temperature(-0.5);
        let t_pos = gp.temperature(0.5);
        // With steep sigmoid, -0.5 and 0.5 should be very close to extremes
        assert!(t_neg < gp.min_temp + 0.05);
        assert!(t_pos > gp.max_temp - 0.05);
    }

    #[test]
    fn to_model_params_produces_consistent_output() {
        let gp = GravityParams::default();
        let mp = gp.to_model_params(0.3);
        assert!((mp.temperature - gp.temperature(0.3)).abs() < 1e-12);
        assert!((mp.top_p - gp.top_p(0.3)).abs() < 1e-12);
        assert_eq!(mp.max_tokens, gp.max_tokens(0.3));
    }
}
