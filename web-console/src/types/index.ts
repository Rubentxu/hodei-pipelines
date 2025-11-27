export interface KPIMetrics {
  jobsInQueue: number;
  jobsRunning: number;
  jobsSucceeded: number;
  jobsFailed: number;
  activeWorkers: number;
  cpuUsage: number;
  memoryUsage: number;
  avgExecutionTime: number;
}

export interface PipelineTask {
  id: string;
  name: string;
  type: "extract" | "transform" | "load" | "validate" | "notify";
  status: "pending" | "running" | "completed" | "failed";
  position: number;
  config?: Record<string, any>;
  dependencies?: string[];
  timeout?: number;
  retryPolicy?: {
    maxRetries: number;
    backoff: number;
  };
}

export interface Pipeline {
  id: string;
  name: string;
  status: "active" | "paused" | "error" | "inactive";
  lastRun: string;
  tags: string[];
  description?: string;
  environment?: string;
  tasks?: PipelineTask[];
  createdAt?: string;
  updatedAt?: string;
  schedule?: {
    cron: string;
    timezone: string;
  };
  triggers?: string[];
}

export interface Execution {
  id: string;
  status: "success" | "error" | "running" | "pending";
  duration: number;
  cost: number;
  startedAt: string;
  trigger: string;
}

export interface Worker {
  id: string;
  name: string;
  status: "healthy" | "failed" | "maintenance";
  currentJobs: number;
  maxJobs: number;
  poolId: string;
}

export interface WorkerPool {
  id: string;
  name: string;
  type: "static" | "dynamic";
  minWorkers: number;
  maxWorkers: number;
  currentWorkers: number;
  scalingPolicy: {
    cpuThreshold: number;
    memoryThreshold: number;
    queueDepthThreshold: number;
  };
}

export interface CostMetrics {
  dailyCost: number;
  monthlyCost: number;
  yearlyCost: number;
  projectedMonthlyCost: number;
  budgetLimit: number;
  budgetUsed: number;
  costPerJob: number;
  costPerHour: number;
  trend: "up" | "down" | "stable";
  trendPercentage: number;
}

export interface CostBreakdown {
  pipelineId: string;
  pipelineName: string;
  cost: number;
  percentage: number;
  jobCount: number;
  avgDuration: number;
}

export interface CostHistory {
  date: string;
  cost: number;
  jobCount: number;
}

export interface BudgetAlert {
  id: string;
  type: "warning" | "critical";
  threshold: number;
  current: number;
  message: string;
  date: string;
}

export interface FinOpsMetrics {
  metrics: CostMetrics;
  breakdown: CostBreakdown[];
  history: CostHistory[];
  alerts: BudgetAlert[];
  lastUpdated: string;
}

export interface User {
  id: string;
  name: string;
  email: string;
  role: Role;
  status: "active" | "inactive" | "pending";
  lastLogin?: string;
  createdAt: string;
  mfaEnabled: boolean;
  groups: Group[];
}

export interface Role {
  id: string;
  name: string;
  description: string;
  permissions: Permission[];
  isSystem: boolean;
  userCount?: number;
}

export interface Permission {
  id: string;
  resource: string;
  action: "read" | "write" | "delete" | "admin";
  scope: "global" | "resource" | "own";
}

export interface Group {
  id: string;
  name: string;
  description: string;
  userCount: number;
  permissions: Permission[];
}

export interface AuditLog {
  id: string;
  userId: string;
  userName: string;
  action: string;
  resource: string;
  resourceId?: string;
  status: "success" | "failure";
  ipAddress: string;
  userAgent: string;
  timestamp: string;
  details?: Record<string, any>;
}

export interface SecurityMetrics {
  activeUsers: number;
  failedLogins: number;
  pendingApprovals: number;
  securityScore: number;
  vulnerabilitiesCritical: number;
  vulnerabilitiesHigh: number;
  vulnerabilitiesMedium: number;
  vulnerabilitiesLow: number;
}

export interface SecurityMetricsResponse {
  metrics: SecurityMetrics;
  recentLogs: AuditLog[];
  lastUpdated: string;
}

export interface LogEntry {
  id: string;
  timestamp: string;
  level: "debug" | "info" | "warn" | "error" | "fatal";
  service: string;
  message: string;
  traceId?: string;
  spanId?: string;
  userId?: string;
  metadata?: Record<string, any>;
}

export interface Trace {
  id: string;
  name: string;
  duration: number;
  startTime: string;
  endTime: string;
  service: string;
  spans: Span[];
  status: "success" | "error" | "timeout";
}

export interface Span {
  id: string;
  name: string;
  duration: number;
  startTime: string;
  service: string;
  tags?: Record<string, any>;
}

export interface ObservabilityMetrics {
  cpuUsage: number;
  memoryUsage: number;
  diskUsage: number;
  networkIn: number;
  networkOut: number;
  requestRate: number;
  errorRate: number;
  avgResponseTime: number;
}

export interface ObservabilityMetricsResponse {
  metrics: ObservabilityMetrics;
  history: {
    timestamp: string;
    cpuUsage: number;
    memoryUsage: number;
    requestRate: number;
    errorRate: number;
  }[];
  lastUpdated: string;
}

export interface AlertRule {
  id: string;
  name: string;
  description: string;
  condition: string;
  severity: "info" | "warning" | "critical";
  status: "active" | "inactive";
  enabled: boolean;
  lastTriggered?: string;
  createdAt: string;
}

export interface Alert {
  id: string;
  ruleId: string;
  ruleName: string;
  message: string;
  severity: "info" | "warning" | "critical";
  timestamp: string;
  acknowledged: boolean;
  acknowledgedBy?: string;
  acknowledgedAt?: string;
}
