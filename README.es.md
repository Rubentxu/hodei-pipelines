![Hodei Pipelines](docs/assets/header.png)

<div align="center">

# Hodei Pipelines

**Plataforma de Orquestación de Trabajos Distribuida de Alto Rendimiento**

[![Estado de la Build](https://img.shields.io/github/actions/workflow/status/Rubentxu/hodei-jobs/ci.yml?branch=main)](https://github.com/Rubentxu/hodei-jobs/actions)
[![Licencia: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Versión de Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org)
[![Docker](https://img.shields.io/badge/docker-ready-blue.svg)](https://www.docker.com/)

[Documentación](docs/) | [Arquitectura](docs/architecture.md) | [Contribuir](CONTRIBUTING.md) | [English](README.md)

</div>

---

## 🚀 Visión General

**Hodei Pipelines** es una plataforma de orquestación de trabajos distribuida de próxima generación, construida enteramente en **Rust**. Está diseñada para ofrecer un rendimiento extremo, baja latencia y una fiabilidad sólida como una roca para pipelines CI/CD complejos, flujos de procesamiento de datos y tareas automatizadas.

A diferencia de los sistemas CI/CD tradicionales que pueden ser pesados y consumir muchos recursos, Hodei Pipelines aprovecha la eficiencia de Rust y la potencia de **NATS JetStream** para gestionar miles de trabajos concurrentes con una sobrecarga mínima.

## ✨ Características Clave

- **⚡ Rendimiento Ultrarrápido**: Construido con Rust para una sobrecarga en tiempo de ejecución casi nula y un uso eficiente de los recursos.
- **🌐 Arquitectura Distribuida**: La arquitectura desacoplada de **Servidor** y **Agente** permite escalar los workers horizontalmente a través de cualquier infraestructura (Kubernetes, VMs, Bare Metal).
- **🔒 Seguridad de Grado Empresarial**:
    - **mTLS** para encriptar toda la comunicación Agente-Servidor.
    - **RBAC** (Control de Acceso Basado en Roles) para una gestión granular de permisos.
    - **Enmascaramiento de Secretos** para proteger datos sensibles en los logs.
- **📡 Bus de Eventos en Tiempo Real**: Impulsado por **NATS JetStream** para el paso de mensajes asíncronos y procesamiento de streams fiable.
- **📊 Observabilidad Profunda**: Integración nativa con **OpenTelemetry** y **Prometheus** para métricas, trazas y logs exhaustivos.
- **🏢 Multi-Tenancy**: Soporte integrado para múltiples inquilinos con aplicación estricta de cuotas y aislamiento de recursos.
- **🐳 Nativo de Contenedores**: Soporte de primera clase para entornos de ejecución Docker y Kubernetes.

## 🏗️ Arquitectura

Hodei Pipelines sigue una arquitectura moderna y modular:

- **Hodei Server**: El plano de control que gestiona las peticiones API, la lógica de orquestación, la planificación y la persistencia del estado (PostgreSQL).
- **HWP Agent**: Agentes worker ligeros que se conectan de forma segura al servidor y ejecutan los trabajos asignados.
- **NATS JetStream**: El sistema nervioso que asegura una comunicación fiable entre componentes.

👉 **[Explorar la Arquitectura (Modelo C4)](docs/architecture.md)** - Diagramas detallados de Contexto, Contenedores y Componentes.
👉 **[Ver Diagramas de Secuencia (Casos de Uso)](docs/sequence_diagrams.md)** - Flujos visuales para Registro de Workers, Envío de Trabajos y más.

## 🛠️ Inicio Rápido

### Prerrequisitos

- Rust 1.75+
- Docker y Docker Compose
- Kubernetes (opcional, para pruebas E2E completas)

### Configuración de Desarrollo Local

1.  **Clonar el repositorio:**
    ```bash
    git clone https://github.com/Rubentxu/hodei-jobs.git
    cd hodei-jobs
    ```

2.  **Iniciar infraestructura (DB, NATS):**
    ```bash
    docker-compose up -d postgres nats
    ```

3.  **Ejecutar el Servidor:**
    ```bash
    cargo run --bin hodei-server
    ```

4.  **Ejecutar un Agente:**
    ```bash
    cargo run --bin hwp-agent
    ```

Para instrucciones detalladas de prueba, incluyendo tests E2E con Testkube, ver **[TESTING.md](TESTING.md)**.

## 📦 Despliegue

Hodei Pipelines está listo para la nube. Proporcionamos **Helm Charts** para un despliegue fácil en Kubernetes.

```bash
make deploy
```

## 🤝 Contribuir

¡Damos la bienvenida a las contribuciones! Por favor, consulta nuestra [Guía de Contribución](CONTRIBUTING.md) para detalles sobre cómo enviar pull requests, reportar problemas y configurar tu entorno de desarrollo.

## 📄 Licencia

Este proyecto está licenciado bajo la [Licencia MIT](LICENSE).

---

<div align="center">
  <sub>Construido con ❤️ por el Equipo Hodei</sub>
</div>
