/**
 * Tests for CicdClient
 */
import { CicdClient } from '../src/cicd-client';
import { PipelineStatus, JobStatus, WorkerStatus, SdkError } from '../src/types';

// Mock fetch globally
const mockFetch = jest.fn();
global.fetch = mockFetch as any;

describe('CicdClient', () => {
  let client: CicdClient;

  beforeEach(() => {
    client = new CicdClient({
      baseUrl: 'http://localhost:8080',
      apiToken: 'test-token',
    });
    mockFetch.mockClear();
  });

  describe('createPipeline', () => {
    it('should create a pipeline successfully', async () => {
      const mockPipeline = {
        id: 'pipeline-123',
        name: 'test-pipeline',
        status: PipelineStatus.PENDING,
        created_at: new Date('2024-01-01T00:00:00Z'),
        updated_at: null,
        config: {
          name: 'test-pipeline',
          description: null,
          stages: [],
          environment: {},
          triggers: [],
        },
      };

      mockFetch.mockResolvedValueOnce({
        ok: true,
        status: 201,
        headers: new Headers({ 'content-type': 'application/json' }),
        json: async () => mockPipeline,
      });

      const config = {
        name: 'test-pipeline',
        stages: [],
      };

      const result = await client.createPipeline(config);

      expect(result.id).toBe('pipeline-123');
      expect(result.name).toBe('test-pipeline');
      expect(result.status).toBe(PipelineStatus.PENDING);
      expect(mockFetch).toHaveBeenCalledWith(
        'http://localhost:8080/api/v1/pipelines',
        expect.objectContaining({
          method: 'POST',
          headers: expect.objectContaining({
            Authorization: 'Bearer test-token',
            'Content-Type': 'application/json',
          }),
        })
      );
    });

    it('should throw error on failed request', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: false,
        status: 400,
        statusText: 'Bad Request',
        text: async () => 'Invalid pipeline configuration',
      });

      const config = {
        name: 'invalid-pipeline',
        stages: [],
      };

      await expect(client.createPipeline(config)).rejects.toThrow(SdkError);
    });
  });

  describe('executePipeline', () => {
    it('should execute pipeline successfully', async () => {
      const mockJob = {
        id: 'job-456',
        pipeline_id: 'pipeline-123',
        name: 'test-job',
        status: JobStatus.QUEUED,
        started_at: new Date('2024-01-01T00:00:00Z'),
        finished_at: null,
        error_message: null,
      };

      mockFetch.mockResolvedValueOnce({
        ok: true,
        status: 200,
        headers: new Headers({ 'content-type': 'application/json' }),
        json: async () => mockJob,
      });

      const result = await client.executePipeline('pipeline-123');

      expect(result.id).toBe('job-456');
      expect(result.pipeline_id).toBe('pipeline-123');
      expect(result.status).toBe(JobStatus.QUEUED);
    });
  });

  describe('listWorkers', () => {
    it('should list workers successfully', async () => {
      const mockWorkers = [
        {
          id: 'worker-1',
          name: 'worker-1',
          status: WorkerStatus.ACTIVE,
          provider: 'kubernetes',
          cpu_usage: 0.5,
          memory_usage: 0.6,
          jobs_processed: 10,
        },
      ];

      mockFetch.mockResolvedValueOnce({
        ok: true,
        status: 200,
        headers: new Headers({ 'content-type': 'application/json' }),
        json: async () => mockWorkers,
      });

      const result = await client.listWorkers();

      expect(result).toHaveLength(1);
      expect(result[0].id).toBe('worker-1');
      expect(result[0].status).toBe(WorkerStatus.ACTIVE);
    });
  });

  describe('error handling', () => {
    it('should handle 404 errors', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: false,
        status: 404,
        statusText: 'Not Found',
        text: async () => 'Pipeline not found',
      });

      try {
        await client.getPipeline('non-existent');
        fail('Should have thrown an error');
      } catch (error) {
        expect(error).toBeInstanceOf(SdkError);
        expect((error as SdkError).isNotFound()).toBe(true);
      }
    });

    it('should handle 401 errors', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: false,
        status: 401,
        statusText: 'Unauthorized',
        text: async () => 'Invalid token',
      });

      try {
        await client.listPipelines();
        fail('Should have thrown an error');
      } catch (error) {
        expect(error).toBeInstanceOf(SdkError);
        expect((error as SdkError).isAuthError()).toBe(true);
      }
    });

    it('should handle timeout errors', async () => {
      mockFetch.mockImplementationOnce(() => {
        return new Promise((_, reject) => {
          setTimeout(() => {
            const error = new Error('Request timeout');
            error.name = 'AbortError';
            reject(error);
          }, 100);
        });
      });

      await expect(client.listPipelines()).rejects.toThrow('Request timeout');
    });
  });

  describe('waitForCompletion', () => {
    it('should wait for job completion', async () => {
      const mockJobRunning = {
        id: 'job-123',
        pipeline_id: 'pipeline-123',
        name: 'test-job',
        status: JobStatus.RUNNING,
        started_at: new Date('2024-01-01T00:00:00Z'),
        finished_at: null,
        error_message: null,
      };

      const mockJobSuccess = {
        ...mockJobRunning,
        status: JobStatus.SUCCESS,
        finished_at: new Date('2024-01-01T00:01:00Z'),
      };

      mockFetch
        .mockResolvedValueOnce({
          ok: true,
          status: 200,
          headers: new Headers({ 'content-type': 'application/json' }),
          json: async () => mockJobRunning,
        })
        .mockResolvedValueOnce({
          ok: true,
          status: 200,
          headers: new Headers({ 'content-type': 'application/json' }),
          json: async () => mockJobSuccess,
        });

      const result = await client.waitForCompletion('job-123', 100);

      expect(result.status).toBe(JobStatus.SUCCESS);
      expect(mockFetch).toHaveBeenCalledTimes(2);
    });
  });
});
