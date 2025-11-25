# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- PostgreSQL Extractors Infrastructure: Created reusable row extractors for database operations
- Comprehensive documentation for DDD tactical improvements

### Changed
- Refactored Specification Pattern: Consolidated duplicated specifications into shared-types module
- Improved WorkerMapper to preserve current_jobs field during persistence roundtrip
- Fixed RowExtractor method call syntax for proper Rust patterns

### Fixed
- Compilation error in hodei-adapters: RowExtractor test calling associated function as method
- WorkerMapper data loss bug: Fixed critical issue where current_jobs field was lost
- Specification duplication: Eliminated 148 lines of duplicated code between shared-types and core

### Removed
- Duplicated job specifications from core module (consolidated into shared-types)
- Unused code paths and dead code

### Test Results
- Total tests: 696 (100% passing)
- All tests verify real business functionality
- Clean workspace compilation with 0 errors

### Development Progress
- Complete EPIC-09: Resource Pool Integration (20 user stories)
- GitHub repository: hodei-pipelines
- Project rebranding: "Hodei Jobs" → "Hodei Pipelines"
- Observability API (US-09.6.4): Health checks, metrics, error tracking, audit logging
- Fixed Rust borrowing errors (E0502) in concurrent operations
- Added Clone traits for request structs
- Corrected Axum route syntax (:param → {param})
- Arc wrapping for QuotaEnforcementEngine
- ResourceQuota fields alignment (cpu_cores/storage_gb → cpu_m/gpu)
- Async test setup fixes

---

## Legend
- [Added] for new features
- [Changed] for changes in existing functionality
- [Deprecated] for soon-to-be removed features
- [Removed] for now removed features
- [Fixed] for any bug fixes
- [Security] for security vulnerabilities
