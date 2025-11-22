/**
 * Hodei TypeScript/JavaScript SDK
 *
 * This SDK provides a comprehensive client for interacting with the Hodei CI/CD platform.
 *
 * @packageDocumentation
 *
 * @example
 * ```typescript
 * import { CicdClient, PipelineBuilder } from '@hodei/sdk';
 *
 * const client = new CicdClient({
 *   baseUrl: 'https://api.hodei.example.com',
 *   apiToken: 'your-token'
 * });
 *
 * const pipeline = new PipelineBuilder('my-pipeline')
 *   .description('Build and deploy')
 *   .stage('build', 'node:18', ['npm install', 'npm run build'])
 *   .stage('test', 'node:18', ['npm test'])
 *   .build();
 *
 * const created = await client.createPipeline(pipeline);
 * const job = await client.executePipeline(created.id);
 * ```
 */

// Export main client
export { CicdClient } from './cicd-client';

// Export builder
export { PipelineBuilder } from './builder';

// Export types
export {
  PipelineStatus,
  JobStatus,
  WorkerStatus,
  SdkError,
} from './types';

export type {
  ClientConfig,
  Pipeline,
  PipelineConfig,
  StageConfig,
  TriggerConfig,
  ResourceRequirements,
  Job,
  Worker,
} from './types';
