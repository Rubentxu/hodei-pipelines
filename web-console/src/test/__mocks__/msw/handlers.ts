import { http, HttpResponse } from "msw";

// Observability API mock
export const handlers = [
  // Dashboard KPIs
  http.get("/api/observability/metrics/kpis", () => {
    return HttpResponse.json({
      activeJobs: 452,
      clusterHealth: 98,
      monthlyCost: 1240,
      queuePressure: 3,
    });
  }),

  // Pipeline list
  http.get("/api/pipelines", () => {
    return HttpResponse.json([
      {
        id: "pipe-123",
        name: "web-app-build",
        status: "active",
        lastRun: "2025-11-27T14:32:00Z",
        tags: ["react", "docker"],
      },
      {
        id: "pipe-124",
        name: "api-deploy",
        status: "paused",
        lastRun: "2025-11-27T13:15:00Z",
        tags: ["nodejs", "postgresql"],
      },
    ]);
  }),

  // Execution history
  http.get("/api/pipelines/:id/executions", () => {
    return HttpResponse.json([
      {
        id: "exec-1247",
        status: "success",
        duration: 323,
        cost: 0.42,
        startedAt: "2025-11-27T14:32:00Z",
        trigger: "manual",
      },
    ]);
  }),

  // Resource Pool List
  http.get("/api/v1/worker-pools", () => {
    return HttpResponse.json([
      {
        id: "pool-1",
        name: "Docker Pool",
        poolType: "Docker",
        providerName: "docker",
        minSize: 1,
        maxSize: 10,
        defaultResources: {
          cpu_m: 2000,
          memory_mb: 4096,
          gpu: null,
        },
        tags: { env: "test" },
      },
      {
        id: "pool-2",
        name: "Kubernetes Pool",
        poolType: "Kubernetes",
        providerName: "kubernetes",
        minSize: 2,
        maxSize: 20,
        defaultResources: {
          cpu_m: 4000,
          memory_mb: 8192,
          gpu: null,
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
      poolType: "Docker",
      providerName: "docker",
      minSize: 1,
      maxSize: 10,
      defaultResources: {
        cpu_m: 2000,
        memory_mb: 4096,
        gpu: null,
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
      poolType: "Docker",
      providerName: "docker",
      minSize: body.minSize || 1,
      maxSize: body.maxSize || 10,
      defaultResources: {
        cpu_m: 2000,
        memory_mb: 4096,
        gpu: null,
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
      poolType: "Docker",
      totalCapacity: 10,
      availableCapacity: 8,
      activeWorkers: 2,
      pendingRequests: 0,
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
    const body = await request.json();

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
    const body = await request.json();
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
    const body = await request.json();
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
    const body = await request.json();
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
    const body = await request.json();
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
    const body = await request.json();
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
    const body = await request.json();
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
