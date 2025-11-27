import { useState } from 'react';
import { useTraces, useTrace } from '../../../hooks/useObservability';
import { Card } from '../card';
import { Button } from '../button';
import { StatusBadge } from '../status-badge';
import { Trace, Span } from '../../../types';

export function TraceViewer() {
  const [selectedTraceId, setSelectedTraceId] = useState<string | null>(null);
  const [filters, setFilters] = useState({
    service: '',
    status: '',
  });

  const { data, isLoading, isFetchingNextPage, fetchNextPage, hasNextPage } = useTraces({
    service: filters.service || undefined,
    status: filters.status || undefined,
  });

  const { data: selectedTrace } = useTrace(selectedTraceId || '');

  const allTraces = data?.pages.flatMap(page => page.traces) || [];

  const handleFilterChange = (key: string, value: string) => {
    setFilters(prev => ({ ...prev, [key]: value }));
  };

  const handleTraceSelect = (traceId: string) => {
    setSelectedTraceId(traceId);
  };

  if (selectedTrace) {
    return <TraceDetailView trace={selectedTrace} onBack={() => setSelectedTraceId(null)} />;
  }

  return (
    <div className="space-y-4">
      <Card className="p-4">
        <h3 className="text-lg font-semibold text-gray-100 mb-4">Distributed Tracing</h3>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div>
            <label className="block text-sm font-medium text-gray-400 mb-1">
              Service
            </label>
            <input
              type="text"
              value={filters.service}
              onChange={(e) => handleFilterChange('service', e.target.value)}
              placeholder="Filter by service..."
              className="w-full bg-gray-800 border border-gray-600 rounded-lg px-3 py-2 text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
            />
          </div>

          <div>
            <label className="block text-sm font-medium text-gray-400 mb-1">
              Status
            </label>
            <select
              value={filters.status}
              onChange={(e) => handleFilterChange('status', e.target.value)}
              className="w-full bg-gray-800 border border-gray-600 rounded-lg px-3 py-2 text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
            >
              <option value="">All Statuses</option>
              <option value="success">Success</option>
              <option value="error">Error</option>
              <option value="timeout">Timeout</option>
            </select>
          </div>
        </div>
      </Card>

      <Card className="h-[600px] flex flex-col">
        <div className="p-4 border-b border-gray-700">
          <h4 className="text-md font-semibold text-gray-100">Traces</h4>
        </div>

        <div className="flex-1 overflow-auto p-4 space-y-3">
          {isLoading ? (
            <div className="flex items-center justify-center h-full">
              <div className="text-gray-400">Loading traces...</div>
            </div>
          ) : allTraces.length === 0 ? (
            <div className="flex items-center justify-center h-full">
              <div className="text-gray-400">No traces found</div>
            </div>
          ) : (
            <>
              {allTraces.map((trace) => (
                <TraceCard
                  key={trace.id}
                  trace={trace}
                  onClick={() => handleTraceSelect(trace.id)}
                />
              ))}
              {hasNextPage && (
                <div className="flex justify-center py-4">
                  <Button
                    onClick={() => fetchNextPage()}
                    disabled={isFetchingNextPage}
                    variant="outline"
                  >
                    {isFetchingNextPage ? 'Loading...' : 'Load More'}
                  </Button>
                </div>
              )}
            </>
          )}
        </div>
      </Card>
    </div>
  );
}

function TraceCard({ trace, onClick }: { trace: Trace; onClick: () => void }) {
  const formatDuration = (ms: number) => {
    if (ms < 1000) return `${ms}ms`;
    return `${(ms / 1000).toFixed(2)}s`;
  };

  const formatTimestamp = (timestamp: string) => {
    return new Date(timestamp).toLocaleTimeString();
  };

  return (
    <div
      className="bg-gray-800 rounded-lg p-4 hover:bg-gray-750 cursor-pointer transition-colors"
      onClick={onClick}
    >
      <div className="flex items-start justify-between mb-2">
        <div className="flex items-center gap-3">
          <StatusBadge
            status={trace.status === 'success' ? 'success' : trace.status === 'error' ? 'error' : 'warning'}
          />
          <h4 className="text-gray-100 font-medium">{trace.name}</h4>
        </div>
        <span className="text-sm text-gray-400">{formatDuration(trace.duration)}</span>
      </div>

      <div className="flex items-center gap-4 text-sm text-gray-400 mb-2">
        <span>{trace.service}</span>
        <span>•</span>
        <span>{formatTimestamp(trace.startTime)}</span>
        <span>•</span>
        <span>{trace.spans.length} spans</span>
      </div>

      <div className="w-full bg-gray-700 rounded-full h-2">
        <div
          className={`h-2 rounded-full ${
            trace.status === 'success'
              ? 'bg-green-500'
              : trace.status === 'error'
              ? 'bg-red-500'
              : 'bg-yellow-500'
          }`}
          style={{ width: '100%' }}
        />
      </div>
    </div>
  );
}

function TraceDetailView({ trace, onBack }: { trace: Trace; onBack: () => void }) {
  const formatDuration = (ms: number) => {
    if (ms < 1000) return `${ms}ms`;
    return `${(ms / 1000).toFixed(2)}s`;
  };

  const formatTimestamp = (timestamp: string) => {
    return new Date(timestamp).toLocaleString();
  };

  const maxDuration = Math.max(...trace.spans.map(s => s.duration));

  return (
    <Card className="h-[600px] flex flex-col">
      <div className="p-4 border-b border-gray-700 flex items-center justify-between">
        <div className="flex items-center gap-3">
          <Button onClick={onBack} variant="outline" size="sm">
            ← Back
          </Button>
          <div>
            <h3 className="text-lg font-semibold text-gray-100">{trace.name}</h3>
            <p className="text-sm text-gray-400">
              {trace.service} • {formatDuration(trace.duration)}
            </p>
          </div>
        </div>
        <StatusBadge
          status={trace.status === 'success' ? 'success' : trace.status === 'error' ? 'error' : 'warning'}
        />
      </div>

      <div className="flex-1 overflow-auto p-4 space-y-4">
        <div className="bg-gray-800 rounded-lg p-4">
          <h4 className="text-sm font-medium text-gray-400 mb-3">Trace Information</h4>
          <div className="grid grid-cols-2 gap-4 text-sm">
            <div>
              <span className="text-gray-500">Trace ID:</span>
              <span className="ml-2 text-gray-300 font-mono">{trace.id}</span>
            </div>
            <div>
              <span className="text-gray-500">Start Time:</span>
              <span className="ml-2 text-gray-300">{formatTimestamp(trace.startTime)}</span>
            </div>
            <div>
              <span className="text-gray-500">End Time:</span>
              <span className="ml-2 text-gray-300">{formatTimestamp(trace.endTime)}</span>
            </div>
            <div>
              <span className="text-gray-500">Total Spans:</span>
              <span className="ml-2 text-gray-300">{trace.spans.length}</span>
            </div>
          </div>
        </div>

        <div>
          <h4 className="text-sm font-medium text-gray-400 mb-3">Spans</h4>
          <div className="space-y-2">
            {trace.spans.map((span) => (
              <SpanView key={span.id} span={span} maxDuration={maxDuration} />
            ))}
          </div>
        </div>
      </div>
    </Card>
  );
}

function SpanView({ span, maxDuration }: { span: Span; maxDuration: number }) {
  const formatDuration = (ms: number) => {
    if (ms < 1000) return `${ms}ms`;
    return `${(ms / 1000).toFixed(2)}s`;
  };

  const formatTimestamp = (timestamp: string) => {
    return new Date(timestamp).toLocaleTimeString();
  };

  const widthPercentage = (span.duration / maxDuration) * 100;

  return (
    <div className="bg-gray-800 rounded-lg p-3">
      <div className="flex items-center justify-between mb-2">
        <div className="flex items-center gap-3">
          <div className="w-3 h-3 rounded-full bg-blue-500" />
          <h5 className="text-gray-100 font-medium">{span.name}</h5>
        </div>
        <span className="text-sm text-gray-400">{formatDuration(span.duration)}</span>
      </div>

      <div className="flex items-center gap-4 text-xs text-gray-400 mb-2">
        <span>{span.service}</span>
        <span>•</span>
        <span>{formatTimestamp(span.startTime)}</span>
        <span>•</span>
        <span className="font-mono">{span.id}</span>
      </div>

      <div className="w-full bg-gray-700 rounded-full h-1">
        <div
          className="h-1 rounded-full bg-blue-500"
          style={{ width: `${widthPercentage}%` }}
        />
      </div>

      {span.tags && Object.keys(span.tags).length > 0 && (
        <div className="mt-2 pt-2 border-t border-gray-700">
          <div className="grid grid-cols-2 gap-2 text-xs">
            {Object.entries(span.tags).map(([key, value]) => (
              <div key={key}>
                <span className="text-gray-500">{key}:</span>
                <span className="ml-1 text-gray-300 font-mono">{String(value)}</span>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
