//! Build script for HWP Agent
//!
//! This build script performs binary size optimizations and verification.

fn main() {
    // Set optimization flags for smaller binary size
    println!("cargo:rerun-if-env-changed=HODEI_BUILD_OPTS");

    // Configure link-time optimizations
    println!("cargo:rustc-link-arg=-s"); // Strip symbols
    println!("cargo:rustc-opt-level=z"); // Optimize for size

    // Generate version info
    let version = format!(
        "{}-{}-{}",
        env!("CARGO_PKG_VERSION"),
        std::env::var("GIT_COMMIT").unwrap_or_else(|_| "unknown".to_string()),
        std::env::var("TARGET").unwrap_or_else(|_| "unknown".to_string())
    );

    println!("cargo:rustc-env=HODEI_AGENT_VERSION={}", version);

    // Print build info
    println!("Building HWP Agent with optimizations for size <5MB");
}
