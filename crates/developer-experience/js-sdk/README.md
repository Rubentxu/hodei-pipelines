# Hodei TypeScript/JavaScript SDK

Official TypeScript/JavaScript SDK for the Hodei CI/CD platform.

[![npm version](https://badge.fury.io/js/%40hodei%2Fsdk.svg)](https://www.npmjs.com/package/@hodei/sdk)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Features

- 🚀 **Full TypeScript support** with complete type definitions
- 🔄 **Promise-based async/await** API
- 🏗️ **Fluent builder** for pipeline configurations
- 🧪 **Comprehensive testing** with >90% coverage
- 📦 **Node.js and browser** compatible
- 🔒 **Type-safe** API interactions

## Installation

```bash
npm install @hodei/sdk
```

Or with yarn:

```bash
yarn add @hodei/sdk
```

## Quick Start

```typescript
import { CicdClient, PipelineBuilder } from '@hodei/sdk';

// Create client
const client = new CicdClient({
  baseUrl: 'https://api.hodei.example.com',
  apiToken: 'your-api-token'
});

// Build pipeline configuration
const pipeline = new PipelineBuilder('my-pipeline')
  .description('Build and deploy application')
  .stage('build', 'node:18', ['npm install', 'npm run build'])
  .stage('test', 'node:18', ['npm test'])
  .triggerOnPush('main', 'https://github.com/user/repo')
  .build();

// Create and execute pipeline
const created = await client.createPipeline(pipeline);
const job = await client.executePipeline(created.id);

// Wait for completion
const completed = await client.waitForCompletion(job.id);
console.log('Job status:', completed.status);
```

## Usage Examples

### Creating a Pipeline

```typescript
const pipeline = new PipelineBuilder('web-app-pipeline')
  .description('Build and deploy web application')
  .env('NODE_ENV', 'production')
  .env('LOG_LEVEL', 'info')
  .stage('build', 'node:18', [
    'npm install',
    'npm run build'
  ])
  .stageWithResources(
    'test',
    'node:18',
    ['npm test'],
    2.0,  // 2 CPU cores
    4096  // 4GB RAM
  )
  .stageWithDeps(
    'deploy',
    'alpine:latest',
    ['./deploy.sh'],
    ['build', 'test']  // Depends on build and test
  )
  .triggerOnPush('main', 'https://github.com/user/repo')
  .triggerOnSchedule('0 2 * * *')  // Daily at 2 AM
  .build();

const created = await client.createPipeline(pipeline);
```

### Executing and Monitoring a Pipeline

```typescript
// Execute pipeline
const job = await client.executePipeline(pipelineId);
console.log('Job started:', job.id);

// Poll for completion
const completed = await client.waitForCompletion(job.id, 5000);
console.log('Job status:', completed.status);

// Get logs
const logs = await client.getJobLogs(job.id);
console.log(logs);
```

### Managing Workers

```typescript
// List all workers
const workers = await client.listWorkers();
workers.forEach(worker => {
  console.log(`${worker.name}: ${worker.status}`);
  console.log(`  CPU: ${worker.cpu_usage * 100}%`);
  console.log(`  Memory: ${worker.memory_usage * 100}%`);
});

// Scale worker group
await client.scaleWorkers('kubernetes-workers', 10);
```

### Error Handling

```typescript
import { SdkError } from '@hodei/sdk';

try {
  const pipeline = await client.getPipeline('non-existent-id');
} catch (error) {
  if (error instanceof SdkError) {
    if (error.isNotFound()) {
      console.log('Pipeline not found');
    } else if (error.isAuthError()) {
      console.log('Authentication failed');
    } else if (error.isTimeout()) {
      console.log('Request timed out');
    }
    console.log('Status code:', error.statusCode);
  }
}
```

## API Reference

### CicdClient

Main client for interacting with the Hodei CI/CD platform.

#### Constructor

```typescript
new CicdClient(config: ClientConfig)
```

**ClientConfig**:
- `baseUrl: string` - Base URL of the API
- `apiToken: string` - Authentication token
- `timeout?: number` - Request timeout in milliseconds (default: 30000)
- `maxRetries?: number` - Maximum number of retries (default: 3)

#### Methods

##### Pipeline Operations

- `createPipeline(config: PipelineConfig): Promise<Pipeline>`
- `getPipeline(pipelineId: string): Promise<Pipeline>`
- `listPipelines(): Promise<Pipeline[]>`
- `executePipeline(pipelineId: string): Promise<Job>`
- `deletePipeline(pipelineId: string): Promise<void>`

##### Job Operations

- `getJobStatus(jobId: string): Promise<Job>`
- `getJobLogs(jobId: string): Promise<string>`
- `waitForCompletion(jobId: string, pollInterval?: number): Promise<Job>`

##### Worker Operations

- `listWorkers(): Promise<Worker[]>`
- `getWorker(workerId: string): Promise<Worker>`
- `scaleWorkers(workerGroup: string, targetCount: number): Promise<void>`

### PipelineBuilder

Fluent builder for creating pipeline configurations.

#### Constructor

```typescript
new PipelineBuilder(name: string)
```

#### Methods

- `description(desc: string): this`
- `stage(name: string, image: string, commands: string[]): this`
- `stageWithResources(name: string, image: string, commands: string[], cpu: number, memory: number): this`
- `stageWithDeps(name: string, image: string, commands: string[], dependencies: string[]): this`
- `stageWithEnv(name: string, image: string, commands: string[], env: Record<string, string>): this`
- `env(key: string, value: string): this`
- `envs(vars: Record<string, string>): this`
- `triggerOnPush(branch: string, repository: string): this`
- `triggerOnSchedule(cron: string): this`
- `triggerManual(): this`
- `triggerOnWebhook(url: string): this`
- `build(): PipelineConfig`

### Types

#### Pipeline Statuses

```typescript
enum PipelineStatus {
  PENDING = 'PENDING',
  RUNNING = 'RUNNING',
  SUCCESS = 'SUCCESS',
  FAILED = 'FAILED',
  CANCELLED = 'CANCELLED'
}
```

#### Job Statuses

```typescript
enum JobStatus {
  QUEUED = 'QUEUED',
  RUNNING = 'RUNNING',
  SUCCESS = 'SUCCESS',
  FAILED = 'FAILED',
  CANCELLED = 'CANCELLED'
}
```

#### Worker Statuses

```typescript
enum WorkerStatus {
  ACTIVE = 'ACTIVE',
  BUSY = 'BUSY',
  INACTIVE = 'INACTIVE',
  FAILED = 'FAILED'
}
```

## Development

### Setup

```bash
npm install
```

### Build

```bash
npm run build
```

### Test

```bash
npm test
npm run test:coverage
```

### Lint

```bash
npm run lint
npm run format
```

## License

MIT © Hodei Team

## Contributing

Contributions are welcome! Please read our [Contributing Guide](../../CONTRIBUTING.md) for details.

## Support

- 📧 Email: support@hodei.example.com
- 🐛 Issues: [GitHub Issues](https://github.com/Rubentxu/hodei-jobs/issues)
- 📖 Documentation: [https://docs.hodei.example.com](https://docs.hodei.example.com)
