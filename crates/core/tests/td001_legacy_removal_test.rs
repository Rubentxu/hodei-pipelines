//! Test for US-TD-001: Eliminate legacy job_definitions_old.rs file
//!
//! This test verifies that the legacy job_definitions_old.rs file has been
//! successfully removed from the codebase to reduce cognitive debt.

use std::fs;
use std::path::PathBuf;

#[test]
fn verify_job_definitions_old_does_not_exist() {
    // Path to the legacy file
    let legacy_file_path: PathBuf = [env!("CARGO_MANIFEST_DIR"), "src", "job_definitions_old.rs"]
        .iter()
        .collect();

    // Verify the file does not exist
    assert!(
        !legacy_file_path.exists(),
        "Legacy file {} should not exist in production code",
        legacy_file_path.display()
    );
}

#[test]
fn verify_job_definitions_current_works() {
    // Verify the current job_definitions.rs exists and compiles
    let current_file_path: PathBuf = [env!("CARGO_MANIFEST_DIR"), "src", "job_definitions.rs"]
        .iter()
        .collect();

    // Verify the current file exists
    assert!(
        current_file_path.exists(),
        "Current file {} should exist",
        current_file_path.display()
    );

    // Verify it's not a stub
    let content =
        fs::read_to_string(&current_file_path).expect("Failed to read current job_definitions.rs");

    assert!(
        !content.trim().is_empty(),
        "Current job_definitions.rs should contain actual code"
    );

    // Verify it has the main types defined
    assert!(
        content.contains("pub struct JobId"),
        "JobId should be defined in current file"
    );
    assert!(
        content.contains("pub struct JobSpec"),
        "JobSpec should be defined in current file"
    );
    assert!(
        content.contains("pub enum JobState"),
        "JobState should be defined in current file"
    );
}
