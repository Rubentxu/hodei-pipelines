import { Route, Routes } from "react-router-dom";
import AlertManagement from "./components/AlertManagement";
import { LogsExplorer } from "./components/LogsExplorer";
import { ResourcePoolDetails } from "./components/ResourcePoolDetails";
import { ResourcePoolForm } from "./components/ResourcePoolForm";
import { ResourcePoolList } from "./components/ResourcePoolList";
import { Layout } from "./components/shared/layout";
import LoginPage from "./pages/auth/login-page";
import { DashboardPage } from "./pages/dashboard-page";
import { FinOpsPage } from "./pages/finops/finops-page";
import { ObservabilityPage } from "./pages/observability/observability-page";
import { ExecutionDetailsPage } from "./pages/pipelines/execution-details-page";
import { PipelinesPage } from "./pages/pipelines/pipelines-page";
import { SecurityPage } from "./pages/security/security-page";
import { TerminalPage } from "./pages/terminal/terminal-page";
import { WorkersPage } from "./pages/workers/workers-page";

function App() {
  return (
    <Layout>
      <Routes>
        <Route path="/login" element={<LoginPage />} />
        <Route path="/" element={<DashboardPage />} />
        <Route path="/pipelines" element={<PipelinesPage />} />
        <Route
          path="/pipelines/:pipelineId/executions/:executionId"
          element={<ExecutionDetailsPage />}
        />
        <Route path="/terminal/:jobId" element={<TerminalPage />} />
        <Route path="/workers" element={<WorkersPage />} />
        <Route path="/resource-pools" element={<ResourcePoolList />} />
        <Route path="/resource-pools/new" element={<ResourcePoolForm />} />
        <Route path="/resource-pools/:id" element={<ResourcePoolDetails />} />
        <Route path="/resource-pools/:id/edit" element={<ResourcePoolForm />} />
        <Route path="/logs" element={<LogsExplorer />} />
        <Route path="/alerts" element={<AlertManagement />} />
        <Route path="/finops" element={<FinOpsPage />} />
        <Route path="/security" element={<SecurityPage />} />
        <Route path="/observability" element={<ObservabilityPage />} />
      </Routes>
    </Layout>
  );
}

export default App;
