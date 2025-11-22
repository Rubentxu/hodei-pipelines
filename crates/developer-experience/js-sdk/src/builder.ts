/**
 * Builder pattern for creating pipeline configurations
 */
import { PipelineConfig, StageConfig, TriggerConfig, ResourceRequirements } from './types';

/**
 * Fluent builder for creating pipeline configurations
 *
 * @example
 * ```typescript
 * const pipeline = new PipelineBuilder('my-pipeline')
 *   .description('Build and test pipeline')
 *   .stage('build', 'node:18', ['npm install', 'npm run build'])
 *   .stageWithResources('deploy', 'alpine:latest', ['./deploy.sh'], 2.0, 2048)
 *   .env('NODE_ENV', 'production')
 *   .triggerOnPush('main', 'https://github.com/user/repo')
 *   .build();
 * ```
 */
export class PipelineBuilder {
  private config: PipelineConfig;

  /**
   * Create a new pipeline builder
   *
   * @param name - Pipeline name
   */
  constructor(name: string) {
    this.config = {
      name,
      stages: [],
      environment: {},
      triggers: [],
    };
  }

  /**
   * Set the pipeline description
   *
   * @param description - Pipeline description
   */
  description(description: string): this {
    this.config.description = description;
    return this;
  }

  /**
   * Add a stage to the pipeline
   *
   * @param name - Stage name
   * @param image - Docker image
   * @param commands - Commands to execute
   */
  stage(name: string, image: string, commands: string[]): this {
    this.config.stages.push({
      name,
      image,
      commands,
      dependencies: [],
      environment: {},
    });
    return this;
  }

  /**
   * Add a stage with resource requirements
   *
   * @param name - Stage name
   * @param image - Docker image
   * @param commands - Commands to execute
   * @param cpu - CPU cores
   * @param memory - Memory in MB
   */
  stageWithResources(
    name: string,
    image: string,
    commands: string[],
    cpu: number,
    memory: number
  ): this {
    this.config.stages.push({
      name,
      image,
      commands,
      dependencies: [],
      environment: {},
      resources: { cpu, memory },
    });
    return this;
  }

  /**
   * Add a stage with dependencies
   *
   * @param name - Stage name
   * @param image - Docker image
   * @param commands - Commands to execute
   * @param dependencies - Stage dependencies
   */
  stageWithDeps(
    name: string,
    image: string,
    commands: string[],
    dependencies: string[]
  ): this {
    this.config.stages.push({
      name,
      image,
      commands,
      dependencies,
      environment: {},
    });
    return this;
  }

  /**
   * Add a stage with custom environment variables
   *
   * @param name - Stage name
   * @param image - Docker image
   * @param commands - Commands to execute
   * @param environment - Environment variables
   */
  stageWithEnv(
    name: string,
    image: string,
    commands: string[],
    environment: Record<string, string>
  ): this {
    this.config.stages.push({
      name,
      image,
      commands,
      dependencies: [],
      environment,
    });
    return this;
  }

  /**
   * Add a global environment variable
   *
   * @param key - Environment variable key
   * @param value - Environment variable value
   */
  env(key: string, value: string): this {
    if (!this.config.environment) {
      this.config.environment = {};
    }
    this.config.environment[key] = value;
    return this;
  }

  /**
   * Add multiple environment variables
   *
   * @param vars - Environment variables
   */
  envs(vars: Record<string, string>): this {
    if (!this.config.environment) {
      this.config.environment = {};
    }
    this.config.environment = { ...this.config.environment, ...vars };
    return this;
  }

  /**
   * Add a git push trigger
   *
   * @param branch - Git branch
   * @param repository - Repository URL
   */
  triggerOnPush(branch: string, repository: string): this {
    if (!this.config.triggers) {
      this.config.triggers = [];
    }
    this.config.triggers.push({
      type: 'git_push',
      branch,
      repository,
    });
    return this;
  }

  /**
   * Add a scheduled trigger (cron)
   *
   * @param cron - Cron expression
   */
  triggerOnSchedule(cron: string): this {
    if (!this.config.triggers) {
      this.config.triggers = [];
    }
    this.config.triggers.push({
      type: 'schedule',
      cron,
    });
    return this;
  }

  /**
   * Add a manual trigger
   */
  triggerManual(): this {
    if (!this.config.triggers) {
      this.config.triggers = [];
    }
    this.config.triggers.push({
      type: 'manual',
    });
    return this;
  }

  /**
   * Add a webhook trigger
   *
   * @param url - Webhook URL
   */
  triggerOnWebhook(url: string): this {
    if (!this.config.triggers) {
      this.config.triggers = [];
    }
    this.config.triggers.push({
      type: 'webhook',
      url,
    });
    return this;
  }

  /**
   * Build the pipeline configuration
   *
   * @returns Pipeline configuration
   */
  build(): PipelineConfig {
    return this.config;
  }
}
