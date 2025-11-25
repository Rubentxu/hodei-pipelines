# EPIC-09: Resource Pool Integration & API Enhancement

## 📋 Epic Overview

This document defines EPIC-09, which focuses on integrating the comprehensive Resource Pool Architecture (EPIC-08) into the Hodei Jobs Server API. This epic will expose all new capabilities through REST API endpoints, gRPC services, and monitoring integrations.

**Epic Duration**: 10 weeks
**Total User Stories**: 22 stories
**Sub-EPICs**: 6 sub-epics with 3-4 stories each

---

## 🎯 Business Objectives

1. **Expose Multi-Tenancy**: Enable tenant isolation and quota management via API
2. **Fair Scheduling**: Implement weighted fair queuing in job scheduling
3. **Burst Capacity**: Support temporary quota exceedance with proper governance
4. **Cost Optimization**: Provide cost visibility and optimization recommendations
5. **Observability**: Full monitoring stack with Grafana dashboards and Prometheus alerts
6. **Dynamic Scaling**: Auto-scaling policies configurable via API

---

## 📚 User Stories by Sub-EPIC

### Sub-EPIC 09.1: Multi-Tenancy API Layer
**Goal**: Expose multi-tenancy and quota management through REST endpoints
**Priority**: High
**Dependencies**: US-08.7.x (Resource Pool Architecture)

| Story ID | Title | Effort | Priority | API Endpoint |
|----------|-------|--------|----------|--------------|
| US-09.1.1 | [Tenant Management API](./US-09.1.1-Tenant-Management-API.md) | 3 days | High | `/api/v1/tenants/*` |
| US-09.1.2 | [Resource Quotas API](./US-09.1.2-Resource-Quotas-API.md) | 3 days | High | `/api/v1/tenants/{id}/quotas` |
| US-09.1.3 | [Quota Enforcement API](./US-09.1.3-Quota-Enforcement-API.md) | 2 days | High | `/api/v1/quotas/enforce` |
| US-09.1.4 | [Burst Capacity API](./US-09.1.4-Burst-Capacity-API.md) | 2 days | Medium | `/api/v1/tenants/{id}/burst` |

### Sub-EPIC 09.2: Job Scheduling & Fair Queuing
**Goal**: Integrate weighted fair queuing into job scheduling pipeline
**Priority**: High
**Dependencies**: US-09.1.4 (Burst Capacity API)

| Story ID | Title | Effort | Priority | Integration Point |
|----------|-------|--------|----------|-------------------|
| US-09.2.1 | [Job Queue Prioritization](./US-09.2.1-Job-Queue-Prioritization.md) | 4 days | High | SchedulerModule |
| US-09.2.2 | [WFQ Integration](./US-09.2.2-WFQ-Integration.md) | 3 days | High | QueueAssignmentEngine |
| US-09.2.3 | [SLA Tracking API](./US-09.2.3-SLA-Tracking-API.md) | 2 days | High | `/api/v1/jobs/{id}/sla` |
| US-09.2.4 | [Queue Status API](./US-09.2.4-Queue-Status-API.md) | 2 days | Medium | `/api/v1/queues/status` |

### Sub-EPIC 09.3: Resource Pool Management
**Goal**: Full lifecycle management of resource pools via API
**Priority**: High
**Dependencies**: US-09.2.4 (Queue Status API)

| Story ID | Title | Effort | Priority | API Endpoint |
|----------|-------|--------|----------|--------------|
| US-09.3.1 | [Resource Pool CRUD](./US-09.3.1-Resource-Pool-CRUD.md) | 4 days | High | `/api/v1/pools/*` |
| US-09.3.2 | [Static Pool Management](./US-09.3.2-Static-Pool-Management.md) | 3 days | High | `/api/v1/pools/{id}/static` |
| US-09.3.3 | [Dynamic Pool Management](./US-09.3.3-Dynamic-Pool-Management.md) | 3 days | High | `/api/v1/pools/{id}/dynamic` |
| US-09.3.4 | [Pool Lifecycle API](./US-09.3.4-Pool-Lifecycle-API.md) | 2 days | Medium | `/api/v1/pools/{id}/lifecycle` |

### Sub-EPIC 09.4: Auto-Scaling & Policies
**Goal**: Configure and monitor auto-scaling policies
**Priority**: High
**Dependencies**: US-09.3.4 (Pool Lifecycle API)

| Story ID | Title | Effort | Priority | API Endpoint |
|----------|-------|--------|----------|--------------|
| US-09.4.1 | [Scaling Policies API](./US-09.4.1-Scaling-Policies-API.md) | 4 days | High | `/api/v1/pools/{id}/scaling/policies` |
| US-09.4.2 | [Scaling Triggers API](./US-09.4.2-Scaling-Triggers-API.md) | 3 days | High | `/api/v1/pools/{id}/scaling/triggers` |
| US-09.4.3 | [Cooldown Management](./US-09.4.3-Cooldown-Management.md) | 2 days | Medium | `/api/v1/pools/{id}/scaling/cooldown` |
| US-09.4.4 | [Scaling History API](./US-09.4.4-Scaling-History-API.md) | 2 days | Medium | `/api/v1/pools/{id}/scaling/history` |

### Sub-EPIC 09.5: Metrics & Cost Optimization
**Goal**: Expose metrics, costs, and optimization recommendations
**Priority**: Medium
**Dependencies**: US-09.4.4 (Scaling History API)

| Story ID | Title | Effort | Priority | API Endpoint |
|----------|-------|--------|----------|--------------|
| US-09.5.1 | [Resource Pool Metrics API](./US-09.5.1-Resource-Pool-Metrics-API.md) | 3 days | Medium | `/api/v1/pools/{id}/metrics` |
| US-09.5.2 | [Cost Optimization API](./US-09.5.2-Cost-Optimization-API.md) | 4 days | Medium | `/api/v1/cost-optimization/*` |
| US-09.5.3 | [Cost Reports API](./US-09.5.3-Cost-Reports-API.md) | 3 days | Medium | `/api/v1/cost-reports/*` |
| US-09.5.4 | [Historical Metrics API](./US-09.5.4-Historical-Metrics-API.md) | 2 days | Medium | `/api/v1/metrics/historical` |

### Sub-EPIC 09.6: Monitoring & Observability
**Goal**: Deploy Grafana dashboards and Prometheus alerting
**Priority**: Medium
**Dependencies**: US-09.5.4 (Historical Metrics API)

| Story ID | Title | Effort | Priority | Integration |
|----------|-------|--------|----------|-------------|
| US-09.6.1 | [Prometheus Integration](./US-09.6.1-Prometheus-Integration.md) | 3 days | Medium | MetricsExporter |
| US-09.6.2 | [Grafana Dashboards](./US-09.6.2-Grafana-Dashboards.md) | 3 days | Medium | DashboardConfig |
| US-09.6.3 | [Alerting Rules](./US-09.6.3-Alerting-Rules.md) | 2 days | Medium | AlertConfig |
| US-09.6.4 | [Observability API](./US-09.6.4-Observability-API.md) | 2 days | Low | `/api/v1/observability/*` |

---

## 🚀 Implementation Order

### Phase 1: Foundation (Week 1-2)
- [US-09.1.1] Tenant Management API
- [US-09.1.2] Resource Quotas API
- [US-09.1.3] Quota Enforcement API
- [US-09.1.4] Burst Capacity API

### Phase 2: Scheduling Integration (Week 3-4)
- [US-09.2.1] Job Queue Prioritization
- [US-09.2.2] WFQ Integration
- [US-09.2.3] SLA Tracking API
- [US-09.2.4] Queue Status API

### Phase 3: Resource Pools (Week 5-6)
- [US-09.3.1] Resource Pool CRUD
- [US-09.3.2] Static Pool Management
- [US-09.3.3] Dynamic Pool Management
- [US-09.3.4] Pool Lifecycle API

### Phase 4: Auto-Scaling (Week 7-8)
- [US-09.4.1] Scaling Policies API
- [US-09.4.2] Scaling Triggers API
- [US-09.4.3] Cooldown Management
- [US-09.4.4] Scaling History API

### Phase 5: Metrics & Costs (Week 9)
- [US-09.5.1] Resource Pool Metrics API
- [US-09.5.2] Cost Optimization API
- [US-09.5.3] Cost Reports API
- [US-09.5.4] Historical Metrics API

### Phase 6: Observability (Week 10)
- [US-09.6.1] Prometheus Integration
- [US-09.6.2] Grafana Dashboards
- [US-09.6.3] Alerting Rules
- [US-09.6.4] Observability API

---

## 🔌 API Integration Points

### REST API Enhancements

#### New Endpoint Categories:
- `/api/v1/tenants/*` - Multi-tenancy management
- `/api/v1/pools/*` - Resource pool management
- `/api/v1/pools/{id}/scaling/*` - Auto-scaling configuration
- `/api/v1/cost-optimization/*` - Cost analysis
- `/api/v1/cost-reports/*` - Cost reporting
- `/api/v1/metrics/*` - Metrics and monitoring
- `/api/v1/observability/*` - Observability configuration

### gRPC Service Extensions

#### New gRPC Methods:
```protobuf
service ResourcePoolService {
  rpc CreatePool(CreatePoolRequest) returns (Pool);
  rpc ListPools(ListPoolsRequest) returns (ListPoolsResponse);
  rpc GetPoolMetrics(GetPoolMetricsRequest) returns (PoolMetrics);
  rpc ConfigureScalingPolicy(ConfigureScalingPolicyRequest) returns (Policy);
}

service TenantService {
  rpc CreateTenant(CreateTenantRequest) returns (Tenant);
  rpc SetQuota(SetQuotaRequest) returns (Quota);
  rpc RequestBurstCapacity(RequestBurstRequest) returns (BurstSession);
  rpc GetTenantMetrics(GetTenantMetricsRequest) returns (TenantMetrics);
}

service CostOptimizationService {
  rpc GenerateReport(GenerateReportRequest) returns (CostReport);
  rpc GetCostBreakdown(GetCostBreakdownRequest) returns (CostBreakdown);
  rpc GetOptimizationRecommendations(GetRecommendationsRequest) returns (Recommendations);
}
```

### Module Integration

#### Updated Modules:
1. **SchedulerModule**: Integrate WFQ engine
2. **WorkerManagementService**: Support pool-based management
3. **New: TenantManagementService**: Multi-tenancy handling
4. **New: ResourcePoolService**: Pool lifecycle management
5. **New: CostOptimizationService**: Cost analysis
6. **New: ObservabilityService**: Metrics and monitoring

---

## 📊 Configuration Changes

### Server Configuration (config.yaml)

```yaml
server:
  port: 8080
  grpc_port: 9090

resource_pools:
  enabled: true
  default_pool_type: static
  pools:
    - id: "pool-default"
      type: "static"
      min_size: 2
      max_size: 20
      auto_scaling_enabled: true
    
multi_tenancy:
  enabled: true
  default_tenant_quota:
    cpu_m: 4000
    memory_mb: 8192
    max_concurrent_jobs: 10
  
burst_capacity:
  enabled: true
  max_overage_percentage: 150
  burst_duration_minutes: 60

cost_tracking:
  enabled: true
  track_by_tenant: true
  track_by_job: true
  retention_days: 90

metrics:
  prometheus:
    enabled: true
    port: 9091
    path: "/metrics"
  
observability:
  grafana:
    enabled: true
    dashboards_dir: "/etc/hodei/dashboards"
  
  alerting:
    enabled: true
    prometheus_alerts_dir: "/etc/hodei/alerts"
```

### Environment Variables

```bash
# Resource Pools
HODEI_RESOURCE_POOLS_ENABLED=true
HODEI_RESOURCE_POOL_TYPE=static
HODEI_AUTO_SCALING_ENABLED=true

# Multi-Tenancy
HODEI_MULTI_TENANCY_ENABLED=true
HODEI_DEFAULT_TENANT_QUOTA_CPU=4000
HODEI_DEFAULT_TENANT_QUOTA_MEMORY=8192

# Burst Capacity
HODEI_BURST_CAPACITY_ENABLED=true
HODEI_BURST_MAX_OVERAGE=150
HODEI_BURST_DURATION_MINUTES=60

# Cost Tracking
HODEI_COST_TRACKING_ENABLED=true
HODEI_COST_RETENTION_DAYS=90

# Metrics & Monitoring
HODEI_PROMETHEUS_ENABLED=true
HODEI_PROMETHEUS_PORT=9091
HODEI_GRAFANA_ENABLED=true
HODEI_ALERTING_ENABLED=true
```

---

## 🔧 Server Module Updates

### Dependencies (server/Cargo.toml)

Add new dependencies:
```toml
[dependencies]
# Existing...
hodei-modules = { workspace = true }

# New dependencies for resource pools
serde_yaml = "0.9"
reqwest = { version = "0.11", features = ["json"] }
criterion = { version = "0.5", features = ["html_reports"] }
```

### New Server Modules

```rust
// src/tenant_management.rs
pub struct TenantManagementService {
    quota_manager: Arc<MultiTenancyQuotaManager>,
    burst_manager: Arc<BurstCapacityManager>,
}

// src/resource_pool_management.rs
pub struct ResourcePoolManagementService {
    pools: Arc<RwLock<HashMap<PoolId, Box<dyn ResourcePool>>>>,
    lifecycle_manager: Arc<PoolLifecycleManager>,
    metrics_collector: Arc<ResourcePoolMetricsCollector>,
}

// src/cost_optimization.rs
pub struct CostOptimizationService {
    engine: Arc<CostOptimizationEngine>,
    reports_store: Arc<InMemoryReportsStore>,
}

// src/observability.rs
pub struct ObservabilityService {
    metrics_registry: Arc<Registry>,
    grafana_client: Arc<GrafanaClient>,
    prometheus_exporter: Arc<PrometheusExporter>,
}
```

---

## 📈 Success Criteria

### Functional Requirements
- [ ] All resource pool features accessible via REST API
- [ ] gRPC services implement all pool operations
- [ ] Multi-tenancy enforcement at API level
- [ ] Cost tracking visible in real-time
- [ ] Grafana dashboards operational
- [ ] Prometheus alerts configured

### Non-Functional Requirements
- [ ] API response time < 200ms for 95th percentile
- [ ] Support 10,000 concurrent jobs
- [ ] Cost tracking accuracy > 99%
- [ ] Dashboard load time < 2 seconds
- [ ] Alert latency < 30 seconds

### Testing Requirements
- [ ] 100% API endpoint coverage
- [ ] Integration tests for each sub-EPIC
- [ ] Load testing for 10K concurrent jobs
- [ ] Cost calculation accuracy tests
- [ ] Observability stack validation

---

## 🔗 Related Documentation

- **EPIC-08 Plan**: `docs/user-stories/README.md`
- **Resource Pool Architecture**: `docs/architecture/resource_pool_architecture.md`
- **Cost Optimization**: `docs/implementation/US-08.8.4-Cost-Optimization-Reporting.md`
- **Grafana Dashboards**: `docs/grafana-dashboards/`
- **Prometheus Alerts**: `docs/prometheus-alerts/`

---

## 📞 Implementation Notes

### Key Integration Points
1. **SchedulerModule**: Must integrate WFQ engine before job execution
2. **Tenant Context**: All API requests must carry tenant context
3. **Cost Attribution**: Every job execution must track cost
4. **Metrics Collection**: Automatic metrics collection for all operations
5. **Alert Routing**: Prometheus alerts must route to configured channels

### Common Patterns
- All CRUD operations follow REST conventions
- All API responses include trace_id for correlation
- All modifications logged with structured logging
- All quota checks enforced at API gateway level
- All metrics exposed in Prometheus format

---

**Document Version**: 1.0
**Created**: 2025-11-25
**Epic Owner**: Backend Team
**API Version**: v1
**Status**: 📋 Ready for Implementation

