import { useState } from "react";
import { MetricsDashboard } from "../../components/ui/observability/metrics-dashboard";
import { LogViewer } from "../../components/ui/observability/log-viewer";
import { TraceViewer } from "../../components/ui/observability/trace-viewer";
import { AlertRules } from "../../components/ui/observability/alert-rules";
import { ServiceMap } from "../../components/ui/observability/service-map";
import { useAlerts, useAcknowledgeAlert } from "../../hooks/useObservability";
import { Button } from "@/components/ui/button";
import { StatusBadge } from "@/components/ui/status-badge";

type TabType = "metrics" | "logs" | "traces" | "alerts" | "services";

export function ObservabilityPage() {
  const [activeTab, setActiveTab] = useState<TabType>("metrics");

  const tabs = [
    { id: "metrics", label: "Metrics", icon: "📊" },
    { id: "logs", label: "Logs", icon: "📝" },
    { id: "traces", label: "Traces", icon: "🔍" },
    { id: "alerts", label: "Alerts", icon: "🚨" },
    { id: "services", label: "Services", icon: "🔗" },
  ] as const;

  const renderContent = () => {
    switch (activeTab) {
      case "metrics":
        return <MetricsDashboard />;
      case "logs":
        return <LogViewer />;
      case "traces":
        return <TraceViewer />;
      case "alerts":
        return <AlertsTabContent />;
      case "services":
        return <ServiceMap />;
      default:
        return null;
    }
  };

  return (
    <div className="p-6 space-y-6">
      <div>
        <h1 className="text-3xl font-bold text-gray-100 mb-2">
          Advanced Observability
        </h1>
        <p className="text-gray-400">
          Monitor, analyze, and troubleshoot your distributed systems
        </p>
      </div>

      <div className="border-b border-gray-700">
        <nav className="-mb-px flex space-x-8">
          {tabs.map((tab) => (
            <button
              key={tab.id}
              onClick={() => setActiveTab(tab.id)}
              className={`group inline-flex items-center gap-2 py-4 px-1 border-b-2 font-medium text-sm transition-colors ${
                activeTab === tab.id
                  ? "border-blue-500 text-blue-400"
                  : "border-transparent text-gray-400 hover:text-gray-300 hover:border-gray-300"
              }`}
            >
              <span>{tab.icon}</span>
              {tab.label}
            </button>
          ))}
        </nav>
      </div>

      <div>{renderContent()}</div>
    </div>
  );
}

function AlertsTabContent() {
  const [view, setView] = useState<"rules" | "active">("rules");

  return (
    <div className="space-y-4">
      <div className="flex items-center gap-4">
        <button
          onClick={() => setView("rules")}
          className={`px-4 py-2 rounded-lg font-medium transition-colors ${
            view === "rules"
              ? "bg-blue-600 text-white"
              : "bg-gray-800 text-gray-300 hover:bg-gray-700"
          }`}
        >
          Alert Rules
        </button>
        <button
          onClick={() => setView("active")}
          className={`px-4 py-2 rounded-lg font-medium transition-colors ${
            view === "active"
              ? "bg-blue-600 text-white"
              : "bg-gray-800 text-gray-300 hover:bg-gray-700"
          }`}
        >
          Active Alerts
        </button>
      </div>

      {view === "rules" ? <AlertRules /> : <ActiveAlerts />}
    </div>
  );
}

function ActiveAlerts() {
  const { data, isLoading, fetchNextPage, hasNextPage } = useAlerts({
    acknowledged: false,
  });

  const acknowledgeMutation = useAcknowledgeAlert();

  const allAlerts = data?.pages.flatMap((page) => page.alerts) || [];

  const handleAcknowledge = async (alertId: string) => {
    try {
      await acknowledgeMutation.mutateAsync({
        alertId,
        userId: "current-user-id",
      });
    } catch (error) {
      console.error("Failed to acknowledge alert:", error);
    }
  };

  const getSeverityColor = (severity: string) => {
    switch (severity) {
      case "critical":
        return "text-red-400";
      case "warning":
        return "text-yellow-400";
      case "info":
        return "text-blue-400";
      default:
        return "text-gray-400";
    }
  };

  if (isLoading) {
    return (
      <div className="bg-gray-800 rounded-lg p-6">
        <div className="flex items-center justify-center h-[400px]">
          <div className="text-gray-400">Loading alerts...</div>
        </div>
      </div>
    );
  }

  return (
    <div className="bg-gray-800 rounded-lg p-6">
      <div className="flex items-center justify-between mb-4">
        <h3 className="text-lg font-semibold text-gray-100">Active Alerts</h3>
        <div className="text-sm text-gray-400">
          {allAlerts.length} active alerts
        </div>
      </div>

      {allAlerts.length === 0 ? (
        <div className="flex items-center justify-center h-[400px]">
          <div className="text-center">
            <div className="text-4xl mb-4">✅</div>
            <div className="text-gray-400 mb-2">No active alerts</div>
            <div className="text-sm text-gray-500">
              All systems are running smoothly
            </div>
          </div>
        </div>
      ) : (
        <div className="space-y-3">
          {allAlerts.map((alert) => (
            <div
              key={alert.id}
              className="bg-gray-750 rounded-lg p-4 hover:bg-gray-700 transition-colors"
            >
              <div className="flex items-start justify-between mb-2">
                <div className="flex items-center gap-3">
                  <StatusBadge
                    status={
                      alert.severity === "critical"
                        ? "error"
                        : alert.severity === "warning"
                          ? "warning"
                          : "info"
                    }
                  />
                  <div>
                    <h4 className="text-gray-100 font-medium">
                      {alert.ruleName}
                    </h4>
                    <p className="text-sm text-gray-400">{alert.message}</p>
                  </div>
                </div>
                <Button
                  onClick={() => handleAcknowledge(alert.id)}
                  variant="outline"
                  size="sm"
                  disabled={acknowledgeMutation.isPending}
                >
                  Acknowledge
                </Button>
              </div>

              <div className="flex items-center gap-4 text-xs text-gray-400">
                <span className={getSeverityColor(alert.severity)}>
                  {alert.severity.toUpperCase()}
                </span>
                <span>•</span>
                <span>{new Date(alert.timestamp).toLocaleString()}</span>
              </div>
            </div>
          ))}

          {hasNextPage && (
            <div className="flex justify-center py-4">
              <Button onClick={() => fetchNextPage()} variant="outline">
                Load More
              </Button>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
