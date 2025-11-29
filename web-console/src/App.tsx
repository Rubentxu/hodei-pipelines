import { Route, Routes } from "react-router-dom";
import { Layout } from "./components/shared/layout";
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
        <Route path="/" element={<DashboardPage />} />
        <Route path="/pipelines" element={<PipelinesPage />} />
        <Route path="/pipelines/:pipelineId/executions/:executionId" element={<ExecutionDetailsPage />} />
        <Route path="/terminal/:jobId" element={<TerminalPage />} />
        <Route path="/workers" element={<WorkersPage />} />
        <Route path="/finops" element={<FinOpsPage />} />
        <Route path="/security" element={<SecurityPage />} />
        <Route path="/observability" element={<ObservabilityPage />} />
      </Routes>
    </Layout>
  );
}

export default App;
