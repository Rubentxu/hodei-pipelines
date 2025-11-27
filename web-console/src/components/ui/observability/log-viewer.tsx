import { useState } from 'react';
import { useLogs } from '../../../hooks/useObservability';
import { Card } from '../card';
import { Button } from '../button';
import { LogEntry } from '../../../types';

export function LogViewer() {
  const [filters, setFilters] = useState({
    level: '',
    service: '',
    search: '',
  });

  const { data, isLoading, isFetchingNextPage, fetchNextPage, hasNextPage } = useLogs({
    level: filters.level || undefined,
    service: filters.service || undefined,
    search: filters.search || undefined,
  });

  const allLogs = data?.pages.flatMap(page => page.logs) || [];

  const getLevelColor = (level: string) => {
    switch (level) {
      case 'error':
      case 'fatal':
        return 'text-red-400';
      case 'warn':
        return 'text-yellow-400';
      case 'info':
        return 'text-blue-400';
      case 'debug':
        return 'text-gray-400';
      default:
        return 'text-gray-300';
    }
  };

  const handleFilterChange = (key: string, value: string) => {
    setFilters(prev => ({ ...prev, [key]: value }));
  };

  return (
    <Card className="h-[600px] flex flex-col">
      <div className="p-4 border-b border-gray-700">
        <h3 className="text-lg font-semibold text-gray-100 mb-4">Log Viewer</h3>

        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          <div>
            <label className="block text-sm font-medium text-gray-400 mb-1">
              Level
            </label>
            <select
              value={filters.level}
              onChange={(e) => handleFilterChange('level', e.target.value)}
              className="w-full bg-gray-800 border border-gray-600 rounded-lg px-3 py-2 text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
            >
              <option value="">All Levels</option>
              <option value="debug">Debug</option>
              <option value="info">Info</option>
              <option value="warn">Warning</option>
              <option value="error">Error</option>
              <option value="fatal">Fatal</option>
            </select>
          </div>

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
              Search
            </label>
            <input
              type="text"
              value={filters.search}
              onChange={(e) => handleFilterChange('search', e.target.value)}
              placeholder="Search logs..."
              className="w-full bg-gray-800 border border-gray-600 rounded-lg px-3 py-2 text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
            />
          </div>
        </div>
      </div>

      <div className="flex-1 overflow-auto p-4 space-y-2 font-mono text-sm">
        {isLoading ? (
          <div className="flex items-center justify-center h-full">
            <div className="text-gray-400">Loading logs...</div>
          </div>
        ) : allLogs.length === 0 ? (
          <div className="flex items-center justify-center h-full">
            <div className="text-gray-400">No logs found</div>
          </div>
        ) : (
          <>
            {allLogs.map((log) => (
              <LogEntryComponent key={log.id} log={log} />
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
  );
}

function LogEntryComponent({ log }: { log: LogEntry }) {
  const [expanded, setExpanded] = useState(false);

  const formatTimestamp = (timestamp: string) => {
    return new Date(timestamp).toLocaleString();
  };

  return (
    <div
      className="bg-gray-800 rounded-lg p-3 hover:bg-gray-750 cursor-pointer transition-colors"
      onClick={() => setExpanded(!expanded)}
    >
      <div className="flex items-start gap-3">
        <div className="flex items-center gap-2 min-w-0">
          <span className="text-gray-500 text-xs whitespace-nowrap">
            {formatTimestamp(log.timestamp)}
          </span>
          <span className={`text-xs font-semibold uppercase ${getLevelColor(log.level)}`}>
            {log.level}
          </span>
          <span className="text-gray-400 text-xs">{log.service}</span>
        </div>
      </div>

      <div className="text-gray-300 mt-1 break-words">
        {log.message}
      </div>

      {expanded && (
        <div className="mt-2 pt-2 border-t border-gray-700 space-y-2">
          {log.traceId && (
            <div className="flex gap-2">
              <span className="text-gray-500 text-xs min-w-0">Trace ID:</span>
              <span className="text-blue-400 text-xs font-mono">{log.traceId}</span>
            </div>
          )}
          {log.spanId && (
            <div className="flex gap-2">
              <span className="text-gray-500 text-xs min-w-0">Span ID:</span>
              <span className="text-blue-400 text-xs font-mono">{log.spanId}</span>
            </div>
          )}
          {log.userId && (
            <div className="flex gap-2">
              <span className="text-gray-500 text-xs min-w-0">User ID:</span>
              <span className="text-gray-300 text-xs">{log.userId}</span>
            </div>
          )}
          {log.metadata && Object.keys(log.metadata).length > 0 && (
            <div className="mt-2">
              <span className="text-gray-500 text-xs">Metadata:</span>
              <pre className="text-xs text-gray-400 mt-1 bg-gray-900 p-2 rounded overflow-auto">
                {JSON.stringify(log.metadata, null, 2)}
              </pre>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

function getLevelColor(level: string): string {
  switch (level) {
    case 'error':
    case 'fatal':
      return 'text-red-400';
    case 'warn':
      return 'text-yellow-400';
    case 'info':
      return 'text-blue-400';
    case 'debug':
      return 'text-gray-400';
    default:
      return 'text-gray-300';
  }
}
