# Plan de Acción - Próximas 2 Semanas

## 🎯 Objetivo
Establecer las bases sólidas de testing para el proyecto hodei-jobs

---

## 📅 Semana 1 (Días 1-5)

### Lunes - Día 1
**Duración:** 8 horas

#### Tarea 1: Eliminar Tests Ignorados (2h)
```bash
# Comando para encontrar tests ignorados
find . -name "*.rs" -type f -exec grep -l "#\[ignore\]" {} \;

# Lista de archivos a actualizar:
- crates/e2e-tests/tests/integration/file_io_evidence_test.rs (5 tests)
- crates/e2e-tests/tests/integration/log_streaming_test.rs (5 tests)
- crates/e2e-tests/tests/integration/real_services_test.rs (5 tests)
```

**Deliverable:** 85% E2E tests ejecutándose

#### Tarea 2: Setup TestContainers (3h)
```bash
# Añadir dependencia
cargo add testcontainers

# Crear test helper
# crates/testing-utils/src/containers.rs
```

**Deliverable:** Infraestructura de testcontainers lista

#### Tarea 3: Crear Mock Infrastructure (3h)
```rust
// Crear mock modules:
// - HTTP client mocks (wiremock)
// - gRPC client mocks
// - Database mocks (sqlx::mock)
```

**Deliverable:** Mock utilities disponibles

---

### Martes - Día 2
**Duración:** 8 horas

#### Tarea 4: Security Test Suite - JWT (4h)
**Archivo:** `crates/adapters/src/security/jwt.rs`

**Tests a implementar:**
1. ✅ `test_valid_token_generation`
2. ✅ `test_token_validation_success`
3. ✅ `test_expired_token_handling`
4. ✅ `test_invalid_token_rejection`
5. ✅ `test_token_without_permissions`
6. ✅ `test_role_based_access`

**Deliverable:** 6 tests para JWT (Cobertura: 0% → 60%)

#### Tarea 5: Security Test Suite - mTLS (4h)
**Archivo:** `crates/adapters/src/security/mtls.rs`

**Tests a implementar:**
1. ✅ `test_certificate_validation_success`
2. ✅ `test_invalid_certificate_rejection`
3. ✅ `test_certificate_expired_handling`
4. ✅ `test_handshake_failure`

**Deliverable:** 4 tests para mTLS (Cobertura: 0% → 50%)

---

### Miércoles - Día 3
**Duración:** 8 horas

#### Tarea 6: Security Test Suite - Audit (2h)
**Archivo:** `crates/adapters/src/security/audit.rs`

**Tests:**
1. ✅ `test_audit_logging_enabled`
2. ✅ `test_audit_logging_disabled`
3. ✅ `test_audit_with_security_context`
4. ✅ `test_audit_without_security_context`

#### Tarea 7: Security Test Suite - Masking (2h)
**Archivo:** `crates/adapters/src/security/masking.rs`

**Tests:**
1. ✅ `test_masking_enabled`
2. ✅ `test_masking_disabled`
3. ✅ `test_pattern_matching`
4. ✅ `test_no_false_positives`

#### Tarea 8: PostgreSQL Integration Tests (4h)
**Archivo:** `crates/adapters/src/postgres.rs`

**Setup Testcontainers:**
```rust
#[cfg(test)]
mod tests {
    use testcontainers::clients::Cli;
    use testcontainers::images::postgres::Postgres;

    #[tokio::test]
    async fn test_save_and_get_job() {
        let docker = Cli::default();
        let node = docker.run(Postgres::default());
        // Test PostgreSQL operations
    }
}
```

**Tests:**
1. ✅ `test_save_and_get_job`
2. ✅ `test_save_job_updates_existing`
3. ✅ `test_get_nonexistent_job`
4. ✅ `test_transaction_rollback`

**Deliverable:** 4 integration tests (Cobertura: 0% → 40%)

---

### Jueves - Día 4
**Duración:** 8 horas

#### Tarea 9: Contract Tests - Repositories (4h)
**Archivos:**
- `crates/ports/src/job_repository.rs`
- `crates/ports/src/pipeline_repository.rs`
- `crates/ports/src/worker_repository.rs`

**Implementar:**
```rust
#[cfg(test)]
mod contract_tests {
    use crate::*;

    #[tokio::test]
    async fn test_job_repository_contract() {
        let repo = create_test_repo();
        // Test contract compliance
    }
}
```

**Tests:** 15 tests de contratos

#### Tarea 10: Redb Integration Tests (4h)
**Archivo:** `crates/adapters/src/redb.rs`

**Tests:**
1. ✅ `test_save_and_get_job_redb`
2. ✅ `test_transaction_rollback_redb`
3. ✅ `test_concurrent_access_redb`

**Deliverable:** 3 integration tests (Cobertura: 0% → 30%)

---

### Viernes - Día 5
**Duración:** 8 horas

#### Tarea 11: Worker Client Tests (4h)
**Archivo:** `crates/adapters/src/worker_client.rs`

**Tests:**
1. ✅ `test_grpc_client_success`
2. ✅ `test_grpc_client_network_error`
3. ✅ `test_retry_mechanism`
4. ✅ `test_circuit_breaker`

#### Tarea 12: CI Pipeline Update (4h)
**Archivo:** `.github/workflows/ci.yml`

**Actualizar:**
```yaml
- name: Run all tests
  run: |
    cargo test --workspace --all-features
    cargo test --package e2e-tests --all-features -- --ignored
```

**Deliverable:** CI ejecuta todos los tests

#### Retrospective Semanal
**Duración:** 1h

**Revisar:**
- ✅ Tests implementados: 35+
- ✅ Coverage gained: 0% → 35%
- ✅ Issues encontrados: 5
- ✅ Plan para siguiente semana

---

## 📅 Semana 2 (Días 6-10)

### Lunes - Día 6
**Duración:** 8 horas

#### Tarea 13: Security Domain Tests (4h)
**Archivo:** `crates/core/src/security.rs`

**Tests:**
1. ✅ `test_permission_check_success`
2. ✅ `test_permission_check_failure`
3. ✅ `test_role_based_access_control`
4. ✅ `test_security_context_propagation`

#### Tarea 14: Server Auth Middleware Tests (4h)
**Archivo:** `server/src/auth.rs`

**Tests:**
1. ✅ `test_auth_middleware_valid_token`
2. ✅ `test_auth_middleware_invalid_token`
3. ✅ `test_auth_middleware_missing_token`

---

### Martes - Día 7
**Duración:** 8 horas

#### Tarea 15: Job Repository Tests (4h)
**Archivo:** `crates/adapters/src/repositories.rs`

**Tests:**
1. ✅ `test_save_job_in_memory`
2. ✅ `test_get_job_in_memory`
3. ✅ `test_concurrent_job_modification`
4. ✅ `test_error_handling`

#### Tarea 16: Event Bus Contract Tests (4h)
**Archivo:** `crates/ports/src/event_bus.rs`

**Tests:**
1. ✅ `test_event_publish_contract`
2. ✅ `test_event_subscribe_contract`
3. ✅ `test_multiple_subscribers`

---

### Miércoles - Día 8
**Duración:** 8 horas

#### Tarea 17: gRPC Server Tests (4h)
**Archivo:** `server/src/grpc.rs`

**Tests:**
1. ✅ `test_grpc_server_startup`
2. ✅ `test_grpc_service_call`
3. ✅ `test_streaming_handling`

#### Tarea 18: Pipeline Domain Tests (4h)
**Archivo:** `crates/core/src/pipeline.rs`

**Tests:**
1. ✅ `test_pipeline_creation`
2. ✅ `test_pipeline_step_validation`
3. ✅ `test_dag_validation`

---

### Jueves - Día 9
**Duración:** 8 horas

#### Tarea 19: Worker Domain Tests (4h)
**Archivo:** `crates/core/src/worker.rs`

**Tests:**
1. ✅ `test_worker_creation`
2. ✅ `test_worker_capacity_check`
3. ✅ `test_worker_heartbeat`
4. ✅ `test_concurrent_jobs`

#### Tarea 20: Metrics Tests (4h)
**Archivo:** `server/src/metrics.rs`

**Tests:**
1. ✅ `test_metrics_collection`
2. ✅ `test_metrics_exposition`
3. ✅ `test_custom_metrics`

---

### Viernes - Día 10
**Duración:** 8 horas

#### Tarea 21: Shared Types Tests (4h)
**Archivos:**
- `crates/shared-types/src/job_definitions.rs`
- `crates/shared-types/src/error.rs`
- `crates/shared-types/src/correlation.rs`

**Tests:** 12 tests

#### Tarea 22: Performance Baseline (2h)
```bash
# Ejecutar todos los tests y medir tiempo
time cargo test --workspace --all-features

# Benchmark inicial
cargo bench
```

#### Retrospectiva Final y Reporte (2h)
**Deliverable:** Reporte de 2 semanas

---

## 📊 Métricas de Progreso

### Semana 1 Targets
- [ ] Tests implementados: 40+
- [ ] Coverage: 0% → 35%
- [ ] Security tests: 0% → 60%
- [ ] Database tests: 0% → 35%

### Semana 2 Targets
- [ ] Tests implementados: 80+
- [ ] Coverage: 35% → 60%
- [ ] Contract tests: 0% → 50%
- [ ] Domain tests: 20% → 60%

### 2 Semanas Total Targets
- [ ] Tests implementados: 80+
- [ ] Coverage: 0% → 60%
- [ ] Security: 0% → 70%
- [ ] Integration: 0% → 50%

---

## 🚨 Riesgos y Mitigaciones

### Riesgo 1: Falta de Conocimiento en TestContainers
**Mitigación:**
- Tutorial: https://docs.rs/testcontainers/
- Ejemplos en repositorios públicos
- Pair programming con equipo

### Riesgo 2: Tests Lentos
**Mitigación:**
- Usar testcontainers solo para integration tests
- Unit tests sin containers
- Paralelización en CI

### Riesgo 3: Mock Complexity
**Mitigación:**
- Crear utilities reutilizables
- Documentar patrones
- Code review de mocks

---

## 📚 Recursos Necesarios

### Dependencias a Añadir
```toml
[dev-dependencies]
testcontainers = "12.0"
wiremock = "0.6"
sqlx = { version = "0.8", features = ["mock"] }
proptest = "1.4"
criterion = "0.5"
```

### Herramientas
- Docker (para testcontainers)
- PostgreSQL (imagen)
- Redis (opcional)

### Documentación
- Testing Strategy (docs/testing_strategy.md)
- Security Testing Guide (docs/security/testing.md)
- Integration Testing Guide (docs/testing/integration.md)

---

## ✅ Definition of Done

Al final de 2 semanas:

1. **Tests:** 80+ tests implementados
2. **Coverage:** 60% coverage global
3. **Security:** 70% coverage en security layer
4. **CI/CD:** Todos los tests ejecutan en CI
5. **Documentación:** Guías de testing actualizadas
6. **Métricas:** Baseline establecido
7. **Plan:** Roadmap para semanas 3-10

---

## 📞 Contacto y Soporte

- **Slack:** #testing-initiative
- **GitHub Project:** https://github.com/Rubentxu/hodei-jobs/projects/1
- **Daily Standup:** 9:00 AM
- **Sprint Review:** Viernes 3:00 PM

---

**Plan creado:** 24 nov 2025
**Período:** 2 semanas (10 días laborables)
**Objetivo:** Establecer foundations sólidas de testing
