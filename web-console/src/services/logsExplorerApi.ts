import { LogEntry } from '@/types';

export interface LogFilterParams {
  level?: string;
  service?: string;
  traceId?: string;
  spanId?: string;
  userId?: string;
  startTime?: string;
  endTime?: string;
  search?: string;
  limit?: number;
  offset?: number;
}

export interface LogQueryResult {
  logs: LogEntry[];
  total: number;
  hasMore: boolean;
}

export interface LogAggregation {
  byService?: Record<string, number>;
  byLevel?: Record<string, number>;
  byTime?: Array<{ timestamp: string; count: number }>;
  total: number;
}

export interface SearchQuery {
  query: string;
  fields?: string[];
  fuzzy?: boolean;
  caseSensitive?: boolean;
}

export interface SearchResult {
  logs: LogEntry[];
  total: number;
  searchQuery: string;
  searchTime: number;
}

/**
 * Fetch logs with optional filtering
 * @param params - Filter parameters
 * @returns Promise containing log entries and pagination info
 */
export async function getLogs(params?: LogFilterParams): Promise<LogQueryResult> {
  const queryParams = new URLSearchParams();

  if (params) {
    Object.entries(params).forEach(([key, value]) => {
      if (value !== undefined && value !== null) {
        queryParams.append(key, value.toString());
      }
    });
  }

  const response = await fetch(`/api/v1/logs?${queryParams.toString()}`);

  if (!response.ok) {
    throw new Error('Failed to fetch logs');
  }

  const data = await response.json();
  return {
    ...data,
    hasMore: data.logs.length >= (params?.limit || 50),
  };
}

/**
 * Get available log levels
 * @returns Promise containing array of log levels
 */
export async function getLogLevels(): Promise<string[]> {
  const response = await fetch('/api/v1/logs/levels');

  if (!response.ok) {
    throw new Error('Failed to fetch log levels');
  }

  return response.json();
}

/**
 * Get available services
 * @returns Promise containing array of service names
 */
export async function getServices(): Promise<string[]> {
  const response = await fetch('/api/v1/logs/services');

  if (!response.ok) {
    throw new Error('Failed to fetch services');
  }

  return response.json();
}

/**
 * Export logs to file
 * @param params - Filter parameters
 * @param options - Export options
 * @returns Promise containing blob with file data
 */
export async function exportLogs(
  params?: LogFilterParams,
  options?: { format?: 'csv' | 'json'; filename?: string }
): Promise<Blob> {
  const queryParams = new URLSearchParams();
  queryParams.append('format', options?.format || 'csv');
  queryParams.append('filename', options?.filename || 'logs');

  if (params) {
    Object.entries(params).forEach(([key, value]) => {
      if (value !== undefined && value !== null) {
        queryParams.append(key, value.toString());
      }
    });
  }

  const response = await fetch(`/api/v1/logs/export?${queryParams.toString()}`);

  if (!response.ok) {
    throw new Error('Failed to export logs');
  }

  return response.blob();
}

/**
 * Aggregate logs by various dimensions
 * @param params - Aggregation parameters
 * @returns Promise containing aggregation results
 */
export async function aggregateLogs(params?: {
  groupBy: 'service' | 'level' | 'time';
  startTime?: string;
  endTime?: string;
  interval?: 'hour' | 'day' | 'week';
}): Promise<LogAggregation> {
  const queryParams = new URLSearchParams();

  if (params) {
    if (params.groupBy) queryParams.append('groupBy', params.groupBy);
    if (params.startTime) queryParams.append('startTime', params.startTime);
    if (params.endTime) queryParams.append('endTime', params.endTime);
    if (params.interval) queryParams.append('interval', params.interval);
  }

  const response = await fetch(`/api/v1/logs/aggregate?${queryParams.toString()}`);

  if (!response.ok) {
    throw new Error('Failed to aggregate logs');
  }

  return response.json();
}

/**
 * Search logs with advanced query
 * @param query - Search query with operators
 * @param params - Additional filter parameters
 * @returns Promise containing search results
 */
export async function searchLogs(
  query: SearchQuery,
  params?: Omit<LogFilterParams, 'search'>
): Promise<SearchResult> {
  const response = await fetch('/api/v1/logs/search', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({
      query,
      params,
    }),
  });

  if (!response.ok) {
    throw new Error('Failed to search logs');
  }

  return response.json();
}

/**
 * Stream logs in real-time using Server-Sent Events
 * @param params - Filter parameters for streaming
 * @param onMessage - Callback for each log entry
 * @returns EventSource instance for cleanup
 */
export function streamLogs(
  params?: LogFilterParams,
  onMessage?: (log: LogEntry) => void
): EventSource {
  const queryParams = new URLSearchParams();

  if (params) {
    Object.entries(params).forEach(([key, value]) => {
      if (value !== undefined && value !== null) {
        queryParams.append(key, value.toString());
      }
    });
  }

  const url = `/api/v1/logs/stream${queryParams.toString() ? `?${queryParams.toString()}` : ''}`;
  const eventSource = new EventSource(url);

  if (onMessage) {
    eventSource.onmessage = (event) => {
      try {
        const log: LogEntry = JSON.parse(event.data);
        onMessage(log);
      } catch (error) {
        console.error('Failed to parse log entry:', error);
      }
    };
  }

  eventSource.onerror = (error) => {
    console.error('Log stream error:', error);
  };

  return eventSource;
}

/**
 * Get log statistics for dashboard
 * @param timeRange - Time range for statistics
 * @returns Promise containing log statistics
 */
export async function getLogStats(timeRange?: {
  startTime?: string;
  endTime?: string;
}): Promise<{
  totalLogs: number;
  errorRate: number;
  topServices: Array<{ service: string; count: number }>;
  topLevels: Array<{ level: string; count: number }>;
}> {
  const queryParams = new URLSearchParams();

  if (timeRange?.startTime) queryParams.append('startTime', timeRange.startTime);
  if (timeRange?.endTime) queryParams.append('endTime', timeRange.endTime);

  const response = await fetch(`/api/v1/logs/stats?${queryParams.toString()}`);

  if (!response.ok) {
    throw new Error('Failed to fetch log statistics');
  }

  return response.json();
}

/**
 * Parse log level from string
 * @param level - Log level string
 * @returns Normalized log level
 */
export function parseLogLevel(level: string): LogEntry['level'] {
  const normalized = level.toLowerCase();

  switch (normalized) {
    case 'debug':
    case 'info':
    case 'warn':
    case 'warning':
    case 'error':
    case 'err':
    case 'fatal':
    case 'crit':
      return normalized as LogEntry['level'];
    default:
      return 'info';
  }
}

/**
 * Format timestamp for display
 * @param timestamp - ISO timestamp string
 * @param format - Output format
 * @returns Formatted timestamp string
 */
export function formatTimestamp(
  timestamp: string,
  format: 'relative' | 'short' | 'long' = 'short'
): string {
  const date = new Date(timestamp);

  if (format === 'relative') {
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffMins = Math.floor(diffMs / 60000);

    if (diffMins < 1) return 'Just now';
    if (diffMins < 60) return `${diffMins}m ago`;

    const diffHours = Math.floor(diffMins / 60);
    if (diffHours < 24) return `${diffHours}h ago`;

    const diffDays = Math.floor(diffHours / 24);
    return `${diffDays}d ago`;
  }

  if (format === 'short') {
    return date.toLocaleTimeString('en-US', {
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
    });
  }

  return date.toLocaleString('en-US', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  });
}

/**
 * Highlight search terms in log message
 * @param message - Log message
 * @param searchTerms - Array of terms to highlight
 * @returns HTML string with highlights
 */
export function highlightSearchTerms(message: string, searchTerms: string[]): string {
  if (!searchTerms.length) return message;

  let highlighted = message;

  searchTerms.forEach((term) => {
    const regex = new RegExp(`(${term})`, 'gi');
    highlighted = highlighted.replace(regex, '<mark>$1</mark>');
  });

  return highlighted;
}

/**
 * Check if log level is error or higher
 * @param level - Log level
 * @returns True if error, warn, or fatal
 */
export function isErrorLevel(level: LogEntry['level']): boolean {
  return ['error', 'warn', 'fatal'].includes(level);
}

/**
 * Get log level color for UI
 * @param level - Log level
 * @returns CSS color class
 */
export function getLogLevelColor(level: LogEntry['level']): string {
  switch (level) {
    case 'debug':
      return 'text-gray-500';
    case 'info':
      return 'text-blue-500';
    case 'warn':
      return 'text-yellow-500';
    case 'error':
      return 'text-red-500';
    case 'fatal':
      return 'text-red-700 font-bold';
    default:
      return 'text-gray-500';
  }
}
