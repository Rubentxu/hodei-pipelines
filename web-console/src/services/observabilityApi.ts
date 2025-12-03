import {
  Alert,
  AlertRule,
  LogEntry,
  ObservabilityMetricsResponse,
  Trace,
} from "../types";

const API_BASE = "/api/v1/observability";

export const observabilityApi = {
  // Logs
  async getLogs(params?: {
    level?: string;
    service?: string;
    traceId?: string;
    startTime?: string;
    endTime?: string;
    search?: string;
    limit?: number;
    offset?: number;
  }): Promise<{ logs: LogEntry[]; total: number }> {
    const queryParams = new URLSearchParams();

    if (params) {
      Object.entries(params).forEach(([key, value]) => {
        if (value !== undefined && value !== null) {
          queryParams.append(key, value.toString());
        }
      });
    }

    const response = await fetch(`${API_BASE}/logs?${queryParams.toString()}`);

    if (!response.ok) {
      throw new Error("Failed to fetch logs");
    }

    return response.json();
  },

  streamLogs(onMessage: (log: LogEntry) => void): EventSource {
    const eventSource = new EventSource(`${API_BASE}/logs/stream`);

    eventSource.onmessage = (event) => {
      const log: LogEntry = JSON.parse(event.data);
      onMessage(log);
    };

    eventSource.onerror = () => {
      eventSource.close();
    };

    return eventSource;
  },

  // Traces
  async getTraces(params?: {
    service?: string;
    status?: string;
    minDuration?: number;
    maxDuration?: number;
    startTime?: string;
    endTime?: string;
    limit?: number;
    offset?: number;
  }): Promise<{ traces: Trace[]; total: number }> {
    const queryParams = new URLSearchParams();

    if (params) {
      Object.entries(params).forEach(([key, value]) => {
        if (value !== undefined && value !== null) {
          queryParams.append(key, value.toString());
        }
      });
    }

    const response = await fetch(
      `${API_BASE}/traces?${queryParams.toString()}`,
    );

    if (!response.ok) {
      throw new Error("Failed to fetch traces");
    }

    return response.json();
  },

  async getTraceById(traceId: string): Promise<Trace> {
    const response = await fetch(`${API_BASE}/traces/${traceId}`);

    if (!response.ok) {
      throw new Error("Failed to fetch trace");
    }

    return response.json();
  },

  // Metrics
  async getMetrics(): Promise<ObservabilityMetricsResponse> {
    const url = `${API_BASE}/metrics`;
    console.log('observabilityApi: getMetrics called', url);
    const response = await fetch(url);
    console.log('observabilityApi: getMetrics response status:', response.status);

    if (!response.ok) {
      throw new Error("Failed to fetch metrics");
    }

    return response.json();
  },

  streamMetrics(
    onMessage: (metrics: ObservabilityMetricsResponse) => void,
  ): EventSource {
    const eventSource = new EventSource(`${API_BASE}/metrics/stream`);

    eventSource.onmessage = (event) => {
      const metrics: ObservabilityMetricsResponse = JSON.parse(event.data);
      onMessage(metrics);
    };

    eventSource.onerror = () => {
      eventSource.close();
    };

    return eventSource;
  },

  // Alert Rules
  async getAlertRules(): Promise<AlertRule[]> {
    const response = await fetch(`${API_BASE}/alerts/rules`);

    if (!response.ok) {
      throw new Error("Failed to fetch alert rules");
    }

    return response.json();
  },

  async getAlertRuleById(ruleId: string): Promise<AlertRule> {
    const response = await fetch(`${API_BASE}/alerts/rules/${ruleId}`);

    if (!response.ok) {
      throw new Error("Failed to fetch alert rule");
    }

    return response.json();
  },

  async createAlertRule(
    rule: Omit<AlertRule, "id" | "createdAt">,
  ): Promise<AlertRule> {
    const response = await fetch(`${API_BASE}/alerts/rules`, {
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
  },

  async updateAlertRule(
    ruleId: string,
    updates: Partial<AlertRule>,
  ): Promise<AlertRule> {
    const response = await fetch(`${API_BASE}/alerts/rules/${ruleId}`, {
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
  },

  async deleteAlertRule(ruleId: string): Promise<void> {
    const response = await fetch(`${API_BASE}/alerts/rules/${ruleId}`, {
      method: "DELETE",
    });

    if (!response.ok) {
      throw new Error("Failed to delete alert rule");
    }
  },

  async toggleAlertRule(ruleId: string, enabled: boolean): Promise<AlertRule> {
    return this.updateAlertRule(ruleId, { enabled });
  },

  // Alerts
  async getAlerts(params?: {
    severity?: string;
    acknowledged?: boolean;
    startTime?: string;
    endTime?: string;
    limit?: number;
    offset?: number;
  }): Promise<{ alerts: Alert[]; total: number }> {
    const queryParams = new URLSearchParams();

    if (params) {
      Object.entries(params).forEach(([key, value]) => {
        if (value !== undefined && value !== null) {
          queryParams.append(key, value.toString());
        }
      });
    }

    const response = await fetch(
      `${API_BASE}/alerts?${queryParams.toString()}`,
    );

    if (!response.ok) {
      throw new Error("Failed to fetch alerts");
    }

    return response.json();
  },

  async acknowledgeAlert(alertId: string, userId: string): Promise<Alert> {
    const response = await fetch(`${API_BASE}/alerts/${alertId}/acknowledge`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({ userId }),
    });

    if (!response.ok) {
      throw new Error("Failed to acknowledge alert");
    }

    return response.json();
  },

  // Service Map
  async getServiceMap(): Promise<{
    services: Array<{
      id: string;
      name: string;
      status: "healthy" | "degraded" | "unhealthy";
      dependencies: string[];
      metrics: {
        requestRate: number;
        errorRate: number;
        avgResponseTime: number;
      };
    }>;
  }> {
    const response = await fetch(`${API_BASE}/services`);

    if (!response.ok) {
      throw new Error("Failed to fetch service map");
    }

    return response.json();
  },
};
