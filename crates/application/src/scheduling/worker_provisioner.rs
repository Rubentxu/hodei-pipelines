//! Worker Provisioner Service
//!
//! This module provides the implementation of the WorkerProvisioner port
//! for provisioning ephemeral workers for pipeline execution with
//! resource-aware scheduling through the Global Resource Controller.

use async_trait::async_trait;
use hodei_pipelines_domain::{
    Pipeline, WorkerId,
    resource_governance::{GlobalResourceController, ResourceRequest, ResourceRequestBuilder},
};
use hodei_pipelines_ports::{
    scheduler_port::SchedulerPort,
    scheduling::worker_provider::{ProviderConfig, WorkerProvider},
    scheduling::worker_provisioner::{
        AllocationMetadata, ProvisionedWorker, ResourceRequirements, WorkerAllocationRequest,
        WorkerProvisioner as WorkerProvisionerTrait, WorkerProvisionerError,
        WorkerProvisionerResult,
    },
    scheduling::worker_template::{EnvVar, WorkerTemplate},
};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, warn};

/// Worker provisioner service implementation
#[derive(Debug, Clone)]
pub struct WorkerProvisionerService<P, S> {
    worker_provider: Arc<P>,
    scheduler: Arc<S>,
    grc: Arc<GlobalResourceController>,
    default_namespace: String,
}

impl<P, S> WorkerProvisionerService<P, S>
where
    P: WorkerProvider + Send + Sync + 'static,
    S: SchedulerPort + Send + Sync + std::fmt::Debug + 'static,
{
    /// Create a new worker provisioner service
    pub fn new(
        worker_provider: Arc<P>,
        scheduler: Arc<S>,
        grc: Arc<GlobalResourceController>,
        default_namespace: String,
    ) -> Self {
        Self {
            worker_provider,
            scheduler,
            grc,
            default_namespace,
        }
    }

    /// Convert pipeline to resource request for GRC
    fn pipeline_to_resource_request(
        &self,
        pipeline: &Pipeline,
        tenant_id: Option<String>,
        priority: u8,
        required_labels: &[(String, String)],
        preferred_labels: &[(String, String)],
    ) -> ResourceRequest {
        // Calculate total resources from all steps
        let (total_cpu, total_memory) = self.calculate_pipeline_resources(pipeline);

        // Convert labels to HashMap
        let required_labels_map: HashMap<String, String> = required_labels
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        let preferred_labels_map: HashMap<String, String> = preferred_labels
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        // Generate request ID
        use hodei_pipelines_domain::resource_governance::RequestId;
        let request_id = RequestId::from(format!("pipeline-{}", pipeline.id));

        let builder = ResourceRequestBuilder::new()
            .request_id(request_id)
            .cpu_millicores(total_cpu)
            .memory_mb(total_memory)
            .priority(priority)
            .pipeline_id(pipeline.id.to_string())
            .required_labels(required_labels_map)
            .preferred_labels(preferred_labels_map);

        let builder = if let Some(tenant) = tenant_id {
            builder.tenant_id(tenant)
        } else {
            builder
        };

        builder.build().expect("Failed to build resource request")
    }

    /// Calculate total resource requirements from pipeline steps
    fn calculate_pipeline_resources(&self, pipeline: &Pipeline) -> (u64, u64) {
        let mut total_cpu = 0u64;
        let mut total_memory = 0u64;

        for step in &pipeline.steps {
            // Get resources from job_spec
            let cpu = step.job_spec.resources.cpu_m;
            let memory = step.job_spec.resources.memory_mb;

            total_cpu += cpu as u64;
            total_memory += memory as u64;
        }

        // Add overhead for tool installation and workspace
        total_cpu += 500; // 0.5 core overhead
        total_memory += 512; // 512MB overhead

        (total_cpu, total_memory)
    }

    /// Wait for worker to be registered in scheduler
    async fn wait_for_worker_registration(
        &self,
        worker_id: &WorkerId,
        timeout_secs: u64,
    ) -> WorkerProvisionerResult<()> {
        use tokio::time::{Duration, sleep, timeout};

        info!("Waiting for worker {} to register in scheduler", worker_id);

        let result = timeout(Duration::from_secs(timeout_secs), async {
            loop {
                match self.scheduler.get_registered_workers().await {
                    Ok(workers) => {
                        if workers.contains(worker_id) {
                            info!("Worker {} successfully registered in scheduler", worker_id);
                            return Ok::<(), WorkerProvisionerError>(());
                        }
                    }
                    Err(e) => {
                        warn!("Failed to get registered workers: {}", e);
                    }
                }
                sleep(Duration::from_millis(500)).await;
            }
        })
        .await;

        match result {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(WorkerProvisionerError::WorkerProvisioning(format!(
                "Failed to wait for worker registration: {}",
                e
            ))),
            Err(_) => Err(WorkerProvisionerError::Timeout(format!(
                "Timeout waiting for worker {} to register ({} seconds)",
                worker_id, timeout_secs
            ))),
        }
    }
}

#[async_trait]
impl<P, S> WorkerProvisionerTrait for WorkerProvisionerService<P, S>
where
    P: WorkerProvider + Send + Sync + 'static,
    S: SchedulerPort + Send + Sync + std::fmt::Debug + 'static,
{
    async fn provision_worker_for_pipeline(
        &self,
        request: WorkerAllocationRequest,
    ) -> WorkerProvisionerResult<ProvisionedWorker> {
        info!(
            "Provisioning worker for pipeline: {} (tenant: {:?})",
            request.pipeline.name, request.tenant_id
        );

        // 1. Create resource request for GRC
        let resource_request = self.pipeline_to_resource_request(
            &request.pipeline,
            request.tenant_id.clone(),
            request.priority,
            &request.required_labels,
            &request.preferred_labels,
        );

        // 2. Find eligible pools using GRC
        let eligible_pools = self
            .grc
            .find_candidate_pools(&resource_request)
            .map_err(|e| {
                WorkerProvisionerError::ResourceAllocation(format!(
                    "Failed to find candidate pools: {}",
                    e
                ))
            })?;

        if eligible_pools.is_empty() {
            return Err(WorkerProvisionerError::ResourceAllocation(
                "No eligible resource pools available for the request".to_string(),
            ));
        }

        // 3. Select best pool (simple strategy: first pool for now)
        let selected_pool = &eligible_pools[0];
        info!(
            "Selected pool for provisioning: {} ({})",
            selected_pool.name, selected_pool.id
        );

        // 4. Allocate resources in GRC
        let allocation_id = format!("pipeline-{}-worker", request.pipeline.id);
        let _allocation = self
            .grc
            .allocate_resources(allocation_id, selected_pool.id.clone(), resource_request)
            .map_err(|e| {
                WorkerProvisionerError::ResourceAllocation(format!(
                    "Failed to allocate resources: {}",
                    e
                ))
            })?;

        // 5. Create worker template
        let worker_template = self
            .create_worker_template(&request.pipeline, &request.resource_requirements)
            .await?;

        // 6. Create provider config
        let provider_config = ProviderConfig {
            provider_type: self.worker_provider.provider_type(),
            name: format!("ephemeral-pipeline-{}", request.pipeline.id),
            namespace: Some(self.default_namespace.clone()),
            docker_host: None,
            kube_config: None,
            template: worker_template.clone(),
            custom_image: None,
            custom_pod_template: None,
        };

        // 7. Generate worker ID
        let worker_id = WorkerId::new();

        // 8. Provision ephemeral worker
        let worker = self
            .worker_provider
            .create_ephemeral_worker(
                worker_id.clone(),
                provider_config,
                Some(request.resource_requirements.estimated_duration_secs + 300), // 5 min grace period
            )
            .await
            .map_err(|e| {
                WorkerProvisionerError::WorkerProvisioning(format!(
                    "Failed to create ephemeral worker: {}",
                    e
                ))
            })?;

        // 9. Wait for worker to register in scheduler
        self.wait_for_worker_registration(&worker_id, 30).await?;

        // 10. Create provisioned worker response
        let provisioned_worker = ProvisionedWorker {
            worker: worker.clone(),
            worker_id: worker_id.clone(),
            template: worker_template,
            pool_id: Some(selected_pool.id.to_string()),
            allocation_metadata: AllocationMetadata {
                allocated_at: chrono::Utc::now(),
                estimated_cost_per_hour: selected_pool
                    .cost_config
                    .as_ref()
                    .map(|c| c.base_hourly_cents as f64),
                auto_cleanup_seconds: Some(
                    request.resource_requirements.estimated_duration_secs + 300,
                ),
                pool_constraints: vec![selected_pool.provider_type.to_string()],
            },
        };

        info!(
            "Successfully provisioned worker {} for pipeline {}",
            worker_id, request.pipeline.id
        );

        Ok(provisioned_worker)
    }

    async fn calculate_resource_requirements(
        &self,
        pipeline: &Pipeline,
    ) -> WorkerProvisionerResult<ResourceRequirements> {
        let (cpu_millicores, memory_mb) = self.calculate_pipeline_resources(pipeline);

        // Estimate duration based on step count and complexity
        let estimated_duration_secs = pipeline.steps.len() as u64 * 60; // 1 minute per step as baseline

        Ok(ResourceRequirements {
            cpu_millicores,
            memory_mb,
            gpu_count: None,      // TODO: Extract from pipeline steps
            storage_gb: Some(10), // Default 10GB workspace
            estimated_duration_secs,
            max_retries: 3,
        })
    }

    async fn create_worker_template(
        &self,
        pipeline: &Pipeline,
        requirements: &ResourceRequirements,
    ) -> WorkerProvisionerResult<WorkerTemplate> {
        // Collect all tools from pipeline steps
        // Note: Currently PipelineStep doesn't have a 'tools' field
        // This needs to be implemented when tool configuration is added to the domain
        let _tools: Vec<hodei_pipelines_ports::scheduling::worker_template::ToolSpec> = Vec::new();

        // Create environment variables from pipeline
        let mut env_vars = Vec::new();
        env_vars.push(EnvVar {
            name: "PIPELINE_ID".to_string(),
            value: pipeline.id.to_string(),
        });
        env_vars.push(EnvVar {
            name: "PIPELINE_NAME".to_string(),
            value: pipeline.name.clone(),
        });

        // Add step-specific environment variables
        for (i, step) in pipeline.steps.iter().enumerate() {
            env_vars.push(EnvVar {
                name: format!("STEP_{}_NAME", i),
                value: step.name.clone(),
            });
            env_vars.push(EnvVar {
                name: format!("STEP_{}_ID", i),
                value: step.id.to_string(),
            });
        }

        // Note: TemplateResourceRequirements is not used directly
        // Resource requirements are set via with_cpu() and with_memory() methods

        // Create worker template
        let mut template = WorkerTemplate::new(
            &format!("pipeline-{}-template", pipeline.id),
            "1.0.0",            // Default version
            "hwp-agent:latest", // Default HWP Agent image
        )
        .with_working_dir("/workspace")
        .with_cpu(&format!("{}m", requirements.cpu_millicores))
        .with_memory(&format!("{}Mi", requirements.memory_mb));

        // Add environment variables
        for env_var in env_vars {
            template = template.with_env(&env_var.name, &env_var.value);
        }

        // Add tools (currently empty as PipelineStep doesn't have tools field)
        // This will be implemented when tool configuration is added to the domain

        Ok(template)
    }
}
