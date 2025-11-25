//! Re-exports from shared-types to maintain compatibility

pub use hodei_shared_types::specifications::{
    AndSpec, NotSpec, OrSpec, Specification, SpecificationResult,
};

pub use hodei_shared_types::job_specifications::{
    JobCommandNotEmptySpec, JobImageNotEmptySpec, JobNameNotEmptySpec, JobTimeoutPositiveSpec,
    ValidJobSpec, validate_job_spec,
};

pub use hodei_shared_types::job_definitions::JobSpec;
