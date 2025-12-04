//! Test de integración para US-TD-003: Migrar RBAC a PostgreSQL
//!
//! Este test verifica que los repositorios RBAC funcionen correctamente
//! con PostgreSQL usando SQLx, aplicando TDD y patrones de diseño.

use hodei_pipelines_domain::{
    DomainError,
    identity_access::value_objects::{PermissionEntity, PermissionId, RoleEntity, RoleId},
};
use hodei_pipelines_integration_tests::helpers::rbac_postgresql::{
    PostgreSQLRepositoryFactory, SqlxPermissionRepository, SqlxRoleRepository,
};
use hodei_pipelines_ports::{PermissionRepository, RoleRepository};
use sqlx::{Connection, Executor, PgConnection, PgPool, postgres::PgPoolOptions};
use std::time::Duration;
use testcontainers::clients::Cli;
use testcontainers::core::WaitFor;
use testcontainers::images::postgres::Postgres;

#[cfg(test)]
mod tests {
    use super::*;

    const POSTGRES_USER: &str = "test_user";
    const POSTGRES_PASSWORD: &str = "test_password";
    const POSTGRES_DB: &str = "test_db";

    /// Setup: Inicia PostgreSQL usando TestContainers y ejecuta migraciones
    async fn setup_postgres() -> Result<PgPool, Box<dyn std::error::Error>> {
        let docker = Cli::default();
        let postgres_container = docker.run(
            Postgres::latest()
                .with_user(POSTGRES_USER)
                .with_password(POSTGRES_PASSWORD)
                .with_database(POSTGRES_DB)
                .with_env_var("POSTGRES_HOST_AUTH_METHOD", "md5")
                .with_wait_for(WaitFor::Duration(Duration::from_secs(10))),
        );

        let port = postgres_container.get_host_port_ipv4(5432);
        let connection_string = format!(
            "postgresql://{}:{}@localhost:{}/{}",
            POSTGRES_USER, POSTGRES_PASSWORD, port, POSTGRES_DB
        );

        // Esperar a que PostgreSQL esté listo
        let mut attempts = 0;
        let max_attempts = 30;
        while attempts < max_attempts {
            match PgConnection::connect(&connection_string).await {
                Ok(mut conn) => {
                    // Ejecutar migraciones
                    let migration_sql =
                        include_str!("../../migrations/20241201_create_rbac_tables.sql");
                    conn.execute(migration_sql).await?;
                    drop(conn);
                    break;
                }
                Err(_) => {
                    attempts += 1;
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
            }
        }

        if attempts >= max_attempts {
            return Err("Failed to connect to PostgreSQL".into());
        }

        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&connection_string)
            .await?;

        Ok(pool)
    }

    /// Tear down: Limpiar recursos
    async fn cleanup(pool: &PgPool) -> Result<(), Box<dyn std::error::Error>> {
        // Limpiar tablas
        sqlx::query("DROP TABLE IF EXISTS roles, permissions CASCADE")
            .execute(pool)
            .await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_role_repository_crud_operations() -> Result<(), Box<dyn std::error::Error>> {
        // ===== RED: Crear test que falla =====
        let pool = setup_postgres().await?;
        let repo = SqlxRoleRepository::new(pool.clone());

        // Crear un role
        let role = RoleEntity::new(
            RoleId::new(),
            "TestRole".to_string(),
            "Test role for integration testing".to_string(),
            vec![],
        )?;

        // Guardar role
        repo.save_role(&role).await?;

        // Verificar que se guardó
        let saved_role = repo.get_role(&role.id).await?;
        assert!(saved_role.is_some(), "Role should be saved");
        let saved_role = saved_role.unwrap();
        assert_eq!(saved_role.id, role.id);
        assert_eq!(saved_role.name, "TestRole");
        assert_eq!(saved_role.description, "Test role for integration testing");

        // Verificar que get_role_by_name funciona
        let role_by_name = repo.get_role_by_name("TestRole").await?;
        assert!(role_by_name.is_some(), "Role should be found by name");
        assert_eq!(role_by_name.unwrap().id, role.id);

        // Listar todos los roles
        let all_roles = repo.list_all_roles().await?;
        assert_eq!(all_roles.len(), 1, "Should have exactly one role");
        assert_eq!(all_roles[0].id, role.id);

        // Verificar que existe
        assert!(repo.exists(&role.id).await?, "Role should exist");

        // Actualizar role
        let mut role_to_update = role.clone();
        role_to_update.update(
            Some("UpdatedRole".to_string()),
            Some("Updated description".to_string()),
        )?;
        repo.save_role(&role_to_update).await?;

        let updated_role = repo.get_role(&role.id).await?.unwrap();
        assert_eq!(updated_role.name, "UpdatedRole");
        assert_eq!(updated_role.description, "Updated description");

        // Eliminar role
        repo.delete_role(&role.id).await?;

        // Verificar que se eliminó
        let deleted_role = repo.get_role(&role.id).await?;
        assert!(deleted_role.is_none(), "Role should be deleted");

        cleanup(&pool).await?;
        println!("✓ test_role_repository_crud_operations passed");

        Ok(())
    }

    #[tokio::test]
    async fn test_permission_repository_crud_operations() -> Result<(), Box<dyn std::error::Error>>
    {
        let pool = setup_postgres().await?;
        let repo = SqlxPermissionRepository::new(pool.clone());

        // Crear un permission
        let permission = PermissionEntity::new(
            PermissionId::new(),
            "TestPermission".to_string(),
            "Test permission for integration testing".to_string(),
            "jobs".to_string(),
            "read".to_string(),
        )?;

        // Guardar permission
        repo.save_permission(&permission).await?;

        // Verificar que se guardó
        let saved_permission = repo.get_permission(&permission.id).await?;
        assert!(saved_permission.is_some(), "Permission should be saved");
        let saved_permission = saved_permission.unwrap();
        assert_eq!(saved_permission.id, permission.id);
        assert_eq!(saved_permission.name, "TestPermission");
        assert_eq!(saved_permission.resource, "jobs");
        assert_eq!(saved_permission.action, "read");

        // Verificar que get_permission_by_key funciona
        let permission_by_key = repo.get_permission_by_key("jobs", "read").await?;
        assert!(
            permission_by_key.is_some(),
            "Permission should be found by key"
        );
        assert_eq!(permission_by_key.unwrap().id, permission.id);

        // Listar todos los permissions
        let all_permissions = repo.list_all_permissions().await?;
        assert_eq!(
            all_permissions.len(),
            1,
            "Should have exactly one permission"
        );
        assert_eq!(all_permissions[0].id, permission.id);

        // Verificar que existe
        assert!(
            repo.exists(&permission.id).await?,
            "Permission should exist"
        );

        // Actualizar permission
        let mut permission_to_update = permission.clone();
        permission_to_update.update(Some("Updated description".to_string()));
        repo.save_permission(&permission_to_update).await?;

        let updated_permission = repo.get_permission(&permission.id).await?.unwrap();
        assert_eq!(updated_permission.description, "Updated description");

        // Eliminar permission
        repo.delete_permission(&permission.id).await?;

        // Verificar que se eliminó
        let deleted_permission = repo.get_permission(&permission.id).await?;
        assert!(deleted_permission.is_none(), "Permission should be deleted");

        cleanup(&pool).await?;
        println!("✓ test_permission_repository_crud_operations passed");

        Ok(())
    }

    #[tokio::test]
    async fn test_role_with_permissions() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup_postgres().await?;
        let role_repo = SqlxRoleRepository::new(pool.clone());
        let permission_repo = SqlxPermissionRepository::new(pool.clone());

        // Crear permissions
        let perm1 = PermissionEntity::new(
            PermissionId::new(),
            "ReadJobs".to_string(),
            "Permission to read jobs".to_string(),
            "jobs".to_string(),
            "read".to_string(),
        )?;
        let perm2 = PermissionEntity::new(
            PermissionId::new(),
            "WriteJobs".to_string(),
            "Permission to write jobs".to_string(),
            "jobs".to_string(),
            "write".to_string(),
        )?;

        permission_repo.save_permission(&perm1).await?;
        permission_repo.save_permission(&perm2).await?;

        // Crear role con permissions
        let mut role = RoleEntity::new(
            RoleId::new(),
            "JobManager".to_string(),
            "Manages jobs".to_string(),
            vec![perm1.id.clone(), perm2.id.clone()],
        )?;

        role_repo.save_role(&role).await?;

        // Verificar que el role tiene los permissions
        let saved_role = role_repo.get_role(&role.id).await?.unwrap();
        assert_eq!(saved_role.permissions.len(), 2);
        assert!(saved_role.permissions.contains(&perm1.id));
        assert!(saved_role.permissions.contains(&perm2.id));

        // Actualizar permissions del role
        role.add_permission(PermissionId::new());
        role_repo.save_role(&role).await?;

        let updated_role = role_repo.get_role(&role.id).await?.unwrap();
        assert_eq!(updated_role.permissions.len(), 3);

        cleanup(&pool).await?;
        println!("✓ test_role_with_permissions passed");

        Ok(())
    }

    #[tokio::test]
    async fn test_repository_error_handling() -> Result<(), Box<dyn std::error::Error>> {
        let pool = setup_postgres().await?;
        let role_repo = SqlxRoleRepository::new(pool.clone());

        // Test: Obtener role que no existe
        let non_existent_id = RoleId::new();
        let result = role_repo.get_role(&non_existent_id).await?;
        assert!(result.is_none(), "Should return None for non-existent role");

        // Test: Verificar que no existe
        assert!(
            !role_repo.exists(&non_existent_id).await?,
            "Should return false for non-existent role"
        );

        // Test: get_role_by_name con nombre inexistente
        let result = role_repo.get_role_by_name("NonExistentRole").await?;
        assert!(
            result.is_none(),
            "Should return None for non-existent role name"
        );

        cleanup(&pool).await?;
        println!("✓ test_repository_error_handling passed");

        Ok(())
    }
}
