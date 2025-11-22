/**
 * CICD Client for Hodei platform
 */
import { HttpClient } from './client';
import {
  ClientConfig,
  Pipeline,
  PipelineConfig,
  Job,
  JobStatus,
  Worker,
} from './types';

/**
 * Main client for interacting with Hodei CI/CD platform
 *
 * @example
 * ```typescript
 * const client = new CicdClient({
 *   baseUrl: 'https://api.hodei.example.com',
 *   apiToken: 'your-token'
 * });
 *
 * const pipeline = await client.createPipeline({
 *   name: 'my-pipeline',
 *   stages: [
 *     {
 *       name: 'build',
 *       image: 'node:18',
 *       commands: ['npm install', 'npm run build']
 *     }
 *   ]
 * });
 * ```
 */
export class CicdClient {
  private httpClient: HttpClient;

  /**
   * Create a new CICD client
   *
   * @param config - Client configuration
   */
  constructor(config: ClientConfig) {
    this.httpClient = new HttpClient(config);
  }

  /**
   * Create a new pipeline
   *
   * @param config - Pipeline configuration
   * @returns Created pipeline
   *
   * @example
   * ```typescript
   * const pipeline = await client.createPipeline({
   *   name: 'my-pipeline',
   *   stages: [{
   *     name: 'build',
   *     image: 'rust:1.70',
   *     commands: ['cargo build']
   *   }]
   * });
   * ```
   */
  async createPipeline(config: PipelineConfig): Promise<Pipeline> {
    return this.httpClient.post<Pipeline, PipelineConfig>(
      '/api/v1/pipelines',
      config
    );
  }

  /**
   * Get a pipeline by ID
   *
   * @param pipelineId - Pipeline identifier
   * @returns Pipeline details
   */
  async getPipeline(pipelineId: string): Promise<Pipeline> {
    return this.httpClient.get<Pipeline>(`/api/v1/pipelines/${pipelineId}`);
  }

  /**
   * List all pipelines
   *
   * @returns Array of pipelines
   */
  async listPipelines(): Promise<Pipeline[]> {
    return this.httpClient.get<Pipeline[]>('/api/v1/pipelines');
  }

  /**
   * Execute a pipeline
   *
   * @param pipelineId - Pipeline identifier
   * @returns Execution job
   *
   * @example
   * ```typescript
   * const job = await client.executePipeline('pipeline-123');
   * console.log('Job started:', job.id);
   * ```
   */
  async executePipeline(pipelineId: string): Promise<Job> {
    return this.httpClient.post<Job>(
      `/api/v1/pipelines/${pipelineId}/execute`,
      {}
    );
  }

  /**
   * Get job status
   *
   * @param jobId - Job identifier
   * @returns Job details
   */
  async getJobStatus(jobId: string): Promise<Job> {
    return this.httpClient.get<Job>(`/api/v1/jobs/${jobId}`);
  }

  /**
   * Get job logs
   *
   * @param jobId - Job identifier
   * @returns Job logs as string
   */
  async getJobLogs(jobId: string): Promise<string> {
    return this.httpClient.get<string>(`/api/v1/jobs/${jobId}/logs`);
  }

  /**
   * Wait for job completion
   *
   * @param jobId - Job identifier
   * @param pollInterval - Interval between status checks in milliseconds (default: 5000)
   * @returns Completed job
   *
   * @example
   * ```typescript
   * const job = await client.executePipeline('pipeline-123');
   * const completedJob = await client.waitForCompletion(job.id, 3000);
   * console.log('Job completed with status:', completedJob.status);
   * ```
   */
  async waitForCompletion(
    jobId: string,
    pollInterval: number = 5000
  ): Promise<Job> {
    while (true) {
      const job = await this.getJobStatus(jobId);

      if (
        job.status === JobStatus.SUCCESS ||
        job.status === JobStatus.FAILED ||
        job.status === JobStatus.CANCELLED
      ) {
        return job;
      }

      // Wait before polling again
      await new Promise((resolve) => setTimeout(resolve, pollInterval));
    }
  }

  /**
   * Delete a pipeline
   *
   * @param pipelineId - Pipeline identifier
   */
  async deletePipeline(pipelineId: string): Promise<void> {
    return this.httpClient.delete<void>(`/api/v1/pipelines/${pipelineId}`);
  }

  /**
   * List all workers
   *
   * @returns Array of workers
   */
  async listWorkers(): Promise<Worker[]> {
    return this.httpClient.get<Worker[]>('/api/v1/workers');
  }

  /**
   * Get worker by ID
   *
   * @param workerId - Worker identifier
   * @returns Worker details
   */
  async getWorker(workerId: string): Promise<Worker> {
    return this.httpClient.get<Worker>(`/api/v1/workers/${workerId}`);
  }

  /**
   * Scale workers in a worker group
   *
   * @param workerGroup - Worker group name
   * @param targetCount - Target number of workers
   *
   * @example
   * ```typescript
   * await client.scaleWorkers('kubernetes-workers', 10);
   * ```
   */
  async scaleWorkers(workerGroup: string, targetCount: number): Promise<void> {
    return this.httpClient.post<void, { target_count: number }>(
      `/api/v1/workers/groups/${workerGroup}/scale`,
      { target_count: targetCount }
    );
  }
}
