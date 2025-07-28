use std::time::{Duration, Instant};

use crate::handlers::cache::{
    EntityRestructurer, FileIndex, FullAnalysis, GameDataCache, TypeCache,
};
use colored::Colorize;

/// Configuration for the initialization process
pub struct InitializationConfig {
    pub timeout: Option<Duration>,
    pub poll_interval: Duration,
    pub silent: bool,
}

impl Default for InitializationConfig {
    fn default() -> Self {
        Self {
            timeout: None,
            poll_interval: Duration::from_millis(100),
            silent: false,
        }
    }
}

/// Result of the initialization process
pub struct InitializationResult {
    pub entity_restructurer: EntityRestructurer,
    pub full_analysis: FullAnalysis,
    pub full_analysis_duration: Duration,
}

/// Unified initialization logic for caches and analysis components
pub struct CacheInitializer;

impl CacheInitializer {
    /// Initialize all caches in background and wait for completion
    pub fn initialize(config: InitializationConfig) -> Result<InitializationResult, &'static str> {
        if !config.silent {
            eprintln!("{}", "Initializing caches...".blue().bold());
        }

        // Initialize caches in background
        TypeCache::initialize_in_background();
        GameDataCache::initialize_in_background();
        FileIndex::initialize_in_background();

        // Wait for caches to be initialized
        let start = Instant::now();
        while !TypeCache::is_initialized()
            || !GameDataCache::is_initialized()
            || !FileIndex::is_initialized()
        {
            if let Some(timeout) = config.timeout {
                if start.elapsed() > timeout {
                    if !config.silent {
                        eprintln!(
                            "{} {}",
                            "Error:".red().bold(),
                            "Timeout waiting for caches to initialize".bright_white()
                        );
                    }
                    return Err("Timeout waiting for caches to initialize");
                }
            }
            std::thread::sleep(config.poll_interval);
        }

        if !config.silent {
            eprintln!("{}", "Restructuring entities...".blue().bold());
        }

        // Create and load entity restructurer
        let entity_restructurer =
            EntityRestructurer::new(GameDataCache::get().unwrap(), TypeCache::get().unwrap());
        entity_restructurer.load();

        if !config.silent {
            eprintln!("{}", "Loading full analysis...".blue().bold());
        }

        // Create and load full analysis
        let full_analysis_start = Instant::now();
        let full_analysis = FullAnalysis::new(TypeCache::get().unwrap());
        full_analysis.load();
        let full_analysis_duration = full_analysis_start.elapsed();

        if !config.silent {
            eprintln!(
                "{} {}",
                "Full analysis loaded in".green().bold(),
                format!("{:?}", full_analysis_duration).bright_yellow()
            );
        }

        Ok(InitializationResult {
            entity_restructurer,
            full_analysis,
            full_analysis_duration,
        })
    }

    /// Initialize with default config (no timeout, with logging)
    pub fn initialize_default() -> Result<InitializationResult, &'static str> {
        Self::initialize(InitializationConfig::default())
    }

    /// Initialize silently with no logging
    pub fn initialize_silent() -> Result<InitializationResult, &'static str> {
        Self::initialize(InitializationConfig {
            silent: true,
            ..Default::default()
        })
    }

    /// Initialize with timeout (for CLI tools)
    pub fn initialize_with_timeout(
        timeout: Duration,
    ) -> Result<InitializationResult, &'static str> {
        Self::initialize(InitializationConfig {
            timeout: Some(timeout),
            ..Default::default()
        })
    }
}
