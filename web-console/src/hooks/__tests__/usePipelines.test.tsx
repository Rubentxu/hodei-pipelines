import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { pipelineApi } from '../../services/pipelineApi';
import { usePipelines } from '../usePipelines';

// Mock pipelineApi
vi.mock('../../services/pipelineApi', () => ({
    pipelineApi: {
        listPipelines: vi.fn(),
        createPipeline: vi.fn(),
        updatePipeline: vi.fn(),
        deletePipeline: vi.fn(),
        executePipeline: vi.fn(),
    },
}));

const queryClient = new QueryClient({
    defaultOptions: {
        queries: {
            retry: false,
        },
    },
});

const wrapper = ({ children }: { children: React.ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
);

describe('usePipelines', () => {
    beforeEach(() => {
        vi.clearAllMocks();
        queryClient.clear();
    });

    it('fetches pipelines successfully', async () => {
        const mockPipelines = [
            { id: '1', name: 'Pipeline 1', status: 'PENDING' },
            { id: '2', name: 'Pipeline 2', status: 'RUNNING' },
        ];
        (pipelineApi.listPipelines as any).mockResolvedValue({ items: mockPipelines, total: mockPipelines.length });

        const { result } = renderHook(() => usePipelines(), { wrapper });

        await waitFor(() => expect(result.current.isLoading).toBe(false));

        expect(result.current.pipelines).toEqual(mockPipelines);
        expect(pipelineApi.listPipelines).toHaveBeenCalledTimes(1);
    });

    it('executes a pipeline successfully', async () => {
        (pipelineApi.executePipeline as any).mockResolvedValue({ success: true });

        const { result } = renderHook(() => usePipelines(), { wrapper });

        result.current.executePipeline('pipeline-123');

        await waitFor(() => expect(result.current.isExecuting).toBe(false));

        expect(pipelineApi.executePipeline).toHaveBeenCalledWith('pipeline-123');
    });
});
