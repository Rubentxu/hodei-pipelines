/**
 * Basic example of using the Hodei TypeScript SDK
 */
import { CicdClient, PipelineBuilder } from '../src';

async function main() {
  // Create the CICD client
  const client = new CicdClient({
    baseUrl: 'http://localhost:8080',
    apiToken: 'your-api-token',
    timeout: 30000,
  });

  console.log('🚀 Hodei CI/CD TypeScript SDK Example\n');

  // Build a pipeline configuration using the fluent builder
  const pipelineConfig = new PipelineBuilder('typescript-build-pipeline')
    .description('Build and test TypeScript project')
    .env('NODE_ENV', 'production')
    .env('LOG_LEVEL', 'info')
    .stage('checkout', 'alpine/git:latest', [
      'git clone https://github.com/user/repo.git /workspace',
      'cd /workspace',
    ])
    .stageWithResources(
      'build',
      'node:18',
      ['npm install', 'npm run build'],
      2.0, // 2 CPU cores
      2048 // 2GB RAM
    )
    .stageWithDeps('test', 'node:18', ['npm test'], ['build']) // Depends on build
    .stageWithDeps('deploy', 'alpine:latest', ['./scripts/deploy.sh'], ['test']) // Depends on test
    .triggerOnPush('main', 'https://github.com/user/repo')
    .triggerOnSchedule('0 2 * * *') // Daily at 2 AM
    .build();

  try {
    // Create the pipeline
    console.log('Creating pipeline...');
    const pipeline = await client.createPipeline(pipelineConfig);
    console.log(`✓ Pipeline created: ${pipeline.name} (ID: ${pipeline.id})`);
    console.log(`  Status: ${pipeline.status}`);
    console.log(`  Created at: ${pipeline.created_at}`);

    // Execute the pipeline
    console.log('\nExecuting pipeline...');
    const job = await client.executePipeline(pipeline.id);
    console.log(`✓ Job started: ${job.id}`);
    console.log(`  Status: ${job.status}`);

    // Wait for completion
    console.log('\nWaiting for job completion...');
    const completedJob = await client.waitForCompletion(job.id, 5000);

    console.log('\n✓ Job completed!');
    console.log(`  Status: ${completedJob.status}`);
    if (completedJob.finished_at && completedJob.started_at) {
      const duration =
        new Date(completedJob.finished_at).getTime() -
        new Date(completedJob.started_at).getTime();
      console.log(`  Duration: ${duration / 1000}s`);
    }

    // Get job logs
    try {
      const logs = await client.getJobLogs(job.id);
      console.log('\nJob logs:');
      console.log(logs);
    } catch (error) {
      console.log('Could not fetch logs:', error);
    }

    // List all workers
    console.log('\nListing workers...');
    const workers = await client.listWorkers();
    console.log(`✓ Found ${workers.length} workers:`);
    workers.forEach((worker) => {
      console.log(`  - ${worker.name} (${worker.status})`);
      console.log(`    CPU: ${(worker.cpu_usage * 100).toFixed(1)}%`);
      console.log(`    Memory: ${(worker.memory_usage * 100).toFixed(1)}%`);
      console.log(`    Jobs processed: ${worker.jobs_processed}`);
    });
  } catch (error) {
    if (error instanceof Error) {
      console.error('❌ Error:', error.message);
      if ('statusCode' in error) {
        console.error('  Status code:', (error as any).statusCode);
      }
    }
    process.exit(1);
  }
}

// Run the example
main().catch((error) => {
  console.error('Fatal error:', error);
  process.exit(1);
});
