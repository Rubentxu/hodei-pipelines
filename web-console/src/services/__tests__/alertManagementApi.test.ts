import { describe, it, expect, beforeEach } from "vitest";
import {
  getAlertRules,
  getAlertRule,
  createAlertRule,
  updateAlertRule,
  deleteAlertRule,
  getAlerts,
  getAlert,
  acknowledgeAlert,
  resolveAlert,
  assignAlert,
  getAlertStats,
  testAlertRule,
  toggleAlertRule,
  getAlertHistory,
  getSeverityColor,
  getStatusColor,
  formatDuration,
  isHighPriority,
  getAlertPriorityScore,
  type AlertRule,
  type CreateAlertRuleRequest,
  type UpdateAlertRuleRequest,
  type Alert,
  type AlertSeverity,
  type AlertStatus,
} from "../alertManagementApi";
import { http, HttpResponse } from "msw";
import { server } from "../../test/__mocks__/msw/server";

describe("alertManagementApi", () => {
  beforeEach(() => {
    server.resetHandlers();
  });

  describe("getAlertRules", () => {
    it("should fetch alert rules successfully", async () => {
      const mockRules: AlertRule[] = [
        {
          id: "rule-1",
          name: "High CPU Usage",
          description: "Alert when CPU usage exceeds 80%",
          severity: "high",
          conditions: [
            { metric: "cpu_usage", operator: "gt", threshold: 80, duration: "5m" },
          ],
          actions: [{ type: "email", target: "admin@example.com" }],
          enabled: true,
          createdAt: "2025-11-30T10:00:00Z",
          updatedAt: "2025-11-30T10:00:00Z",
          triggerCount: 12,
        },
        {
          id: "rule-2",
          name: "Low Memory",
          description: "Alert when memory usage is low",
          severity: "medium",
          conditions: [{ metric: "memory", operator: "lt", threshold: 20 }],
          actions: [{ type: "webhook", target: "https://hooks.example.com" }],
          enabled: false,
          createdAt: "2025-11-30T09:00:00Z",
          updatedAt: "2025-11-30T09:00:00Z",
          triggerCount: 3,
        },
      ];

      server.use(
        http.get("/api/v1/alerts/rules", () => {
          return HttpResponse.json(mockRules);
        }),
      );

      const result = await getAlertRules();
      expect(result).toEqual(mockRules);
      expect(result).toHaveLength(2);
    });

    it("should filter alert rules by enabled status", async () => {
      const mockRules: AlertRule[] = [
        {
          id: "rule-1",
          name: "Active Rule",
          severity: "high",
          conditions: [],
          actions: [],
          enabled: true,
          createdAt: "2025-11-30T10:00:00Z",
          updatedAt: "2025-11-30T10:00:00Z",
          triggerCount: 5,
        },
      ];

      server.use(
        http.get("/api/v1/alerts/rules", ({ request }) => {
          const url = new URL(request.url);
          expect(url.searchParams.get("enabled")).toBe("true");
          return HttpResponse.json(mockRules);
        }),
      );

      await getAlertRules({ enabled: true });
    });

    it("should filter alert rules by severity", async () => {
      const mockRules: AlertRule[] = [
        {
          id: "rule-1",
          name: "Critical Rule",
          severity: "critical",
          conditions: [],
          actions: [],
          enabled: true,
          createdAt: "2025-11-30T10:00:00Z",
          updatedAt: "2025-11-30T10:00:00Z",
          triggerCount: 5,
        },
      ];

      server.use(
        http.get("/api/v1/alerts/rules", ({ request }) => {
          const url = new URL(request.url);
          expect(url.searchParams.get("severity")).toBe("critical");
          return HttpResponse.json(mockRules);
        }),
      );

      await getAlertRules({ severity: "critical" });
    });

    it("should throw error when fetch fails", async () => {
      server.use(
        http.get("/api/v1/alerts/rules", () => {
          return HttpResponse.json(
            { message: "Internal Server Error" },
            { status: 500 },
          );
        }),
      );

      await expect(getAlertRules()).rejects.toThrow(
        "Failed to fetch alert rules",
      );
    });
  });

  describe("getAlertRule", () => {
    it("should fetch a single alert rule by id", async () => {
      const mockRule: AlertRule = {
        id: "rule-1",
        name: "High CPU Usage",
        description: "Alert when CPU usage exceeds 80%",
        severity: "high",
        conditions: [{ metric: "cpu", operator: "gt", threshold: 80 }],
        actions: [{ type: "email", target: "admin@example.com" }],
        enabled: true,
        createdAt: "2025-11-30T10:00:00Z",
        updatedAt: "2025-11-30T10:00:00Z",
        triggerCount: 12,
      };

      server.use(
        http.get("/api/v1/alerts/rules/:id", ({ params }) => {
          const { id } = params;
          if (id === "non-existent") {
            return HttpResponse.json(
              { message: "Rule not found" },
              { status: 404 },
            );
          }
          return HttpResponse.json(mockRule);
        }),
      );

      const result = await getAlertRule("rule-1");
      expect(result).toEqual(mockRule);
    });

    it("should throw error when rule not found", async () => {
      server.use(
        http.get("/api/v1/alerts/rules/:id", ({ params }) => {
          const { id } = params;
          if (id === "non-existent") {
            return HttpResponse.json(
              { message: "Rule not found" },
              { status: 404 },
            );
          }
          return HttpResponse.json({});
        }),
      );

      await expect(getAlertRule("non-existent")).rejects.toThrow(
        "Failed to fetch alert rule",
      );
    });
  });

  describe("createAlertRule", () => {
    it("should create a new alert rule successfully", async () => {
      const request: CreateAlertRuleRequest = {
        name: "New Rule",
        description: "Test rule",
        severity: "medium",
        conditions: [{ metric: "cpu", operator: "gt", threshold: 70 }],
        actions: [{ type: "email", target: "test@example.com" }],
      };

      const mockCreatedRule: AlertRule = {
        id: "new-rule-id",
        name: "New Rule",
        description: "Test rule",
        severity: "medium",
        conditions: [{ metric: "cpu", operator: "gt", threshold: 70 }],
        actions: [{ type: "email", target: "test@example.com" }],
        enabled: true,
        createdAt: "2025-11-30T10:00:00Z",
        updatedAt: "2025-11-30T10:00:00Z",
        triggerCount: 0,
      };

      server.use(
        http.post("/api/v1/alerts/rules", async ({ request }) => {
          const body = await request.json();
          return HttpResponse.json({
            id: "new-rule-id",
            ...body,
            enabled: true,
            createdAt: "2025-11-30T10:00:00Z",
            updatedAt: "2025-11-30T10:00:00Z",
            triggerCount: 0,
          });
        }),
      );

      const result = await createAlertRule(request);
      expect(result).toEqual(mockCreatedRule);
    });

    it("should throw error when validation fails", async () => {
      const request: CreateAlertRuleRequest = {
        name: "",
        severity: "high",
        conditions: [],
        actions: [],
      };

      server.use(
        http.post("/api/v1/alerts/rules", () => {
          return HttpResponse.json(
            { message: "Validation failed" },
            { status: 400 },
          ),
        );
      });

      await expect(createAlertRule(request)).rejects.toThrow(
        "Failed to create alert rule",
      );
    });
  });

  describe("updateAlertRule", () => {
    it("should update an alert rule successfully", async () => {
      const updates: UpdateAlertRuleRequest = {
        name: "Updated Rule",
        description: "Updated description",
        enabled: false,
      };

      const mockUpdatedRule: AlertRule = {
        id: "rule-1",
        name: "Updated Rule",
        description: "Updated description",
        severity: "high",
        conditions: [{ metric: "cpu", operator: "gt", threshold: 80 }],
        actions: [{ type: "email", target: "admin@example.com" }],
        enabled: false,
        createdAt: "2025-11-30T10:00:00Z",
        updatedAt: "2025-11-30T10:05:00Z",
        triggerCount: 12,
      };

      server.use(
        http.patch("/api/v1/alerts/rules/:id", async ({ params, request }) => {
          const { id } = params;
          const body = await request.json();
          if (id === "non-existent") {
            return HttpResponse.json(
              { message: "Rule not found" },
              { status: 404 },
            );
          }
          return HttpResponse.json({
            id,
            name: body.name || "Updated Rule",
            description: body.description,
            severity: "high",
            conditions: [{ metric: "cpu", operator: "gt", threshold: 80 }],
            actions: [{ type: "email", target: "admin@example.com" }],
            enabled: body.enabled ?? true,
            createdAt: "2025-11-30T10:00:00Z",
            updatedAt: "2025-11-30T10:05:00Z",
            triggerCount: 12,
          });
        }),
      );

      const result = await updateAlertRule("rule-1", updates);
      expect(result.name).toBe("Updated Rule");
      expect(result.enabled).toBe(false);
    });

    it("should throw error when rule not found", async () => {
      server.use(
        http.patch("/api/v1/alerts/rules/:id", ({ params }) => {
          const { id } = params;
          if (id === "non-existent") {
            return HttpResponse.json(
              { message: "Rule not found" },
              { status: 404 },
            );
          }
          return HttpResponse.json({});
        }),
      );

      await expect(
        updateAlertRule("non-existent", { name: "Updated" }),
      ).rejects.toThrow("Failed to update alert rule");
    });
  });

  describe("deleteAlertRule", () => {
    it("should delete an alert rule successfully", async () => {
      server.use(
        http.delete("/api/v1/alerts/rules/:id", ({ params }) => {
          const { id } = params;
          if (id === "non-existent") {
            return HttpResponse.json(
              { message: "Rule not found" },
              { status: 404 },
            );
          }
          return new HttpResponse(null, { status: 204 });
        }),
      );

      await expect(deleteAlertRule("rule-1")).resolves.toBeUndefined();
    });

    it("should throw error when rule not found", async () => {
      server.use(
        http.delete("/api/v1/alerts/rules/:id", ({ params }) => {
          const { id } = params;
          if (id === "non-existent") {
            return HttpResponse.json(
              { message: "Rule not found" },
              { status: 404 },
            );
          }
          return new HttpResponse(null, { status: 204 });
        }),
      );

      await expect(deleteAlertRule("non-existent")).rejects.toThrow(
        "Failed to delete alert rule",
      );
    });
  });

  describe("getAlerts", () => {
    it("should fetch alerts successfully", async () => {
      const mockAlerts: Alert[] = [
        {
          id: "alert-1",
          ruleId: "rule-1",
          ruleName: "High CPU Usage",
          severity: "critical",
          status: "open",
          message: "CPU usage exceeded 90%",
          triggeredAt: "2025-11-30T10:05:00Z",
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
      ];

      server.use(
        http.get("/api/v1/alerts", ({ request }) => {
          const url = new URL(request.url);
          const status = url.searchParams.getAll("status");
          const severity = url.searchParams.getAll("severity");

          let filteredAlerts = mockAlerts;

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
      );

      const result = await getAlerts({ limit: 100 });
      expect(result.alerts).toHaveLength(2);
      expect(result.total).toBe(2);
    });

    it("should filter alerts by status", async () => {
      server.use(
        http.get("/api/v1/alerts", ({ request }) => {
          const url = new URL(request.url);
          expect(url.searchParams.getAll("status")).toContain("open");
          return HttpResponse.json({
            alerts: [],
            total: 0,
          });
        }),
      );

      await getAlerts({ status: ["open"] });
    });

    it("should apply pagination", async () => {
      server.use(
        http.get("/api/v1/alerts", ({ request }) => {
          const url = new URL(request.url);
          expect(url.searchParams.get("limit")).toBe("50");
          expect(url.searchParams.get("offset")).toBe("0");
          return HttpResponse.json({
            alerts: [],
            total: 100,
          });
        }),
      );

      await getAlerts({ limit: 50, offset: 0 });
    });

    it("should throw error on fetch failure", async () => {
      server.use(
        http.get("/api/v1/alerts", () => {
          return HttpResponse.json(
            { message: "Server error" },
            { status: 500 },
          ),
        );
      });

      await expect(getAlerts()).rejects.toThrow("Failed to fetch alerts");
    });
  });

  describe("getAlert", () => {
    it("should fetch a single alert by id", async () => {
      const mockAlert: Alert = {
        id: "alert-1",
        ruleId: "rule-1",
        ruleName: "High CPU Usage",
        severity: "critical",
        status: "open",
        message: "CPU usage exceeded 90%",
        triggeredAt: "2025-11-30T10:05:00Z",
      };

      server.use(
        http.get("/api/v1/alerts/:id", ({ params }) => {
          const { id } = params;
          if (id === "non-existent") {
            return HttpResponse.json(
              { message: "Alert not found" },
              { status: 404 },
            );
          }
          return HttpResponse.json(mockAlert);
        }),
      );

      const result = await getAlert("alert-1");
      expect(result).toEqual(mockAlert);
    });

    it("should throw error when alert not found", async () => {
      server.use(
        http.get("/api/v1/alerts/:id", ({ params }) => {
          const { id } = params;
          if (id === "non-existent") {
            return HttpResponse.json(
              { message: "Alert not found" },
              { status: 404 },
            );
          }
          return HttpResponse.json({});
        }),
      );

      await expect(getAlert("non-existent")).rejects.toThrow(
        "Failed to fetch alert",
      );
    });
  });

  describe("acknowledgeAlert", () => {
    it("should acknowledge an alert successfully", async () => {
      const mockAlert: Alert = {
        id: "alert-1",
        ruleId: "rule-1",
        ruleName: "High CPU Usage",
        severity: "critical",
        status: "acknowledged",
        message: "CPU usage exceeded 90%",
        triggeredAt: "2025-11-30T10:05:00Z",
        acknowledgedAt: "2025-11-30T10:07:00Z",
        acknowledgedBy: "user-1",
      };

      server.use(
        http.post("/api/v1/alerts/:id/acknowledge", async ({ params, request }) => {
          const { id } = params;
          if (id === "non-existent") {
            return HttpResponse.json(
              { message: "Alert not found" },
              { status: 404 },
            );
          }
          const body = await request.json();
          return HttpResponse.json({
            ...mockAlert,
            status: "acknowledged",
            acknowledgedAt: "2025-11-30T10:07:00Z",
            acknowledgedBy: "user-1",
          });
        }),
      );

      const result = await acknowledgeAlert("alert-1");
      expect(result.status).toBe("acknowledged");
    });

    it("should throw error when alert not found", async () => {
      server.use(
        http.post("/api/v1/alerts/:id/acknowledge", ({ params }) => {
          const { id } = params;
          if (id === "non-existent") {
            return HttpResponse.json(
              { message: "Alert not found" },
              { status: 404 },
            );
          }
          return HttpResponse.json({});
        }),
      );

      await expect(acknowledgeAlert("non-existent")).rejects.toThrow(
        "Failed to acknowledge alert",
      );
    });
  });

  describe("resolveAlert", () => {
    it("should resolve an alert successfully", async () => {
      const mockAlert: Alert = {
        id: "alert-1",
        ruleId: "rule-1",
        ruleName: "High CPU Usage",
        severity: "critical",
        status: "resolved",
        message: "CPU usage exceeded 90%",
        triggeredAt: "2025-11-30T10:05:00Z",
        resolvedAt: "2025-11-30T10:10:00Z",
        resolvedBy: "user-1",
      };

      server.use(
        http.post("/api/v1/alerts/:id/resolve", async ({ params, request }) => {
          const { id } = params;
          if (id === "non-existent") {
            return HttpResponse.json(
              { message: "Alert not found" },
              { status: 404 },
            );
          }
          const body = await request.json();
          return HttpResponse.json({
            ...mockAlert,
            status: "resolved",
            resolvedAt: "2025-11-30T10:10:00Z",
            resolvedBy: "user-1",
          });
        }),
      );

      const result = await resolveAlert("alert-1");
      expect(result.status).toBe("resolved");
    });
  });

  describe("assignAlert", () => {
    it("should assign an alert to a user", async () => {
      const mockAlert: Alert = {
        id: "alert-1",
        ruleId: "rule-1",
        ruleName: "High CPU Usage",
        severity: "critical",
        status: "open",
        message: "CPU usage exceeded 90%",
        triggeredAt: "2025-11-30T10:05:00Z",
        assignments: [
          {
            userId: "user-1",
            assignedAt: "2025-11-30T10:07:00Z",
            assignedBy: "user-2",
          },
        ],
      };

      server.use(
        http.post("/api/v1/alerts/:id/assign", async ({ params, request }) => {
          const { id } = params;
          if (id === "non-existent") {
            return HttpResponse.json(
              { message: "Alert not found" },
              { status: 404 },
            );
          }
          const body = await request.json();
          return HttpResponse.json({
            ...mockAlert,
            assignments: [
              {
                userId: body.userId,
                assignedAt: "2025-11-30T10:07:00Z",
                assignedBy: "user-2",
              },
            ],
          });
        }),
      );

      const result = await assignAlert("alert-1", "user-1");
      expect(result.assignments).toHaveLength(1);
      expect(result.assignments?.[0].userId).toBe("user-1");
    });
  });

  describe("getAlertStats", () => {
    it("should fetch alert statistics successfully", async () => {
      const mockStats = {
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
      };

      server.use(
        http.get("/api/v1/alerts/stats", () => {
          return HttpResponse.json(mockStats);
        }),
      );

      const result = await getAlertStats();
      expect(result.totalAlerts).toBe(150);
      expect(result.openAlerts).toBe(12);
      expect(result.bySeverity.critical).toBe(15);
      expect(result.topRules).toHaveLength(2);
    });
  });

  describe("testAlertRule", () => {
    it("should test an alert rule successfully", async () => {
      server.use(
        http.post("/api/v1/alerts/rules/:id/test", ({ params }) => {
          const { id } = params;
          if (id === "non-existent") {
            return HttpResponse.json(
              { message: "Rule not found" },
              { status: 404 },
            );
          }
          return HttpResponse.json({
            success: true,
            triggered: true,
            message: "Rule would trigger",
            triggeredAt: "2025-11-30T10:15:00Z",
          });
        }),
      );

      const result = await testAlertRule("rule-1");
      expect(result.success).toBe(true);
      expect(result.triggered).toBe(true);
    });
  });

  describe("toggleAlertRule", () => {
    it("should toggle alert rule enabled status", async () => {
      const mockRule: AlertRule = {
        id: "rule-1",
        name: "Test Rule",
        severity: "high",
        conditions: [],
        actions: [],
        enabled: false,
        createdAt: "2025-11-30T10:00:00Z",
        updatedAt: "2025-11-30T10:15:00Z",
        triggerCount: 5,
      };

      server.use(
        http.post("/api/v1/alerts/rules/:id/toggle", async ({ params, request }) => {
          const { id } = params;
          const body = await request.json();
          if (id === "non-existent") {
            return HttpResponse.json(
              { message: "Rule not found" },
              { status: 404 },
            );
          }
          return HttpResponse.json({
            ...mockRule,
            enabled: body.enabled,
          });
        }),
      );

      const result = await toggleAlertRule("rule-1", false);
      expect(result.enabled).toBe(false);
    });
  });

  describe("getAlertHistory", () => {
    it("should fetch alert history for a rule", async () => {
      const mockHistory: Alert[] = [
        {
          id: "alert-1",
          ruleId: "rule-1",
          ruleName: "High CPU",
          severity: "high",
          status: "resolved",
          message: "CPU high",
          triggeredAt: "2025-11-30T10:00:00Z",
          resolvedAt: "2025-11-30T10:05:00Z",
        },
      ];

      server.use(
        http.get("/api/v1/alerts/rules/:ruleId/history", ({ params }) => {
          const { ruleId } = params;
          if (ruleId === "non-existent") {
            return HttpResponse.json(
              { message: "Rule not found" },
              { status: 404 },
            );
          }
          return HttpResponse.json(mockHistory);
        }),
      );

      const result = await getAlertHistory("rule-1", 100);
      expect(result).toHaveLength(1);
    });
  });

  describe("Utility Functions", () => {
    describe("getSeverityColor", () => {
      it("should return correct color for each severity", () => {
        expect(getSeverityColor("critical")).toBe(
          "text-red-700 bg-red-100 dark:bg-red-900/20",
        );
        expect(getSeverityColor("high")).toBe(
          "text-red-600 bg-red-50 dark:bg-red-900/10",
        );
        expect(getSeverityColor("medium")).toBe(
          "text-yellow-600 bg-yellow-50 dark:bg-yellow-900/10",
        );
        expect(getSeverityColor("low")).toBe(
          "text-blue-600 bg-blue-50 dark:bg-blue-900/10",
        );
        expect(getSeverityColor("info")).toBe(
          "text-gray-600 bg-gray-50 dark:bg-gray-900/10",
        );
      });
    });

    describe("getStatusColor", () => {
      it("should return correct color for each status", () => {
        expect(getStatusColor("open")).toBe(
          "text-red-600 bg-red-50 dark:bg-red-900/10",
        );
        expect(getStatusColor("acknowledged")).toBe(
          "text-yellow-600 bg-yellow-50 dark:bg-yellow-900/10",
        );
        expect(getStatusColor("resolved")).toBe(
          "text-green-600 bg-green-50 dark:bg-green-900/10",
        );
        expect(getStatusColor("suppressed")).toBe(
          "text-gray-600 bg-gray-50 dark:bg-gray-900/10",
        );
      });
    });

    describe("formatDuration", () => {
      it("should format duration correctly", () => {
        expect(formatDuration(30)).toBe("30s");
        expect(formatDuration(90)).toBe("1m 30s");
        expect(formatDuration(3720)).toBe("1h 2m");
      });
    });

    describe("isHighPriority", () => {
      it("should identify high priority severities", () => {
        expect(isHighPriority("critical")).toBe(true);
        expect(isHighPriority("high")).toBe(true);
        expect(isHighPriority("medium")).toBe(false);
        expect(isHighPriority("low")).toBe(false);
        expect(isHighPriority("info")).toBe(false);
      });
    });

    describe("getAlertPriorityScore", () => {
      it("should calculate correct priority scores", () => {
        const criticalOpen: Alert = {
          id: "1",
          ruleId: "rule-1",
          ruleName: "Test",
          severity: "critical",
          status: "open",
          message: "Test",
          triggeredAt: "2025-11-30T10:00:00Z",
        };

        const lowResolved: Alert = {
          id: "2",
          ruleId: "rule-2",
          ruleName: "Test",
          severity: "low",
          status: "resolved",
          message: "Test",
          triggeredAt: "2025-11-30T10:00:00Z",
        };

        expect(getAlertPriorityScore(criticalOpen)).toBeGreaterThan(
          getAlertPriorityScore(lowResolved),
        );
      });
    });
  });
});
