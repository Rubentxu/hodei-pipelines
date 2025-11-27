import * as React from 'react';
import { Card, CardContent, CardHeader, CardTitle } from './card';
import { cn } from '@/utils/cn';

interface KPICardProps {
  title: string;
  value: number | string;
  trend?: 'up' | 'down' | 'neutral';
  trendValue?: string;
  threshold?: number;
  format?: 'number' | 'currency' | 'percentage';
  loading?: boolean;
  className?: string;
  icon?: React.ReactNode;
}

function formatValue(value: number | string, format: 'number' | 'currency' | 'percentage'): string {
  if (typeof value === 'string') return value;
  
  switch (format) {
    case 'currency':
      return new Intl.NumberFormat('en-US', {
        style: 'currency',
        currency: 'USD',
        minimumFractionDigits: 0,
        maximumFractionDigits: 0,
      }).format(value);
    
    case 'percentage':
      return `${value}%`;
    
    case 'number':
    default:
      return new Intl.NumberFormat('en-US').format(value);
  }
}

const trendColors = {
  up: 'text-success',
  down: 'text-error',
  neutral: 'text-muted-foreground',
};

const trendIcons = {
  up: '↑',
  down: '↓',
  neutral: '→',
};

export function KPICard({
  title,
  value,
  trend,
  trendValue,
  threshold,
  format = 'number',
  loading = false,
  className,
  icon,
}: KPICardProps) {
  const displayValue = typeof value === 'number' ? formatValue(value, format) : value;
  
  const isThresholdExceeded = threshold !== undefined && typeof value === 'number' && value > threshold;
  const thresholdPercentage = threshold ? (value as number / threshold) * 100 : 0;

  return (
    <Card className={cn('', className)}>
      <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
        <CardTitle className="text-sm font-medium text-muted-foreground">
          {title}
        </CardTitle>
        {icon && <div className="h-4 w-4 text-muted-foreground">{icon}</div>}
      </CardHeader>
      <CardContent>
        {loading ? (
          <div className="space-y-2">
            <div className="skeleton h-8 w-24" />
            <div className="skeleton h-4 w-16" />
          </div>
        ) : (
          <>
            <div className="flex items-baseline justify-between">
              <div className="text-2xl font-bold">{displayValue}</div>
              {trend && (
                <span
                  className={cn(
                    'text-sm font-medium flex items-center gap-1',
                    trendColors[trend]
                  )}
                >
                  {trendIcons[trend]} {trendValue}
                </span>
              )}
            </div>
            
            {isThresholdExceeded && (
              <div className="mt-2 text-xs text-error flex items-center gap-1">
                <span className="inline-block h-1.5 w-1.5 rounded-full bg-error" />
                threshold exceeded ({thresholdPercentage.toFixed(0)}%)
              </div>
            )}
            
            {trend && (
              <svg
                className="mt-2 h-12 w-full"
                viewBox="0 0 100 30"
                xmlns="http://www.w3.org/2000/svg"
              >
                <polyline
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="2"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  points="0,25 20,20 40,22 60,15 80,18 100,10"
                  className={trendColors[trend]}
                />
              </svg>
            )}
          </>
        )}
      </CardContent>
    </Card>
  );
}
