export interface AlertRule {
  id: string;
  name: string;
  description?: string;
  severity: AlertSeverity;
  conditions: AlertCondition[];
  actions: AlertAction[];
  enabled: boolean;
  createdAt: string;
  updatedAt: string;
  lastTriggeredAt?: string;
  triggerCount: number;
}

export interface AlertCondition {
  metric: string;
  operator: 'gt' | 'gte' | 'lt' | 'lte' | 'eq' | 'neq';
  threshold: number;
  duration?: string; // e.g., "5m", "30s"
}

export interface AlertAction {
  type: 'email' | 'webhook' | 'slack' | 'pagerduty';
  target: string;
  config?: Record<string, any>;
}

export type AlertSeverity = 'critical' | 'high' | 'medium' | 'low' | 'info';

export interface Alert {
  id: string;
  ruleId: string;
  ruleName: string;
  severity: AlertSeverity;
  status: AlertStatus;
  message: string;
  details?: Record<string, any>;
  triggeredAt: string;
  acknowledgedAt?: string;
  acknowledgedBy?: string;
  resolvedAt?: string;
  resolvedBy?: string;
  assignments?: AlertAssignment[];
}

export type AlertStatus = 'open' | 'acknowledged' | 'resolved' | 'suppressed';

export interface AlertAssignment {
  userId: string;
  assignedAt: string;
  assignedBy: string;
}

export interface CreateAlertRuleRequest {
  name: string;
  description?: string;
  severity: AlertSeverity;
  conditions: AlertCondition[];
  actions: AlertAction[];
}

export interface UpdateAlertRuleRequest {
  name?: string;
  description?: string;
  severity?: AlertSeverity;
  conditions?: AlertCondition[];
  actions?: AlertAction[];
  enabled?: boolean;
}

export interface AlertQueryParams {
  status?: AlertStatus[];
  severity?: AlertSeverity[];
  ruleId?: string;
  assignedTo?: string;
  search?: string;
  startTime?: string;
  endTime?: string;
  limit?: number;
  offset?: number;
}

export interface AlertRuleQueryParams {
  enabled?: boolean;
  severity?: AlertSeverity;
  search?: string;
  limit?: number;
  offset?: number;
}

export interface AlertStats {
  totalAlerts: number;
  openAlerts: number;
  acknowledgedAlerts: number;
  resolvedAlerts: number;
  bySeverity: Record<AlertSeverity, number>;
  avgResolutionTime: number; // in minutes
  topRules: Array<{ ruleId: string; ruleName: string; count: number }>;
}

/**
 * Fetch all alert rules
 * @param params - Query parameters
 * @returns Promise containing list of alert rules
 */
export async function getAlertRules(
  params?: AlertRuleQueryParams,
): Promise<AlertRule[]> {
  const queryParams = new URLSearchParams();

  if (params) {
    if (params.enabled !== undefined)
      queryParams.append("enabled", params.enabled.toString());
    if (params.severity) queryParams.append("severity", params.severity);
    if (params.search) queryParams.append("search", params.search);
    if (params.limit) queryParams.append("limit", params.limit.toString());
    if (params.offset) queryParams.append("offset", params.offset.toString());
  }

  const response = await fetch(`/api/v1/alerts/rules?${queryParams.toString()}`);

  if (!response.ok) {
    throw new Error("Failed to fetch alert rules");
  }

  return response.json();
}

/**
 * Fetch a single alert rule by ID
 * @param id - Alert rule ID
 * @returns Promise containing alert rule
 */
export async function getAlertRule(id: string): Promise<AlertRule> {
  const response = await fetch(`/api/v1/alerts/rules/${id}`);

  if (!response.ok) {
    throw new Error("Failed to fetch alert rule");
  }

  return response.json();
}

/**
 * Create a new alert rule
 * @param rule - Alert rule data
 * @returns Promise containing created alert rule
 */
export async function createAlertRule(
  rule: CreateAlertRuleRequest,
): Promise<AlertRule> {
  const response = await fetch("/api/v1/alerts/rules", {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify(rule),
  });

  if (!response.ok) {
    throw new Error("Failed to create alert rule");
  }

  return response.json();
}

/**
 * Update an existing alert rule
 * @param id - Alert rule ID
 * @param updates - Update data
 * @returns Promise containing updated alert rule
 */
export async function updateAlertRule(
  id: string,
  updates: UpdateAlertRuleRequest,
): Promise<AlertRule> {
  const response = await fetch(`/api/v1/alerts/rules/${id}`, {
    method: "PATCH",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify(updates),
  });

  if (!response.ok) {
    throw new Error("Failed to update alert rule");
  }

  return response.json();
}

/**
 * Delete an alert rule
 * @param id - Alert rule ID
 * @returns Promise that resolves when deleted
 */
export async function deleteAlertRule(id: string): Promise<void> {
  const response = await fetch(`/api/v1/alerts/rules/${id}`, {
    method: "DELETE",
  });

  if (!response.ok) {
    throw new Error("Failed to delete alert rule");
  }
}

/**
 * Fetch alerts with optional filtering
 * @param params - Query parameters
 * @returns Promise containing list of alerts and pagination info
 */
export async function getAlerts(
  params?: AlertQueryParams,
): Promise<{ alerts: Alert[]; total: number; hasMore: boolean }> {
  const queryParams = new URLSearchParams();

  if (params) {
    if (params.status) {
      params.status.forEach((s) => queryParams.append("status", s));
    }
    if (params.severity) {
      params.severity.forEach((s) => queryParams.append("severity", s));
    }
    if (params.ruleId) queryParams.append("ruleId", params.ruleId);
    if (params.assignedTo) queryParams.append("assignedTo", params.assignedTo);
    if (params.search) queryParams.append("search", params.search);
    if (params.startTime) queryParams.append("startTime", params.startTime);
    if (params.endTime) queryParams.append("endTime", params.endTime);
    if (params.limit) queryParams.append("limit", params.limit.toString());
    if (params.offset) queryParams.append("offset", params.offset.toString());
  }

  const response = await fetch(`/api/v1/alerts?${queryParams.toString()}`);

  if (!response.ok) {
    throw new Error("Failed to fetch alerts");
  }

  const data = await response.json();
  return {
    ...data,
    hasMore: data.alerts.length >= (params?.limit || 50),
  };
}

/**
 * Get a single alert by ID
 * @param id - Alert ID
 * @returns Promise containing alert
 */
export async function getAlert(id: string): Promise<Alert> {
  const response = await fetch(`/api/v1/alerts/${id}`);

  if (!response.ok) {
    throw new Error("Failed to fetch alert");
  }

  return response.json();
}

/**
 * Acknowledge an alert
 * @param id - Alert ID
 * @param comment - Optional comment
 * @returns Promise containing updated alert
 */
export async function acknowledgeAlert(
  id: string,
  comment?: string,
): Promise<Alert> {
  const response = await fetch(`/api/v1/alerts/${id}/acknowledge`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify({ comment }),
  });

  if (!response.ok) {
    throw new Error("Failed to acknowledge alert");
  }

  return response.json();
}

/**
 * Resolve an alert
 * @param id - Alert ID
 * @param comment - Optional resolution comment
 * @returns Promise containing updated alert
 */
export async function resolveAlert(
  id: string,
  comment?: string,
): Promise<Alert> {
  const response = await fetch(`/api/v1/alerts/${id}/resolve`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify({ comment }),
  });

  if (!response.ok) {
    throw new Error("Failed to resolve alert");
  }

  return response.json();
}

/**
 * Assign an alert to a user
 * @param id - Alert ID
 * @param userId - User ID to assign to
 * @returns Promise containing updated alert
 */
export async function assignAlert(id: string, userId: string): Promise<Alert> {
  const response = await fetch(`/api/v1/alerts/${id}/assign`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify({ userId }),
  });

  if (!response.ok) {
    throw new Error("Failed to assign alert");
  }

  return response.json();
}

/**
 * Get alert statistics
 * @param timeRange - Optional time range for statistics
 * @returns Promise containing alert statistics
 */
export async function getAlertStats(timeRange?: {
  startTime?: string;
  endTime?: string;
}): Promise<AlertStats> {
  const queryParams = new URLSearchParams();

  if (timeRange?.startTime) queryParams.append("startTime", timeRange.startTime);
  if (timeRange?.endTime) queryParams.append("endTime", timeRange.endTime);

  const response = await fetch(`/api/v1/alerts/stats?${queryParams.toString()}`);

  if (!response.ok) {
    throw new Error("Failed to fetch alert statistics");
  }

  return response.json();
}

/**
 * Test an alert rule
 * @param id - Alert rule ID
 * @returns Promise containing test results
 */
export async function testAlertRule(id: string): Promise<{
  success: boolean;
  triggered: boolean;
  message: string;
  triggeredAt?: string;
}> {
  const response = await fetch(`/api/v1/alerts/rules/${id}/test`, {
    method: "POST",
  });

  if (!response.ok) {
    throw new Error("Failed to test alert rule");
  }

  return response.json();
}

/**
 * Toggle alert rule enabled status
 * @param id - Alert rule ID
 * @param enabled - Enabled status
 * @returns Promise containing updated alert rule
 */
export async function toggleAlertRule(
  id: string,
  enabled: boolean,
): Promise<AlertRule> {
  const response = await fetch(`/api/v1/alerts/rules/${id}/toggle`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify({ enabled }),
  });

  if (!response.ok) {
    throw new Error("Failed to toggle alert rule");
  }

  return response.json();
}

/**
 * Get alert history for a rule
 * @param ruleId - Alert rule ID
 * @param limit - Number of history entries to fetch
 * @returns Promise containing alert history
 */
export async function getAlertHistory(
  ruleId: string,
  limit: number = 100,
): Promise<Alert[]> {
  const response = await fetch(
    `/api/v1/alerts/rules/${ruleId}/history?limit=${limit}`,
  );

  if (!response.ok) {
    throw new Error("Failed to fetch alert history");
  }

  return response.json();
}

/**
 * Get severity color for UI
 * @param severity - Alert severity
 * @returns CSS color class
 */
export function getSeverityColor(severity: AlertSeverity): string {
  switch (severity) {
    case "critical":
      return "text-red-700 bg-red-100 dark:bg-red-900/20";
    case "high":
      return "text-red-600 bg-red-50 dark:bg-red-900/10";
    case "medium":
      return "text-yellow-600 bg-yellow-50 dark:bg-yellow-900/10";
    case "low":
      return "text-blue-600 bg-blue-50 dark:bg-blue-900/10";
    case "info":
      return "text-gray-600 bg-gray-50 dark:bg-gray-900/10";
    default:
      return "text-gray-600 bg-gray-50 dark:bg-gray-900/10";
  }
}

/**
 * Get status color for UI
 * @param status - Alert status
 * @returns CSS color class
 */
export function getStatusColor(status: AlertStatus): string {
  switch (status) {
    case "open":
      return "text-red-600 bg-red-50 dark:bg-red-900/10";
    case "acknowledged":
      return "text-yellow-600 bg-yellow-50 dark:bg-yellow-900/10";
    case "resolved":
      return "text-green-600 bg-green-50 dark:bg-green-900/10";
    case "suppressed":
      return "text-gray-600 bg-gray-50 dark:bg-gray-900/10";
    default:
      return "text-gray-600 bg-gray-50 dark:bg-gray-900/10";
  }
}

/**
 * Format duration in human-readable format
 * @param seconds - Duration in seconds
 * @returns Formatted duration string
 */
export function formatDuration(seconds: number): string {
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const secs = seconds % 60;

  if (hours > 0) {
    return `${hours}h ${minutes}m`;
  } else if (minutes > 0) {
    return `${minutes}m ${secs}s`;
  } else {
    return `${secs}s`;
  }
}

/**
 * Check if severity is high priority
 * @param severity - Alert severity
 * @returns True if critical or high
 */
export function isHighPriority(severity: AlertSeverity): boolean {
  return severity === "critical" || severity === "high";
}

/**
 * Get alert priority score for sorting
 * @param alert - Alert object
 * @returns Priority score (higher = more priority)
 */
export function getAlertPriorityScore(alert: Alert): number {
  const severityScore: Record<AlertSeverity, number> = {
    critical: 100,
    high: 80,
    medium: 60,
    low: 40,
    info: 20,
  };

  const statusScore: Record<AlertStatus, number> = {
    open: 100,
    acknowledged: 50,
    resolved: 0,
    suppressed: 0,
  };

  return severityScore[alert.severity] + statusScore[alert.status];
}
