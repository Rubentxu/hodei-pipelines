import { Routes, Route } from "react-router-dom";
import { Layout } from "./components/shared/layout";
import { DashboardPage } from "./pages/dashboard-page";
import { PipelinesPage } from "./pages/pipelines/pipelines-page";
import { TerminalPage } from "./pages/terminal/terminal-page";
import { WorkersPage } from "./pages/workers/workers-page";
import { FinOpsPage } from "./pages/finops/finops-page";
import { SecurityPage } from "./pages/security/security-page";
import { ObservabilityPage } from "./pages/observability/observability-page";

function App() {
  return (
    <Layout>
      <Routes>
        <Route path="/" element={<DashboardPage />} />
        <Route path="/pipelines" element={<PipelinesPage />} />
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
