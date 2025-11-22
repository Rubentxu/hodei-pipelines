/// Builder pattern for creating pipelines ergonomically
use hodei_sdk_core::{PipelineConfig, ResourceRequirements, StageConfig, TriggerConfig};
use std::collections::HashMap;

/// Builder for creating pipeline configurations
///
/// # Example
/// ```
/// use hodei_rust_sdk::PipelineBuilder;
///
/// let pipeline = PipelineBuilder::new("my-pipeline")
///     .description("Build and test pipeline")
///     .stage("build", "rust:1.70", vec!["cargo build", "cargo test"])
///     .stage_with_resources(
///         "deploy",
///         "alpine:latest",
///         vec!["./deploy.sh"],
///         2.0,  // CPU cores
///         2048, // Memory MB
///     )
///     .env("RUST_LOG", "info")
///     .trigger_on_push("main", "https://github.com/user/repo")
///     .build();
/// ```
pub struct PipelineBuilder {
    config: PipelineConfig,
}

impl PipelineBuilder {
    /// Create a new pipeline builder
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            config: PipelineConfig {
                name: name.into(),
                description: None,
                stages: Vec::new(),
                environment: HashMap::new(),
                triggers: Vec::new(),
            },
        }
    }

    /// Set the pipeline description
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.config.description = Some(description.into());
        self
    }

    /// Add a stage to the pipeline
    pub fn stage(
        mut self,
        name: impl Into<String>,
        image: impl Into<String>,
        commands: Vec<impl Into<String>>,
    ) -> Self {
        self.config.stages.push(StageConfig {
            name: name.into(),
            image: image.into(),
            commands: commands.into_iter().map(Into::into).collect(),
            dependencies: Vec::new(),
            environment: HashMap::new(),
            resources: None,
        });
        self
    }

    /// Add a stage with resource requirements
    pub fn stage_with_resources(
        mut self,
        name: impl Into<String>,
        image: impl Into<String>,
        commands: Vec<impl Into<String>>,
        cpu: f32,
        memory: u32,
    ) -> Self {
        self.config.stages.push(StageConfig {
            name: name.into(),
            image: image.into(),
            commands: commands.into_iter().map(Into::into).collect(),
            dependencies: Vec::new(),
            environment: HashMap::new(),
            resources: Some(ResourceRequirements {
                cpu,
                memory,
                disk: None,
            }),
        });
        self
    }

    /// Add a stage with dependencies on other stages
    pub fn stage_with_deps(
        mut self,
        name: impl Into<String>,
        image: impl Into<String>,
        commands: Vec<impl Into<String>>,
        dependencies: Vec<impl Into<String>>,
    ) -> Self {
        self.config.stages.push(StageConfig {
            name: name.into(),
            image: image.into(),
            commands: commands.into_iter().map(Into::into).collect(),
            dependencies: dependencies.into_iter().map(Into::into).collect(),
            environment: HashMap::new(),
            resources: None,
        });
        self
    }

    /// Add a stage with custom environment variables
    pub fn stage_with_env(
        mut self,
        name: impl Into<String>,
        image: impl Into<String>,
        commands: Vec<impl Into<String>>,
        environment: HashMap<String, String>,
    ) -> Self {
        self.config.stages.push(StageConfig {
            name: name.into(),
            image: image.into(),
            commands: commands.into_iter().map(Into::into).collect(),
            dependencies: Vec::new(),
            environment,
            resources: None,
        });
        self
    }

    /// Add a global environment variable
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.config.environment.insert(key.into(), value.into());
        self
    }

    /// Add multiple environment variables
    pub fn envs(mut self, vars: HashMap<String, String>) -> Self {
        self.config.environment.extend(vars);
        self
    }

    /// Add a git push trigger
    pub fn trigger_on_push(
        mut self,
        branch: impl Into<String>,
        repository: impl Into<String>,
    ) -> Self {
        self.config.triggers.push(TriggerConfig::GitPush {
            branch: branch.into(),
            repository: repository.into(),
        });
        self
    }

    /// Add a scheduled trigger (cron)
    pub fn trigger_on_schedule(mut self, cron: impl Into<String>) -> Self {
        self.config.triggers.push(TriggerConfig::Schedule {
            cron: cron.into(),
        });
        self
    }

    /// Add a manual trigger
    pub fn trigger_manual(mut self) -> Self {
        self.config.triggers.push(TriggerConfig::Manual);
        self
    }

    /// Add a webhook trigger
    pub fn trigger_on_webhook(mut self, url: impl Into<String>) -> Self {
        self.config.triggers.push(TriggerConfig::Webhook {
            url: url.into(),
        });
        self
    }

    /// Build the pipeline configuration
    pub fn build(self) -> PipelineConfig {
        self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_pipeline_builder() {
        let pipeline = PipelineBuilder::new("test-pipeline")
            .description("Test pipeline")
            .stage("build", "rust:1.70", vec!["cargo build"])
            .build();

        assert_eq!(pipeline.name, "test-pipeline");
        assert_eq!(pipeline.description, Some("Test pipeline".to_string()));
        assert_eq!(pipeline.stages.len(), 1);
        assert_eq!(pipeline.stages[0].name, "build");
    }

    #[test]
    fn test_multi_stage_pipeline() {
        let pipeline = PipelineBuilder::new("multi-stage")
            .stage("build", "rust:1.70", vec!["cargo build"])
            .stage("test", "rust:1.70", vec!["cargo test"])
            .stage_with_deps(
                "deploy",
                "alpine:latest",
                vec!["./deploy.sh"],
                vec!["build", "test"],
            )
            .build();

        assert_eq!(pipeline.stages.len(), 3);
        assert_eq!(pipeline.stages[2].dependencies, vec!["build", "test"]);
    }

    #[test]
    fn test_pipeline_with_resources() {
        let pipeline = PipelineBuilder::new("resource-pipeline")
            .stage_with_resources(
                "build",
                "rust:1.70",
                vec!["cargo build"],
                4.0,
                4096,
            )
            .build();

        let resources = pipeline.stages[0].resources.as_ref().unwrap();
        assert_eq!(resources.cpu, 4.0);
        assert_eq!(resources.memory, 4096);
    }

    #[test]
    fn test_pipeline_with_environment() {
        let mut env = HashMap::new();
        env.insert("KEY1".to_string(), "value1".to_string());

        let pipeline = PipelineBuilder::new("env-pipeline")
            .env("RUST_LOG", "debug")
            .envs(env)
            .stage("build", "rust:1.70", vec!["cargo build"])
            .build();

        assert_eq!(pipeline.environment.get("RUST_LOG"), Some(&"debug".to_string()));
        assert_eq!(pipeline.environment.get("KEY1"), Some(&"value1".to_string()));
    }

    #[test]
    fn test_pipeline_with_triggers() {
        let pipeline = PipelineBuilder::new("trigger-pipeline")
            .trigger_on_push("main", "https://github.com/user/repo")
            .trigger_on_schedule("0 0 * * *")
            .trigger_manual()
            .build();

        assert_eq!(pipeline.triggers.len(), 3);

        match &pipeline.triggers[0] {
            TriggerConfig::GitPush { branch, repository } => {
                assert_eq!(branch, "main");
                assert_eq!(repository, "https://github.com/user/repo");
            }
            _ => panic!("Expected GitPush trigger"),
        }

        match &pipeline.triggers[1] {
            TriggerConfig::Schedule { cron } => {
                assert_eq!(cron, "0 0 * * *");
            }
            _ => panic!("Expected Schedule trigger"),
        }

        assert!(matches!(pipeline.triggers[2], TriggerConfig::Manual));
    }
}
