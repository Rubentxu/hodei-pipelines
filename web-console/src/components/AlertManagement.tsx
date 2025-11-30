import { useCallback, useEffect, useState } from "react";
import {
  acknowledgeAlert,
  Alert,
  AlertRule,
  AlertSeverity,
  AlertStatus,
  assignAlert,
  getAlertPriorityScore,
  getAlertRules,
  getAlerts,
  getAlertStats,
  getSeverityColor,
  getStatusColor,
  resolveAlert,
} from "../services/alertManagementApi";

interface TabType {
  id: string;
  name: string;
  count?: number;
}

export default function AlertManagement() {
  const [activeTab, setActiveTab] = useState("alerts");
  const [alerts, setAlerts] = useState<Alert[]>([]);
  const [alertRules, setAlertRules] = useState<AlertRule[]>([]);
  const [stats, setStats] = useState<any>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Filters
  const [alertStatusFilter, setAlertStatusFilter] = useState<AlertStatus[]>([
    "open",
  ]);
  const [severityFilter, setSeverityFilter] = useState<AlertSeverity[]>([]);
  const [searchQuery, setSearchQuery] = useState("");

  // Tabs configuration
  const tabs: TabType[] = [
    { id: "alerts", name: "Active Alerts", count: stats?.openAlerts },
    { id: "all", name: "All Alerts", count: stats?.totalAlerts },
    { id: "rules", name: "Alert Rules", count: alertRules.length },
    { id: "stats", name: "Statistics" },
  ];

  // Fetch data
  const fetchAlerts = useCallback(async () => {
    try {
      const result = await getAlerts({
        status: alertStatusFilter.length > 0 ? alertStatusFilter : undefined,
        severity: severityFilter.length > 0 ? severityFilter : undefined,
        search: searchQuery || undefined,
        limit: 100,
      });
      setAlerts(result.alerts);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to fetch alerts");
    }
  }, [alertStatusFilter, severityFilter, searchQuery]);

  const fetchAlertRules = useCallback(async () => {
    try {
      const rules = await getAlertRules({ limit: 100 });
      setAlertRules(rules);
    } catch (err) {
      setError(
        err instanceof Error ? err.message : "Failed to fetch alert rules",
      );
    }
  }, []);

  const fetchStats = useCallback(async () => {
    try {
      const statsData = await getAlertStats();
      setStats(statsData);
    } catch (err) {
      setError(
        err instanceof Error ? err.message : "Failed to fetch statistics",
      );
    }
  }, []);

  useEffect(() => {
    const loadData = async () => {
      setLoading(true);
      setError(null);
      await Promise.all([fetchAlerts(), fetchAlertRules(), fetchStats()]);
      setLoading(false);
    };

    loadData();
  }, [fetchAlerts, fetchAlertRules, fetchStats]);

  // Auto-refresh every 30 seconds
  useEffect(() => {
    const interval = setInterval(() => {
      fetchAlerts();
      fetchStats();
    }, 30000);

    return () => clearInterval(interval);
  }, [fetchAlerts, fetchStats]);

  // Handle acknowledge alert
  const handleAcknowledge = async (alertId: string) => {
    try {
      await acknowledgeAlert(alertId);
      await fetchAlerts();
      await fetchStats();
    } catch (err) {
      setError(
        err instanceof Error ? err.message : "Failed to acknowledge alert",
      );
    }
  };

  // Handle resolve alert
  const handleResolve = async (alertId: string) => {
    try {
      await resolveAlert(alertId);
      await fetchAlerts();
      await fetchStats();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to resolve alert");
    }
  };

  // Handle assign alert
  const handleAssign = async (alertId: string, userId: string) => {
    try {
      await assignAlert(alertId, userId);
      await fetchAlerts();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to assign alert");
    }
  };

  // Render statistics dashboard
  const renderStats = () => {
    if (!stats) return null;

    return (
      <div className="space-y-6">
        {/* Stats Grid */}
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
          <div className="bg-card rounded-lg border border-border p-6">
            <div className="text-sm text-muted-foreground">Total Alerts</div>
            <div className="text-3xl font-bold mt-2">{stats.totalAlerts}</div>
          </div>
          <div className="bg-card rounded-lg border border-border p-6">
            <div className="text-sm text-muted-foreground">Open</div>
            <div className="text-3xl font-bold mt-2 text-red-600">
              {stats.openAlerts}
            </div>
          </div>
          <div className="bg-card rounded-lg border border-border p-6">
            <div className="text-sm text-muted-foreground">Acknowledged</div>
            <div className="text-3xl font-bold mt-2 text-yellow-600">
              {stats.acknowledgedAlerts}
            </div>
          </div>
          <div className="bg-card rounded-lg border border-border p-6">
            <div className="text-sm text-muted-foreground">Resolved</div>
            <div className="text-3xl font-bold mt-2 text-green-600">
              {stats.resolvedAlerts}
            </div>
          </div>
        </div>

        {/* Severity Breakdown */}
        <div className="bg-card rounded-lg border border-border p-6">
          <h3 className="text-lg font-semibold mb-4">Alerts by Severity</h3>
          <div className="space-y-3">
            {Object.entries(stats.bySeverity).map(([severity, count]) => (
              <div key={severity} className="flex items-center justify-between">
                <span
                  className={`px-3 py-1 rounded-full text-sm font-medium ${getSeverityColor(
                    severity as AlertSeverity,
                  )}`}
                >
                  {severity}
                </span>
                <span className="font-semibold">{count as number}</span>
              </div>
            ))}
          </div>
        </div>

        {/* Top Triggered Rules */}
        <div className="bg-card rounded-lg border border-border p-6">
          <h3 className="text-lg font-semibold mb-4">
            Most Frequently Triggered Rules
          </h3>
          <div className="space-y-3">
            {stats.topRules?.map((rule: any) => (
              <div
                key={rule.ruleId}
                className="flex items-center justify-between"
              >
                <div>
                  <div className="font-medium">{rule.ruleName}</div>
                  <div className="text-sm text-muted-foreground">
                    ID: {rule.ruleId}
                  </div>
                </div>
                <span className="px-3 py-1 bg-primary/10 text-primary rounded-full text-sm font-medium">
                  {rule.count} triggers
                </span>
              </div>
            )) || <div className="text-muted-foreground">No data</div>}
          </div>
        </div>
      </div>
    );
  };

  // Render alerts list
  const renderAlerts = () => {
    // Sort alerts by priority
    const sortedAlerts = [...alerts].sort(
      (a, b) => getAlertPriorityScore(b) - getAlertPriorityScore(a),
    );

    return (
      <div className="space-y-4">
        {/* Filters */}
        <div className="bg-card rounded-lg border border-border p-4">
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
            <div>
              <label htmlFor="search-alerts" className="block text-sm font-medium mb-2">
                Search
              </label>
              <input
                id="search-alerts"
                type="text"
                placeholder="Search alerts..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className="w-full px-3 py-2 border border-border rounded-lg bg-background"
              />
            </div>
            <div>
              <label htmlFor="status-filter" className="block text-sm font-medium mb-2">
                Status
              </label>
              <select
                id="status-filter"
                multiple
                value={alertStatusFilter}
                onChange={(e) =>
                  setAlertStatusFilter(
                    Array.from(e.target.selectedOptions, (o) => o.value) as AlertStatus[],
                  )
                }
                className="w-full px-3 py-2 border border-border rounded-lg bg-background"
              >
                <option value="open">Open</option>
                <option value="acknowledged">Acknowledged</option>
                <option value="resolved">Resolved</option>
                <option value="suppressed">Suppressed</option>
              </select>
            </div>
            <div>
              <label htmlFor="severity-filter" className="block text-sm font-medium mb-2">
                Severity
              </label>
              <select
                id="severity-filter"
                multiple
                value={severityFilter}
                onChange={(e) =>
                  setSeverityFilter(
                    Array.from(e.target.selectedOptions, (o) => o.value) as AlertSeverity[],
                  )
                }
                className="w-full px-3 py-2 border border-border rounded-lg bg-background"
              >
                <option value="critical">Critical</option>
                <option value="high">High</option>
                <option value="medium">Medium</option>
                <option value="low">Low</option>
                <option value="info">Info</option>
              </select>
            </div>
          </div>
        </div>

        {/* Alerts List */}
        <div className="space-y-3">
          {sortedAlerts.map((alert) => (
            <div
              key={alert.id}
              className="bg-card rounded-lg border border-border p-4 hover:border-primary/50 transition-colors"
            >
              <div className="flex items-start justify-between">
                <div className="flex-1">
                  <div className="flex items-center gap-2 mb-2">
                    <span
                      className={`px-2 py-1 rounded-full text-xs font-medium ${getSeverityColor(
                        alert.severity,
                      )}`}
                    >
                      {alert.severity.toUpperCase()}
                    </span>
                    <span
                      className={`px-2 py-1 rounded-full text-xs font-medium ${getStatusColor(
                        alert.status,
                      )}`}
                    >
                      {alert.status}
                    </span>
                    <span className="text-sm text-muted-foreground">
                      {new Date(alert.triggeredAt).toLocaleString()}
                    </span>
                  </div>
                  <h4 className="text-lg font-semibold mb-1">
                    {alert.ruleName}
                  </h4>
                  <p className="text-muted-foreground mb-2">{alert.message}</p>
                  {alert.assignments && alert.assignments.length > 0 && (
                    <div className="text-sm text-muted-foreground">
                      Assigned to:{" "}
                      {alert.assignments
                        .map((a) => a.userId)
                        .join(", ")}
                    </div>
                  )}
                </div>
                <div className="flex gap-2 ml-4">
                  {alert.status === "open" && (
                    <button
                      onClick={() => handleAcknowledge(alert.id)}
                      className="px-3 py-1 bg-yellow-600 text-white rounded-lg hover:bg-yellow-700 text-sm"
                    >
                      Acknowledge
                    </button>
                  )}
                  {(alert.status === "open" ||
                    alert.status === "acknowledged") && (
                      <button
                        onClick={() => handleResolve(alert.id)}
                        className="px-3 py-1 bg-green-600 text-white rounded-lg hover:bg-green-700 text-sm"
                      >
                        Resolve
                      </button>
                    )}
                </div>
              </div>
            </div>
          ))}

          {alerts.length === 0 && (
            <div className="text-center py-12 text-muted-foreground">
              No alerts found matching your criteria
            </div>
          )}
        </div>
      </div>
    );
  };

  // Render alert rules
  const renderAlertRules = () => {
    return (
      <div className="space-y-4">
        <div className="flex justify-between items-center">
          <h3 className="text-lg font-semibold">Alert Rules</h3>
          <button className="px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90">
            Create Rule
          </button>
        </div>

        <div className="space-y-3">
          {alertRules.map((rule) => (
            <div
              key={rule.id}
              className="bg-card rounded-lg border border-border p-4 hover:border-primary/50 transition-colors"
            >
              <div className="flex items-start justify-between">
                <div className="flex-1">
                  <div className="flex items-center gap-2 mb-2">
                    <h4 className="text-lg font-semibold">{rule.name}</h4>
                    {rule.enabled ? (
                      <span className="px-2 py-1 bg-green-100 text-green-800 dark:bg-green-900/20 dark:text-green-400 rounded-full text-xs font-medium">
                        Enabled
                      </span>
                    ) : (
                      <span className="px-2 py-1 bg-gray-100 text-gray-800 dark:bg-gray-900/20 dark:text-gray-400 rounded-full text-xs font-medium">
                        Disabled
                      </span>
                    )}
                    <span
                      className={`px-2 py-1 rounded-full text-xs font-medium ${getSeverityColor(
                        rule.severity,
                      )}`}
                    >
                      {rule.severity}
                    </span>
                  </div>
                  {rule.description && (
                    <p className="text-muted-foreground mb-2">
                      {rule.description}
                    </p>
                  )}
                  <div className="text-sm text-muted-foreground">
                    {rule.conditions.length} condition(s) • {rule.actions.length} action(s)
                    {" • "}
                    Triggered {rule.triggerCount} time(s)
                    {rule.lastTriggeredAt && (
                      <>
                        {" • Last triggered: "}
                        {new Date(rule.lastTriggeredAt).toLocaleString()}
                      </>
                    )}
                  </div>
                </div>
                <div className="flex gap-2 ml-4">
                  <button className="px-3 py-1 border border-border rounded-lg hover:bg-accent text-sm">
                    Edit
                  </button>
                  <button className="px-3 py-1 border border-border rounded-lg hover:bg-accent text-sm">
                    Test
                  </button>
                  <button className="px-3 py-1 text-red-600 hover:bg-red-50 dark:hover:bg-red-900/10 text-sm">
                    Delete
                  </button>
                </div>
              </div>
            </div>
          ))}

          {alertRules.length === 0 && (
            <div className="text-center py-12 text-muted-foreground">
              No alert rules configured
            </div>
          )}
        </div>
      </div>
    );
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center min-h-[400px]">
        <div className="text-muted-foreground">Loading alerts...</div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="bg-destructive/10 text-destructive p-4 rounded-lg">
        Error: {error}
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div>
        <h1 className="text-3xl font-bold">Alert Management</h1>
        <p className="text-muted-foreground mt-1">
          Monitor and manage system alerts and notification rules
        </p>
      </div>

      {/* Tabs */}
      <div className="border-b border-border">
        <nav className="flex gap-4">
          {tabs.map((tab) => (
            <button
              key={tab.id}
              onClick={() => setActiveTab(tab.id)}
              className={`px-4 py-2 border-b-2 font-medium text-sm transition-colors ${activeTab === tab.id
                  ? "border-primary text-primary"
                  : "border-transparent text-muted-foreground hover:text-foreground"
                }`}
            >
              {tab.name}
              {tab.count !== undefined && (
                <span className="ml-2 px-2 py-0.5 bg-primary/10 text-primary rounded-full text-xs">
                  {tab.count}
                </span>
              )}
            </button>
          ))}
        </nav>
      </div>

      {/* Content */}
      <div>
        {activeTab === "alerts" && renderAlerts()}
        {activeTab === "all" && renderAlerts()}
        {activeTab === "rules" && renderAlertRules()}
        {activeTab === "stats" && renderStats()}
      </div>
    </div>
  );
}
