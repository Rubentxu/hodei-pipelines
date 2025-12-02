import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { CostBreakdown } from '@/types';
import { cn } from '@/utils/cn';
import * as echarts from 'echarts';
import { useEffect, useRef } from 'react';

interface CostBreakdownChartProps {
  breakdown: CostBreakdown[];
  className?: string;
}

const palette = [
  '#3b82f6', // blue
  '#a855f7', // purple
  '#06b6d4', // cyan
  '#10b981', // emerald
  '#f59e0b', // amber
  '#ef4444', // red
];

export function CostBreakdownChart({ breakdown, className }: CostBreakdownChartProps) {
  const chartRef = useRef<HTMLDivElement>(null);
  const chartInstance = useRef<echarts.ECharts | null>(null);

  useEffect(() => {
    if (!chartRef.current || breakdown.length === 0) return;

    if (chartInstance.current) {
      chartInstance.current.dispose();
    }

    chartInstance.current = echarts.init(chartRef.current, 'dark');

    const option: echarts.EChartsOption = {
      backgroundColor: 'transparent',
      tooltip: {
        trigger: 'item',
        formatter: (params: any) => {
          const data = breakdown[params.dataIndex];
          return `
            <div style="padding: 8px;">
              <div style="font-weight: bold; margin-bottom: 4px;">${params.name}</div>
              <div>Costo: $${params.value.toFixed(2)}</div>
              <div>Porcentaje: ${params.percent}%</div>
              <div>Jobs: ${data.jobCount}</div>
              <div>Duración Prom: ${Math.floor(data.avgDuration / 60)}m ${data.avgDuration % 60}s</div>
            </div>
          `;
        },
        backgroundColor: '#1e293b',
        borderColor: '#334155',
        textStyle: {
          color: '#f1f5f9',
        },
      },
      legend: {
        orient: 'vertical',
        left: 'left',
        textStyle: {
          color: '#94a3b8',
        },
      },
      series: [
        {
          name: 'Costos',
          type: 'pie',
          radius: ['40%', '70%'],
          avoidLabelOverlap: false,
          itemStyle: {
            borderRadius: 10,
            borderColor: '#0f172a',
            borderWidth: 2,
          },
          label: {
            show: false,
            position: 'center',
          },
          emphasis: {
            label: {
              show: true,
              fontSize: 16,
              fontWeight: 'bold',
            },
          },
          labelLine: {
            show: false,
          },
          data: breakdown.map((item, index) => ({
            value: item.cost,
            name: item.pipelineName,
            itemStyle: {
              color: palette[index % palette.length],
            },
          })),
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
  }, [breakdown]);

  const formatDuration = (seconds: number) => {
    const mins = Math.floor(seconds / 60);
    const secs = seconds % 60;
    return `${mins}m ${secs}s`;
  };

  const totalCost = breakdown.reduce((sum, item) => sum + item.cost, 0);

  if (breakdown.length === 0) {
    return (
      <Card className={cn('', className)}>
        <CardHeader>
          <CardTitle className="text-nebula-text-primary">
            Desglose de Costos por Pipeline
          </CardTitle>
        </CardHeader>
        <CardContent>
          <p className="text-nebula-text-secondary text-center py-8">
            No hay datos de costos disponibles
          </p>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card className={cn('', className)}>
      <CardHeader>
        <CardTitle className="text-nebula-text-primary">
          Desglose de Costos por Pipeline
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-4">
        <div ref={chartRef} style={{ width: '100%', height: '300px' }} data-testid="cost-breakdown-chart" />

        <div className="space-y-2">
          <div className="text-sm font-semibold text-nebula-text-primary mb-2">
            Detalle por Pipeline
          </div>
          {breakdown.map((item, index) => (
            <div
              key={item.pipelineId}
              className="flex items-center justify-between p-3 bg-nebula-surface-secondary rounded"
            >
              <div className="flex items-center gap-3">
                <div
                  className="w-3 h-3 rounded-full"
                  style={{ backgroundColor: palette[index % palette.length] }}
                />
                <div>
                  <div className="text-sm font-medium text-nebula-text-primary">
                    {item.pipelineName}
                  </div>
                  <div className="text-xs text-nebula-text-secondary">
                    {item.jobCount} jobs • {formatDuration(item.avgDuration)}
                  </div>
                </div>
              </div>
              <div className="text-right">
                <div className="text-sm font-semibold text-nebula-text-primary">
                  ${item.cost.toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
                </div>
                <div className="text-xs text-nebula-text-secondary">
                  {item.percentage}%
                </div>
              </div>
            </div>
          ))}
          <div className="pt-2 border-t border-nebula-surface-secondary">
            <div className="flex justify-between font-semibold text-nebula-text-primary">
              <span>Total:</span>
              <span>${totalCost.toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}</span>
            </div>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
