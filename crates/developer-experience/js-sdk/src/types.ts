/**
 * Shared types for the Hodei SDK
 */

/**
 * Pipeline status enumeration
 */
export enum PipelineStatus {
  PENDING = 'PENDING',
  RUNNING = 'RUNNING',
  SUCCESS = 'SUCCESS',
  FAILED = 'FAILED',
  CANCELLED = 'CANCELLED',
}

/**
 * Job status enumeration
 */
export enum JobStatus {
  QUEUED = 'QUEUED',
  RUNNING = 'RUNNING',
  SUCCESS = 'SUCCESS',
  FAILED = 'FAILED',
  CANCELLED = 'CANCELLED',
}

/**
 * Worker status enumeration
 */
export enum WorkerStatus {
  ACTIVE = 'ACTIVE',
  BUSY = 'BUSY',
  INACTIVE = 'INACTIVE',
  FAILED = 'FAILED',
}

/**
 * Resource requirements for a stage
 */
export interface ResourceRequirements {
  /** CPU cores */
  cpu: number;
  /** Memory in MB */
  memory: number;
  /** Disk space in MB (optional) */
  disk?: number;
}

/**
 * Stage configuration
 */
export interface StageConfig {
  /** Stage name */
  name: string;
  /** Docker image to use */
  image: string;
  /** Commands to execute */
  commands: string[];
  /** Stage dependencies */
  dependencies?: string[];
  /** Stage-specific environment variables */
  environment?: Record<string, string>;
  /** Resource requirements */
  resources?: ResourceRequirements;
}

/**
 * Trigger configuration (discriminated union)
 */
export type TriggerConfig =
  | { type: 'git_push'; branch: string; repository: string }
  | { type: 'schedule'; cron: string }
  | { type: 'manual' }
  | { type: 'webhook'; url: string };

/**
 * Pipeline configuration
 */
export interface PipelineConfig {
  /** Pipeline name */
  name: string;
  /** Pipeline description */
  description?: string;
  /** List of stages in the pipeline */
  stages: StageConfig[];
  /** Environment variables */
  environment?: Record<string, string>;
  /** Pipeline triggers */
  triggers?: TriggerConfig[];
}

/**
 * Pipeline entity
 */
export interface Pipeline {
  /** Pipeline unique identifier */
  id: string;
  /** Pipeline name */
  name: string;
  /** Pipeline status */
  status: PipelineStatus;
  /** Creation timestamp */
  created_at: Date;
  /** Last update timestamp */
  updated_at?: Date;
  /** Pipeline configuration */
  config: PipelineConfig;
}

/**
 * Job execution result
 */
export interface Job {
  /** Job unique identifier */
  id: string;
  /** Pipeline ID this job belongs to */
  pipeline_id: string;
  /** Job name */
  name: string;
  /** Job status */
  status: JobStatus;
  /** Start time */
  started_at: Date;
  /** End time */
  finished_at?: Date;
  /** Error message if failed */
  error_message?: string;
}

/**
 * Worker information
 */
export interface Worker {
  /** Worker unique identifier */
  id: string;
  /** Worker name */
  name: string;
  /** Worker status */
  status: WorkerStatus;
  /** Provider type */
  provider: string;
  /** CPU usage (0.0 to 1.0) */
  cpu_usage: number;
  /** Memory usage (0.0 to 1.0) */
  memory_usage: number;
  /** Number of jobs processed */
  jobs_processed: number;
}

/**
 * SDK Error class
 */
export class SdkError extends Error {
  constructor(
    message: string,
    public readonly statusCode?: number,
    public readonly cause?: Error
  ) {
    super(message);
    this.name = 'SdkError';
    Object.setPrototypeOf(this, SdkError.prototype);
  }

  /**
   * Check if error is a 404 Not Found
   */
  isNotFound(): boolean {
    return this.statusCode === 404;
  }

  /**
   * Check if error is authentication related
   */
  isAuthError(): boolean {
    return this.statusCode === 401 || this.statusCode === 403;
  }

  /**
   * Check if error is a timeout
   */
  isTimeout(): boolean {
    return this.message.includes('timeout') || this.message.includes('ETIMEDOUT');
  }
}

/**
 * Client configuration
 */
export interface ClientConfig {
  /** Base URL for the API */
  baseUrl: string;
  /** API authentication token */
  apiToken: string;
  /** Request timeout in milliseconds */
  timeout?: number;
  /** Maximum number of retries */
  maxRetries?: number;
}
