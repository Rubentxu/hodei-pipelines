import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { SystemEvent, useWebSocket } from '@/hooks/useWebSocket';
import ReactECharts from 'echarts-for-react';
import React, { useState } from 'react';

interface ResourceUsage {
    cpu_usage_m: number;
    memory_usage_mb: number;
    active_jobs: number;
    timestamp: number;
}

interface WorkerMetrics {
    [workerId: string]: {
        cpu: [number, number][];
        memory: [number, number][];
        activeJobs: [number, number][];
    };
}

export const LiveMetricsPanel: React.FC = () => {
    const [metrics, setMetrics] = useState<WorkerMetrics>({});

    const handleMessage = (event: SystemEvent) => {
        if (event.type === 'WorkerHeartbeat') {
            const { worker_id, resource_usage } = event.payload;
            const timestamp = resource_usage.timestamp;

            setMetrics((prev) => {
                const workerMetrics = prev[worker_id] || { cpu: [], memory: [], activeJobs: [] };

                // Keep last 50 data points
                const newCpu = [...workerMetrics.cpu, [timestamp, resource_usage.cpu_usage_m]].slice(-50);
                const newMemory = [...workerMetrics.memory, [timestamp, resource_usage.memory_usage_mb]].slice(-50);
                const newActiveJobs = [...workerMetrics.activeJobs, [timestamp, resource_usage.active_jobs]].slice(-50);

                return {
                    ...prev,
                    [worker_id]: {
                        cpu: newCpu,
                        memory: newMemory,
                        activeJobs: newActiveJobs,
                    },
                };
            });
        }
    };

    useWebSocket({
        onMessage: handleMessage,
    });

    const getOption = (title: string, data: { [key: string]: [number, number][] }, unit: string) => {
        const series = Object.keys(data).map((workerId) => ({
            name: workerId,
            type: 'line',
            showSymbol: false,
            data: data[workerId],
        }));

        return {
            title: { text: title },
            tooltip: { trigger: 'axis' },
            legend: { data: Object.keys(data) },
            xAxis: { type: 'time' },
            yAxis: { type: 'value', name: unit },
            series,
            animation: false,
        };
    };

    return (
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <Card>
                <CardHeader>
                    <CardTitle>CPU Usage (mCores)</CardTitle>
                </CardHeader>
                <CardContent>
                    <ReactECharts
                        option={getOption(
                            'CPU Usage',
                            Object.fromEntries(Object.entries(metrics).map(([k, v]) => [k, v.cpu])),
                            'mCores'
                        )}
                        style={{ height: '300px' }}
                    />
                </CardContent>
            </Card>

            <Card>
                <CardHeader>
                    <CardTitle>Memory Usage (MB)</CardTitle>
                </CardHeader>
                <CardContent>
                    <ReactECharts
                        option={getOption(
                            'Memory Usage',
                            Object.fromEntries(Object.entries(metrics).map(([k, v]) => [k, v.memory])),
                            'MB'
                        )}
                        style={{ height: '300px' }}
                    />
                </CardContent>
            </Card>

            <Card>
                <CardHeader>
                    <CardTitle>Active Jobs</CardTitle>
                </CardHeader>
                <CardContent>
                    <ReactECharts
                        option={getOption(
                            'Active Jobs',
                            Object.fromEntries(Object.entries(metrics).map(([k, v]) => [k, v.activeJobs])),
                            'Count'
                        )}
                        style={{ height: '300px' }}
                    />
                </CardContent>
            </Card>
        </div>
    );
};
