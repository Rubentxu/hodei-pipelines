import { Card } from '../card';
import { KpiCard } from '../kpi-card';
import { useObservabilityMetrics } from '../../../hooks/useObservability';
import * as echarts from 'echarts';
import { useEffect, useRef } from 'react';

export function MetricsDashboard() {
  const { data, isLoading } = useObservabilityMetrics();
  const chartRef = useRef<HTMLDivElement>(null);
  const chartInstanceRef = useRef<echarts.ECharts | null>(null);

  useEffect(() => {
    if (!chartRef.current || !data) return;

    if (!chartInstanceRef.current) {
      chartInstanceRef.current = echarts.init(chartRef.current);
    }

    const chart = chartInstanceRef.current;
    const option: echarts.EChartsOption = {
      backgroundColor: 'transparent',
      tooltip: {
        trigger: 'axis',
        backgroundColor: '#1f2937',
        borderColor: '#374151',
        textStyle: {
          color: '#d1d5db',
        },
      },
      legend: {
        data: ['CPU', 'Memory', 'Request Rate', 'Error Rate'],
        textStyle: {
          color: '#9ca3af',
        },
      },
      grid: {
        left: '3%',
        right: '4%',
        bottom: '3%',
        containLabel: true,
      },
      xAxis: {
        type: 'category',
        boundaryGap: false,
        data: data.history.map(h => new Date(h.timestamp).toLocaleTimeString()),
        axisLine: {
          lineStyle: {
            color: '#4b5563',
          },
        },
        axisLabel: {
          color: '#9ca3af',
        },
      },
      yAxis: {
        type: 'value',
        axisLine: {
          lineStyle: {
            color: '#4b5563',
          },
        },
        axisLabel: {
          color: '#9ca3af',
        },
        splitLine: {
          lineStyle: {
            color: '#374151',
          },
        },
      },
      series: [
        {
          name: 'CPU',
          type: 'line',
          data: data.history.map(h => h.cpuUsage),
          smooth: true,
          lineStyle: {
            color: '#3b82f6',
          },
          areaStyle: {
            color: new echarts.graphic.LinearGradient(0, 0, 0, 1, [
              { offset: 0, color: 'rgba(59, 130, 246, 0.3)' },
              { offset: 1, color: 'rgba(59, 130, 246, 0)' },
            ]),
          },
        },
        {
          name: 'Memory',
          type: 'line',
          data: data.history.map(h => h.memoryUsage),
          smooth: true,
          lineStyle: {
            color: '#10b981',
          },
          areaStyle: {
            color: new echarts.graphic.LinearGradient(0, 0, 0, 1, [
              { offset: 0, color: 'rgba(16, 185, 129, 0.3)' },
              { offset: 1, color: 'rgba(16, 185, 129, 0)' },
            ]),
          },
        },
        {
          name: 'Request Rate',
          type: 'line',
          data: data.history.map(h => h.requestRate),
          smooth: true,
          lineStyle: {
            color: '#8b5cf6',
          },
        },
        {
          name: 'Error Rate',
          type: 'line',
          data: data.history.map(h => h.errorRate),
          smooth: true,
          lineStyle: {
            color: '#ef4444',
          },
        },
      ],
    };

    chart.setOption(option);

    const handleResize = () => {
      chart.resize();
    };

    window.addEventListener('resize', handleResize);

    return () => {
      window.removeEventListener('resize', handleResize);
    };
  }, [data]);

  if (isLoading || !data) {
    return (
      <Card className="p-6">
        <div className="flex items-center justify-center h-[400px]">
          <div className="text-gray-400">Loading metrics...</div>
        </div>
      </Card>
    );
  }

  const metrics = data.metrics;

  return (
    <div className="space-y-4">
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        <KpiCard
          title="CPU Usage"
          value={metrics.cpuUsage}
          format="percentage"
          trend="stable"
          trendPercentage={0}
        />
        <KpiCard
          title="Memory Usage"
          value={metrics.memoryUsage}
          format="percentage"
          trend="stable"
          trendPercentage={0}
        />
        <KpiCard
          title="Request Rate"
          value={metrics.requestRate}
          format="number"
          suffix=" req/s"
          trend="up"
          trendPercentage={5.2}
        />
        <KpiCard
          title="Error Rate"
          value={metrics.errorRate}
          format="percentage"
          trend="down"
          trendPercentage={-2.1}
        />
      </div>

      <Card className="p-6">
        <h3 className="text-lg font-semibold text-gray-100 mb-4">
          System Performance
        </h3>
        <div ref={chartRef} style={{ width: '100%', height: '400px' }} />
      </Card>

      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <Card className="p-4">
          <h4 className="text-sm font-medium text-gray-400 mb-2">Disk Usage</h4>
          <div className="flex items-end gap-2">
            <div className="text-2xl font-semibold text-gray-100">
              {metrics.diskUsage}%
            </div>
          </div>
          <div className="mt-3 w-full bg-gray-700 rounded-full h-2">
            <div
              className="bg-blue-500 h-2 rounded-full"
              style={{ width: `${metrics.diskUsage}%` }}
            />
          </div>
        </Card>

        <Card className="p-4">
          <h4 className="text-sm font-medium text-gray-400 mb-2">Network In</h4>
          <div className="flex items-end gap-2">
            <div className="text-2xl font-semibold text-gray-100">
              {(metrics.networkIn / 1024 / 1024).toFixed(2)}
            </div>
            <div className="text-sm text-gray-400 mb-1">MB/s</div>
          </div>
        </Card>

        <Card className="p-4">
          <h4 className="text-sm font-medium text-gray-400 mb-2">Network Out</h4>
          <div className="flex items-end gap-2">
            <div className="text-2xl font-semibold text-gray-100">
              {(metrics.networkOut / 1024 / 1024).toFixed(2)}
            </div>
            <div className="text-sm text-gray-400 mb-1">MB/s</div>
          </div>
        </Card>
      </div>
    </div>
  );
}
