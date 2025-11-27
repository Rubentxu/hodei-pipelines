import { useEffect, useRef } from 'react';
import * as echarts from 'echarts';
import { cn } from '@/utils/cn';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { CostHistory } from '@/types';
import { TrendingUpIcon } from 'lucide-react';

interface CostHistoryChartProps {
  history: CostHistory[];
  className?: string;
}

export function CostHistoryChart({ history, className }: CostHistoryChartProps) {
  const chartRef = useRef<HTMLDivElement>(null);
  const chartInstance = useRef<echarts.ECharts | null>(null);

  useEffect(() => {
    if (!chartRef.current || history.length === 0) return;

    if (chartInstance.current) {
      chartInstance.current.dispose();
    }

    chartInstance.current = echarts.init(chartRef.current, 'dark');

    const option: echarts.EChartsOption = {
      backgroundColor: 'transparent',
      tooltip: {
        trigger: 'axis',
        formatter: (params: any) => {
          const data = params[0];
          const jobCount = history[data.dataIndex]?.jobCount || 0;
          return `
            <div style="padding: 8px;">
              <div style="font-weight: bold; margin-bottom: 4px;">${data.axisValue}</div>
              <div>Costo: $${data.value.toFixed(2)}</div>
              <div>Jobs: ${jobCount}</div>
            </div>
          `;
        },
        backgroundColor: '#1e293b',
        borderColor: '#334155',
        textStyle: {
          color: '#f1f5f9',
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
        data: history.map(h => h.date),
        axisLine: {
          lineStyle: {
            color: '#334155',
          },
        },
        axisLabel: {
          color: '#94a3b8',
        },
      },
      yAxis: {
        type: 'value',
        axisLine: {
          lineStyle: {
            color: '#334155',
          },
        },
        axisLabel: {
          color: '#94a3b8',
          formatter: (value: number) => `$${value.toFixed(0)}`,
        },
        splitLine: {
          lineStyle: {
            color: '#1e293b',
          },
        },
      },
      series: [
        {
          name: 'Costo',
          type: 'line',
          smooth: true,
          symbol: 'circle',
          symbolSize: 6,
          lineStyle: {
            width: 3,
            color: '#3b82f6',
          },
          itemStyle: {
            color: '#3b82f6',
          },
          areaStyle: {
            color: {
              type: 'linear',
              x: 0,
              y: 0,
              x2: 0,
              y2: 1,
              colorStops: [
                {
                  offset: 0,
                  color: 'rgba(59, 130, 246, 0.5)',
                },
                {
                  offset: 1,
                  color: 'rgba(59, 130, 246, 0.05)',
                },
              ],
            },
          },
          data: history.map(h => h.cost),
        },
      ],
    };

    chartInstance.current.setOption(option);

    const handleResize = () => {
      chartInstance.current?.resize();
    };

    window.addEventListener('resize', handleResize);

    return () => {
      window.removeEventListener('resize', handleResize);
      chartInstance.current?.dispose();
    };
  }, [history]);

  if (history.length === 0) {
    return (
      <Card className={cn('', className)}>
        <CardHeader>
          <CardTitle className="text-nebula-text-primary">
            Historial de Costos
          </CardTitle>
        </CardHeader>
        <CardContent>
          <p className="text-nebula-text-secondary text-center py-8">
            No hay datos históricos disponibles
          </p>
        </CardContent>
      </Card>
    );
  }

  const totalCost = history.reduce((sum, h) => sum + h.cost, 0);
  const avgCost = totalCost / history.length;
  const maxCost = Math.max(...history.map(h => h.cost));
  const minCost = Math.min(...history.map(h => h.cost));
  const totalJobs = history.reduce((sum, h) => sum + h.jobCount, 0);

  return (
    <Card className={cn('', className)}>
      <CardHeader>
        <CardTitle className="text-nebula-text-primary flex items-center gap-2">
          <TrendingUpIcon className="w-5 h-5" />
          Historial de Costos
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-4">
        <div ref={chartRef} style={{ width: '100%', height: '300px' }} />

        <div className="grid grid-cols-4 gap-4 pt-4 border-t border-nebula-surface-secondary">
          <div className="text-center">
            <div className="text-xs text-nebula-text-secondary">Promedio</div>
            <div className="text-lg font-semibold text-nebula-text-primary">
              ${avgCost.toFixed(2)}
            </div>
          </div>
          <div className="text-center">
            <div className="text-xs text-nebula-text-secondary">Máximo</div>
            <div className="text-lg font-semibold text-nebula-accent-red">
              ${maxCost.toFixed(2)}
            </div>
          </div>
          <div className="text-center">
            <div className="text-xs text-nebula-text-secondary">Mínimo</div>
            <div className="text-lg font-semibold text-nebula-accent-green">
              ${minCost.toFixed(2)}
            </div>
          </div>
          <div className="text-center">
            <div className="text-xs text-nebula-text-secondary">Total Jobs</div>
            <div className="text-lg font-semibold text-nebula-text-primary">
              {totalJobs}
            </div>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
