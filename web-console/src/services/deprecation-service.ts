/**
 * API Endpoint Deprecation Service
 *
 * This service manages deprecated API endpoints, tracking their usage
 * and providing warnings to developers about deprecated endpoints.
 */

export interface DeprecatedEndpoint {
  /** HTTP method (GET, POST, PUT, DELETE, etc.) */
  method: string;
  /** Endpoint path pattern */
  path: string;
  /** Deprecation reason */
  reason: string;
  /** Alternative endpoint to use instead */
  alternative?: string;
  /** Date when endpoint was marked as deprecated */
  deprecatedAt: string;
  /** Version when endpoint was deprecated */
  deprecatedInVersion: string;
  /** Scheduled removal date (optional) */
  plannedRemoval?: string;
  /** Usage count (tracked) */
  usageCount: number;
  /** Whether to log warnings for this endpoint */
  logWarning: boolean;
  /** Custom warning message */
  customMessage?: string;
}

/**
 * Configuration for endpoint deprecation tracking
 */
export interface DeprecationConfig {
  /** Whether to enable deprecation tracking */
  enabled: boolean;
  /** Whether to log warnings to console */
  logToConsole: boolean;
  /** Custom log function (e.g., Sentry, analytics) */
  customLogger?: (message: string, endpoint: DeprecatedEndpoint) => void;
  /** Endpoints to ignore (not track) */
  ignoredEndpoints?: Array<{ method: string; path: string }>;
}

/**
 * Internal tracking data
 */
const deprecatedEndpoints = new Map<string, DeprecatedEndpoint>();

// Safely determine environment
const getEnv = () => {
  if (typeof process !== "undefined" && process.env && process.env.NODE_ENV) {
    return process.env.NODE_ENV;
  }
  if (
    typeof import.meta !== "undefined" &&
    import.meta.env &&
    import.meta.env.MODE
  ) {
    return import.meta.env.MODE;
  }
  return "development";
};

const ENV = getEnv();

const config: DeprecationConfig = {
  enabled: true, // Always enabled for tracking in this implementation
  logToConsole: ENV === "development" || ENV === "test",
};

/**
 * Register a deprecated endpoint
 */
export function registerDeprecatedEndpoint(endpoint: DeprecatedEndpoint): void {
  const key = `${endpoint.method.toUpperCase()} ${endpoint.path}`;

  deprecatedEndpoints.set(key, {
    ...endpoint,
    method: endpoint.method.toUpperCase(),
    usageCount: 0,
    logWarning: endpoint.logWarning ?? true,
  });
}

/**
 * Check if an endpoint is deprecated
 */
export function isEndpointDeprecated(
  method: string,
  path: string,
): DeprecatedEndpoint | undefined {
  const key = `${method.toUpperCase()} ${path}`;
  return deprecatedEndpoints.get(key);
}

/**
 * Track usage of an endpoint (call this when making API requests)
 */
export function trackEndpointUsage(
  method: string,
  path: string,
  options?: {
    silent?: boolean;
    customMessage?: string;
  },
): DeprecatedEndpoint | undefined {
  const endpoint = isEndpointDeprecated(method, path);

  if (!endpoint) {
    return undefined;
  }

  // Increment usage count
  endpoint.usageCount++;

  // Log warning if enabled
  if (endpoint.logWarning && !options?.silent && config.logToConsole) {
    const message =
      options?.customMessage ||
      endpoint.customMessage ||
      generateDefaultMessage(endpoint);
    console.warn(`[DEPRECATED API] ${message}`);
  }

  // Call custom logger if configured
  if (endpoint.logWarning && config.customLogger) {
    const message =
      options?.customMessage ||
      endpoint.customMessage ||
      generateDefaultMessage(endpoint);
    config.customLogger(message, endpoint);
  }

  return endpoint;
}

/**
 * Generate default deprecation message
 */
function generateDefaultMessage(endpoint: DeprecatedEndpoint): string {
  let message = `API endpoint ${endpoint.method} ${endpoint.path} is deprecated.`;

  if (endpoint.reason) {
    message += ` Reason: ${endpoint.reason}.`;
  }

  if (endpoint.alternative) {
    message += ` Use ${endpoint.alternative} instead.`;
  }

  if (endpoint.plannedRemoval) {
    message += ` Scheduled for removal: ${endpoint.plannedRemoval}.`;
  }

  return message;
}

/**
 * Configure deprecation service
 */
export function configureDeprecationService(
  newConfig: Partial<DeprecationConfig>,
): void {
  Object.assign(config, newConfig);
}

/**
 * Get all deprecated endpoints
 */
export function getDeprecatedEndpoints(): DeprecatedEndpoint[] {
  return Array.from(deprecatedEndpoints.values()).sort((a, b) =>
    a.path.localeCompare(b.path),
  );
}

/**
 * Get deprecated endpoint by key
 */
export function getDeprecatedEndpoint(
  method: string,
  path: string,
): DeprecatedEndpoint | undefined {
  const key = `${method.toUpperCase()} ${path}`;
  return deprecatedEndpoints.get(key);
}

/**
 * Generate deprecation report
 */
export function generateDeprecationReport(): {
  totalDeprecated: number;
  totalUsage: number;
  endpoints: DeprecatedEndpoint[];
  mostUsed: DeprecatedEndpoint[];
  neverUsed: DeprecatedEndpoint[];
} {
  const endpoints = getDeprecatedEndpoints();
  const totalDeprecated = endpoints.length;
  const totalUsage = endpoints.reduce((sum, ep) => sum + ep.usageCount, 0);

  const mostUsed = [...endpoints]
    .filter((ep) => ep.usageCount > 0)
    .sort((a, b) => b.usageCount - a.usageCount);

  // Only include endpoints with the highest usage count
  const maxUsage = mostUsed.length > 0 ? mostUsed[0].usageCount : 0;
  const mostUsedFiltered = mostUsed.filter((ep) => ep.usageCount === maxUsage);

  const neverUsed = endpoints.filter((ep) => ep.usageCount === 0);

  return {
    totalDeprecated,
    totalUsage,
    endpoints,
    mostUsed: mostUsedFiltered,
    neverUsed,
  };
}

/**
 * Reset usage statistics (useful for testing)
 */
export function resetDeprecationStats(): void {
  deprecatedEndpoints.forEach((endpoint) => {
    endpoint.usageCount = 0;
  });
}

/**
 * Reset all deprecation data (useful for testing)
 * This will remove all registered endpoints
 */
export function resetAllDeprecationData(): void {
  deprecatedEndpoints.clear();
}

/**
 * Helper to mark an endpoint as not logging warnings
 */
export function silenceDeprecationWarning(method: string, path: string): void {
  const endpoint = isEndpointDeprecated(method, path);
  if (endpoint) {
    endpoint.logWarning = false;
  }
}

/**
 * Helper to re-enable deprecation warnings
 */
export function enableDeprecationWarning(method: string, path: string): void {
  const endpoint = isEndpointDeprecated(method, path);
  if (endpoint) {
    endpoint.logWarning = true;
  }
}

// Pre-register some example deprecated endpoints (these would be configured based on actual API)
// In production, these would come from API documentation or configuration

if (ENV === "production") {
  // Example: Old pipeline endpoint (if it exists)
  // registerDeprecatedEndpoint({
  //   method: 'GET',
  //   path: '/api/v1/pipelines/legacy',
  //   reason: 'Superseded by /api/v2/pipelines',
  //   alternative: 'GET /api/v2/pipelines',
  //   deprecatedAt: '2025-10-01',
  //   deprecatedInVersion: '1.5.0',
  //   plannedRemoval: '2026-01-01',
  //   logWarning: true,
  // });
}

// Export config for external access
export { config };
