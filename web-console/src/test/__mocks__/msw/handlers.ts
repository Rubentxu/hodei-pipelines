import { http, HttpResponse } from "msw";

// Observability API mock
export const handlers = [
  // Dashboard KPIs
  http.get("/api/v1/metrics/dashboard", () => {
    return HttpResponse.json({
      active_pipelines: 42,
      avg_duration: 125,
      cost_per_run: 0.45,
      queue_time: 12,
      success_rate: 94.5,
      total_executions_today: 128,
      total_pipelines: 50,
      timestamp: new Date().toISOString(),
    });
  }),

  // Pipeline list
  http.get("/api/v1/pipelines", () => {
    return HttpResponse.json({
      items: [
        {
          id: "pipe-123",
          name: "web-app-build",
          description: "Builds the web application",
          steps: [],
          created_at: "2025-11-27T14:32:00Z",
          updated_at: "2025-11-27T14:32:00Z",
        },
        {
          id: "pipe-124",
          name: "api-deploy",
          description: "Deploys the API",
          steps: [],
          created_at: "2025-11-27T13:15:00Z",
          updated_at: "2025-11-27T13:15:00Z",
        },
      ],
      total: 2,
    });
  }),

  // Get Pipeline
  http.get("/api/v1/pipelines/:id", ({ params }) => {
    const { id } = params;
    if (id === "non-existent") {
      return new HttpResponse(null, { status: 404 });
    }
    return HttpResponse.json({
      id: id,
      name: "web-app-build",
      description: "Builds the web application",
      steps: [],
      created_at: "2025-11-27T14:32:00Z",
      updated_at: "2025-11-27T14:32:00Z",
    });
  }),

  // Create Pipeline
  http.post("/api/v1/pipelines", async ({ request }) => {
    const body = (await request.json()) as any;
    return HttpResponse.json({
      id: "new-pipe-123",
      name: body.name,
      description: body.description,
      steps: body.steps || [],
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
    });
  }),

  // Update Pipeline
  http.put("/api/v1/pipelines/:id", async ({ params, request }) => {
    const { id } = params;
    const body = (await request.json()) as any;
    return HttpResponse.json({
      id: id,
      name: body.name,
      description: body.description,
      steps: body.steps || [],
      created_at: "2025-11-27T14:32:00Z",
      updated_at: new Date().toISOString(),
    });
  }),

  // Delete Pipeline
  http.delete("/api/v1/pipelines/:id", ({ params }) => {
    return new HttpResponse(null, { status: 200 });
  }),

  // Execute Pipeline
  http.post("/api/v1/pipelines/:id/execute", ({ params }) => {
    return HttpResponse.json({
      execution_id: "exec-123",
      pipeline_id: params.id,
      status: "pending",
    });
  }),

  // Execution history (Note: The OpenAPI path is /api/v1/pipelines/{id}/executions/{execution_id}/logs, but the test might be looking for a list)
  // Based on previous handlers, let's keep a generic executions list if needed, but update the path
  // However, the OpenAPI doesn't seem to have a simple "list executions for pipeline" endpoint in the snippet I saw.
  // I'll check the file content again if needed, but for now I'll add the specific one I saw.

  // Resource Pool List
  http.get("/api/v1/worker-pools", () => {
    return HttpResponse.json([
      {
        id: "pool-1",
        name: "Docker Pool",
        pool_type: { type: "Docker" },
        provider_name: "docker",
        min_size: 1,
        max_size: 10,
        default_resources: {
          cpu_cores: 2,
          memory_mb: 4096,
        },
        tags: { env: "test" },
      },
      {
        id: "pool-2",
        name: "Kubernetes Pool",
        pool_type: { type: "Kubernetes" },
        provider_name: "kubernetes",
        min_size: 2,
        max_size: 20,
        default_resources: {
          cpu_cores: 4,
          memory_mb: 8192,
        },
        tags: { env: "prod" },
      },
    ]);
  }),

  // Resource Pool Detail
  http.get("/api/v1/worker-pools/:id", ({ params }) => {
    const { id } = params;
    if (id === "non-existent") {
      return HttpResponse.json({ message: "Pool not found" }, { status: 404 });
    }
    return HttpResponse.json({
      id,
      name: "Docker Pool",
      pool_type: { type: "Docker" },
      provider_name: "docker",
      min_size: 1,
      max_size: 10,
      default_resources: {
        cpu_cores: 2,
        memory_mb: 4096,
      },
      tags: { env: "test" },
    });
  }),

  // Create Resource Pool
  http.post("/api/v1/worker-pools", async ({ request }) => {
    const body = (await request.json()) as any;
    if (body.name === "Existing Pool") {
      return HttpResponse.json(
        { message: "Pool already exists" },
        { status: 409 },
      );
    }
    return HttpResponse.json({
      id: "new-pool-id",
      ...body,
    });
  }),

  // Update Resource Pool
  http.patch("/api/v1/worker-pools/:id", async ({ params, request }) => {
    const { id } = params;
    if (id === "non-existent") {
      return HttpResponse.json({ message: "Pool not found" }, { status: 404 });
    }
    const body = (await request.json()) as any;
    return HttpResponse.json({
      id,
      name: body.name || "Updated Pool",
      pool_type: { type: "Docker" },
      provider_name: "docker",
      min_size: body.min_size || 1,
      max_size: body.max_size || 10,
      default_resources: {
        cpu_cores: 2,
        memory_mb: 4096,
      },
      tags: body.tags || {},
    });
  }),

  // Delete Resource Pool
  http.delete("/api/v1/worker-pools/:id", ({ params }) => {
    const { id } = params;
    if (id === "non-existent") {
      return HttpResponse.json({ message: "Pool not found" }, { status: 404 });
    }
    return new HttpResponse(null, { status: 204 });
  }),

  // Resource Pool Status
  http.get("/api/v1/worker-pools/:id/status", ({ params }) => {
    const { id } = params;
    if (id === "non-existent") {
      return HttpResponse.json(
        { message: "Pool status not found" },
        { status: 404 },
      );
    }
    return HttpResponse.json({
      name: "Docker Pool",
      pool_type: { type: "Docker" },
      total_capacity: 10,
      available_capacity: 8,
      active_workers: 2,
      pending_requests: 0,
    });
  }),

  // Logs API
  http.get("/api/v1/logs", ({ request }) => {
    const url = new URL(request.url);
    const level = url.searchParams.get("level");
    const search = url.searchParams.get("search");
    const service = url.searchParams.get("service");

    const allLogs = [
      {
        id: "log-1",
        timestamp: "2025-11-30T09:30:00Z",
        level: "error",
        service: "worker-01",
        message: "Failed to process job job-123",
        traceId: "trace-123",
        spanId: "span-456",
        userId: "user-789",
        metadata: { jobId: "job-123" },
      },
      {
        id: "log-2",
        timestamp: "2025-11-30T09:29:45Z",
        level: "info",
        service: "worker-01",
        message: "Job completed successfully",
        metadata: { jobId: "job-123" },
      },
      {
        id: "log-3",
        timestamp: "2025-11-30T09:29:30Z",
        level: "warn",
        service: "worker-02",
        message: "High memory usage detected",
        metadata: { memoryUsage: "85%" },
      },
      {
        id: "log-4",
        timestamp: "2025-11-30T09:29:15Z",
        level: "debug",
        service: "worker-01",
        message: "Starting job execution",
      },
      {
        id: "log-5",
        timestamp: "2025-11-30T09:29:00Z",
        level: "info",
        service: "worker-02",
        message: "Database connection established",
      },
      {
        id: "log-6",
        timestamp: "2025-11-30T09:28:45Z",
        level: "error",
        service: "worker-03",
        message: "Timeout waiting for response",
        metadata: { timeoutMs: 5000 },
      },
    ];

    let filteredLogs = allLogs;

    if (level) {
      filteredLogs = filteredLogs.filter((log) => log.level === level);
    }

    if (service) {
      filteredLogs = filteredLogs.filter((log) => log.service === service);
    }

    if (search) {
      filteredLogs = filteredLogs.filter((log) =>
        log.message.toLowerCase().includes(search.toLowerCase()),
      );
    }

    return HttpResponse.json({
      logs: filteredLogs,
      total: filteredLogs.length,
    });
  }),

  http.get("/api/v1/logs/levels", () => {
    return HttpResponse.json(["debug", "info", "warn", "error", "fatal"]);
  }),

  http.get("/api/v1/logs/services", () => {
    return HttpResponse.json(["worker-01", "worker-02", "worker-03", "server"]);
  }),

  http.get("/api/v1/logs/export", ({ request }) => {
    const url = new URL(request.url);
    const format = url.searchParams.get("format") || "csv";

    const logs = [
      "timestamp,level,service,message",
      "2025-11-30T09:30:00Z,error,worker-01,Failed to process job",
      "2025-11-30T09:29:45Z,info,worker-01,Job completed successfully",
    ].join("\n");

    return new HttpResponse(logs, {
      headers: {
        "Content-Type": format === "csv" ? "text/csv" : "application/json",
        "Content-Disposition": `attachment; filename="logs.${format}"`,
      },
    });
  }),

  http.get("/api/v1/logs/aggregate", ({ request }) => {
    const url = new URL(request.url);
    const groupBy = url.searchParams.get("groupBy") || "service";

    if (groupBy === "service") {
      return HttpResponse.json({
        byService: {
          "worker-01": 45,
          "worker-02": 32,
          "worker-03": 28,
        },
        total: 105,
      });
    } else if (groupBy === "level") {
      return HttpResponse.json({
        byLevel: {
          error: 12,
          warn: 8,
          info: 85,
          debug: 10,
        },
        total: 115,
      });
    } else if (groupBy === "time") {
      return HttpResponse.json({
        byTime: [
          { timestamp: "2025-11-30T09:00:00Z", count: 15 },
          { timestamp: "2025-11-30T10:00:00Z", count: 22 },
          { timestamp: "2025-11-30T11:00:00Z", count: 18 },
        ],
        total: 55,
      });
    }

    return HttpResponse.json({
      total: 0,
    });
  }),

  http.post("/api/v1/logs/search", async ({ request }) => {
    const body = await request.json() as any;

    // Ensure the response is valid
    return HttpResponse.json(
      {
        logs: [
          {
            id: "log-1",
            timestamp: "2025-11-30T09:30:00Z",
            level: "error",
            service: "worker-01",
            message: "Database connection failed",
            traceId: "trace-123",
            metadata: { errorCode: "DB_CONN_ERROR" },
          },
        ],
        total: 1,
        searchQuery:
          typeof body.query === "string" ? body.query : body.query?.query || "",
        searchTime: 0.045,
      },
      { status: 200 },
    );
  }),

  http.get("/api/v1/logs/stats", () => {
    return HttpResponse.json({
      totalLogs: 115,
      errorRate: 0.11,
      topServices: [
        { service: "worker-01", count: 45 },
        { service: "worker-02", count: 32 },
        { service: "worker-03", count: 28 },
      ],
      topLevels: [
        { level: "info", count: 85 },
        { level: "warn", count: 8 },
        { level: "error", count: 12 },
        { level: "debug", count: 10 },
      ],
    });
  }),

  // Alert Management API
  http.get("/api/v1/alerts", ({ request }) => {
    const url = new URL(request.url);
    const status = url.searchParams.getAll("status");
    const severity = url.searchParams.getAll("severity");

    const allAlerts = [
      {
        id: "alert-1",
        ruleId: "rule-1",
        ruleName: "High CPU Usage",
        severity: "critical",
        status: "open",
        message: "CPU usage exceeded 90% for 5 minutes",
        triggeredAt: "2025-11-30T10:05:00Z",
        metadata: { cpuUsage: 92 },
      },
      {
        id: "alert-2",
        ruleId: "rule-2",
        ruleName: "Low Memory",
        severity: "medium",
        status: "acknowledged",
        message: "Memory usage below 20%",
        triggeredAt: "2025-11-30T10:00:00Z",
        acknowledgedAt: "2025-11-30T10:02:00Z",
        acknowledgedBy: "user-1",
      },
      {
        id: "alert-3",
        ruleId: "rule-3",
        ruleName: "Disk Space Critical",
        severity: "high",
        status: "resolved",
        message: "Disk space usage at 95%",
        triggeredAt: "2025-11-30T09:30:00Z",
        resolvedAt: "2025-11-30T09:45:00Z",
        resolvedBy: "user-2",
      },
    ];

    let filteredAlerts = allAlerts;

    if (status.length > 0) {
      filteredAlerts = filteredAlerts.filter((alert) =>
        status.includes(alert.status),
      );
    }

    if (severity.length > 0) {
      filteredAlerts = filteredAlerts.filter((alert) =>
        severity.includes(alert.severity),
      );
    }

    return HttpResponse.json({
      alerts: filteredAlerts,
      total: filteredAlerts.length,
    });
  }),

  http.get("/api/v1/alerts/:id", ({ params }) => {
    const { id } = params;
    if (id === "non-existent") {
      return HttpResponse.json({ message: "Alert not found" }, { status: 404 });
    }
    return HttpResponse.json({
      id,
      ruleId: "rule-1",
      ruleName: "High CPU Usage",
      severity: "critical",
      status: "open",
      message: "CPU usage exceeded 90%",
      triggeredAt: "2025-11-30T10:05:00Z",
    });
  }),

  http.post("/api/v1/alerts/:id/acknowledge", async ({ params, request }) => {
    const { id } = params;
    if (id === "non-existent") {
      return HttpResponse.json({ message: "Alert not found" }, { status: 404 });
    }
    const body = await request.json() as any;
    return HttpResponse.json({
      id,
      ruleId: "rule-1",
      ruleName: "High CPU Usage",
      severity: "critical",
      status: "acknowledged",
      message: "CPU usage exceeded 90%",
      triggeredAt: "2025-11-30T10:05:00Z",
      acknowledgedAt: "2025-11-30T10:07:00Z",
      acknowledgedBy: body.userId || "user-1",
    });
  }),

  http.post("/api/v1/alerts/:id/resolve", async ({ params, request }) => {
    const { id } = params;
    if (id === "non-existent") {
      return HttpResponse.json({ message: "Alert not found" }, { status: 404 });
    }
    const body = await request.json() as any;
    return HttpResponse.json({
      id,
      ruleId: "rule-1",
      ruleName: "High CPU Usage",
      severity: "critical",
      status: "resolved",
      message: "CPU usage exceeded 90%",
      triggeredAt: "2025-11-30T10:05:00Z",
      resolvedAt: "2025-11-30T10:10:00Z",
      resolvedBy: body.userId || "user-1",
    });
  }),

  http.post("/api/v1/alerts/:id/assign", async ({ params, request }) => {
    const { id } = params;
    if (id === "non-existent") {
      return HttpResponse.json({ message: "Alert not found" }, { status: 404 });
    }
    const body = await request.json() as any;
    return HttpResponse.json({
      id,
      ruleId: "rule-1",
      ruleName: "High CPU Usage",
      severity: "critical",
      status: "open",
      message: "CPU usage exceeded 90%",
      triggeredAt: "2025-11-30T10:05:00Z",
      assignments: [
        {
          userId: body.userId,
          assignedAt: "2025-11-30T10:07:00Z",
          assignedBy: "user-2",
        },
      ],
    });
  }),

  http.get("/api/v1/alerts/rules", ({ request }) => {
    const url = new URL(request.url);
    const enabled = url.searchParams.get("enabled");
    const severity = url.searchParams.get("severity");

    const allRules = [
      {
        id: "rule-1",
        name: "High CPU Usage",
        description: "Alert when CPU usage exceeds 80%",
        severity: "critical",
        conditions: [
          { metric: "cpu", operator: "gt", threshold: 80, duration: "5m" },
        ],
        actions: [{ type: "email", target: "admin@example.com" }],
        enabled: true,
        createdAt: "2025-11-30T10:00:00Z",
        updatedAt: "2025-11-30T10:00:00Z",
        triggerCount: 25,
        lastTriggeredAt: "2025-11-30T10:05:00Z",
      },
      {
        id: "rule-2",
        name: "Low Memory Warning",
        description: "Alert when memory usage is below 20%",
        severity: "medium",
        conditions: [{ metric: "memory", operator: "lt", threshold: 20 }],
        actions: [{ type: "webhook", target: "https://hooks.example.com" }],
        enabled: true,
        createdAt: "2025-11-30T09:00:00Z",
        updatedAt: "2025-11-30T09:00:00Z",
        triggerCount: 8,
      },
      {
        id: "rule-3",
        name: "Disk Space Critical",
        description: "Alert when disk space exceeds 95%",
        severity: "high",
        conditions: [{ metric: "disk", operator: "gt", threshold: 95 }],
        actions: [{ type: "slack", target: "#alerts" }],
        enabled: false,
        createdAt: "2025-11-30T08:00:00Z",
        updatedAt: "2025-11-30T08:00:00Z",
        triggerCount: 3,
      },
    ];

    let filteredRules = allRules;

    if (enabled !== null) {
      filteredRules = filteredRules.filter(
        (rule) => rule.enabled === (enabled === "true"),
      );
    }

    if (severity) {
      filteredRules = filteredRules.filter(
        (rule) => rule.severity === severity,
      );
    }

    return HttpResponse.json(filteredRules);
  }),

  http.get("/api/v1/alerts/rules/:id", ({ params }) => {
    const { id } = params;
    if (id === "non-existent") {
      return HttpResponse.json({ message: "Rule not found" }, { status: 404 });
    }
    return HttpResponse.json({
      id,
      name: "High CPU Usage",
      description: "Alert when CPU usage exceeds 80%",
      severity: "critical",
      conditions: [
        { metric: "cpu", operator: "gt", threshold: 80, duration: "5m" },
      ],
      actions: [{ type: "email", target: "admin@example.com" }],
      enabled: true,
      createdAt: "2025-11-30T10:00:00Z",
      updatedAt: "2025-11-30T10:00:00Z",
      triggerCount: 25,
    });
  }),

  http.post("/api/v1/alerts/rules", async ({ request }) => {
    const body = await request.json() as any;
    return HttpResponse.json({
      id: "new-rule-id",
      ...body,
      enabled: true,
      createdAt: "2025-11-30T10:30:00Z",
      updatedAt: "2025-11-30T10:30:00Z",
      triggerCount: 0,
    });
  }),

  http.patch("/api/v1/alerts/rules/:id", async ({ params, request }) => {
    const { id } = params;
    if (id === "non-existent") {
      return HttpResponse.json({ message: "Rule not found" }, { status: 404 });
    }
    const body = await request.json() as any;
    return HttpResponse.json({
      id,
      name: body.name || "Updated Rule",
      description: body.description,
      severity: "critical",
      conditions: [{ metric: "cpu", operator: "gt", threshold: 80 }],
      actions: [{ type: "email", target: "admin@example.com" }],
      enabled: body.enabled ?? true,
      createdAt: "2025-11-30T10:00:00Z",
      updatedAt: "2025-11-30T10:35:00Z",
      triggerCount: 25,
    });
  }),

  http.delete("/api/v1/alerts/rules/:id", ({ params }) => {
    const { id } = params;
    if (id === "non-existent") {
      return HttpResponse.json({ message: "Rule not found" }, { status: 404 });
    }
    return new HttpResponse(null, { status: 204 });
  }),

  http.post("/api/v1/alerts/rules/:id/toggle", async ({ params, request }) => {
    const { id } = params;
    const body = await request.json() as any;
    if (id === "non-existent") {
      return HttpResponse.json({ message: "Rule not found" }, { status: 404 });
    }
    return HttpResponse.json({
      id,
      name: "Test Rule",
      severity: "critical",
      conditions: [],
      actions: [],
      enabled: body.enabled,
      createdAt: "2025-11-30T10:00:00Z",
      updatedAt: "2025-11-30T10:40:00Z",
      triggerCount: 5,
    });
  }),

  http.post("/api/v1/alerts/rules/:id/test", ({ params }) => {
    const { id } = params;
    if (id === "non-existent") {
      return HttpResponse.json({ message: "Rule not found" }, { status: 404 });
    }
    return HttpResponse.json({
      success: true,
      triggered: true,
      message: "Rule would trigger with current metrics",
      triggeredAt: "2025-11-30T10:45:00Z",
    });
  }),

  http.get("/api/v1/alerts/rules/:ruleId/history", ({ params }) => {
    const { ruleId } = params;
    if (ruleId === "non-existent") {
      return HttpResponse.json({ message: "Rule not found" }, { status: 404 });
    }
    return HttpResponse.json([
      {
        id: "alert-history-1",
        ruleId,
        ruleName: "High CPU",
        severity: "critical",
        status: "resolved",
        message: "CPU usage was high",
        triggeredAt: "2025-11-30T10:00:00Z",
        resolvedAt: "2025-11-30T10:05:00Z",
      },
    ]);
  }),

  http.get("/api/v1/alerts/stats", () => {
    return HttpResponse.json({
      totalAlerts: 150,
      openAlerts: 12,
      acknowledgedAlerts: 8,
      resolvedAlerts: 130,
      bySeverity: {
        critical: 15,
        high: 25,
        medium: 60,
        low: 40,
        info: 10,
      },
      avgResolutionTime: 45,
      topRules: [
        { ruleId: "rule-1", ruleName: "CPU Alert", count: 35 },
        { ruleId: "rule-2", ruleName: "Memory Alert", count: 28 },
      ],
    });
  }),
];
