//! Logging utilities for tests

use tracing::{error, info};

/// Initialize test logging
pub fn init() {
    tracing_subscriber::fmt::init();
}

/// Log test step
pub fn log_step(step: &str) {
    info!("[TEST STEP] {}", step);
}

/// Log test error
pub fn log_error(error: &str) {
    error!("[TEST ERROR] {}", error);
}
