import React, { useState, useEffect, useCallback } from 'react';
import {
  getLogs,
  getLogLevels,
  getServices,
  searchLogs,
  exportLogs,
  getLogStats,
  formatTimestamp,
  highlightSearchTerms,
  getLogLevelColor,
  type LogEntry,
  type LogFilterParams,
} from '../services/logsExplorerApi';

export function LogsExplorer() {
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [filters, setFilters] = useState<LogFilterParams>({
    level: undefined,
    service: undefined,
    search: undefined,
    limit: 50,
  });
  const [levels, setLevels] = useState<string[]>([]);
  const [services, setServices] = useState<string[]>([]);
  const [stats, setStats] = useState<{
    totalLogs: number;
    errorRate: number;
    topServices: Array<{ service: string; count: number }>;
    topLevels: Array<{ level: string; count: number }>;
  } | null>(null);
  const [searchQuery, setSearchQuery] = useState('');
  const [searching, setSearching] = useState(false);

  useEffect(() => {
    loadInitialData();
  }, []);

  useEffect(() => {
    loadLogs();
  }, [filters]);

  const loadInitialData = async () => {
    try {
      const [levelsData, servicesData, statsData] = await Promise.all([
        getLogLevels(),
        getServices(),
        getLogStats(),
      ]);
      setLevels(levelsData);
      setServices(servicesData);
      setStats(statsData);
    } catch (err) {
      console.error('Failed to load initial data:', err);
    }
  };

  const loadLogs = async () => {
    try {
      setLoading(true);
      const data = await getLogs(filters);
      setLogs(data.logs);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load logs');
    } finally {
      setLoading(false);
    }
  };

  const handleSearch = async () => {
    if (!searchQuery.trim()) {
      loadLogs();
      return;
    }

    try {
      setSearching(true);
      const result = await searchLogs({
        query: searchQuery,
        fields: ['message', 'service', 'traceId'],
        fuzzy: true,
      });
      setLogs(result.logs);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Search failed');
    } finally {
      setSearching(false);
    }
  };

  const handleFilterChange = (key: keyof LogFilterParams, value: any) => {
    setFilters((prev) => ({
      ...prev,
      [key]: value || undefined,
    }));
  };

  const handleExport = async (format: 'csv' | 'json') => {
    try {
      const blob = await exportLogs(filters, { format });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `logs.${format}`;
      a.click();
      URL.revokeObjectURL(url);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Export failed');
    }
  };

  const getLevelBadgeClass = (level: string) => {
    switch (level) {
      case 'error':
        return 'bg-red-100 text-red-800';
      case 'warn':
        return 'bg-yellow-100 text-yellow-800';
      case 'info':
        return 'bg-blue-100 text-blue-800';
      case 'debug':
        return 'bg-gray-100 text-gray-800';
      default:
        return 'bg-gray-100 text-gray-800';
    }
  };

  return (
    <div className="space-y-6">
      <div className="flex justify-between items-center">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">Logs Explorer</h1>
          <p className="mt-1 text-sm text-gray-500">
            Search and analyze application logs
          </p>
        </div>
        <div className="flex space-x-2">
          <button
            onClick={() => handleExport('csv')}
            className="inline-flex items-center px-3 py-2 border border-gray-300 rounded-md shadow-sm text-sm font-medium text-gray-700 bg-white hover:bg-gray-50"
          >
            Export CSV
          </button>
          <button
            onClick={() => handleExport('json')}
            className="inline-flex items-center px-3 py-2 border border-gray-300 rounded-md shadow-sm text-sm font-medium text-gray-700 bg-white hover:bg-gray-50"
          >
            Export JSON
          </button>
        </div>
      </div>

      {stats && (
        <div className="grid grid-cols-1 gap-6 sm:grid-cols-2 lg:grid-cols-4">
          <div className="bg-white overflow-hidden shadow rounded-lg">
            <div className="p-5">
              <dt className="text-sm font-medium text-gray-500 truncate">Total Logs</dt>
              <dd className="mt-1 text-3xl font-semibold text-gray-900">{stats.totalLogs}</dd>
            </div>
          </div>

          <div className="bg-white overflow-hidden shadow rounded-lg">
            <div className="p-5">
              <dt className="text-sm font-medium text-gray-500 truncate">Error Rate</dt>
              <dd className="mt-1 text-3xl font-semibold text-red-600">
                {(stats.errorRate * 100).toFixed(1)}%
              </dd>
            </div>
          </div>

          <div className="bg-white overflow-hidden shadow rounded-lg">
            <div className="p-5">
              <dt className="text-sm font-medium text-gray-500 truncate">Top Service</dt>
              <dd className="mt-1 text-2xl font-semibold text-gray-900">
                {stats.topServices[0]?.service || 'N/A'}
              </dd>
            </div>
          </div>

          <div className="bg-white overflow-hidden shadow rounded-lg">
            <div className="p-5">
              <dt className="text-sm font-medium text-gray-500 truncate">Most Common Level</dt>
              <dd className="mt-1">
                <span
                  className={`inline-flex items-center px-2.5 py-0.5 rounded-full text-sm font-medium ${getLevelBadgeClass(
                    stats.topLevels[0]?.level || 'info'
                  )}`}
                >
                  {stats.topLevels[0]?.level || 'info'}
                </span>
              </dd>
            </div>
          </div>
        </div>
      )}

      <div className="bg-white shadow rounded-lg p-4">
        <div className="grid grid-cols-1 gap-4 sm:grid-cols-4">
          <div>
            <label className="block text-sm font-medium text-gray-700">Log Level</label>
            <select
              value={filters.level || ''}
              onChange={(e) => handleFilterChange('level', e.target.value)}
              className="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 sm:text-sm"
            >
              <option value="">All levels</option>
              {levels.map((level) => (
                <option key={level} value={level}>
                  {level.toUpperCase()}
                </option>
              ))}
            </select>
          </div>

          <div>
            <label className="block text-sm font-medium text-gray-700">Service</label>
            <select
              value={filters.service || ''}
              onChange={(e) => handleFilterChange('service', e.target.value)}
              className="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 sm:text-sm"
            >
              <option value="">All services</option>
              {services.map((service) => (
                <option key={service} value={service}>
                  {service}
                </option>
              ))}
            </select>
          </div>

          <div className="sm:col-span-2">
            <label className="block text-sm font-medium text-gray-700">Search</label>
            <div className="mt-1 flex">
              <input
                type="text"
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                onKeyDown={(e) => e.key === 'Enter' && handleSearch()}
                placeholder="Search logs..."
                className="flex-1 rounded-l-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 sm:text-sm"
              />
              <button
                onClick={handleSearch}
                disabled={searching}
                className="inline-flex items-center px-4 py-2 border border-l-0 border-gray-300 rounded-r-md shadow-sm text-sm font-medium text-gray-700 bg-gray-50 hover:bg-gray-100 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 disabled:opacity-50"
              >
                {searching ? 'Searching...' : 'Search'}
              </button>
            </div>
          </div>
        </div>
      </div>

      {error && (
        <div className="bg-red-50 border border-red-200 rounded-md p-4">
          <p className="text-sm text-red-800">{error}</p>
        </div>
      )}

      <div className="bg-white shadow overflow-hidden sm:rounded-md">
        <ul className="divide-y divide-gray-200">
          {logs.map((log) => (
            <li key={log.id} className="px-4 py-4 hover:bg-gray-50">
              <div className="flex items-start space-x-3">
                <div className="flex-shrink-0">
                  <span
                    className={`inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium ${getLevelBadgeClass(
                      log.level
                    )}`}
                  >
                    {log.level.toUpperCase()}
                  </span>
                </div>
                <div className="flex-1 min-w-0">
                  <div className="flex items-center justify-between">
                    <p
                      className="text-sm font-medium text-gray-900 break-words"
                      dangerouslySetInnerHTML={{
                        __html: highlightSearchTerms(log.message, [searchQuery]),
                      }}
                    />
                    <p className="text-sm text-gray-500 whitespace-nowrap ml-4">
                      {formatTimestamp(log.timestamp, 'relative')}
                    </p>
                  </div>
                  <div className="mt-1 flex items-center space-x-4 text-sm text-gray-500">
                    <span>{log.service}</span>
                    {log.traceId && <span>Trace: {log.traceId}</span>}
                    {log.spanId && <span>Span: {log.spanId}</span>}
                  </div>
                  {log.metadata && Object.keys(log.metadata).length > 0 && (
                    <div className="mt-2 text-xs text-gray-500">
                      {Object.entries(log.metadata).map(([key, value]) => (
                        <span key={key} className="mr-4">
                          <span className="font-medium">{key}:</span> {String(value)}
                        </span>
                      ))}
                    </div>
                  )}
                </div>
              </div>
            </li>
          ))}
        </ul>
      </div>

      {logs.length === 0 && !loading && !error && (
        <div className="text-center py-12 bg-white rounded-md shadow">
          <p className="text-sm text-gray-500">No logs found matching your criteria.</p>
        </div>
      )}

      {loading && (
        <div className="flex items-center justify-center h-32">
          <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-500"></div>
        </div>
      )}
    </div>
  );
}
