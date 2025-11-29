import { useCallback, useEffect, useRef, useState } from 'react';

export type SystemEvent =
    | { type: 'JobCreated'; payload: any }
    | { type: 'JobScheduled'; payload: { job_id: string; worker_id: string } }
    | { type: 'JobStarted'; payload: { job_id: string; worker_id: string } }
    | { type: 'JobCompleted'; payload: { job_id: string; exit_code: number } }
    | { type: 'JobFailed'; payload: { job_id: string; error: string } }
    | { type: 'PipelineCreated'; payload: any }
    | { type: 'PipelineStarted'; payload: { pipeline_id: string } }
    | { type: 'PipelineCompleted'; payload: { pipeline_id: string } }
    | { type: 'PipelineExecutionStarted'; payload: { pipeline_id: string; execution_id: string } }
    | { type: 'LogChunkReceived'; payload: any }
    | { type: 'WorkerHeartbeat'; payload: { worker_id: string; timestamp: string; resource_usage: any } };

type WebSocketStatus = 'connecting' | 'connected' | 'disconnected' | 'error';

interface UseWebSocketOptions {
    url?: string;
    onMessage?: (event: SystemEvent) => void;
    reconnectInterval?: number;
}

export const useWebSocket = ({
    url = 'ws://127.0.0.1:8080/ws', // Use IP to avoid potential localhost proxy issues
    onMessage,
    reconnectInterval = 3000,
}: UseWebSocketOptions = {}) => {
    const [status, setStatus] = useState<WebSocketStatus>('disconnected');
    const wsRef = useRef<WebSocket | null>(null);
    const reconnectTimeoutRef = useRef<NodeJS.Timeout | null>(null);

    const connect = useCallback(() => {
        if (wsRef.current?.readyState === WebSocket.OPEN) return;

        setStatus('connecting');
        const ws = new WebSocket(url);

        ws.onopen = () => {
            console.log('WebSocket connected');
            setStatus('connected');
        };

        ws.onclose = () => {
            console.log('WebSocket disconnected');
            setStatus('disconnected');
            wsRef.current = null;

            // Attempt reconnect
            if (reconnectTimeoutRef.current) clearTimeout(reconnectTimeoutRef.current);
            reconnectTimeoutRef.current = setTimeout(() => {
                console.log('Reconnecting WebSocket...');
                connect();
            }, reconnectInterval);
        };

        ws.onerror = (error) => {
            console.error('WebSocket error:', error);
            setStatus('error');
        };

        ws.onmessage = (event) => {
            try {
                const data = JSON.parse(event.data);
                // Determine event type based on JSON structure (Rust enum serialization)
                // Rust serde default for enums is {"Variant": content}
                const type = Object.keys(data)[0] as any;
                const payload = data[type];

                if (onMessage) {
                    onMessage({ type, payload });
                }
            } catch (e) {
                console.error('Failed to parse WebSocket message:', e);
            }
        };

        wsRef.current = ws;
    }, [url, onMessage, reconnectInterval]);

    useEffect(() => {
        connect();

        return () => {
            if (wsRef.current) {
                wsRef.current.close();
            }
            if (reconnectTimeoutRef.current) {
                clearTimeout(reconnectTimeoutRef.current);
            }
        };
    }, [connect]);

    return { status, ws: wsRef.current };
};
