# Estrategias modernas de auto-scaling en sistemas distribuidos: fundamentos, implementaciones y casos reales

## Resumen ejecutivo

La elasticidad es una propiedad esencial de los sistemas distribuidos modernos. En términos operativos, el auto‑scaling permite que una plataforma ajuste de forma automática y continua la capacidad de cómputo —en número de instancias, tamaño de recursos por proceso o ambos— para sostener objetivos de latencia y disponibilidad con el menor costo posible. En ausencia de mecanismos de elasticidad, los sistemas要么 están sobreaprovisionados (costes innecesarios)要么 sufren subaprovisionamiento (SLA degradados, errores, pérdida de ingresos).

Los avances recientes convergen en cuatro enfoques complementarios:

- Escalado horizontal (H): aumenta o reduce el número de unidades de ejecución (réplicas/pods/VMs).
- Escalado vertical (V): ajusta CPU/RAM por unidad de ejecución.
- Predictivo: usa señales históricas y de series temporales para predecir demanda y aprovisionar con antelación.
- Reactivo: responde a condiciones actuales (utilización, latencia, eventos) para mantener objetivos operativos.

Kubernetes aporta una columna vertebral de elasticidad a nivel de clúster y de carga de trabajo con Horizontal Pod Autoscaler (HPA), Vertical Pod Autoscaler (VPA), KEDA para escalado basado en eventos y el Cluster Autoscaler (CA) para infraestructura. En la nube, AWS, Azure y Google Cloud ofrecen escalado reactivo y predictivo a nivel de grupos de instancias gestionadas, con combinaciones que se adaptan a patrones de carga, tiempos de inicialización y objetivos de costo.

Hallazgos clave:

- Dónde brilla cada enfoque:
  - Reactivo (HPA/VMSS/Autoscaling) es el estándar para variaciones intradía y como red de seguridad ante la incertidumbre. [^1][^7][^10]
  - Predictivo (AWS Predictive, GCP Predictive, Azure Predictive) destaca con cargas cíclicas y tiempos de inicialización largos; reduce cold starts y suaviza la experiencia del usuario al anticipar picos. [^4][^6][^5]
  - Hybrid es lo que mejores resultados da en producción: combinar predictivo a nivel de infraestructura (p. ej., MIG/VMSS) con reactivo a nivel de aplicación (HPA/KEDA), añadiendo VPA para optimización de tamaño y límites de recursos. [^6][^1][^8][^9][^19]

- Riesgos y mitigaciones:
  - Cold starts en reactivo: mitigar con predictivo y políticas de warmup/cooldown. [^6][^5]
  - Sobreaprovisionamiento: combinar predictivo con HPA/KEDA; revisar objetivos de utilización; aprovechar escalado a cero en cargas event‑driven. [^8][^9]
  - Subaprovisionamiento: activar guard rails y límites por zona; planificar capacidad con redundancia N+1/N+2. [^17][^3]
  - Deriva de modelos predictivos: vigilar MAPE y señales de drift; recalibrar periódicamente. [^15][^5]

- Recomendación ejecutiva:
  - Adoptar un patrón híbrido progresivo: 1) asegurar reactivo con HPA/KEDA y CA, 2) añadir predictivo en infraestructura para cargas cíclicas o con warmup, 3) optimizar tamaño con VPA y gobernanza de costos.
  - Instrumentar métricas de “señales doradas” (latencia, errores, saturación, throughput), gobernar límites y cooldowns, y establecer revisiones semanales de desempeño y costo. [^17]

Este informe profundiza en los fundamentos, presenta las capacidades de Kubernetes y de los principales proveedores cloud, y extrae lecciones de Netflix, Uber y Google para ofrecer una guía pragmática de implementación y gobernanza.

## Fundamentos: escalado horizontal vs vertical y señales de control

Escalar horizontalmente significa aumentar el número de réplicas (pods/VMs/instancias); escalar verticalmente significa aumentar CPU/RAM por réplica. El escalado horizontal aporta resiliencia ante fallos de nodo y permite expandir capacidad sin límites prácticos del tamaño de máquina; el vertical simplifica la operación cuando la aplicación no es fácilmente paralelizable, pero enfrenta límites físicos y riesgos de “big box failure”. En sistemas modernos, lo habitual es combinar ambos: dimensionar correctamente cada pod (V) y replicar según demanda (H). [^2][^1]

La efectividad del auto‑scaling descansa en la selección de señales de control:

- Utilización de CPU/memoria: indicativas de presión de recursos a nivel de proceso; útiles para servicios compute‑bound o memory‑bound.
- Latencia end‑to‑end: la métrica más cercana a la experiencia de usuario; ideal cuando el throughput aumenta sin degradación (aplicación “elástica”).
- Throughput (RPS/QPS): captura la demanda de negocio; correlaciona bien con capacidad requerida en servicios sin estado. [^17]
- Colas y eventos: longitud de cola, lag, tasa de mensajes; son señales directas de necesidad de capacidad en cargas event‑driven. [^8]

Las mejores prácticas SRE recomiendan controlar la saturación, los errores y la latencia como “señales doradas” y ajustar la capacidad para mantener estas métricas bajo control a lo largo del tiempo. [^17] La Tabla 1 resume los compromisos operativos de cada enfoque.

Para ilustrar estos trade‑offs, la siguiente tabla contrasta escalado horizontal y vertical.

Tabla 1. Comparativa técnico‑operativa: Horizontal vs Vertical

| Criterio                     | Horizontal (réplicas)                                      | Vertical (CPU/RAM por réplica)                                     |
|-----------------------------|------------------------------------------------------------|---------------------------------------------------------------------|
| Latencia de ajuste          | Rápida si imágenes son ligeras y hay warm‑up bajo          | Puede requerir reinicio (dependiendo de soporte in‑place)          |
| Resiliencia                 | Alta: falla de nodo no elimina capacidad global            | Riesgo de “big box failure”: una máquina grande falla impacta más  |
| Límites de escalado         | Altos (miles de pods/VMs)                                  | Límites por tamaño de máquina                                      |
| Complejidad operativa       | Mayor: balanceo, afinidad, distribución por zona           | Menor: un solo proceso mayor                                       |
| Afinidad de datos/estado    | Desafiante en estado (necesita sharding/replicación)       | Más simple si estado es local y estable                            |
| Casos ideales               | APIs sin estado, workers paralelizables, tráfico web       | Bases embed, cargas no paralelizables, control fino de recursos    |

Fuente: síntesis de principios SRE y documentación de Kubernetes. [^17][^1]

La implicación práctica: comenzar con H para elasticidad y resiliencia, usar V para optimizar la “huella” por réplica y reducir contención, y combinar ambos bajo políticas de gobernanza.

### Señales de control y objetivos de calidad (QoS)

El control por latencia (p95/p99) es preferible cuando el objetivo es experiencia de usuario, pero requiere una métrica confiable y baja varianza. CPU/memoria son señales fáciles de instrumentar y escalan bien con HPA; throughput (RPS) aproxima la demanda de negocio y, combinado con utilización, permite dimensionar capacidad por núcleo de forma robusta, como demostró Uber. [^17][^18]

Evitar thundering herd y sobre‑oscilación exige:

- Cooldowns y estabilización tras escalados.
- Objetivos de utilización prudentes (no “apuntar al 100%”).
- Límites por zona (para evitar concentrarlo todo en una región con hardware más eficiente).
- Guard rails que corten decisiones si la calidad del modelo o la señal es dudosa. [^17][^18]

## Estrategias modernas de auto‑scaling: predictivo, reactivo e híbrido

El escalado reactivo responde a lo que ocurre ahora; el predictivo, a lo que预计 ocurre en los próximos minutos u horas. El primero es indispensable como red de seguridad; el segundo es ventajoso cuando hay estacionalidad marcada o tiempos de inicialización significativos. El enfoque híbrido orquesta ambos, normalmente aprovisionando infraestructura por adelantado y dejando que la aplicación ajuste réplicas y tamaño según su propia métrica.

Para situar cada enfoque en su terreno, la Tabla 2 compara sus supuestos, señales y riesgos.

Tabla 2. Comparativa de estrategias: Predictivo vs Reactivo vs Híbrido

| Dimensión            | Predictivo                                                  | Reactivo                                                       | Híbrido                                                                 |
|----------------------|-------------------------------------------------------------|----------------------------------------------------------------|-------------------------------------------------------------------------|
| Señales              | Historial (días/semanas), estacionalidad, tendencia         | CPU/mem, latencia, RPS, lag de cola                            | Predictivo en infraestructura + reactivo a nivel aplicación             |
| Requisitos de datos  | Historial suficiente; estabilidad de patrones               | Métricas confiables en tiempo real                             | Ambos conjuntos de señales                                             |
| Tiempos de warmup    | Críticos; el predictor programa con antelación              | Afectan latencia de respuesta durante picos                     | Predictivo reduce warmup; reactivo afina y corrige desvíos              |
| Tolerancia a picos   | Buena si se predicen; limitada ante novedad                 | Depende de warmup y límites                                    | Alta: reactivo cubre novedades, predictivo suaviza variaciones          |
| Riesgo de sobre‑costos | Potencial si la predicción es generosa                     | Sobreaprovisionamiento por hysteresis o umbrales inadecuados   | Mitigado por objetivos prudentes y guard rails                          |
| Casos de uso         | Cargas cíclicas, batch, “spike” esperados                   | APIs sin estado, variación intradía, eventos inesperados       | La mayoría de plataformas productivas                                   |

Fuentes: documentación de AWS, GCP, Azure y prácticas ML para forecasting. [^4][^6][^5][^15]

### Predictive scaling

Funcionamiento. Los proveedores analizan historial para extraer patrones diarios/semanales, combinan tendencias con datos recientes y calculan capacidad futura. AWS EC2 Auto Scaling ofrece políticas predictivas que escalan con antelación; Google Compute Engine utiliza hasta tres semanas de historia, requiere al menos tres días de métricas para iniciar predicciones y recalcula cada pocos minutos; Azure VMSS integra “predictive autoscale” para cargas cíclicas con horarios recurrentes. [^4][^6][^5][^15]

Requisitos y límites. Las capacidades difieren: GCP soporta únicamente utilización de CPU para el autoscaler predictivo; AWS y Azure permiten métricas personalizadas o de aplicación con más flexibilidad. GCP recalcula predicciones con granularidad de minutos y cede ante señales en tiempo real si la demanda excede el pronóstico. Azure ofrece perfiles predictivos orientados a cargas con ciclos diarios/semanales. [^6][^4][^5]

La Tabla 3 sintetiza la funcionalidad predictiva entre nubes.

Tabla 3. Resumen de capacidades de predictive autoscaling

| Proveedor | Métrica base soportada | Horizonte y datos | Recalculo | Interacción reactivo | Notas clave |
|-----------|-------------------------|-------------------|-----------|----------------------|-------------|
| AWS       | CPU y métricas personalizadas | Historial (puede usar hasta ~15 meses) | Continuo según política | Puede combinar con escalado dinámico | Ahorros y fiabilidad en cargas cíclicas [^4][^15] |
| Azure     | Métricas de host/invitado, App Insights | Recomendado ≥ 2 semanas para mejor precisión | Basado en perfiles/políticas | Integra reglas reactivas | Predictivo para VMSS con ciclos recurrentes [^5] |
| GCP       | Solo CPU | ≥3 días para iniciar; hasta 3 semanas de historia | Cada pocos minutos | Cede ante datos en tiempo real | Warmup/cool‑down configurables [^6] |

El impacto operativo es claro: reduce cold starts, suaviza latencia durante picos esperados y, bien afinado, puede reducir costos al evitar sobreaprovisionamiento crónico.

### Reactive scaling

El reactivo es el caballo de batalla del auto‑scaling. HPA ajusta réplicas de un Deployment/StatefulSet en función de utilización de CPU/memoria o métricas personalizadas; el escalado de VMSS y grupos gestionados sigue reglas por umbrales; KEDA permite triggers por eventos (colas, HTTP, Prometheus) y escalado a cero. Las mejores prácticas incluyen cooldowns para evitar oscilación, límites máximos prudentes y métricas que correlacionen con latencia/SLA. [^7][^10][^8][^9][^11]

### Hybrid approaches

El patrón ganador en producción suele ser combinar:

- Autoscaler predictivo a nivel de infraestructura (MIG/VMSS) para aprovisionar capacidad por adelantado.
- HPA/KEDA a nivel de aplicación para ajustar réplicas con señales de negocio y eventos, además de CPU/memoria.
- VPA para optimizar tamaño de pods, dentro de límites operativos y con salvaguardas, incluyendo redimensionamiento in‑situ según soporte. [^6][^1][^8][^9][^19]

Este patrón reduce latencia en picos esperados, mantiene elasticidad ante imprevistos y mejora el costo medio por transacción. La gobernanza define quién lidera en caso de conflicto (habitualmente infraestructura predictiva marca el piso y aplicación reactiva marca el techo), y se refuerzan límites por zona, cooldowns y cuotas para evitar efectos colaterales.

## Kubernetes: mecanismos y patrones de autoscaling

Kubernetes ofrece una cadena de controles que van de la aplicación al clúster:

- Horizontal Pod Autoscaler (HPA): ajusta réplicas de una carga de trabajo según métricas (CPU/memoria, personalizadas). [^1][^7]
- Vertical Pod Autoscaler (VPA): ajusta solicitudes/limits de contenedores; modos Auto/Recreate/Initial/Off. [^1][^19]
- KEDA: escalado impulsado por eventos con más de 50 escaladores; escala a cero; integra triggers HTTP, colas, Prometheus, etc. [^8][^9][^11]
- Cluster Autoscaler (CA): añade o elimina nodos para acomodar pods pendientes; interactúa con el provisionamiento de la nube. [^1][^14]

La Tabla 4 compara estas opciones.

Tabla 4. HPA vs VPA vs KEDA

| Atributo             | HPA                                               | VPA                                                | KEDA                                                  |
|----------------------|---------------------------------------------------|----------------------------------------------------|-------------------------------------------------------|
| Dirección            | Horizontal (réplicas)                             | Vertical (recursos por contenedor)                 | Horizontal (a veces vertical), impulsado por eventos  |
| Triggers             | CPU/memoria, métricas personalizadas              | Uso observado de CPU/memoria                       | Colas, HTTP, Prometheus, cron y otros (50+ escaladores) |
| Escalado a cero      | No                                                | No                                                 | Sí                                                    |
| Casos de uso         | APIs sin estado, workers, servicios web           | Optimización de tamaño, mejora de utilización      | Cargas event‑driven, trabajos batch, microservicios   |
| Requisitos           | Metrics Server, definición de requests/limits     | VPA instalado; recomendaciones y modos de update   | Controlador KEDA y TriggerAuthentication              |
| Interacción          | Con CA cuando faltan nodos                        | Con HPA/CA para completar la elasticidad           | Integra con HPA; usa métricas externas                |

Fuentes: documentación oficial y comparativas recientes. [^1][^7][^8][^9][^11][^19]

Una arquitectura elástica de Kubernetes combina:

- HPA para variaciones intradía.
- KEDA para servicios event‑driven o con escalado a cero.
- VPA para dimensionamiento fino en cargas estables y con baja frecuencia de despliegue.
- Cluster Autoscaler para infraestructura.

#### Horizontal Pod Autoscaler (HPA)

HPA observa utilización promedio frente a objetivos (por ejemplo, CPU al 50%) y ajusta min/max de réplicas. Funciona con Deployments y StatefulSets; para métricas personalizadas requiere un backend (Prometheus, APIs) integrado con el Metrics Server o el sistema de métricas externo. [^7]

#### Vertical Pod Autoscaler (VPA)

VPA recomienda y aplica cambios a solicitudes/limits de contenedores. Sus modos incluyen Auto/Recreate, Initial y Off. En versiones recientes se ha avanzado en redimensionamiento in‑situ de contenedores, pero su disponibilidad y comportamiento pueden variar; los equipos deben validar en su versión de Kubernetes y activar guard rails para evitar reinicios disruptivos. [^19][^1]

#### Kubernetes Event‑Driven Autoscaling (KEDA)

KEDA aporta escalado por eventos y a cero con más de 50 escaladores (SQS, Kafka, Azure Queue, Prometheus, cron). Integra con HPA, y define recursos ScaledObject/ScaledJob y TriggerAuthentication para conectar fuentes externas de forma segura. [^8][^9][^11]

#### Cluster Autoscaler (CA) y escalado de infraestructura

CA monitorea pods pendientes por falta de recursos y escala nodos de acuerdo con los grupos gestionados por el proveedor cloud. Es fundamental para que HPA/KEDA no colisionen con la falta de capacidad; su comportamiento depende de límites por grupo, tipos de instancia y políticas de expansión. [^1][^14]

## Auto‑scaling nativo en proveedores cloud

Los tres grandes proveedores ofrecen escalado reactivo y, en distintos grados, predictivo. La Tabla 5 resume una comparación funcional.

Tabla 5. AWS Auto Scaling vs Azure VMSS vs GCP Managed Instance Groups

| Capacidad                    | AWS EC2 Auto Scaling                               | Azure VMSS                                         | GCP Managed Instance Groups (MIG)            |
|-----------------------------|-----------------------------------------------------|----------------------------------------------------|----------------------------------------------|
| Reactivo (umbral/métricas)  | Sí: políticas dinámicas                            | Sí: reglas por métricas y programación             | Sí: target tracking CPU y otras señales      |
| Predictivo                  | Sí: políticas predictivas                           | Sí: predictive autoscale para VMSS                 | Sí: predictive autoscaler (CPU únicamente)   |
| Programación                | Sí                                                  | Sí                                                 | Sí                                           |
| Warmup/Cooldown             | Sí (instance warmup, cooldowns)                     | Sí (cooldowns y perfiles)                          | Sí (warmup/cool‑down y stabilization period) |
| Métricas soportadas         | CPU, red, personalizadas                            | Host, invitado, App Insights                       | CPU (predictivo), balanceo y monitoreo separadamente |
| Límites operativos          | Depende de ASG y servicios asociados                | Reglas y límites por conjunto; reparaciones auto   | Integración con autoscaler; políticas múltiples |

Fuentes: documentación de AWS, Azure y GCP. [^4][^10][^12][^13][^6][^20]

### AWS Auto Scaling

AWS ofrece escalado dinámico por métricas, escalado programado y, de forma destacada, políticas predictivas que pronostican demanda basada en historial para escalado proactivo. Se recomienda combinarlo con escalado dinámico para cubrir picos inesperados y usar métricas personalizadas cuando CPU no representa bien la carga. [^4]

### Azure Virtual Machine Scale Sets (VMSS)

Azure VMSS habilita reglas por métricas de host, métricas de VM invitada (extensión de diagnóstico) y métricas a nivel de aplicación (Application Insights), además de escalado programado y predictivo para cargas cíclicas. Las operaciones de escalado admiten incrementos por conteo o porcentaje y políticas de reducción (scale‑in) configurables. [^10][^12][^13][^5]

### Google Cloud Platform (Compute Engine)

El autoscaler de GCP para Managed Instance Groups (MIG) soporta escalado por CPU y otros métodos. El modo predictivo usa únicamente CPU, requiere tres días de datos para iniciar, emplea hasta tres semanas de historia para el modelo y recalcula predicciones cada pocos minutos. Si la demanda real supera el pronóstico, cede ante el escalado en tiempo real; esto exige considerar tiempos de warmup para evitar latencias elevadas en picos cortos e intensos. [^6][^20]

## Lecciones y patrones de Netflix, Uber y Google

Las arquitecturas a escala de Netflix, Uber y Google muestran tres principios convergentes: planificar la capacidad con antelación, gobernar la distribución geográfica y operar con redundancia N+X y mecanismos de degradación elegante para manejar sobrecargas.

- Google Borg y SRE: la asignación dinámica de tareas a nivel de clúster, el balanceo global (GSLB) y la redundancia N+1/N+2 permiten absorber picos y fallos, reduciendo la probabilidad de caídas en cascada. [^17]
- Uber Capacity Recommendation Engine (CRE): un servicio interno de recomendación de capacidad que usa throughput por núcleo (TPC) y regresión lineal para estimar recursos por zona y recomendar capacidad de forma proactiva con guard rails de calidad. [^18]
- Netflix: para picos súbitos, combina prescaling programado, escalado reactivo por servicio y políticas de downscale prudentes para evitar costo y oscilación. [^16]

La Tabla 6 resume estos patrones.

Tabla 6. Patrones comparados: Google (Borg/SRE), Uber (CRE), Netflix (picos súbitos)

| Dimensión                 | Google (Borg/SRE)                                 | Uber (CRE)                                                   | Netflix                                              |
|--------------------------|----------------------------------------------------|--------------------------------------------------------------|------------------------------------------------------|
| Filosofía                | Asignación dinámica + GSLB + N+1/N+2             | Datos‑dirigida: TPC + regresión + guard rails                | Prescaling + reactivo + downscale controlado         |
| Señales de control       | QPS/capacidad declarada por servicio               | RPS, utilización, throughput por núcleo                      | Tráfego y latencia por servicio                      |
| Predictivo vs Reactivo   | Predictivo de capacidad y redundancia              | Predictivo (recomendación), extensible a reactivo horario    | Ambos: prescaling predictivo y reactivo por servicio |
| Gestión de picos         | Redundancia, degradación elegante, GSLB           | Estimación de picos por zona con guard rails                 | Spikes súbitos amortiguados por prescaling           |

Fuentes: libro SRE, blog de Uber y sesión re:Invent sobre Netflix. [^17][^18][^16]

### Google (Borg/SRE)

Borg orquesta la asignación de tareas con bin‑packing consciente de dominios de fallo; GSLB dirige tráfico al datacenter con capacidad disponible. La política de redundancia N+X mitiga picos y mantenimiento planificado; la degradación elegante evita fallas en cascada. [^17]

### Uber (CRE)

CRE estima la capacidad objetivo con una relación lineal entre utilización y throughput por núcleo (TPC), derivada de descomposición de series temporales (tendencia y estacionalidad). Recomienda núcleos totales e instancias por zona, con guard rails que verifican calidad del modelo y límites operativos. [^18]

### Netflix

Para picos súbitos, Netflix aplica prescaling antes del evento (lanzamientos, picos de audiencia), escalado reactivo por servicio, y downscale gradual para contener costos y estabilidad. [^16]

## Métricas, observabilidad y gobernanza del auto‑scaling

La elasticidad eficaz exige medir lo correcto, con granularidad suficiente, y gobernar decisiones:

- Métricas clave:
  - CPU/memoria: presión de recursos a nivel de pod/VM.
  - Latencia p95/p99: experiencia de usuario y saturación del sistema.
  - Throughput (RPS/QPS): demanda de negocio; base de dimensionamiento en servicios sin estado. [^17]
  - Colas/lag: acumulación de trabajo pendiente en sistemas event‑driven. [^8]

- Herramientas:
  - Kubernetes: Metrics Server para HPA; observabilidad con Prometheus/Grafana o APMs externos. [^14]
  - Cloud monitoring: CloudWatch (AWS), Azure Monitor, Cloud Monitoring (GCP) para reglas y paneles.
  - Alertas y cooldowns: evitar oscilación, escalado brusco y thundering herd. [^10][^6]

- Gobernanza:
  - Objetivos de utilización por servicio y por zona; cooldowns mínimos; límites máximos de réplicas e instancias.
  - Guard rails: validaciones de calidad de modelo (MAPE), límites de cambio por período, diferenciación por zona (eficiencia de hardware). [^18][^15]

La Tabla 7 mapea métricas por herramienta.

Tabla 7. Mapa de métricas y herramientas de observabilidad

| Capa                | Métricas principales                       | Herramientas típicas                         |
|---------------------|--------------------------------------------|----------------------------------------------|
| Aplicación          | Latencia p95/p99, throughput (RPS), errores | APM/Tracing, Prometheus, Cloud Monitoring    |
| Pod/Container       | CPU/memoria, restarts                      | Metrics Server, Prometheus                   |
| VM/Instancia        | CPU, red, disco                            | CloudWatch Agent, Azure Diagnostics, GCP Agent |
| Event‑driven        | Longitud de cola, lag, mensajes/seg        | KEDA scalers, Prometheus, cloud queue stats  |

Fuentes: documentación de Kubernetes y de cada proveedor. [^14][^7][^8][^10][^6]

## Diseño de estrategias híbridas por escenarios

Para acelerar la adopción, proponemos cuatro plantillas:

1) API sin estado, variación intradía, latencia sensible:
- Base reactiva: HPA por latencia/CPU; KEDA si hay bursts HTTP/eventos.
- Infraestructura: CA para nodos; políticas reactivas en ASG/VMSS/MIG.
- Híbrido: si hay patrones diarios claros, añadir predictivo a nivel de infraestructura; ajustar cooldowns para evitar oscilación. [^7][^8][^9][^10][^6]

2) Cargas event‑driven (colas, streams), latencia de procesamiento sensible:
- KEDA con triggers de cola/stream; HPA para consolidar métricas de CPU y evitar “spiky” solo en eventos.
- VPA para reducir sobre‑asignación y mejorar densidad por pod. [^8][^11][^9]

3) Batch/de datos, workloads con warm‑up alto:
- Predictivo en infraestructura para preaprovisionar capacidad antes de ventanas (noches, fines de semana, jobs).
- Escalado programado adicional como salvaguarda; reactivo mínimo para incidentes. [^4][^5][^6]

4) Híbrido multicloud/Kubernetes:
- Gobernanza central de límites por zona y cooldowns; separación de autoridad predictiva (infra) vs reactiva (app).
- Métricas armonizadas y paneles de costo/latencia; revisiones semanales. [^20][^7][^6]

La Tabla 8 ofrece una guía de selección.

Tabla 8. Guía de selección por carga y señal dominante

| Tipo de carga                | Señal dominante         | Herramienta base                 | Complementos predictivos/híbridos             |
|-----------------------------|-------------------------|----------------------------------|-----------------------------------------------|
| API sin estado              | Latencia, RPS           | HPA (latencia/CPU)               | Predictivo infra + CA                         |
| Event‑driven (colas/streams)| Lag/longitud de cola    | KEDA (trigger cola)              | HPA reactivo + VPA para densidad              |
| Batch con warmup alto       | Horarios (cadencia)     | Programado + predictivo infra    | Reactivo mínimo (emergencias)                 |
| Tráfico cíclico (web)       | CPU + patrones diarios  | HPA + KEDA opcional              | Predictivo en MIG/VMSS/ASG                    |

Fuentes: documentación de Kubernetes y proveedores. [^7][^8][^10][^6][^9]

## Riesgos, limitaciones y mitigaciones

- Cold starts y latencia en picos:
  - Mitigar con predictivo (warmup) y cooldowns; dimensionar imágenes y warm‑up scripts; usar balanceo regional con capacidad de reserva. [^6][^5][^4]

- Sobre/subaprovisionamiento:
  - Revisar objetivos de utilización; combinar predictivo con reactivo a nivel de aplicación; aprovechar escalado a cero en cargas event‑driven; usar VPA para ajustar tamaño. [^8][^1][^19]

- Deriva de modelos predictivos:
  - Vigilar MAPE; recalibrar políticas; validar calidad de modelo y activar guard rails que pauten decisiones si empeora el ajuste. [^15][^18]

- Multi‑tenant y contención de recursos:
  - Políticas por espacio de nombres; límites por equipo/servicio; CA/Karpenter con pools dedicados; gobernanza de cooldowns y límites máximos por租户. [^1][^14]

## Roadmap de implementación y checklist operativo

Proponemos una adopción en cinco fases:

- Fase 1: Línea base reactiva. HPA en servicios críticos; CA activo; observabilidad básica (latencia, CPU, RPS). [^7][^1]
- Fase 2: Eventos y escala a cero. Integrar KEDA para cargas con colas/streams; definir límites por servicio y cooldowns. [^8][^9][^11]
- Fase 3: Predictivo en infraestructura. Habilitar AWS Predictive, Azure Predictive o GCP Predictive según patrones y métricas soportadas; configurar warmup y stabilization. [^4][^5][^6]
- Fase 4: Optimización vertical. Desplegar VPA en modos controlados (Initial/Off → Auto/Recreate), con guard rails y ventanas de mantenimiento. [^19]
- Fase 5: Gobernanza yFinOps. Umbrales por servicio, límites por zona, paneles de costo/latencia/SLA, revisiones semanales y políticas de escala‑in prudentes. [^10][^6]

La Tabla 9 es un checklist práctico por fase.

Tabla 9. Checklist por fase

| Fase | Configuraciones y validaciones clave                                                                 | Señales y monitoreo                                    |
|------|-------------------------------------------------------------------------------------------------------|--------------------------------------------------------|
| 1    | HPA por latencia/CPU; CA habilitado; límites max/replicas; cooldowns                                 | Latencia p95/p99, CPU, RPS; alertas por saturación     |
| 2    | KEDA triggers (colas/HTTP); TriggerAuthentication; política de scale‑out/in; límites por servicio     | Lag de cola, mensajes/seg, errores por pico            |
| 3    | Predictive en ASG/VMSS/MIG; warmup/cool‑down; combinación con reactivo                                | CPU (GCP), métricas personalizadas (AWS/Azure), MAPE   |
| 4    | VPA en Initial/Off → Auto/Recreate; guard rails; límites de cambio por período                        | Utilización por contenedor, restarts, jitter           |
| 5    | Gobernanza: límites por zona; FinOps; política de scale‑in; revisión semanal de desempeño y costo     | Costo por transacción, SLA, error budget               |

Fuentes: documentación de Kubernetes y proveedores. [^7][^1][^8][^10][^6]

## Conclusiones y recomendaciones

- El reactivo es la base indispensable de la elasticidad; el predictivo aporta ventaja cuando hay estacionalidad y warmup; el híbrido maximiza disponibilidad y costo.
- Kubernetes ofrece un andamiaje robusto (HPA, VPA, KEDA, CA) que, combinado con autoscaling nativo cloud, permite escalar desde la aplicación hasta la infraestructura.
- Recomendación: adoptar híbrido progresivo, con gobernanza de límites y cooldowns, métricas de “señales doradas”, y revisiones sistemáticas de desempeño y costo. Aprovechar predictivo para cargas cíclicas y con cold starts, mantener reactivo como red de seguridad, y optimizar tamaño con VPA.
- Siguientes pasos: ejecutar el roadmap por fases, instrumentar paneles de latencia/RPS/costo, y establecer comités de revisión (SRE/Plataforma/FinOps) con cadencia semanal para ajustar políticas y evitar deriva.

## Lagunas de información y consideraciones

- Netflix: no hay detalles públicos completos sobre el prescaling más allá de la sesión de re:Invent; el benchmarking cuantitativo (ahorro de costo, latencia) es limitado. [^16]
- Kubernetes: el soporte y estabilidad de redimensionamiento in‑situ de VPA evoluciona; validar en la versión objetivo. [^19][^1]
- Métricas personalizadas predictivas: la cobertura específica (p. ej., soporte de RPS en GCP predictive) no está documentada en las fuentes empleadas; assume CPU‑only. [^6]
- Benchmarks independientes comparando HPA vs KEDA por tipo de carga son escasos en las referencias; usar pilotos controlados para validar. [^9][^11]
- Uber: detalles de despliegue en Kubernetes del CRE y su interacción con autoscalers nativos no están disponibles en las fuentes; inferir arquitectura general. [^18]

---

## Referencias

[^1]: Autoscaling de cargas de trabajo - Documentación de Kubernetes. https://kubernetes.io/docs/concepts/workloads/autoscaling/

[^2]: CloudZero. Horizontal Vs. Vertical Scaling: Which Should You Choose? https://www.cloudzero.com/blog/horizontal-vs-vertical-scaling/

[^3]: Google SRE Book. Best practices for production environment. https://sre.google/sre-book/production-environment/

[^4]: AWS EC2 Auto Scaling: Predictive scaling. https://docs.aws.amazon.com/autoscaling/ec2/userguide/ec2-auto-scaling-predictive-scaling.html

[^5]: Microsoft Learn. Use predictive autoscale to scale out before load demands in virtual machine scale sets. https://learn.microsoft.com/en-us/azure/azure-monitor/autoscale/autoscale-predictive

[^6]: Google Cloud Compute Engine. Predictive autoscaling for managed instance groups. https://docs.cloud.google.com/compute/docs/autoscaler/predictive-autoscaling

[^7]: Kubernetes. Horizontal Pod Autoscaling (walkthrough y referencia). https://kubernetes.io/docs/tasks/run-application/horizontal-pod-autoscale/

[^8]: KEDA - Kubernetes Event-Driven Autoscaling. https://keda.sh/

[^9]: Rafay. Comparing HPA and KEDA: Choosing the Right Tool for Kubernetes Autoscaling. https://docs.rafay.co/blog/2025/05/20/comparing-hpa-and-keda-choosing-the-right-tool-for-kubernetes-autoscaling/

[^10]: Azure Virtual Machine Scale Sets: Overview of autoscale. https://learn.microsoft.com/en-us/azure/virtual-machine-scale-sets/virtual-machine-scale-sets-autoscale-overview

[^11]: Spectro Cloud. Kubernetes autoscaling patterns: HPA, VPA and KEDA. https://www.spectrocloud.com/blog/kubernetes-autoscaling-patterns-hpa-vpa-and-keda

[^12]: Azure VMSS: Use scale-in policies. https://learn.microsoft.com/en-us/azure/virtual-machine-scale-sets/virtual-machine-scale-sets-scale-in-policy

[^13]: Azure Monitor: Get started with autoscale. https://learn.microsoft.com/en-us/azure/azure-monitor/autoscale/autoscale-get-started

[^14]: Kubernetes Metrics Server (GitHub). https://github.com/kubernetes-sigs/metrics-server

[^15]: Hokstad Consulting. Predictive scaling with machine learning: how it works. https://hokstadconsulting.com/blog/predictive-scaling-with-machine-learning-how-it-works

[^16]: AWS re:Invent 2024. How Netflix handles sudden load spikes in the cloud (NFX301). https://reinvent.awsevents.com/content/dam/reinvent/2024/slides/nfx/NFX301_How-Netflix-handles-sudden-load-spikes-in-the-cloud.pdf

[^17]: Google SRE Book. The four golden signals: monitoring distributed systems. https://sre.google/sre-book/monitoring-distributed-systems/#xref_monitoring_golden-signals

[^18]: Uber Engineering Blog. Capacity Recommendation Engine: Throughput and Utilization. https://www.uber.com/blog/capacity-recommendation-engine/

[^19]: Vertical Pod Autoscaler - Repositorio oficial. https://github.com/kubernetes/autoscaler/tree/9f87b78df0f1d6e142234bb32e8acbd71295585a/vertical-pod-autoscaler

[^20]: Google Cloud. Compare AWS and Azure services to Google Cloud. https://docs.cloud.google.com/docs/get-started/aws-azure-gcp-service-comparison