/**
 * Tests for PipelineBuilder
 */
import { PipelineBuilder } from '../src/builder';

describe('PipelineBuilder', () => {
  describe('basic pipeline', () => {
    it('should build a basic pipeline', () => {
      const pipeline = new PipelineBuilder('test-pipeline')
        .description('Test pipeline')
        .stage('build', 'node:18', ['npm install', 'npm run build'])
        .build();

      expect(pipeline.name).toBe('test-pipeline');
      expect(pipeline.description).toBe('Test pipeline');
      expect(pipeline.stages).toHaveLength(1);
      expect(pipeline.stages[0].name).toBe('build');
      expect(pipeline.stages[0].image).toBe('node:18');
      expect(pipeline.stages[0].commands).toEqual(['npm install', 'npm run build']);
    });

    it('should build pipeline without description', () => {
      const pipeline = new PipelineBuilder('simple-pipeline')
        .stage('test', 'node:18', ['npm test'])
        .build();

      expect(pipeline.name).toBe('simple-pipeline');
      expect(pipeline.description).toBeUndefined();
      expect(pipeline.stages).toHaveLength(1);
    });
  });

  describe('multi-stage pipeline', () => {
    it('should build pipeline with multiple stages', () => {
      const pipeline = new PipelineBuilder('multi-stage')
        .stage('build', 'node:18', ['npm run build'])
        .stage('test', 'node:18', ['npm test'])
        .stageWithDeps('deploy', 'alpine:latest', ['./deploy.sh'], ['build', 'test'])
        .build();

      expect(pipeline.stages).toHaveLength(3);
      expect(pipeline.stages[2].dependencies).toEqual(['build', 'test']);
    });

    it('should build pipeline with resource requirements', () => {
      const pipeline = new PipelineBuilder('resource-pipeline')
        .stageWithResources('build', 'node:18', ['npm run build'], 2.0, 4096)
        .build();

      expect(pipeline.stages[0].resources).toEqual({
        cpu: 2.0,
        memory: 4096,
      });
    });

    it('should build pipeline with stage environment variables', () => {
      const pipeline = new PipelineBuilder('env-pipeline')
        .stageWithEnv('build', 'node:18', ['npm run build'], {
          NODE_ENV: 'production',
          DEBUG: 'false',
        })
        .build();

      expect(pipeline.stages[0].environment).toEqual({
        NODE_ENV: 'production',
        DEBUG: 'false',
      });
    });
  });

  describe('environment variables', () => {
    it('should add single environment variable', () => {
      const pipeline = new PipelineBuilder('env-pipeline')
        .env('NODE_ENV', 'production')
        .stage('build', 'node:18', ['npm run build'])
        .build();

      expect(pipeline.environment).toEqual({
        NODE_ENV: 'production',
      });
    });

    it('should add multiple environment variables', () => {
      const envVars = {
        NODE_ENV: 'production',
        DEBUG: 'false',
        LOG_LEVEL: 'info',
      };

      const pipeline = new PipelineBuilder('multi-env-pipeline')
        .envs(envVars)
        .stage('build', 'node:18', ['npm run build'])
        .build();

      expect(pipeline.environment).toEqual(envVars);
    });

    it('should merge environment variables', () => {
      const pipeline = new PipelineBuilder('merge-env-pipeline')
        .env('VAR1', 'value1')
        .envs({ VAR2: 'value2', VAR3: 'value3' })
        .env('VAR4', 'value4')
        .build();

      expect(pipeline.environment).toEqual({
        VAR1: 'value1',
        VAR2: 'value2',
        VAR3: 'value3',
        VAR4: 'value4',
      });
    });
  });

  describe('triggers', () => {
    it('should add git push trigger', () => {
      const pipeline = new PipelineBuilder('git-pipeline')
        .triggerOnPush('main', 'https://github.com/user/repo')
        .stage('build', 'node:18', ['npm run build'])
        .build();

      expect(pipeline.triggers).toHaveLength(1);
      expect(pipeline.triggers![0]).toEqual({
        type: 'git_push',
        branch: 'main',
        repository: 'https://github.com/user/repo',
      });
    });

    it('should add scheduled trigger', () => {
      const pipeline = new PipelineBuilder('scheduled-pipeline')
        .triggerOnSchedule('0 0 * * *')
        .stage('build', 'node:18', ['npm run build'])
        .build();

      expect(pipeline.triggers).toHaveLength(1);
      expect(pipeline.triggers![0]).toEqual({
        type: 'schedule',
        cron: '0 0 * * *',
      });
    });

    it('should add manual trigger', () => {
      const pipeline = new PipelineBuilder('manual-pipeline')
        .triggerManual()
        .stage('build', 'node:18', ['npm run build'])
        .build();

      expect(pipeline.triggers).toHaveLength(1);
      expect(pipeline.triggers![0]).toEqual({
        type: 'manual',
      });
    });

    it('should add webhook trigger', () => {
      const pipeline = new PipelineBuilder('webhook-pipeline')
        .triggerOnWebhook('https://example.com/webhook')
        .stage('build', 'node:18', ['npm run build'])
        .build();

      expect(pipeline.triggers).toHaveLength(1);
      expect(pipeline.triggers![0]).toEqual({
        type: 'webhook',
        url: 'https://example.com/webhook',
      });
    });

    it('should add multiple triggers', () => {
      const pipeline = new PipelineBuilder('multi-trigger-pipeline')
        .triggerOnPush('main', 'https://github.com/user/repo')
        .triggerOnSchedule('0 2 * * *')
        .triggerManual()
        .build();

      expect(pipeline.triggers).toHaveLength(3);
    });
  });

  describe('complex pipeline', () => {
    it('should build a complete complex pipeline', () => {
      const pipeline = new PipelineBuilder('complete-pipeline')
        .description('Complete CI/CD pipeline')
        .env('NODE_ENV', 'production')
        .env('LOG_LEVEL', 'info')
        .stage('checkout', 'alpine/git:latest', ['git clone https://github.com/user/repo.git'])
        .stageWithResources('build', 'node:18', ['npm install', 'npm run build'], 2.0, 4096)
        .stageWithDeps('test', 'node:18', ['npm test'], ['build'])
        .stageWithEnv(
          'deploy',
          'alpine:latest',
          ['./deploy.sh'],
          { DEPLOY_ENV: 'staging' }
        )
        .triggerOnPush('main', 'https://github.com/user/repo')
        .triggerOnSchedule('0 2 * * *')
        .build();

      expect(pipeline.name).toBe('complete-pipeline');
      expect(pipeline.description).toBe('Complete CI/CD pipeline');
      expect(pipeline.stages).toHaveLength(4);
      expect(pipeline.environment).toEqual({
        NODE_ENV: 'production',
        LOG_LEVEL: 'info',
      });
      expect(pipeline.triggers).toHaveLength(2);

      // Verify build stage has resources
      expect(pipeline.stages[1].resources).toEqual({
        cpu: 2.0,
        memory: 4096,
      });

      // Verify test stage has dependencies
      expect(pipeline.stages[2].dependencies).toEqual(['build']);

      // Verify deploy stage has environment
      expect(pipeline.stages[3].environment).toEqual({
        DEPLOY_ENV: 'staging',
      });
    });
  });
});
