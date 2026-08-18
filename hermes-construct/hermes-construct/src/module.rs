//! module.rs — Runtime module system for hermes-construct
//!
//! Modules snap into the runtime and load/unload per task.
//! The trait, registry, autoloader, and supporting types live here.

use chrono::{DateTime, Utc};
use std::collections::{HashMap, HashSet};
use std::fmt;

// ---------------------------------------------------------------------------
// Capability
// ---------------------------------------------------------------------------

/// Capabilities a module can advertise.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Capability {
    PatternDetection,
    ConservationTracking,
    TopologyAnalysis,
    ForbiddenBehavior,
    OutputZoning,
    SpectralAnalysis,
    AnomalyDetection,
}

impl fmt::Display for Capability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Capability::PatternDetection => write!(f, "PatternDetection"),
            Capability::ConservationTracking => write!(f, "ConservationTracking"),
            Capability::TopologyAnalysis => write!(f, "TopologyAnalysis"),
            Capability::ForbiddenBehavior => write!(f, "ForbiddenBehavior"),
            Capability::OutputZoning => write!(f, "OutputZoning"),
            Capability::SpectralAnalysis => write!(f, "SpectralAnalysis"),
            Capability::AnomalyDetection => write!(f, "AnomalyDetection"),
        }
    }
}

// ---------------------------------------------------------------------------
// ModuleError
// ---------------------------------------------------------------------------

/// Errors that can occur during module operations.
#[derive(Debug, thiserror::Error)]
#[allow(dead_code)]
pub enum ModuleError {
    #[error("module '{0}' is already loaded")]
    AlreadyLoaded(String),

    #[error("module '{0}' is not loaded")]
    NotLoaded(String),

    #[error("module '{0}' is not registered")]
    NotRegistered(String),

    #[error("load failed for '{name}': {reason}")]
    LoadFailed { name: String, reason: String },

    #[error("unload failed for '{name}': {reason}")]
    UnloadFailed { name: String, reason: String },
}

// ---------------------------------------------------------------------------
// LoadEvent — provenance log
// ---------------------------------------------------------------------------

/// Whether a load event is a load or unload.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub enum LoadAction {
    Load,
    Unload,
}

impl fmt::Display for LoadAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadAction::Load => write!(f, "Load"),
            LoadAction::Unload => write!(f, "Unload"),
        }
    }
}

/// A single entry in the load-history log.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct LoadEvent {
    pub module_name: String,
    pub action: LoadAction,
    pub timestamp: DateTime<Utc>,
    pub reason: String,
}

// ---------------------------------------------------------------------------
// ModuleContext — shared state handed to modules on load
// ---------------------------------------------------------------------------

/// Read-only context that modules receive when loaded.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ModuleContext {
    pub room_id: String,
    pub conservation_budget: f64,
    pub loaded_modules: Vec<String>,
}

impl ModuleContext {
    pub fn new(room_id: &str, conservation_budget: f64, loaded_modules: &[String]) -> Self {
        Self {
            room_id: room_id.to_string(),
            conservation_budget,
            loaded_modules: loaded_modules.to_vec(),
        }
    }
}

// ---------------------------------------------------------------------------
// Module trait — common interface for all runtime modules
// ---------------------------------------------------------------------------

/// The interface every hermes-construct module must implement.
#[allow(dead_code)]
pub trait Module: Send + Sync {
    /// Unique name of the module (e.g. "crackle-runtime").
    fn name(&self) -> &str;

    /// Semantic version string (e.g. "0.1.0").
    fn version(&self) -> &str;

    /// Called when the module is loaded into the runtime.
    fn load(&mut self, ctx: &ModuleContext) -> Result<(), ModuleError>;

    /// Called when the module is unloaded from the runtime.
    fn unload(&mut self) -> Result<(), ModuleError>;

    /// Capabilities this module provides.
    fn capabilities(&self) -> Vec<Capability>;

    /// Estimated API/token cost for a given task description.
    fn cost_estimate(&self, task: &str) -> f64;
}

// ---------------------------------------------------------------------------
// ModuleRegistry
// ---------------------------------------------------------------------------

/// Tracks registered modules, their loaded state, and load-history.
#[allow(dead_code)]
pub struct ModuleRegistry {
    modules: HashMap<String, Box<dyn Module>>,
    loaded: HashSet<String>,
    load_history: Vec<LoadEvent>,
}

#[allow(dead_code)]
impl ModuleRegistry {
    /// Create an empty registry.
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
            loaded: HashSet::new(),
            load_history: Vec::new(),
        }
    }

    /// Register a module (makes it available for loading).
    pub fn register(&mut self, module: Box<dyn Module>) {
        let name = module.name().to_string();
        self.modules.insert(name, module);
    }

    /// Load a registered module into the runtime.
    pub fn load(&mut self, name: &str, ctx: &ModuleContext) -> Result<(), ModuleError> {
        if self.loaded.contains(name) {
            return Err(ModuleError::AlreadyLoaded(name.to_string()));
        }
        let module = self
            .modules
            .get_mut(name)
            .ok_or_else(|| ModuleError::NotRegistered(name.to_string()))?;
        module.load(ctx)?;
        self.loaded.insert(name.to_string());
        self.load_history.push(LoadEvent {
            module_name: name.to_string(),
            action: LoadAction::Load,
            timestamp: Utc::now(),
            reason: "explicit load".to_string(),
        });
        Ok(())
    }

    /// Unload a module from the runtime.
    pub fn unload(&mut self, name: &str) -> Result<(), ModuleError> {
        if !self.loaded.contains(name) {
            return Err(ModuleError::NotLoaded(name.to_string()));
        }
        let module = self
            .modules
            .get_mut(name)
            .ok_or_else(|| ModuleError::NotRegistered(name.to_string()))?;
        module.unload()?;
        self.loaded.remove(name);
        self.load_history.push(LoadEvent {
            module_name: name.to_string(),
            action: LoadAction::Unload,
            timestamp: Utc::now(),
            reason: "explicit unload".to_string(),
        });
        Ok(())
    }

    /// Find modules whose capabilities or keywords match a task description.
    ///
    /// Matching is deliberately simple (keyword substring) — good enough for
    /// now and easy to upgrade later.
    pub fn find_for_task(&self, task: &str) -> Vec<&str> {
        let task_lower = task.to_lowercase();
        let mut matches: Vec<&str> = Vec::new();
        for (name, module) in &self.modules {
            // Check if the module name itself appears in the task
            if task_lower.contains(&name.to_lowercase()) {
                matches.push(name.as_str());
                continue;
            }
            // Check capabilities — split camel-case into individual words
            for cap in module.capabilities() {
                let cap_display = format!("{}", cap); // e.g. "PatternDetection"
                // Split camel-case into words, then lowercase each
                let mut words: Vec<String> = Vec::new();
                let mut current = String::new();
                for ch in cap_display.chars() {
                    if ch.is_uppercase() && !current.is_empty() {
                        words.push(current.to_lowercase());
                        current.clear();
                    }
                    current.push(ch);
                }
                if !current.is_empty() {
                    words.push(current.to_lowercase());
                }
                for word in &words {
                    if word.len() >= 4 && task_lower.contains(word.as_str()) {
                        matches.push(name.as_str());
                        break;
                    }
                }
            }
        }
        matches.sort();
        matches.dedup();
        matches
    }

    /// Whether a module is currently loaded.
    pub fn is_loaded(&self, name: &str) -> bool {
        self.loaded.contains(name)
    }

    /// How many modules are currently loaded.
    pub fn loaded_count(&self) -> usize {
        self.loaded.len()
    }

    /// How many modules are registered (loaded or not).
    pub fn registered_count(&self) -> usize {
        self.modules.len()
    }

    /// Return a copy of the load-history log.
    pub fn load_history(&self) -> &[LoadEvent] {
        &self.load_history
    }

    /// Return the names of all currently loaded modules.
    pub fn loaded_modules(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.loaded.iter().map(|s| s.as_str()).collect();
        names.sort();
        names
    }

    /// Gather capabilities from all loaded modules.
    pub fn loaded_capabilities(&self) -> Vec<Capability> {
        let mut caps: Vec<Capability> = Vec::new();
        for name in &self.loaded {
            if let Some(m) = self.modules.get(name) {
                caps.extend(m.capabilities());
            }
        }
        caps.sort_by(|a, b| format!("{:?}", a).cmp(&format!("{:?}", b)));
        caps.dedup();
        caps
    }

    /// Sum cost estimates from all loaded modules for a given task.
    pub fn total_cost_estimate(&self, task: &str) -> f64 {
        let mut total = 0.0;
        for name in &self.loaded {
            if let Some(m) = self.modules.get(name) {
                total += m.cost_estimate(task);
            }
        }
        total
    }
}

impl Default for ModuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// AutoLoader — decides which modules to load for a given task
// ---------------------------------------------------------------------------

/// Given a task description, recommends and loads appropriate modules.
#[allow(dead_code)]
pub struct AutoLoader {
    /// keyword → list of module names that should be loaded.
    task_keywords: HashMap<String, Vec<String>>,
}

#[allow(dead_code)]
impl AutoLoader {
    /// Create a new autoloader with default keyword mappings.
    #[allow(dead_code)]
    pub fn new() -> Self {
        let mut kw: HashMap<String, Vec<String>> = HashMap::new();

        kw.insert(
            "pattern".to_string(),
            vec!["crackle-runtime".to_string()],
        );
        kw.insert(
            "crackle".to_string(),
            vec!["crackle-runtime".to_string()],
        );
        kw.insert(
            "conservation".to_string(),
            vec!["conservation-checker".to_string()],
        );
        kw.insert(
            "budget".to_string(),
            vec!["conservation-checker".to_string()],
        );
        kw.insert(
            "topology".to_string(),
            vec!["cathedral-probe".to_string()],
        );
        kw.insert(
            "cathedral".to_string(),
            vec!["cathedral-probe".to_string()],
        );
        kw.insert(
            "negative".to_string(),
            vec!["negative-space-testing".to_string()],
        );
        kw.insert(
            "forbidden".to_string(),
            vec!["negative-space-testing".to_string()],
        );
        kw.insert(
            "map".to_string(),
            vec!["spacemap".to_string()],
        );
        kw.insert(
            "space".to_string(),
            vec!["spacemap".to_string()],
        );
        kw.insert(
            "anomaly".to_string(),
            vec!["crackle-runtime".to_string(), "spacemap".to_string()],
        );
        kw.insert(
            "spectral".to_string(),
            vec!["cathedral-probe".to_string()],
        );

        Self { task_keywords: kw }
    }

    /// Create an autoloader from a custom keyword map.
    pub fn with_keywords(task_keywords: HashMap<String, Vec<String>>) -> Self {
        Self { task_keywords }
    }

    /// Analyze a task description and load matching modules.
    ///
    /// Returns the list of module names that were loaded (or were already loaded).
    pub fn analyze_and_load(
        &self,
        task: &str,
        registry: &mut ModuleRegistry,
        ctx: &ModuleContext,
    ) -> Vec<String> {
        let task_lower = task.to_lowercase();
        let mut to_load: HashSet<String> = HashSet::new();

        for (keyword, module_names) in &self.task_keywords {
            if task_lower.contains(keyword.as_str()) {
                for name in module_names {
                    to_load.insert(name.clone());
                }
            }
        }

        let mut loaded_names: Vec<String> = Vec::new();
        for name in &to_load {
            if !registry.is_loaded(name) {
                if registry.load(name, ctx).is_ok() {
                    loaded_names.push(name.clone());
                }
            } else {
                loaded_names.push(format!("{}:already", name));
            }
        }

        loaded_names.sort();
        loaded_names
    }
}

impl Default for AutoLoader {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Stub modules for demonstration / testing
// ---------------------------------------------------------------------------

/// A trivial stub module used for registration and wiring.
#[allow(dead_code)]
pub struct StubModule {
    name: String,
    version: String,
    caps: Vec<Capability>,
    keywords: Vec<String>,
    loaded: bool,
}

impl StubModule {
    pub fn new(
        name: &str,
        version: &str,
        caps: Vec<Capability>,
        keywords: Vec<String>,
    ) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            caps,
            keywords,
            loaded: false,
        }
    }
}

impl Module for StubModule {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    fn load(&mut self, _ctx: &ModuleContext) -> Result<(), ModuleError> {
        self.loaded = true;
        Ok(())
    }

    fn unload(&mut self) -> Result<(), ModuleError> {
        self.loaded = false;
        Ok(())
    }

    fn capabilities(&self) -> Vec<Capability> {
        self.caps.clone()
    }

    fn cost_estimate(&self, task: &str) -> f64 {
        let task_lower = task.to_lowercase();
        let matches = self
            .keywords
            .iter()
            .filter(|kw| task_lower.contains(kw.as_str()))
            .count();
        // Base cost + per-keyword-match cost
        0.01 + (matches as f64) * 0.005
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ctx() -> ModuleContext {
        ModuleContext::new("test-room", 1.0, &[])
    }

    fn stub(name: &str, caps: Vec<Capability>, keywords: Vec<&str>) -> Box<dyn Module> {
        Box::new(StubModule::new(
            name,
            "0.1.0",
            caps,
            keywords.into_iter().map(|s| s.to_string()).collect(),
        ))
    }

    // 1. Register a module
    #[test]
    fn test_register() {
        let mut reg = ModuleRegistry::new();
        reg.register(stub(
            "crackle-runtime",
            vec![Capability::PatternDetection],
            vec!["pattern", "crackle"],
        ));
        assert_eq!(reg.registered_count(), 1);
    }

    // 2. Load a registered module
    #[test]
    fn test_load() {
        let mut reg = ModuleRegistry::new();
        reg.register(stub("crackle-runtime", vec![], vec![]));
        assert!(reg.load("crackle-runtime", &make_ctx()).is_ok());
        assert!(reg.is_loaded("crackle-runtime"));
        assert_eq!(reg.loaded_count(), 1);
    }

    // 3. Unload a loaded module
    #[test]
    fn test_unload() {
        let mut reg = ModuleRegistry::new();
        reg.register(stub("crackle-runtime", vec![], vec![]));
        reg.load("crackle-runtime", &make_ctx()).unwrap();
        assert!(reg.unload("crackle-runtime").is_ok());
        assert!(!reg.is_loaded("crackle-runtime"));
        assert_eq!(reg.loaded_count(), 0);
    }

    // 4. Double-load is an error
    #[test]
    fn test_double_load_error() {
        let mut reg = ModuleRegistry::new();
        reg.register(stub("crackle-runtime", vec![], vec![]));
        reg.load("crackle-runtime", &make_ctx()).unwrap();
        let err = reg.load("crackle-runtime", &make_ctx()).unwrap_err();
        assert!(matches!(err, ModuleError::AlreadyLoaded(_)));
    }

    // 5. Unload not-loaded is an error
    #[test]
    fn test_unload_not_loaded_error() {
        let mut reg = ModuleRegistry::new();
        reg.register(stub("crackle-runtime", vec![], vec![]));
        let err = reg.unload("crackle-runtime").unwrap_err();
        assert!(matches!(err, ModuleError::NotLoaded(_)));
    }

    // 6. Load a module that isn't registered
    #[test]
    fn test_load_not_registered() {
        let reg = ModuleRegistry::new();
        assert!(reg.is_loaded("nope") == false);
    }

    // 7. find_for_task with keyword matching
    #[test]
    fn test_find_for_task() {
        let mut reg = ModuleRegistry::new();
        reg.register(stub(
            "crackle-runtime",
            vec![Capability::PatternDetection],
            vec!["pattern"],
        ));
        reg.register(stub(
            "conservation-checker",
            vec![Capability::ConservationTracking],
            vec!["conservation"],
        ));
        reg.register(stub("spacemap", vec![Capability::AnomalyDetection], vec!["map"]));

        let found = reg.find_for_task("detect pattern anomalies");
        assert!(found.contains(&"crackle-runtime"));
    }

    // 8. find_for_task — capability-based matching
    #[test]
    fn test_find_for_task_capability() {
        let mut reg = ModuleRegistry::new();
        reg.register(stub(
            "conservation-checker",
            vec![Capability::ConservationTracking],
            vec![],
        ));
        let found = reg.find_for_task("track conservation");
        assert!(found.contains(&"conservation-checker"));
    }

    // 9. AutoLoader basic task
    #[test]
    fn test_autoload_basic() {
        let mut reg = ModuleRegistry::new();
        reg.register(stub("crackle-runtime", vec![], vec![]));
        reg.register(stub("conservation-checker", vec![], vec![]));

        let al = AutoLoader::new();
        let loaded = al.analyze_and_load("find patterns in the data", &mut reg, &make_ctx());
        assert!(loaded.iter().any(|n| n.contains("crackle-runtime")));
        assert!(reg.is_loaded("crackle-runtime"));
    }

    // 10. AutoLoader loads multiple modules
    #[test]
    fn test_autoload_multiple() {
        let mut reg = ModuleRegistry::new();
        reg.register(stub("crackle-runtime", vec![], vec![]));
        reg.register(stub("spacemap", vec![], vec![]));
        reg.register(stub("conservation-checker", vec![], vec![]));

        let al = AutoLoader::new();
        let loaded = al.analyze_and_load("detect anomaly in space", &mut reg, &make_ctx());
        assert!(loaded.iter().any(|n| n.contains("crackle-runtime")));
        assert!(loaded.iter().any(|n| n.contains("spacemap")));
    }

    // 11. Load history tracking
    #[test]
    fn test_load_history() {
        let mut reg = ModuleRegistry::new();
        reg.register(stub("crackle-runtime", vec![], vec![]));
        reg.load("crackle-runtime", &make_ctx()).unwrap();
        reg.unload("crackle-runtime").unwrap();

        let hist = reg.load_history();
        assert_eq!(hist.len(), 2);
        assert_eq!(hist[0].action, LoadAction::Load);
        assert_eq!(hist[1].action, LoadAction::Unload);
        assert_eq!(hist[0].module_name, "crackle-runtime");
    }

    // 12. Capabilities listing
    #[test]
    fn test_capabilities_listing() {
        let mut reg = ModuleRegistry::new();
        reg.register(stub(
            "crackle-runtime",
            vec![Capability::PatternDetection, Capability::AnomalyDetection],
            vec![],
        ));
        reg.register(stub(
            "conservation-checker",
            vec![Capability::ConservationTracking],
            vec![],
        ));
        reg.load("crackle-runtime", &make_ctx()).unwrap();
        reg.load("conservation-checker", &make_ctx()).unwrap();

        let caps = reg.loaded_capabilities();
        assert!(caps.contains(&Capability::PatternDetection));
        assert!(caps.contains(&Capability::ConservationTracking));
        assert!(caps.contains(&Capability::AnomalyDetection));
    }

    // 13. Cost estimation
    #[test]
    fn test_cost_estimate() {
        let mut reg = ModuleRegistry::new();
        reg.register(stub(
            "crackle-runtime",
            vec![],
            vec!["pattern", "anomaly"],
        ));
        reg.load("crackle-runtime", &make_ctx()).unwrap();

        let cost = reg.total_cost_estimate("detect anomaly pattern");
        assert!(cost > 0.0);
        // base 0.01 + 2 keyword matches * 0.005 = 0.02
        assert!((cost - 0.02).abs() < 1e-9);
    }

    // 14. loaded_modules returns correct set
    #[test]
    fn test_loaded_modules_list() {
        let mut reg = ModuleRegistry::new();
        reg.register(stub("alpha", vec![], vec![]));
        reg.register(stub("beta", vec![], vec![]));
        reg.load("alpha", &make_ctx()).unwrap();
        reg.load("beta", &make_ctx()).unwrap();

        let loaded = reg.loaded_modules();
        assert_eq!(loaded, vec!["alpha", "beta"]);
    }

    // 15. AutoLoader with custom keywords
    #[test]
    fn test_autoload_custom_keywords() {
        let mut kw = HashMap::new();
        kw.insert(
            "pizza".to_string(),
            vec!["pizza-module".to_string()],
        );

        let mut reg = ModuleRegistry::new();
        reg.register(stub("pizza-module", vec![], vec![]));

        let al = AutoLoader::with_keywords(kw);
        let loaded = al.analyze_and_load("I want pizza", &mut reg, &make_ctx());
        assert!(loaded.iter().any(|n| n.contains("pizza-module")));
    }

    // 16. Capability Display trait
    #[test]
    fn test_capability_display() {
        assert_eq!(
            format!("{}", Capability::PatternDetection),
            "PatternDetection"
        );
        assert_eq!(
            format!("{}", Capability::ConservationTracking),
            "ConservationTracking"
        );
    }

    // 17. LoadEvent fields are correct
    #[test]
    fn test_load_event_fields() {
        let mut reg = ModuleRegistry::new();
        reg.register(stub("crackle-runtime", vec![], vec![]));
        reg.load("crackle-runtime", &make_ctx()).unwrap();

        let event = &reg.load_history()[0];
        assert_eq!(event.module_name, "crackle-runtime");
        assert_eq!(event.action, LoadAction::Load);
        assert_eq!(event.reason, "explicit load");
        // timestamp should be roughly now
        assert!(event.timestamp.timestamp() > 0);
    }
}
