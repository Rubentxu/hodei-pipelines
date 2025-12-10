//! Test de integración real con TestContainers
//!
//! Este test demuestra la funcionalidad real del TestContainers Manager
//! usando contenedores reales de PostgreSQL y NATS con el patrón
//! Single Instance + Resource Pooling.
//!
//! ⚠️ IMPORTANTE: Estos tests requieren Docker daemon ejecutándose.
//! Se ignorarán por defecto si Docker no está disponible.
//! Para ejecutarlos localmente: cargo test --test testcontainers_real_integration -- --ignored
//! Para ejecutarlos en CI/CD: asegúrese de que Docker esté disponible.

#[cfg(test)]
mod integration_tests {
    use async_nats::Client;
    use futures::StreamExt;
    use integration_pipelines_tests::testcontainers_manager::TestEnvironment;
    use sqlx::PgPool;
    use tracing::info;

    /// Verifica si Docker está disponible
    fn is_docker_available() -> bool {
        std::process::Command::new("docker")
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    #[tokio::test]
    #[ignore = "Requiere Docker daemon ejecutándose. Ejecutar con: cargo test --test testcontainers_real_integration -- --ignored"]
    async fn test_postgres_container_real_functionality() -> Result<(), Box<dyn std::error::Error>>
    {
        if !is_docker_available() {
            tracing::warn!("⚠️ Docker no está disponible. Saltando test de integración real.");
            return Ok(());
        }

        info!("=== Iniciando test de PostgreSQL con contenedor real ===");

        // Crear entorno con TestContainers
        let env = TestEnvironment::new().await?;
        let postgres = env.postgres().await?;

        info!("Contenedor PostgreSQL iniciado:");
        info!("  Host IP: {}", postgres.host_ip);
        info!("  Host Port: {}", postgres.host_port);
        info!("  Database: {}", postgres.database);
        info!("  Username: {}", postgres.username);

        // Verificar conexión directa a PostgreSQL
        let pool = PgPool::connect(&postgres.connection_string()).await?;
        info!("Conexión a PostgreSQL establecida");

        // Ejecutar query de verificación
        let result: (i32,) = sqlx::query_as("SELECT 42").fetch_one(&pool).await?;

        assert_eq!(result.0, 42);
        info!("Query de verificación ejecutada: SELECT 42 = {}", result.0);

        // Crear tabla de prueba
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS test_table (
                id SERIAL PRIMARY KEY,
                name VARCHAR(100) NOT NULL,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )
        "#,
        )
        .execute(&pool)
        .await?;

        info!("Tabla de prueba creada");

        // Insertar datos de prueba
        for i in 1..=5 {
            sqlx::query("INSERT INTO test_table (name) VALUES ($1)")
                .bind(format!("test_record_{}", i))
                .execute(&pool)
                .await?;
        }
        info!("5 registros insertados");

        // Consultar datos
        let rows: Vec<(i32, String, chrono::DateTime<chrono::Utc>)> =
            sqlx::query_as("SELECT id, name, created_at FROM test_table ORDER BY id")
                .fetch_all(&pool)
                .await?;

        assert_eq!(rows.len(), 5);
        info!("Registros consultados: {}", rows.len());

        for (id, name, _) in rows {
            assert_eq!(name, format!("test_record_{}", id));
        }

        // Cleanup
        sqlx::query("DROP TABLE IF EXISTS test_table")
            .execute(&pool)
            .await?;
        info!("Tabla de prueba eliminada");

        // Verificar health check
        let is_healthy = postgres.health_check().await?;
        assert!(is_healthy, "PostgreSQL debe estar saludable");
        info!("Health check de PostgreSQL: OK");

        env.cleanup().await?;
        info!("=== Test PostgreSQL completado exitosamente ===");

        Ok(())
    }

    #[tokio::test]
    #[ignore = "Requiere Docker daemon ejecutándose. Ejecutar con: cargo test --test testcontainers_real_integration -- --ignored"]
    async fn test_nats_container_real_functionality() -> Result<(), Box<dyn std::error::Error>> {
        if !is_docker_available() {
            tracing::warn!("⚠️ Docker no está disponible. Saltando test de integración real.");
            return Ok(());
        }

        info!("=== Iniciando test de NATS con contenedor real ===");

        // Crear entorno con TestContainers
        let env = TestEnvironment::new().await?;
        let nats = env.nats().await?;

        info!("Contenedor NATS iniciado:");
        info!("  Host IP: {}", nats.host_ip);
        info!("  Host Port: {}", nats.host_port);
        info!("  Monitoring Port: {}", nats.monitoring_port);

        // Conectar a NATS
        let client: Client = async_nats::connect(&nats.connection_url()).await?;
        info!("Conexión a NATS establecida");

        // Suscribirse a un tema de prueba
        let mut subscriber = client.subscribe("test.subject").await?;
        info!("Suscripción creada a 'test.subject'");

        // Publicar mensajes de prueba
        for i in 1..=5 {
            let message = format!("test_message_{}", i);
            client
                .publish("test.subject", message.into_bytes().into())
                .await?;
        }
        info!("5 mensajes publicados");

        // Recibir mensajes
        for i in 1..=5 {
            let message = subscriber.next().await.ok_or("No message received")?;
            let text = String::from_utf8(message.payload.to_vec())?;
            assert_eq!(text, format!("test_message_{}", i));
        }
        info!("5 mensajes recibidos y verificados");

        // Verificar health check
        let is_healthy = nats.health_check().await?;
        assert!(is_healthy, "NATS debe estar saludable");
        info!("Health check de NATS: OK");

        env.cleanup().await?;
        info!("=== Test NATS completado exitosamente ===");

        Ok(())
    }

    #[tokio::test]
    #[ignore = "Requiere Docker daemon ejecutándose. Ejecutar con: cargo test --test testcontainers_real_integration -- --ignored"]
    async fn test_singleton_pattern_with_real_containers() -> Result<(), Box<dyn std::error::Error>>
    {
        if !is_docker_available() {
            tracing::warn!("⚠️ Docker no está disponible. Saltando test de integración real.");
            return Ok(());
        }

        info!("=== Iniciando test del patrón Singleton ===");

        // Crear entorno
        let env = TestEnvironment::new().await?;

        // Obtener contenedores múltiples veces
        let pg1 = env.postgres().await?;
        let pg2 = env.postgres().await?;
        let nats1 = env.nats().await?;
        let nats2 = env.nats().await?;

        // Verificar que son singletons (misma IP y puerto)
        assert_eq!(pg1.host_ip, pg2.host_ip);
        assert_eq!(pg1.host_port, pg2.host_port);
        assert_eq!(nats1.host_ip, nats2.host_ip);
        assert_eq!(nats1.host_port, nats2.host_port);

        info!("Patrón Singleton verificado:");
        info!(
            "  PostgreSQL: {}:{} (consistente)",
            pg1.host_ip, pg1.host_port
        );
        info!(
            "  NATS: {}:{} (consistente)",
            nats1.host_ip, nats1.host_port
        );

        // Verificar que ambos pueden conectarse
        let pool1 = PgPool::connect(&pg1.connection_string()).await?;
        let pool2 = PgPool::connect(&pg2.connection_string()).await?;

        let result1: (i32,) = sqlx::query_as("SELECT 1").fetch_one(&pool1).await?;
        let result2: (i32,) = sqlx::query_as("SELECT 2").fetch_one(&pool2).await?;

        assert_eq!(result1.0, 1);
        assert_eq!(result2.0, 2);

        let _client1 = async_nats::connect(&nats1.connection_url()).await?;
        let _client2 = async_nats::connect(&nats2.connection_url()).await?;

        info!("Ambas conexiones a PostgreSQL y NATS funcionando correctamente");

        env.cleanup().await?;
        info!("=== Test Singleton completado exitosamente ===");

        Ok(())
    }

    #[tokio::test]
    #[ignore = "Requiere Docker daemon ejecutándose. Ejecutar con: cargo test --test testcontainers_real_integration -- --ignored"]
    async fn test_combined_postgres_and_nats_workflow() -> Result<(), Box<dyn std::error::Error>> {
        if !is_docker_available() {
            tracing::warn!("⚠️ Docker no está disponible. Saltando test de integración real.");
            return Ok(());
        }

        info!("=== Iniciando test de workflow combinado ===");

        // Crear entorno
        let env = TestEnvironment::new().await?;
        let postgres = env.postgres().await?;
        let nats = env.nats().await?;

        // Conectar a ambas bases
        let pool = PgPool::connect(&postgres.connection_string()).await?;
        let client = async_nats::connect(&nats.connection_url()).await?;

        info!("Conectado a PostgreSQL y NATS");

        // Crear tabla para pipeline
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS pipeline_executions (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                name VARCHAR(200) NOT NULL,
                status VARCHAR(50) NOT NULL,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )
        "#,
        )
        .execute(&pool)
        .await?;

        info!("Tabla pipeline_executions creada");

        // Simular workflow de pipeline:
        // 1. Registrar ejecución en PostgreSQL
        let pipeline_id = uuid::Uuid::new_v4();
        sqlx::query("INSERT INTO pipeline_executions (id, name, status) VALUES ($1, $2, $3)")
            .bind(&pipeline_id)
            .bind("test-pipeline")
            .bind("running")
            .execute(&pool)
            .await?;

        info!("Pipeline registrada en PostgreSQL: {}", pipeline_id);

        // 2. Publicar evento en NATS
        let event = format!(r#"{{"pipeline_id":"{}","status":"started"}}"#, pipeline_id);
        client
            .publish("pipeline.events", event.into_bytes().into())
            .await?;

        info!("Evento publicado en NATS");

        // 3. Simular actualización de estado
        sqlx::query("UPDATE pipeline_executions SET status = $1 WHERE id = $2")
            .bind("completed")
            .bind(&pipeline_id)
            .execute(&pool)
            .await?;

        info!("Estado actualizado en PostgreSQL");

        // 4. Verificar estado final
        let (status,): (String,) =
            sqlx::query_as("SELECT status FROM pipeline_executions WHERE id = $1")
                .bind(&pipeline_id)
                .fetch_one(&pool)
                .await?;

        assert_eq!(status, "completed");

        // 5. Recibir evento de completado
        let mut subscriber = client.subscribe("pipeline.events").await?;
        let message = subscriber.next().await.ok_or("No event received")?;
        let event_text = String::from_utf8(message.payload.to_vec())?;
        assert!(event_text.contains(&pipeline_id.to_string()));
        assert!(event_text.contains("started"));

        info!("Workflow completo verificado:");
        info!("  Pipeline ID: {}", pipeline_id);
        info!("  Estado final: {}", status);

        // Cleanup
        sqlx::query("DROP TABLE IF EXISTS pipeline_executions")
            .execute(&pool)
            .await?;

        env.cleanup().await?;
        info!("=== Test de workflow combinado completado exitosamente ===");

        Ok(())
    }
}
