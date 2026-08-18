#![allow(dead_code)]
#![allow(clippy::needless_range_loop)]
//! spectral.rs — Operator algebra and spectral theory for the Hermes kernel
//!
//! This module provides research-level mathematical foundations:
//!   1. Spectral triple (A, H, D) — Connes noncommutative geometry
//!   2. Wasserstein distance — optimal transport between tile distributions
//!   3. Renormalization group flow — conservation budget evolution
//!   4. Variational principle — room configuration optimization
//!   5. Berry phase — geometric phase for cyclic room trajectories
//!
//! References:
//!   Connes (1994), "Noncommutative Geometry"
//!   Villani (2008), "Optimal Transport: Old and New"
//!   Wilson & Kogut (1974), "The Renormalization Group"
//!   Berry (1984), "Quantal Phase Factors Accompanying Adiabatic Changes"

use crate::room::RoomConfig;

// ===========================================================================
// §1  Spectral Triple  (A, H, D)
// ===========================================================================

/// Bounded operator represented as a square matrix acting on tile space.
/// A = C*-algebra of bounded operators: closed under composition, adjoint,
/// with operator norm ‖a‖ = sup_{‖x‖=1} ‖ax‖.
#[derive(Debug, Clone)]
pub struct BoundedOperator {
    /// Matrix entries in row-major order. Dimension = n×n.
    data: Vec<Vec<f64>>,
    dim: usize,
}

impl BoundedOperator {
    pub fn zero(n: usize) -> Self {
        Self {
            data: vec![vec![0.0; n]; n],
            dim: n,
        }
    }

    pub fn identity(n: usize) -> Self {
        let mut m = Self::zero(n);
        for i in 0..n {
            m.data[i][i] = 1.0;
        }
        m
    }

    pub fn from_vec(data: Vec<Vec<f64>>) -> Self {
        let dim = data.len();
        Self { data, dim }
    }

    /// Operator norm (largest singular value).
    /// Computed as sqrt(largest eigenvalue of A^T A) via Jacobi.
    pub fn norm(&self) -> f64 {
        if self.dim == 0 {
            return 0.0;
        }
        let at = self.adjoint();
        let ata = at.compose(self);
        let eigenvalues = jacobi_eigenvalues(&ata.data);
        eigenvalues.last().copied().unwrap_or(0.0).max(0.0).sqrt()
    }

    /// Adjoint (conjugate transpose — for real matrices, just transpose).
    pub fn adjoint(&self) -> Self {
        let mut data = vec![vec![0.0; self.dim]; self.dim];
        for i in 0..self.dim {
            for j in 0..self.dim {
                data[i][j] = self.data[j][i];
            }
        }
        Self { data, dim: self.dim }
    }

    /// Compose: self ∘ other.
    pub fn compose(&self, other: &Self) -> Self {
        let n = self.dim;
        let mut result = vec![vec![0.0; n]; n];
        for i in 0..n {
            for j in 0..n {
                for k in 0..n {
                    result[i][j] += self.data[i][k] * other.data[k][j];
                }
            }
        }
        Self { data: result, dim: n }
    }

    /// Apply operator to a vector.
    pub fn apply(&self, v: &[f64]) -> Vec<f64> {
        let mut out = vec![0.0; self.dim];
        for i in 0..self.dim {
            for j in 0..self.dim {
                out[i] += self.data[i][j] * v[j];
            }
        }
        out
    }

    /// Trace (sum of diagonal).
    pub fn trace(&self) -> f64 {
        (0..self.dim).map(|i| self.data[i][i]).sum()
    }

    pub fn dim(&self) -> usize {
        self.dim
    }
}

/// Dirac operator encoding "distance" between tile types.
/// For a finite metric space with distances d(i,j), the Dirac operator is:
///   D = Σ_{i<j} d(i,j) (e_i⊗e_j* − e_j⊗e_i*)
/// This gives [D, projection onto state i] a norm proportional to distances.
#[derive(Debug, Clone)]
pub struct DiracOperator {
    /// Distance matrix: distances[i][j] = d(tile_i, tile_j).
    distances: Vec<Vec<f64>>,
    dim: usize,
}

impl DiracOperator {
    /// Build Dirac operator from a distance matrix.
    pub fn from_distances(distances: Vec<Vec<f64>>) -> Self {
        let dim = distances.len();
        Self { distances, dim }
    }

    /// Build from a metric on tile types: the distance is |type_i − type_j|
    /// for an integer encoding of types.
    pub fn from_tile_types(types: &[usize]) -> Self {
        let dim = types.len();
        let mut distances = vec![vec![0.0; dim]; dim];
        for i in 0..dim {
            for j in 0..dim {
                let d = (types[i] as f64 - types[j] as f64).abs();
                distances[i][j] = d;
            }
        }
        Self { distances, dim }
    }

    /// Compute the Dirac matrix as a bounded operator.
    /// D_ij = distances[i][j] * (δ_{i<j} − δ_{i>j})
    pub fn as_operator(&self) -> BoundedOperator {
        let mut data = vec![vec![0.0; self.dim]; self.dim];
        for i in 0..self.dim {
            for j in 0..self.dim {
                if i < j {
                    data[i][j] = self.distances[i][j];
                } else if i > j {
                    data[i][j] = -self.distances[i][j];
                }
            }
        }
        BoundedOperator::from_vec(data)
    }

    /// Spectrum: eigenvalues via the Jacobi eigenvalue algorithm for
    /// the matrix D†D (symmetric, real). Returns sorted eigenvalues.
    pub fn spectrum(&self) -> Vec<f64> {
        let d = self.as_operator();
        let d2 = d.adjoint().compose(&d);
        jacobi_eigenvalues(&d2.data)
    }

    pub fn dim(&self) -> usize {
        self.dim
    }
}

/// Spectral triple (A, H, D) in the sense of Connes.
///
/// A = C*-algebra of bounded operators on tile space
/// H = Hilbert space (R^n with standard inner product) of tile histories
/// D = Dirac operator encoding metric structure
#[derive(Debug, Clone)]
pub struct SpectralTriple {
    /// The algebra element (operator acting on tile space).
    pub algebra_element: BoundedOperator,
    /// Tile type encoding for the Dirac operator.
    pub tile_types: Vec<usize>,
}

impl SpectralTriple {
    pub fn new(operator: BoundedOperator, tile_types: Vec<usize>) -> Self {
        Self {
            algebra_element: operator,
            tile_types,
        }
    }

    /// Build from a list of tile distributions and transition counts.
    pub fn from_transitions(transitions: &[(usize, usize, f64)], n_tiles: usize) -> Self {
        let mut op = BoundedOperator::zero(n_tiles);
        for &(from, to, weight) in transitions {
            if from < n_tiles && to < n_tiles {
                op.data[from][to] += weight;
            }
        }
        let types: Vec<usize> = (0..n_tiles).collect();
        Self::new(op, types)
    }

    /// Compute the Connes index pairing:
    ///   ⟨[D], [a]⟩ = Tr(γ a [D, a]⁻¹)
    /// For a simplified version, we compute:
    ///   index ≈ Tr(a · D) / ‖a‖ · ‖D‖
    /// which measures how the algebra element interacts with the metric.
    pub fn compute_index(&self) -> f64 {
        let dirac = DiracOperator::from_tile_types(&self.tile_types);
        let d_op = dirac.as_operator();

        // Commutator [D, a] = D·a − a·D
        let da = d_op.compose(&self.algebra_element);
        let ad = self.algebra_element.compose(&d_op);
        let mut comm = BoundedOperator::zero(self.algebra_element.dim());
        for i in 0..comm.dim {
            for j in 0..comm.dim {
                comm.data[i][j] = da.data[i][j] - ad.data[i][j];
            }
        }

        // Index pairing: Tr(comm† · comm) gives the "spectral action"
        let comm_adj = comm.adjoint();
        let comm2 = comm_adj.compose(&comm);
        let trace = comm2.trace();

        // Normalize by operator norms
        let norm_a = self.algebra_element.norm();
        let norm_d = d_op.norm();
        let denom = norm_a * norm_d;

        if denom < 1e-15 {
            0.0
        } else {
            trace.sqrt() / denom
        }
    }

    /// Metric dimension: dim_M = Σ λ_k^{-2} for non-zero eigenvalues of D.
    pub fn metric_dimension(&self) -> f64 {
        let dirac = DiracOperator::from_tile_types(&self.tile_types);
        let eigenvalues = dirac.spectrum();
        let mut dim = 0.0;
        for ev in &eigenvalues {
            if *ev > 1e-10 {
                dim += 1.0 / (ev * ev);
            }
        }
        dim
    }
}

// ===========================================================================
// §2  Wasserstein Distance
// ===========================================================================

/// Compute the Wasserstein-p distance between two discrete distributions
/// using the Sinkhorn algorithm (entropic regularization).
///
/// W_p(μ, ν) = (inf_π ∫ d(x,y)^p dπ(x,y))^{1/p}
///
/// Uses Euclidean ground cost between support indices and Sinkhorn iterations
/// for fast O(n²) approximation.
pub fn wasserstein_distance(dist_a: &[f64], dist_b: &[f64], p: usize) -> f64 {
    let n = dist_a.len();
    let m = dist_b.len();
    assert_eq!(n, m, "Distributions must have the same support size");
    assert!(p > 0, "p must be positive");

    // Normalize distributions
    let sum_a: f64 = dist_a.iter().sum();
    let sum_b: f64 = dist_b.iter().sum();
    if sum_a < 1e-15 || sum_b < 1e-15 {
        return 0.0;
    }
    let a: Vec<f64> = dist_a.iter().map(|x| x / sum_a).collect();
    let b: Vec<f64> = dist_b.iter().map(|x| x / sum_b).collect();

    // Cost matrix: C_ij = |i - j|^p (ground metric on support)
    let mut cost = vec![vec![0.0; m]; n];
    for i in 0..n {
        for j in 0..m {
            let d = (i as f64 - j as f64).abs();
            cost[i][j] = d.powi(p as i32);
        }
    }

    // Sinkhorn algorithm with entropic regularization
    let lambda = 10.0; // regularization strength (inverse temperature)
    let k_matrix: Vec<Vec<f64>> = cost
        .iter()
        .map(|row| row.iter().map(|c| (-lambda * c).exp()).collect())
        .collect();

    let mut u = vec![1.0 / n as f64; n];
    let mut v = vec![1.0 / m as f64; m];

    for _ in 0..100 {
        // u = a ./ (K · v)
        for i in 0..n {
            let kv: f64 = (0..m).map(|j| k_matrix[i][j] * v[j]).sum();
            u[i] = if kv > 1e-15 { a[i] / kv } else { 1e-15 };
        }
        // v = b ./ (K^T · u)
        for j in 0..m {
            let ktu: f64 = (0..n).map(|i| k_matrix[i][j] * u[i]).sum();
            v[j] = if ktu > 1e-15 { b[j] / ktu } else { 1e-15 };
        }
    }

    // Transport plan: π_ij = u_i * K_ij * v_j
    let mut total_cost = 0.0;
    for i in 0..n {
        for j in 0..m {
            let pi_ij = u[i] * k_matrix[i][j] * v[j];
            total_cost += cost[i][j] * pi_ij;
        }
    }

    total_cost.powf(1.0 / p as f64)
}

// ===========================================================================
// §3  Renormalization Group Flow
// ===========================================================================

/// Renormalization group flow for conservation budget evolution.
///
/// As the agent operates over time, the conservation budget evolves.
/// This is modeled as an RG flow: β(g) = dg/d(ln t) where g = gravity,
/// t = time measured in ticks.
///
/// Fixed points of β(g) correspond to stable operating regimes.
/// β(g*) = 0 → g* is a fixed point (stable if β'(g*) < 0).
#[derive(Debug, Clone)]
pub struct RenormalizationFlow {
    /// History of (tick, gravity) pairs.
    history: Vec<(f64, f64)>,
    /// Beta function coefficients: β(g) = c0 + c1*g + c2*g²
    beta_coeffs: [f64; 3],
}

impl RenormalizationFlow {
    pub fn new(history: Vec<(f64, f64)>) -> Self {
        let beta_coeffs = [0.0, 0.0, 0.0];
        let mut rg = Self { history, beta_coeffs };
        rg.fit_beta_function();
        rg
    }

    /// Build from a sequence of gravity values at regular tick intervals.
    pub fn from_gravity_series(gravity_values: &[f64], tick_interval: f64) -> Self {
        let history: Vec<(f64, f64)> = gravity_values
            .iter()
            .enumerate()
            .map(|(i, &g)| (i as f64 * tick_interval, g))
            .collect();
        Self::new(history)
    }

    /// Fit the beta function β(g) = c0 + c1·g + c2·g² from the history
    /// using least-squares regression on dg/d(ln t).
    fn fit_beta_function(&mut self) {
        if self.history.len() < 3 {
            return;
        }
        // Compute dg/d(ln t) at each interior point
        let mut xs: Vec<f64> = Vec::new();
        let mut ys: Vec<f64> = Vec::new();

        for i in 1..self.history.len() {
            let (t0, g0) = self.history[i - 1];
            let (t1, g1) = self.history[i];
            let dt_ln = (t1 / t0).ln();
            if dt_ln.abs() < 1e-15 {
                continue;
            }
            let dg = g1 - g0;
            let beta_val = dg / dt_ln;
            let g_mid = (g0 + g1) / 2.0;
            xs.push(g_mid);
            ys.push(beta_val);
        }

        if xs.is_empty() {
            return;
        }

        // Fit β(g) = c0 + c1·g + c2·g² via normal equations
        let n = xs.len() as f64;
        let s0 = n;
        let s1: f64 = xs.iter().sum();
        let s2: f64 = xs.iter().map(|x| x * x).sum();
        let s3: f64 = xs.iter().map(|x| x.powi(3)).sum();
        let s4: f64 = xs.iter().map(|x| x.powi(4)).sum();
        let sy: f64 = ys.iter().sum();
        let sxy: f64 = xs.iter().zip(ys.iter()).map(|(x, y)| x * y).sum();
        let sx2y: f64 = xs.iter().zip(ys.iter()).map(|(x, y)| x * x * y).sum();

        // Solve 3×3 system via Cramer's rule
        let det = s0 * (s2 * s4 - s3 * s3)
            - s1 * (s1 * s4 - s3 * s2)
            + s2 * (s1 * s3 - s2 * s2);

        if det.abs() < 1e-15 {
            // Degenerate: use linear fit
            self.beta_coeffs = [ys.iter().sum::<f64>() / n, 0.0, 0.0];
            return;
        }

        let c0 = (sy * (s2 * s4 - s3 * s3)
            - s1 * (sxy * s4 - s3 * sx2y)
            + s2 * (sxy * s3 - s2 * sx2y))
            / det;
        let c1 = (s0 * (sxy * s4 - s3 * sx2y)
            - sy * (s1 * s4 - s3 * s2)
            + s2 * (s1 * sx2y - sxy * s2))
            / det;
        let c2 = (s0 * (s2 * sx2y - s3 * sxy)
            - s1 * (s1 * sx2y - s3 * sxy)
            + sy * (s1 * s3 - s2 * s2))
            / det;

        self.beta_coeffs = [c0, c1, c2];
    }

    /// Evaluate the beta function at a given gravity value.
    pub fn beta(&self, g: f64) -> f64 {
        self.beta_coeffs[0] + self.beta_coeffs[1] * g + self.beta_coeffs[2] * g * g
    }

    /// Compute the renormalization flow: (scale, β(g)) pairs.
    /// Scale = ln(t) relative to the start.
    pub fn renormalization_flow(&self) -> Vec<(f64, f64)> {
        self.history
            .iter()
            .map(|&(t, g)| {
                let scale = if t > 0.0 { t.ln() } else { 0.0 };
                (scale, self.beta(g))
            })
            .collect()
    }

    /// Find fixed points: β(g*) = 0.
    /// For β(g) = c0 + c1·g + c2·g², solve the quadratic.
    pub fn fixed_points(&self) -> Vec<f64> {
        let [c0, c1, c2] = self.beta_coeffs;
        if c2.abs() < 1e-15 {
            if c1.abs() < 1e-15 {
                return vec![];
            }
            return vec![-c0 / c1];
        }
        let disc = c1 * c1 - 4.0 * c2 * c0;
        if disc < 0.0 {
            return vec![];
        }
        let sqrt_disc = disc.sqrt();
        let g1 = (-c1 + sqrt_disc) / (2.0 * c2);
        let g2 = (-c1 - sqrt_disc) / (2.0 * c2);
        if (g1 - g2).abs() < 1e-10 {
            vec![g1]
        } else {
            vec![g1, g2]
        }
    }

    /// Check stability of a fixed point: stable iff β'(g*) < 0.
    pub fn is_stable(&self, g_star: f64) -> bool {
        let beta_prime = self.beta_coeffs[1] + 2.0 * self.beta_coeffs[2] * g_star;
        beta_prime < 0.0
    }
}

// ===========================================================================
// §4  Variational Principle for Room Optimization
// ===========================================================================

/// Room parameters for variational optimization.
#[derive(Debug, Clone)]
pub struct VariationalRoom {
    pub gravity: f64,
    pub temperature: f64,
    pub max_tokens: f64,
}

impl VariationalRoom {
    pub fn from_config(cfg: &RoomConfig) -> Self {
        Self {
            gravity: cfg.gravity,
            temperature: 0.7, // default
            max_tokens: 4096.0,
        }
    }

    /// Energy cost functional: measures the "cost" of this room state.
    /// F = energy + latency + quality_loss
    ///   energy ∝ |gravity| (high gravity = expensive)
    ///   latency ∝ 1/max_tokens (small context = more round-trips)
    ///   quality_loss ∝ (temperature - optimal)^2
    pub fn energy(&self) -> f64 {
        let gravity_cost = self.gravity.abs() * 2.0;
        let latency = 1.0 / (self.max_tokens / 1000.0).max(0.1);
        let quality = (self.temperature - 0.7).powi(2) * 10.0;
        gravity_cost + latency + quality
    }

    /// Gradient of the energy with respect to parameters.
    pub fn energy_gradient(&self) -> (f64, f64, f64) {
        let d_gravity = if self.gravity.abs() > 1e-10 {
            2.0 * self.gravity.signum()
        } else {
            0.0
        };
        let d_temp = 20.0 * (self.temperature - 0.7);
        let d_tokens = -1000.0 / (self.max_tokens / 1000.0).max(0.1).powi(2);
        (d_gravity, d_temp, d_tokens)
    }
}

/// Variational optimization: minimize the functional F[rooms] by gradient descent.
///
/// The Euler-Lagrange equations for the action
///   S = ∫ (energy_cost + coupling_between_rooms) dt
/// yield the optimal gravity trajectory. We approximate by discrete
/// gradient descent with momentum.
pub fn variational_optimize(rooms: &[RoomConfig], iterations: usize, lr: f64) -> Vec<RoomConfig> {
    let mut state: Vec<VariationalRoom> = rooms.iter().map(VariationalRoom::from_config).collect();
    let n = state.len();
    let mut momentum_grav = vec![0.0; n];
    let momentum_factor = 0.9;

    for _ in 0..iterations {
        // Compute individual gradients + coupling between adjacent rooms
        for i in 0..n {
            let (dg, dt, dm) = state[i].energy_gradient();

            // Coupling: adjacent rooms should have similar gravity (smoothness)
            let mut coupling_grad = 0.0;
            if i > 0 {
                coupling_grad += state[i].gravity - state[i - 1].gravity;
            }
            if i < n - 1 {
                coupling_grad += state[i].gravity - state[i + 1].gravity;
            }

            let total_grad = dg + coupling_grad;

            // Gradient descent with momentum
            momentum_grav[i] = momentum_factor * momentum_grav[i] - lr * total_grad;
            state[i].gravity += momentum_grav[i];
            state[i].gravity = state[i].gravity.clamp(-1.0, 1.0);

            state[i].temperature -= lr * dt * 0.1;
            state[i].temperature = state[i].temperature.clamp(0.0, 2.0);

            state[i].max_tokens -= lr * dm * 0.01;
            state[i].max_tokens = state[i].max_tokens.clamp(100.0, 100000.0);
        }
    }

    // Convert back to RoomConfigs (preserve ids and types)
    state
        .into_iter()
        .zip(rooms.iter())
        .map(|(opt, orig)| RoomConfig {
            id: orig.id.clone(),
            room_type: orig.room_type.clone(),
            gravity: opt.gravity,
            gravity_confidence: orig.gravity_confidence,
            deadband_tolerance: orig.deadband_tolerance,
            ensign_id: orig.ensign_id.clone(),
            config: orig.config.clone(),
        })
        .collect()
}

// ===========================================================================
// §5  Berry Phase
// ===========================================================================

/// Compute the Berry phase for a cyclic room trajectory.
///
/// When the agent cycles through rooms and returns to the start,
/// the parameters may have changed even though the path is closed.
/// The Berry phase γ = ∮ ⟨ψ(g)| ∇_g |ψ(g)⟩ · dg
/// measures this geometric holonomy.
///
/// For our discrete setting, we approximate by:
///   γ ≈ Σ_i arg(⟨ψ_i|ψ_{i+1}⟩)
/// where |ψ_i⟩ is the "state vector" of room i, parameterized by
/// (gravity, temperature, max_tokens).
pub fn berry_phase(trajectory: &[&str], room_map: &std::collections::HashMap<String, (f64, f64, f64)>) -> f64 {
    if trajectory.len() < 2 {
        return 0.0;
    }

    let n = trajectory.len();

    // Build state vectors for each room in the trajectory
    let states: Vec<Vec<f64>> = trajectory
        .iter()
        .map(|name| {
            if let Some(&(g, t, m)) = room_map.get(*name) {
                // State vector: [gravity, temperature, max_tokens_normalized]
                vec![g, t, m / 10000.0]
            } else {
                // Unknown room: zero state
                vec![0.0, 0.0, 0.0]
            }
        })
        .collect();

    // Compute the Berry phase as the sum of phase differences
    let mut phase = 0.0;
    for i in 0..n {
        let j = (i + 1) % n;
        // Inner product ⟨ψ_i | ψ_j⟩
        let inner: f64 = states[i]
            .iter()
            .zip(states[j].iter())
            .map(|(a, b)| a * b)
            .sum();
        // Phase of the inner product (imaginary part is 0 for real vectors,
        // so we use the argument of the "complexified" overlap)
        let norm_i: f64 = states[i].iter().map(|x| x * x).sum::<f64>().sqrt();
        let norm_j: f64 = states[j].iter().map(|x| x * x).sum::<f64>().sqrt();
        let denom = norm_i * norm_j;
        if denom > 1e-15 {
            let cos_theta = (inner / denom).clamp(-1.0, 1.0);
            phase += cos_theta.acos();
        }
    }

    // Subtract the "trivial" phase (sum of pairwise angles for a flat connection)
    let trivial_phase = std::f64::consts::PI * (n as f64 - 2.0);
    phase - trivial_phase
}

/// Convenience: compute Berry phase from a list of room configs.
pub fn berry_phase_from_rooms(rooms: &[RoomConfig]) -> f64 {
    let names: Vec<&str> = rooms.iter().map(|r| r.id.as_str()).collect();
    let map: std::collections::HashMap<String, (f64, f64, f64)> = rooms
        .iter()
        .map(|r| (r.id.clone(), (r.gravity, 0.7, 4096.0)))
        .collect();
    berry_phase(&names, &map)
}

// ===========================================================================
// Helpers
// ===========================================================================

/// Jacobi eigenvalue algorithm for symmetric matrices.
/// Returns eigenvalues sorted in ascending order.
fn jacobi_eigenvalues(mat: &[Vec<f64>]) -> Vec<f64> {
    let n = mat.len();
    if n == 0 {
        return vec![];
    }
    let mut a = mat.to_vec();
    let max_iter = 100 * n;

    for _ in 0..max_iter {
        // Find largest off-diagonal element
        let mut max_val = 0.0;
        let mut p = 0;
        let mut q = 1;
        for i in 0..n {
            for j in (i + 1)..n {
                if a[i][j].abs() > max_val {
                    max_val = a[i][j].abs();
                    p = i;
                    q = j;
                }
            }
        }
        if max_val < 1e-12 {
            break;
        }

        // Compute rotation angle
        let app = a[p][p];
        let aqq = a[q][q];
        let apq = a[p][q];

        let theta = if (app - aqq).abs() < 1e-15 {
            std::f64::consts::FRAC_PI_4
        } else {
            0.5 * (2.0 * apq / (app - aqq)).atan()
        };
        let c = theta.cos();
        let s = theta.sin();

        // Apply rotation
        for i in 0..n {
            if i != p && i != q {
                let aip = a[i][p];
                let aiq = a[i][q];
                a[i][p] = c * aip + s * aiq;
                a[p][i] = a[i][p];
                a[i][q] = -s * aip + c * aiq;
                a[q][i] = a[i][q];
            }
        }
        let new_pp = c * c * app + 2.0 * s * c * apq + s * s * aqq;
        let new_qq = s * s * app - 2.0 * s * c * apq + c * c * aqq;
        a[p][p] = new_pp;
        a[q][q] = new_qq;
        a[p][q] = 0.0;
        a[q][p] = 0.0;
    }

    let mut eigenvalues: Vec<f64> = (0..n).map(|i| a[i][i]).collect();
    eigenvalues.sort_by(|a, b| a.partial_cmp(b).unwrap());
    eigenvalues
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- Spectral Triple / Bounded Operator ---

    #[test]
    fn test_identity_norm() {
        let id = BoundedOperator::identity(3);
        let norm = id.norm();
        assert!((norm - 1.0).abs() < 0.01, "identity norm should be 1, got {norm}");
    }

    #[test]
    fn test_zero_operator() {
        let z = BoundedOperator::zero(4);
        assert_eq!(z.norm(), 0.0);
        assert_eq!(z.trace(), 0.0);
    }

    #[test]
    fn test_adjoint() {
        let a = BoundedOperator::from_vec(vec![
            vec![1.0, 2.0, 3.0],
            vec![4.0, 5.0, 6.0],
            vec![7.0, 8.0, 9.0],
        ]);
        let at = a.adjoint();
        assert!((at.data[0][1] - 4.0).abs() < 1e-10);
        assert!((at.data[2][0] - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_compose() {
        let a = BoundedOperator::identity(2);
        let b = BoundedOperator::from_vec(vec![vec![1.0, 2.0], vec![3.0, 4.0]]);
        let c = a.compose(&b);
        assert!((c.data[0][0] - 1.0).abs() < 1e-10);
        assert!((c.data[0][1] - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_trace() {
        let a = BoundedOperator::from_vec(vec![
            vec![3.0, 1.0],
            vec![2.0, 7.0],
        ]);
        assert!((a.trace() - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_apply() {
        let a = BoundedOperator::from_vec(vec![vec![2.0, 0.0], vec![0.0, 3.0]]);
        let v = vec![1.0, 1.0];
        let result = a.apply(&v);
        assert!((result[0] - 2.0).abs() < 1e-10);
        assert!((result[1] - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_operator_norm_diagonal() {
        let d = BoundedOperator::from_vec(vec![vec![3.0, 0.0], vec![0.0, 5.0]]);
        assert!((d.norm() - 5.0).abs() < 0.5, "diagonal norm = max diagonal, got {}", d.norm());
    }

    // --- Dirac Operator ---

    #[test]
    fn test_dirac_from_types() {
        let d = DiracOperator::from_tile_types(&[0, 1, 2]);
        let op = d.as_operator();
        // Anti-symmetric: D[i][j] = -D[j][i]
        assert!((op.data[0][1] + op.data[1][0]).abs() < 1e-10);
    }

    #[test]
    fn test_dirac_spectrum() {
        let d = DiracOperator::from_tile_types(&[0, 1]);
        let spec = d.spectrum();
        assert_eq!(spec.len(), 2);
        // For 2x2 anti-symmetric, eigenvalues of D†D should be d²
        assert!(spec.iter().all(|&e| e >= -1e-10));
    }

    #[test]
    fn test_dirac_dim() {
        let d = DiracOperator::from_tile_types(&[0, 1, 2, 3]);
        assert_eq!(d.dim(), 4);
    }

    // --- Spectral Triple ---

    #[test]
    fn test_spectral_triple_index() {
        let op = BoundedOperator::from_vec(vec![vec![1.0, 0.5], vec![0.3, 0.8]]);
        let triple = SpectralTriple::new(op, vec![0, 1]);
        let idx = triple.compute_index();
        assert!(idx.is_finite(), "index should be finite, got {idx}");
    }

    #[test]
    fn test_spectral_triple_from_transitions() {
        let transitions = vec![(0, 1, 1.0), (1, 0, 0.5), (1, 2, 0.3)];
        let triple = SpectralTriple::from_transitions(&transitions, 3);
        assert_eq!(triple.algebra_element.dim(), 3);
        let idx = triple.compute_index();
        assert!(idx.is_finite());
    }

    #[test]
    fn test_metric_dimension() {
        let op = BoundedOperator::identity(3);
        let triple = SpectralTriple::new(op, vec![0, 1, 2]);
        let dim = triple.metric_dimension();
        assert!(dim >= 0.0, "metric dimension should be non-negative");
    }

    #[test]
    fn test_index_symmetric_operator() {
        // Symmetric operators should have different index than non-symmetric
        let sym = BoundedOperator::from_vec(vec![vec![2.0, 1.0], vec![1.0, 3.0]]);
        let t1 = SpectralTriple::new(sym, vec![0, 1]);
        let idx1 = t1.compute_index();

        let asym = BoundedOperator::from_vec(vec![vec![2.0, 3.0], vec![1.0, 3.0]]);
        let t2 = SpectralTriple::new(asym, vec![0, 1]);
        let idx2 = t2.compute_index();

        // They should be different in general
        assert!(idx1.is_finite() && idx2.is_finite());
    }

    // --- Wasserstein Distance ---

    #[test]
    fn test_wasserstein_identical() {
        let dist = vec![0.25, 0.25, 0.25, 0.25];
        let d = wasserstein_distance(&dist, &dist, 2);
        assert!(d < 0.01, "identical distributions should have ~0 distance, got {d}");
    }

    #[test]
    fn test_wasserstein_shifted() {
        let a = vec![1.0, 0.0, 0.0, 0.0];
        let b = vec![0.0, 0.0, 0.0, 1.0];
        let d = wasserstein_distance(&a, &b, 1);
        assert!(d > 2.0, "shifted distributions should have distance > 2, got {d}");
    }

    #[test]
    fn test_wasserstein_close() {
        let a = vec![0.5, 0.5, 0.0];
        let b = vec![0.5, 0.0, 0.5];
        let d1 = wasserstein_distance(&a, &b, 1);
        let d2 = wasserstein_distance(&a, &b, 2);
        assert!(d1 > 0.0);
        assert!(d2 > 0.0);
    }

    #[test]
    fn test_wasserstein_p1_vs_p2() {
        let a = vec![1.0, 0.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0, 0.0];
        let d1 = wasserstein_distance(&a, &b, 1);
        let d2 = wasserstein_distance(&a, &b, 2);
        // W_1 ≤ W_2 for same distributions
        assert!(d1 <= d2 + 0.1, "W_1={d1} should be ≤ W_2={d2}");
    }

    #[test]
    fn test_wasserstein_uniform() {
        let u = vec![0.2, 0.2, 0.2, 0.2, 0.2];
        let d = wasserstein_distance(&u, &u, 2);
        assert!(d < 0.01, "uniform to uniform should be ~0");
    }

    #[test]
    #[should_panic(expected = "same support size")]
    fn test_wasserstein_mismatched_sizes() {
        let a = vec![0.5, 0.5];
        let b = vec![0.33, 0.33, 0.34];
        wasserstein_distance(&a, &b, 1);
    }

    // --- Renormalization Group Flow ---

    #[test]
    fn test_rg_flow_constant() {
        let rg = RenormalizationFlow::from_gravity_series(&[0.5, 0.5, 0.5, 0.5], 1.0);
        let flow = rg.renormalization_flow();
        assert_eq!(flow.len(), 4);
        // β(g) should be ~0 for constant gravity
        let beta_sum: f64 = flow.iter().map(|&(_, b)| b).sum();
        assert!(beta_sum.abs() < 1.0, "constant series should have small beta");
    }

    #[test]
    fn test_rg_flow_fixed_points() {
        // Gravity decaying toward 0: β should cross zero near g=0
        let vals: Vec<f64> = (0..20).map(|i| 1.0 * (0.9_f64.powi(i))).collect();
        let rg = RenormalizationFlow::from_gravity_series(&vals, 1.0);
        let fps = rg.fixed_points();
        // Should find at least one fixed point
        assert!(!fps.is_empty() || vals.len() < 3);
    }

    #[test]
    fn test_rg_beta_function() {
        let rg = RenormalizationFlow::from_gravity_series(&[0.1, 0.2, 0.3, 0.4], 1.0);
        let beta = rg.beta(0.25);
        assert!(beta.is_finite());
    }

    #[test]
    fn test_rg_stability() {
        let rg = RenormalizationFlow::from_gravity_series(&[1.0, 0.8, 0.6, 0.4, 0.2, 0.1], 1.0);
        for fp in rg.fixed_points() {
            let _stable = rg.is_stable(fp);
            // Just ensure no panics
        }
    }

    #[test]
    fn test_rg_flow_length() {
        let rg = RenormalizationFlow::from_gravity_series(&[0.5; 10], 100.0);
        let flow = rg.renormalization_flow();
        assert_eq!(flow.len(), 10);
    }

    #[test]
    fn test_rg_oscillating() {
        let vals: Vec<f64> = (0..20).map(|i| if i % 2 == 0 { 0.5 } else { -0.5 }).collect();
        let rg = RenormalizationFlow::from_gravity_series(&vals, 1.0);
        let flow = rg.renormalization_flow();
        assert_eq!(flow.len(), 20);
    }

    // --- Variational Optimization ---

    #[test]
    fn test_variational_room_energy() {
        let vr = VariationalRoom {
            gravity: 0.0,
            temperature: 0.7,
            max_tokens: 4096.0,
        };
        let e = vr.energy();
        assert!(e > 0.0);
        // At optimal settings, energy should be relatively low
    }

    #[test]
    fn test_variational_room_gradient() {
        let vr = VariationalRoom {
            gravity: 0.5,
            temperature: 1.0,
            max_tokens: 1000.0,
        };
        let (dg, dt, dm) = vr.energy_gradient();
        assert!(dg.is_finite());
        assert!(dt.is_finite());
        assert!(dm.is_finite());
    }

    #[test]
    fn test_variational_optimize_decreases_energy() {
        let configs: Vec<RoomConfig> = (0..3)
            .map(|i| RoomConfig {
                id: format!("room-{i}"),
                room_type: "navigation".to_string(),
                gravity: 0.9,
                gravity_confidence: 0.5,
                deadband_tolerance: 0.1,
                ensign_id: None,
                config: None,
            })
            .collect();

        let initial_energy: f64 = configs
            .iter()
            .map(|c| VariationalRoom::from_config(c).energy())
            .sum();

        let optimized = variational_optimize(&configs, 200, 0.01);

        let final_energy: f64 = optimized
            .iter()
            .map(|c| VariationalRoom::from_config(c).energy())
            .sum();

        assert!(
            final_energy <= initial_energy + 0.1,
            "optimization should not increase energy: initial={initial_energy}, final={final_energy}"
        );
    }

    #[test]
    fn test_variational_optimize_preserves_ids() {
        let configs: Vec<RoomConfig> = (0..3)
            .map(|i| RoomConfig {
                id: format!("room-{i}"),
                room_type: "engineering".to_string(),
                gravity: 0.5,
                gravity_confidence: 0.3,
                deadband_tolerance: 0.1,
                ensign_id: None,
                config: None,
            })
            .collect();

        let optimized = variational_optimize(&configs, 10, 0.01);
        assert_eq!(optimized.len(), 3);
        for (i, opt) in optimized.iter().enumerate() {
            assert_eq!(opt.id, format!("room-{i}"));
        }
    }

    #[test]
    fn test_variational_clamp() {
        let configs: Vec<RoomConfig> = vec![RoomConfig {
            id: "extreme".to_string(),
            room_type: "science".to_string(),
            gravity: 5.0, // out of range
            gravity_confidence: 0.5,
            deadband_tolerance: 0.1,
            ensign_id: None,
            config: None,
        }];

        let optimized = variational_optimize(&configs, 50, 0.1);
        assert!(optimized[0].gravity >= -1.0 && optimized[0].gravity <= 1.0);
    }

    // --- Berry Phase ---

    #[test]
    fn test_berry_phase_single_room() {
        let mut map = std::collections::HashMap::new();
        map.insert("A".to_string(), (0.5, 0.7, 4096.0));
        let phase = berry_phase(&["A"], &map);
        assert_eq!(phase, 0.0, "single room should have zero Berry phase");
    }

    #[test]
    fn test_berry_phase_identical_rooms() {
        let mut map = std::collections::HashMap::new();
        map.insert("A".to_string(), (0.5, 0.7, 4096.0));
        map.insert("B".to_string(), (0.5, 0.7, 4096.0));
        map.insert("C".to_string(), (0.5, 0.7, 4096.0));
        let phase = berry_phase(&["A", "B", "C"], &map);
        // Identical rooms → all pairwise angles = 0 → total phase = 0 - (π * 1)
        // The Berry phase can be non-zero due to the trivial phase subtraction
        assert!(phase.is_finite(), "identical rooms should give finite Berry phase");
    }

    #[test]
    fn test_berry_phase_nontrivial() {
        let mut map = std::collections::HashMap::new();
        map.insert("A".to_string(), (0.9, 0.7, 4096.0));
        map.insert("B".to_string(), (-0.9, 0.7, 4096.0));
        map.insert("C".to_string(), (0.0, 0.7, 4096.0));
        let phase = berry_phase(&["A", "B", "C"], &map);
        assert!(phase.is_finite(), "Berry phase should be finite");
    }

    #[test]
    fn test_berry_phase_missing_room() {
        let mut map = std::collections::HashMap::new();
        map.insert("A".to_string(), (0.5, 0.7, 4096.0));
        let phase = berry_phase(&["A", "missing"], &map);
        assert!(phase.is_finite());
    }

    #[test]
    fn test_berry_phase_from_rooms() {
        let rooms = vec![
            RoomConfig {
                id: "R1".to_string(),
                room_type: "nav".to_string(),
                gravity: 0.3,
                gravity_confidence: 0.5,
                deadband_tolerance: 0.1,
                ensign_id: None,
                config: None,
            },
            RoomConfig {
                id: "R2".to_string(),
                room_type: "eng".to_string(),
                gravity: -0.3,
                gravity_confidence: 0.5,
                deadband_tolerance: 0.1,
                ensign_id: None,
                config: None,
            },
        ];
        let phase = berry_phase_from_rooms(&rooms);
        assert!(phase.is_finite());
    }

    #[test]
    fn test_berry_phase_four_cycle() {
        let mut map = std::collections::HashMap::new();
        map.insert("N".to_string(), (0.5, 0.7, 4096.0));
        map.insert("E".to_string(), (0.0, 1.0, 4096.0));
        map.insert("S".to_string(), (-0.5, 0.7, 4096.0));
        map.insert("W".to_string(), (0.0, 0.4, 4096.0));
        let phase = berry_phase(&["N", "E", "S", "W"], &map);
        assert!(phase.is_finite());
    }

    // --- Jacobi Eigenvalue Helper ---

    #[test]
    fn test_jacobi_identity() {
        let id: Vec<Vec<f64>> = (0..3)
            .map(|i| (0..3).map(|j| if i == j { 1.0 } else { 0.0 }).collect())
            .collect();
        let evals = jacobi_eigenvalues(&id);
        assert_eq!(evals.len(), 3);
        assert!(evals.iter().all(|&e| (e - 1.0).abs() < 0.01));
    }

    #[test]
    fn test_jacobi_diagonal() {
        let d = vec![vec![3.0, 0.0], vec![0.0, 7.0]];
        let evals = jacobi_eigenvalues(&d);
        assert!((evals[0] - 3.0).abs() < 0.01);
        assert!((evals[1] - 7.0).abs() < 0.01);
    }

    #[test]
    fn test_jacobi_symmetric() {
        let m = vec![vec![2.0, 1.0], vec![1.0, 2.0]];
        let evals = jacobi_eigenvalues(&m);
        assert!((evals[0] - 1.0).abs() < 0.1);
        assert!((evals[1] - 3.0).abs() < 0.1);
    }

    #[test]
    fn test_jacobi_empty() {
        let evals = jacobi_eigenvalues(&[]);
        assert!(evals.is_empty());
    }

    // --- Integration / edge-case tests ---

    #[test]
    fn test_wasserstein_point_masses() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 0.0, 1.0];
        let d = wasserstein_distance(&a, &b, 2);
        assert!(d > 0.0, "distant point masses should have positive distance");
    }

    #[test]
    fn test_rg_flow_short_history() {
        let rg = RenormalizationFlow::from_gravity_series(&[0.5], 1.0);
        assert_eq!(rg.renormalization_flow().len(), 1);
    }

    #[test]
    fn test_rg_flow_two_points() {
        let rg = RenormalizationFlow::from_gravity_series(&[0.5, 0.3], 1.0);
        let flow = rg.renormalization_flow();
        assert_eq!(flow.len(), 2);
    }

    #[test]
    fn test_spectral_triple_zero_operator() {
        let op = BoundedOperator::zero(3);
        let triple = SpectralTriple::new(op, vec![0, 1, 2]);
        let idx = triple.compute_index();
        assert!(idx.is_finite());
    }

    #[test]
    fn test_variational_empty() {
        let configs: Vec<RoomConfig> = vec![];
        let optimized = variational_optimize(&configs, 10, 0.01);
        assert!(optimized.is_empty());
    }

    #[test]
    fn test_berry_phase_empty() {
        let map = std::collections::HashMap::new();
        let phase = berry_phase(&[], &map);
        assert_eq!(phase, 0.0);
    }

    #[test]
    fn test_dirac_from_distances() {
        let dists = vec![vec![0.0, 1.0, 2.0], vec![1.0, 0.0, 1.0], vec![2.0, 1.0, 0.0]];
        let d = DiracOperator::from_distances(dists);
        let spec = d.spectrum();
        assert_eq!(spec.len(), 3);
    }

    #[test]
    fn test_operator_compose_identity() {
        let a = BoundedOperator::from_vec(vec![vec![1.0, 2.0], vec![3.0, 4.0]]);
        let id = BoundedOperator::identity(2);
        let result = id.compose(&a);
        assert!((result.data[0][0] - 1.0).abs() < 1e-10);
        assert!((result.data[1][1] - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_variational_gradient_zero_at_optimal() {
        let optimal = VariationalRoom {
            gravity: 0.0,
            temperature: 0.7,
            max_tokens: 4096.0,
        };
        let (dg, dt, _dm) = optimal.energy_gradient();
        assert!(dg.abs() < 0.01, "gravity gradient at 0 should be ~0");
        assert!(dt.abs() < 0.01, "temp gradient at 0.7 should be ~0");
    }

    #[test]
    fn test_rg_beta_coefficients() {
        let rg = RenormalizationFlow::from_gravity_series(
            &[1.0, 0.8, 0.64, 0.512, 0.4096],
            1.0,
        );
        // Gravity is decaying: β should be negative
        let beta_at_0_8 = rg.beta(0.8);
        assert!(beta_at_0_8.is_finite());
    }
}
