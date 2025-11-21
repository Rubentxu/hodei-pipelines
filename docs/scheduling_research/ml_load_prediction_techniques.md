# Técnicas de ML para predicción de carga en sistemas distribuidos: forecasting, reconocimiento de patrones, detección de anomalías y reinforcement learning para scheduling

## Resumen ejecutivo

La predicción de carga y la planificación proactiva de recursos se han convertido en pilares para la operación de sistemas distribuidos modernos. En entornos de cloud computing, pipelines de integración y entrega continua (CI/CD) y sistemas de procesamiento por lotes, las cargas exhiben patrones complejos, correlaciones cruzadas y dinámicas no estacionarias que estresan los enfoques puramente reactivos. Este informe sintetiza cuatro familias de técnicas de aprendizaje automático (ML) —pronóstico de series temporales (ARIMA, Prophet, LSTM), reconocimiento de patrones (clustering, extracción de características), detección de anomalías (logs, métricas, red/IoT) y aprendizaje por refuerzo (RL) para scheduling— y propone una arquitectura de referencia para integrarlas en plataformas existentes.

Los hallazgos clave son tres. Primero, el pronóstico de series temporales sigue siendo la base de la planificación proactiva: los modelos autoregresivos y de descomposición son sólidos para señales estables, mientras que los modelos de aprendizaje profundo —en particular redes Long Short-Term Memory (LSTM) y arquitecturas híbridas LSTM–Transformer— aportan capacidad de modelar dependencias largas y no linealidades, con mejoras consistentes cuando se combinan con descomposición y selección de características[^1][^13][^16][^17]. Segundo, la detección de anomalías, tanto en logs como en tráfico de red/IoT, eleva la robustez operacional al reducir falsos positivos mediante arquitecturas híbridas reglas+ML y al capturar desviaciones sutiles con modelos especializados (p. ej., redes con atención para logs)[^6][^7][^8][^9]. Tercero, el RL aplicado a scheduling y autoscaling —y a decisiones finas como el dimensionamiento de lotes en entrenamiento distribuido— entrega adaptatividad y optimiza objetivos compuestos (latencia, coste, energía, utilización), mostrando ventajas sobre heurísticas estáticas, especialmente bajo heterogeneidad y dinámicas cambiantes[^5][^18][^19][^20][^21][^3].

La recomendación principal es evolucionar desde mecanismos reactivos (reglas/umbrales) hacia una arquitectura de bucle cerrado que combine pronóstico proactivo, detección de anomalías y control/autoscaling con RL, apoyados por telemetría unificada y MLOps disciplinado. Los riesgos más relevantes —deriva de datos y concepto, sesgos de medición, estabilidad de políticas de RL y coste operacional— se mitigan mediante guardas operacionales, aprendizaje continuo y políticas conservadoras en la fase de producción[^2][^4].

## Contexto y problema: carga en sistemas distribuidos

La “carga” en sistemas distribuidos abarca múltiples dimensiones: utilización de CPU y memoria, throughput de red, latencias de servicio, colas de peticiones, consumo energético y, en contextos de CI/CD, tiempos de build y duración de jobs de entrenamiento. En cloud computing, estas señales sustentan decisiones de autoscaling (horizontal y vertical) y orquestación; en CI/CD, determinan ventanas de ejecución, asignación de runners y colas de pruebas; y en procesamiento por lotes, guían la secuenciación y el reparto de recursos entre jobs.

Las plataformas enfrentan retos estructurales: estacionalidad marcada (ciclos diarios/semanales), bursts por despliegues o eventos, dependencia entre microservicios que amplifica perturbaciones y efectos de cola, y una creciente heterogeneidad de hardware y redes. En este panorama, la adopción de ML para “anticipar” la carga y automatizar decisiones se justifica por tres razones. Primero, los enfoques reactivos —reglas y umbrales— son simples pero tienden a llegar tarde ante picos, produciendo sobre/infra-provisión y degradación de SLOs. Segundo, la combinación de pronóstico y control permite absorber variaciones con holguras razonables, reduciendo coste y energía[^2][^4]. Tercero, los objetivos operativos modernos son multi-dimensionalmente óptimos (latencia, coste, energía, utilización), lo que favorece políticas aprendidas sobre reglas estáticas.

Para visualizar la relación entre métricas de carga y decisiones operativas, la Tabla 1 propone un mapa de señales y objetivos.

Tabla 1. Mapa de métricas de carga y objetivos operativos

| Dimensión de carga       | Métricas principales                          | Objetivo operativo asociado                                 |
|--------------------------|-----------------------------------------------|--------------------------------------------------------------|
| Compute (CPU/GPU)        | Utilización, run queue, Throttling            | Latencia de respuesta; utilización eficiente                 |
| Memoria                  | Uso, paging, swaps                            | Estabilidad y throughput; evitar OOM                         |
| Red                      | Throughput, retransmisiones, latencias        | Throughput end-to-end; calidad de enrutamiento               |
| Almacenamiento           | IOPS, latencias de disco                      | SLO dejobs I/O-bound; consistencia y durabilidad             |
| Energía                  | Potencia, energía acumulada                   | Coste y sostenibilidad (carbono intensity-aware)             |
| CI/CD (build/train)      | Duración de etapas, colas, retries            | Throughput de despliegues; fiabilidad de pipelines           |

Este mapa subraya que el pronóstico de señales clave y la detección de anomalías deben integrarse en un bucle de control que traduzca predicciones en acciones de scheduling y autoscaling bajo restricciones de SLOs[^2][^4].

## Pronóstico de series temporales para predicción de carga

Las series de carga en plataformas distribuidas suelen presentar tendencias, estacionalidades múltiples, ruido y cambios de régimen. La selección de técnicas debe considerar la estacionariedad, la granularidad (segundos/minutos/horas), la longitud del horizonte (corto/medio plazo), la interpretabilidad y los costes de operación y mantenimiento (TCO). En términos generales:

- ARIMA y variantes son eficaces cuando la serie es relativamente estable y estacionaria tras diferenciación, con ciclos bien definidos; su coste computacional es moderado y la interpretabilidad es alta, pero sufren en no linealidades y dependencias largas[^15][^16].
- Prophet, al capturar estacionalidades y cambios de tendencia con regresores simples, ofrece robustez y facilidad de configuración en entornos operativos con patrones calendar-driven; su rendimiento puede degradarse bajo no linealidades fuertes y acoplamientos complejos[^15][^16].
- LSTM y modelos híbridos (LSTM–Transformer, descomposición+ML) destacan en dependencias temporales largas, efectos cruzados y señales ruidosas, especialmente cuando se enriquecen con selección de características y descomposición en frecuencia, con resultados superiores en diversas escalas de tiempo[^1][^13][^14][^17].

Para orientar la elección, la Tabla 2 resume una comparativa cualitativa.

Tabla 2. Comparativa ARIMA vs Prophet vs LSTM/híbridos

| Criterio                       | ARIMA                                 | Prophet                                     | LSTM/Híbridos (LSTM, LSTM+FDD, LSTM–Transformer)                     |
|-------------------------------|----------------------------------------|---------------------------------------------|------------------------------------------------------------------------|
| Supuestos/estacionariedad     | Requiere estacionariedad (diferenciación) | Flexible a estacionalidades y cambios de tendencia | No requiere estacionariedad; modela no linealidades y dependencias largas |
| Manejo de estacionalidad      | Manual (seasonal ARIMA)                | Automática (múltiples saisonalidades)        | Aprende patrones complejos, incluidos efectos calendario                |
| Robustez al ruido             | Moderada                               | Buena (por regresores simples)               | Alta con descomposición y regularización                                |
| Interpretabilidad             | Alta                                   | Alta                                        | Media-baja (depende de arquitectura y técnicas de explicación)          |
| Horizonte de predicción       | Corto-medio                            | Corto-medio                                  | Corto-largo (mejor con dependencias largas)                             |
| Coste computacional           | Bajo-medio                             | Bajo                                        | Medio-alto (entrenamiento y tuning)                                     |
| Requisitos de datos           | Bajos-medios                           | Bajos                                        | Medios-altos (volumen, variables exógenas)                              |
| Riesgos                       | Fragilidad ante cambios de régimen     | Menor sensibilidad a no linealidades        | Sobreajuste, coste de operación, estabilidad en producción              |

Más allá de la teoría, la evidencia empírica en dominios de carga y voltaje muestra que arquitecturas LSTM enriquecidas con descomposición en el dominio de la frecuencia (FDD) y selección de características (XGBoost) reducen el error absoluto medio (MAE) respecto a RNNs y SVMs en escalas de 1h a 4h, manteniendo robustez entre estaciones[^1]. La Tabla 3 sintetiza estos resultados.

Tabla 3. Resultados empíricos (MAE) LSTM+FDD vs RNN/SVM en pronósticos a 1h y 4h

| Escala temporal | FDD+LSTM (MAE) | RNN (MAE) | SVM (MAE) |
|-----------------|-----------------|-----------|-----------|
| 1h              | 0.4554          | 0.535     | 1.012     |
| 4h              | 1.085           | —         | —         |

La mejora del MAE, especialmente a 1h, indica que la descomposición de señales en bandas de frecuencia y el uso de selección de características ayudan a separar tendencias previsibles de fluctuaciones irregulares, reforzando la capacidad del modelo para generalizar en horizontes cortos y medios[^1]. En paralelo, estudios comparativos en cargas eléctricas concluyen que modelos LSTM y sus variantes superan a técnicas clásicas en precisión, proporcionando base para decisiones de provisión y control de calidad del servicio[^17].

### ARIMA y Prophet

Los modelos ARIMA y Prophet se recomiendan cuando las series exhiben estacionalidades claras y cambios de tendencia relativamente suaves. Sus ventajas radican en la rapidez de ajuste, la interpretabilidad y el menor coste operacional. En cloud computing, donde métricas como CPU, memoria y latencia pueden ser calendar-driven (picos office-hours, ventanas de mantenimiento), Prophet ha probado ser útil para escalado predictivo en contenedores y pods, con evidencias de que supera a modelos alternativos en escenarios de seguimiento de capacidad y ajuste de escalado[^15][^16]. Sin embargo, ante patrones más complejos —acoplamientos entre microservicios, bursts por despliegues concurrentes o dependencias largas— estos modelos pueden subestimar la volatilidad, por lo que conviene combinarlos con detectores de anomalías y heurísticas de seguridad.

### LSTM e híbridos (LSTM–Transformer, FDD+LSTM)

Las redes LSTM capturan dependencias largas y efectos no lineales, y su rendimiento mejora al introducir descomposición en frecuencia (FDD) y selección de características (XGBoost), tal como se demostró en contextos de carga/voltaje de redes de distribución, con reducciones sustantivas del MAE respecto a RNNs y SVMs[^1]. Arquitecturas que combinan LSTM con mecanismos de atención y Transformers (LSTM–Transformer) aportan capacidades adicionales para series con patrones de largo alcance y cambios abruptos, balanceando memoria temporal y enfoque en segmentos relevantes[^13]. La literatura reciente subraya que modelos de memoria como LSTM y sus extensiones son particularmente efectivos cuando se cuenta con suficientes datos, etiquetas y variables exógenas (p. ej., calendario, despliegues, colas), y cuando el horizonte de predicción requiere capturar efectos rezagados y estacionalidades superpuestas[^14][^17].

En producción, la operación es tan importante como el modelo: pipelines de MLOps deben automatizar el retraining ante drift, la evaluación continua y el despliegue seguro (canary, blue/green, shadow), junto con guardas operacionales que limiten decisiones de escalado extremas y protejan SLOs[^11][^2][^15].

## Reconocimiento de patrones y extracción de características

La predicción de carga rara vez se limita a una sola serie univariante. En plataformas reales, coexisten múltiples señales (CPU, memoria, throughput de red, colas, latencias, tiempos de build) con correlaciones cruzadas y dependencias temporales. El reconocimiento de patrones —clustering y extracción de características— sirve para segmentar comportamientos (estables, diarios, semanales, bursts), identificar regímenes y detectar cambios de distribución (concept drift) que impactan la precisión de los modelos y la estabilidad del control.

Para cargas cloud, la extracción de características temporales (lagos, medias móviles, medidas de volatilidad), variables categóricas (servicio, entorno, región), y señales de infraestructura (uso de CPU/memoria, latencias, longitud de cola, duración de jobs) permite alimentar modelos de pronóstico y controladores de autoscaling con información rica. Estudios sobre forecasting multi-time series para optimización de recursos en la nube muestran que el tratamiento conjunto de múltiples series y la integración de señales operativas mejora la eficiencia del provisionamiento y el equilibrio de cargas[^15].

A continuación, la Tabla 4 ofrece una taxonomía de características útiles y su procedencia.

Tabla 4. Taxonomía de features para carga en sistemas distribuidos

| Tipo de feature                     | Ejemplos                                     | Procedencia                            |
|------------------------------------|----------------------------------------------|----------------------------------------|
| Temporales                         | Lags, medias móviles, volatilidad, estacionalidad | Series de métricas (CPU, memoria, latencias) |
| Categóricas                        | Servicio, región, entorno (prod/stage), tipo de instancia | Metadatos de despliegue y orquestación |
| Infraestructura                    | Utilización CPU/memoria, cola de tareas, retransmisiones de red | Telemetría de plataforma (Kubernetes, VMs) |
| CI/CD                              | Duración de build/test, número de jobs, retries, colas | Orquestador de pipelines (GitHub Actions, Jenkins) |
| Energía y coste                    | Potencia, coste por hora, intensidad de carbono | Monitores energéticos y proveedores cloud |
| Red                                | Throughput, latencias, pérdida de paquetes   | Observabilidad de red (SDN, switches)  |

En la práctica, pipelines de MLOps proveen el esqueleto operativo para calcular, almacenar y servir estas características a los modelos y controladores, asegurando consistencia entre entrenamiento e inferencia[^11][^12].

## Detección de anomalías en sistemas distribuidos

La detección de anomalías complementa el pronóstico, asegurando que picos, caídas o comportamientos atípicos no tomen por sorpresa a los controladores. La taxonomía distingue anomalías puntuales (spikes), contextuales (desviaciones condicionadas por contexto, como hora o servicio) y colectivas (secuencias anómalas, patrones infrecuentes). En sistemas distribuidos, las fuentes más relevantes son logs de aplicaciones y componentes (servidores, bases de datos, balanceadores), métricas de infraestructura y telemetría de red/IoT.

La evidencia empírica en log-based anomaly detection muestra que modelos basados en redes neuronales profundas (p. ej., LSTM) y arquitecturas con atención temporal/lógica mejoran la detección de dependencias complejas entre eventos, reduciendo falsos positivos y elevando la precisión en escenarios reales (HDFS, BGL)[^6][^9]. En telemetría de red/IoT, la arquitectura recomendada combina un Módulo Inteligente de Detección de Anomalías (IADM) con enfoques híbridos reglas+ML para balancear sensibilidad y especificidad, y para operar con baja sobrecarga en dispositivos y gateways con recursos limitados[^7]. En grandes sistemas cloud, la disponibilidad de datasets recientes facilita el benchmarking y la transferencia de métodos entre dominios, aunque la diversidad de cargas y la presencia de ruido y drift exigen diseños adaptativos[^8].

La Tabla 5 resume, con datos del informe ITU-T, el desempeño de algoritmos ML sobre el dataset Bot-IoT 2018.

Tabla 5. Desempeño en Bot-IoT 2018 (IADM, ITU-T 07/2024)

| Algoritmo     | Accuracy | Precision (macro) | Recall (macro) | F1-score (macro) | CV F1 (macro) |
|---------------|----------|-------------------|----------------|------------------|---------------|
| Random Forest | 1.00     | 1.00              | 1.00           | 1.00             | 99.91         |
| LightGBM      | 1.00     | 1.00              | 1.00           | 1.00             | 49.31         |
| KNN (k=5)     | 1.00     | 0.96              | 0.90           | 0.93             | 92.72         |
| AdaBoost      | 1.00     | 0.96              | 0.90           | 0.93             | 99.74         |

Los resultados muestran que Random Forest y AdaBoost alcanzan F1 macro cercanos a 1.00 con validación cruzada elevada, mientras que KNN ofrece un balance intermedio; LightGBM, en este escenario y configuración, presenta una caída notable en CV F1, señalando la sensibilidad a hiperparámetros y la necesidad de ajuste cuidadoso[^7]. Para plataformas cloud, la Tabla 6 describe datasets y escenarios de evaluación.

Tabla 6. Datasets en sistemas cloud (arXiv 2411.09047)

| Dataset (arXiv 2411.09047) | Dominio           | Métricas                         | Desafíos principales                           |
|----------------------------|-------------------|----------------------------------|-----------------------------------------------|
| Conjuntocloud2024 (ej.)    | Cloud (multi-servicio) | Latencia, throughput, uso CPU/memoria | Ruido, heterogeneidad, drift de datos y concepto |

En producción, la arquitectura debe integrar detección de anomalías en el bucle de decisión: si una desviación significativa se detecta, se ajustan holguras de escalado, se prioriza el tráfico o se aplican políticas conservadoras mientras se resuelve la causa raíz[^8].

### Logs y flujos de eventos

Los logs condensan información valiosa sobre estados y transiciones en sistemas distribuidos. Modelos LSTM y redes con atención temporal/lógica capturan dependencias entre eventos y patrones secuenciales infrecuentes, elevando la detección de anomalías sobre enfoques basados en reglas o contadores simples[^6][^9]. Las buenas prácticas incluyen normalización de formatos, etiquetado controlado y validación cruzada, considerando la sobrecarga operacional y la necesidad de monitoreo continuo.

### Telemetría de red/IoT

El enfoque híbrido reglas+ML del IADM propuesto por la ITU-T combina un detector basado en reglas para clasificaciones inmediatas y un clasificador ML que refina la decisión, reduciendo falsos positivos/negativos. Los requisitos de un sistema así incluyen baja sobrecarga, escalabilidad, autogestión y capacidad de localización de fuentes de anomalías, fundamentales en entornos IoT heterogéneos[^7].

## Aprendizaje por refuerzo (RL) para scheduling y autoscaling

El RL formaliza la toma de decisiones secuencial en entornos dinámicos, donde el agente observa estados, ejecuta acciones y recibe recompensas, con el objetivo de maximizar retorno acumulado. En scheduling y autoscaling, el estado puede incluir métricas de carga y recursos, la acción corresponde a asignar tareas, ajustar réplicas o dimensionar lotes, y la recompensa integra objetivos compuestos (latencia, coste, utilización, energía).

Las ventajas del RL incluyen la adaptación a espacios continuos y multidimensionales, la capacidad de aprender políticas contra heuristics estáticas y la flexibilidad para incorporar objetivos complejos. Sin embargo, la estabilidad del entrenamiento y la explicabilidad son desafíos, especialmente en entornos de producción con SLOs estrictos[^5][^2].

La Tabla 7 sintetiza algoritmos RL y su idoneidad.

Tabla 7. Algoritmos RL para scheduling/autoscaling

| Algoritmo            | Estado/Acción típicos             | Objetivos optimizados                 | Idoneidad por escenario                                  |
|----------------------|-----------------------------------|---------------------------------------|----------------------------------------------------------|
| DQN/DDQN             | Estado tabular o embedding; acción discreta (asignación/ escalado) | Latencia, makespan, coste            | Tareas independientes; acciones discretas; estabilidad mayor |
| A3C/A2C              | Estados continuos; acciones continuas o discretas | Latencia, utilización, energía        | Entornos con variabilidad; paralelismo de agentes         |
| DDPG                 | Estados continuos; acción continua (p. ej., factor de escalado) | Latencia y coste con control fino     | Ajuste fino de recursos; control continuo                 |
| PPO                  | Estados continuos; acción continua/discreta | Compuestos (multi-objetivo)           | Políticas estables; multi-objetivo y producción           |
| Multi-Agent RL (MARL)| Estados locales por nodo; acciones cooperativas/competitivas | Objetivos globales/locales            | Clusters y microservicios con coordinación distribuida    |

El estado del arte documenta que el RL supera a algoritmos clásicos en múltiples escenarios de scheduling en la nube, mejorando latencia y utilización, y permitiendo entrenar políticas con percepción del entorno y objetivos compuestos[^5].

### Formulación y objetivos de scheduling

La formulación matemática clásica define matrices de asignación (qué tarea va a qué recurso) y de tiempos de inicio, con objetivos como minimizar el makespan (tiempo de finalización máximo), el consumo de energía y la varianza de carga. En cloud, las variables incluyen dimensiones de recursos (CPU, RAM, GPU, disco, ancho de banda), características de tareas y estado de nodos. La Tabla 8 resume estas variables y objetivos.

Tabla 8. Variables y objetivos de scheduling

| Elemento         | Descripción                                                      |
|------------------|------------------------------------------------------------------|
| Tareas (M)       | Conjunto de tareas i con parámetros V_i (recursos requeridos)   |
| Nodos (N)        | Conjunto de nodos j con estado de carga L_j                     |
| Asignación (X)   | Matriz x_ij ∈ {0,1} (tarea i a nodo j)                           |
| Tiempos (S)      | Matriz de tiempos de inicio                                      |
| Objetivos        | min makespan, min energía, min varianza de carga, min tiempo total |

Esta formulación permite mapear algoritmos RL a decisiones concretas de asignación y secuenciación, optimizando una combinación ponderada de objetivos según políticas y restricciones[^5].

### Autoscaling proactivo con RL

En autoscaling, el agente RL ajusta horizontal/verticalmente recursos de microservicios, aprendiendo a anticipar picos y a equilibrar SLOs y coste. La evidencia indica que los enfoques proactivos con RL superan a los reactivos (reglas/umbrales) en métricas de recompensa y utilización, particularmente en entornos con variabilidad y dependencias complejas[^2]. Las guardas operacionales —límites de escalado, tasas máximas de cambio, ventanas de cooldown— y la monitorización de SLOs son imprescindibles para operar de forma segura.

### RL para entrenamiento distribuido (DYNAMIX)

El framework DYNAMIX plantea una aplicación singularmente práctica del RL: la optimización adaptativa del tamaño de lote por nodo durante entrenamiento distribuido, formulando el problema como toma de decisiones secuencial y utilizando PPO para ajustar el batch size basándose en métricas de red, sistema y eficiencia estadística[^3]. Los resultados muestran mejoras simultáneas en precisión final y tiempo de convergencia, con overhead marginal.

La Tabla 9 detalla resultados en escenarios representativos.

Tabla 9. Resultados DYNAMIX vs batch estático

| Caso                                    | Batch estático             | DYNAMIX                         | Mejora                     |
|-----------------------------------------|-----------------------------|---------------------------------|----------------------------|
| VGG16/CIFAR-10, 8 nodos, SGD            | 85.3% precisión, 853s       | 91.3% precisión, 652s           | −30.1% tiempo              |
| VGG16/CIFAR-10, 16 nodos, SGD           | 83.4% precisión, 543s       | 91.5% precisión, 479s           | −13.7% tiempo              |
| VGG16/CIFAR-10, 32 nodos, SGD           | 81.3% precisión, 734s       | 92.6% precisión, 421s           | −42.6% tiempo              |
| VGG11/SGD (comparable precisión)        | 190 min (estático)          | 30 min (DYNAMIX)                | 6.3× más rápido            |
| VGG11/Adam (precisión superior)         | 80 min (estático)           | 30 min (DYNAMIX, +6% precisión) | 2.67× más rápido           |

El patrón de adaptación observado —de lotes grandes a medianos y pequeños— se alinea con teorías sobre el ruido del gradiente y su papel en la convergencia; la integración de métricas de red y sistema (p. ej., retransmisiones, tiempo de iteración, variación de gradientes) aporta señales efectivas para el agente[^3]. La arquitectura se apoya en instrumentación ligera (eBPF) y comunicación eficiente (gRPC), con soporte para paradigmas de entrenamiento BSP y servidores de parámetros.

## Casos de uso y patrones de implementación

La adopción debe partir de una arquitectura de referencia con módulos claramente definidos: forecasting (ARIMA/Prophet/LSTM), detección de anomalías (logs/red/IoT), reconocimiento de patrones y RL para scheduling/autoscaling. La integración con orquestadores (Kubernetes), pipelines de CI/CD y sistemas de observabilidad permite cerrar el bucle: predicción → decisión → acción → monitoreo → aprendizaje.

La Tabla 10 sintetiza la relación entre caso de uso, técnica y herramientas/plataformas.

Tabla 10. Matriz caso de uso → técnica → herramientas/plataformas

| Caso de uso                | Técnica principal                | Herramientas/plataformas                                 |
|---------------------------|----------------------------------|-----------------------------------------------------------|
| Cloud (autoscaling)       | Forecasting + anomalías + RL     | Kubernetes, Kubeflow, SageMaker, Vertex AI[^11][^12][^25][^26] |
| CI/CD (build/train)       | Forecasting de carga y colas; CT | GitHub Actions, Jenkins, MLflow, DVC, W&B[^11][^30][^31][^32] |
| Batch processing          | Forecasting + RL (si aplica)     | Frameworks de scheduling + observabilidad                 |

### Cloud computing

En cloud, la predicción multi-time series (CPU, memoria, latencia) alimenta decisiones de escalado horizontal y vertical. Estudios sobre forecasting multi-serie y autoscaling en Kubernetes demuestran la viabilidad del escalado proactivo con Prophet y LSTM, mejorando la eficiencia de recursos y el cumplimiento de SLOs[^15]. El balanceo de carga —dinámico y predictivo— se beneficia de señales de latencias y throughput para decisiones de enrutamiento; la literatura reciente presenta algoritmos distribuidos que optimizan la asignación bajo latencias variables y condiciones de red no homogéneas[^22][^23][^24][^25]. En planificación de capacity y FinOps, el pronóstico de coste combinado con métricas de uso guía compras de reservas y asignación de capacidad, con enfoques que integran objetivos de sostenibilidad[^25].

Tabla 11. Métricas cloud y estrategias de autoscaling

| Métrica      | Estrategia de autoscaling           | Modelo recomendado                     |
|--------------|-------------------------------------|----------------------------------------|
| CPU/memoria  | Horizontal/vertical basado en forecast | Prophet/LSTM con variables exógenas    |
| Latencia     | Ajuste de réplicas y rutas           | LSTM–Transformer para dependencias largas |
| Colas        | Escalado orientado a SLOs            | ARIMA para señales estabilizadas        |
| Coste/energía| Planificación multi-objetivo         | RL multi-objetivo (A3C/PPO)             |

### CI/CD pipelines

En CI/CD, el forecasting de duración de builds y etapas, junto con la gestión de colas de tests, permite planificar recursos, evitar cuellos de botella y mejorar throughput. El Entrenamiento Continuo (CT) se integra con CI/CD para retraining automatizado ante drift o degradación de rendimiento; plataformas como Kubernetes con Kubeflow, SageMaker y Vertex AI ofrecen operadores de entrenamiento y orquestación para escalar jobs en GPU/TPU[^11][^12][^25][^26]. La detección de anomalías en pipelines (fallos de build, timeouts) combinada con pruebas A/B y despliegues controlados (canary, blue/green, shadow) permite introducir nuevas versiones con riesgo mínimo[^11][^28].

Tabla 12. CI/CD: triggers → acciones → recursos

| Trigger de CT/CD                 | Acción principal                       | Recursos requeridos                 |
|----------------------------------|----------------------------------------|-------------------------------------|
| Post-CI de modelo                | Entrenar y validar                     | GPUs/TPUs; orquestación (Kubeflow)  |
| Programación (cron)              | Retraining y evaluación                | Capacidad gestionada (SageMaker/Vertex) |
| Degradación de KPIs              | Reentrenar + despliegue controlado     | Canary/shadow; monitoreo continuo   |
| Drift de datos                   | Actualizar features y reentrenar       | Feature store; control de versiones |

### Sistemas de procesamiento por lotes

En scheduling por lotes, la evidencia en pipelines multiproducto muestra que ML mejora la planificación y la monitorización de condiciones operativas, reduciendo tiempos de cálculo (p. ej., de horas a minutos en MILP con redes neuronales entrenadas sobre planes históricos) y elevando precisión en la detección de eventos operativos[^10]. La predicción de carga y de colas permite ajustar asignación de recursos y mitigar cuellos de botella; en algunos escenarios, decisiones finas como batch sizing pueden beneficiarse de RL, particularmente en entrenamientos distribuidos en clústeres heterogéneos[^3].

Tabla 13. Batch scheduling: tareas → técnicas → outputs

| Tarea                          | Técnica ML                           | Output esperado                      |
|--------------------------------|--------------------------------------|--------------------------------------|
| Planificación de lotes         | Forecasting + redes neuronales       | Plan optimizado en menor tiempo      |
| Seguimiento de interfaces      | Modelos físicos+ML (FDD/PINNs)       | Posición de interfaz en tiempo casi real |
| Monitorización operativa       | Clasificación (SVM/CNN/árboles)      | Detección y localización de eventos  |
| Asignación de recursos         | RL/heurísticas informadas por forecast | Secuenciación con SLOs y coste       |

## Métricas, evaluación y validación

La evaluación debe alinearse con objetivos operativos. Para pronóstico, se recomiendan MAE, MAPE, RMSE y SMAPE, además de latencia de inferencia y drift de datos/concepto. En detección de anomalías, precision, recall, F1-score (macro/micro) y tasas de falsos positivos/negativos, con validación cruzada y pruebas en datasets reales. En scheduling/autoscaling, métricas como makespan, tiempo total, latencia p95/p99, utilización de recursos y consumo de energía, y para CT/CD, KPIs de fiabilidad de pipelines (tasa de fallos, tiempos de etapa, throughput).

La Tabla 14 define las métricas clave y su aplicación.

Tabla 14. Definiciones de métricas y aplicación

| Métrica         | Definición breve                          | Aplicación principal                       |
|-----------------|--------------------------------------------|--------------------------------------------|
| MAE             | Error absoluto medio                       | Forecast de carga                           |
| MAPE            | Error porcentual absoluto medio            | Comparabilidad entre series                 |
| RMSE            | Raíz del error cuadrático medio            | Penalización de errores grandes             |
| SMAPE           | MAE normalizado sobre promedio             | Estabilidad ante ceros/valores bajos        |
| F1 (macro/micro)| Media armónica de precision y recall       | Detección de anomalías                      |
| p95/p99         | Percentiles de latencia                    | SLOs de servicio                            |
| Makespan        | Tiempo de finalización máximo              | Scheduling de jobs                          |
| Energía         | Energía acumulada                          | Sostenibilidad/FinOps                       |

La validación exige backtesting temporal, particiones rolling-origin, pruebas A/B y despliegues controlados (canary/shadow), con MLOps que automatice el ciclo de vida (registro de modelos, despliegues, monitoreo y reentrenamiento)[^11].

## Recomendaciones de implementación

La transición a un bucle de control proactivo debe seguir un roadmap escalonado, con capacidades mínimas de datos, observabilidad y MLOps.

Tabla 15. Roadmap de implementación

| Fase      | Capacidades requeridas                                     | Riesgos clave                       | Mitigaciones                                |
|-----------|-------------------------------------------------------------|-------------------------------------|---------------------------------------------|
| 0 (Reactivo) | Reglas/umbrbrales; monitoreo básico                       | Picos inesperados; SLO breaches     | Guardas y límites de escalado               |
| 1 (Pronóstico) | Forecasting (ARIMA/Prophet/LSTM); feature store; observabilidad | Drift; falsos positivos en control | Backtesting; alertas; validación continua   |
| 2 (Detección) | Híbrido reglas+ML (logs/red); KPIs de anomalías           | Ruido; sobrecarga en IoT            | Arquitectura IADM; baja sobrecarga          |
| 3 (RL)    | Políticas de scheduling/autoscaling; evaluación offline/online | Estabilidad; seguridad operativa    | Sandbox; guardas; despliegue progresivo     |

Guardrails operacionales: límites de escalado (máx./mín. réplicas), ventanas de cooldown, tasas máximas de cambio, políticas conservadoras bajo incertidumbre y auditoría de decisiones. Integraciones: Kubernetes para orquestación; MLflow/DVC/W&B para ciclo de vida de modelos y experimentos; pipelines CI/CD robustos para construir, probar y desplegar con despliegues controlados[^11][^12][^4].

## Riesgos, limitaciones y ética

Los riesgos técnicos incluyen deriva de datos y concepto (que erosiona precisión), medición sesgada (p. ej., latencias no representativas), heterogeneidad de hardware/red y costos de ML (entrenamiento, inferencia, almacenamiento de features). En RL, la estabilidad del entrenamiento y la seguridad operativa requieren especial atención: políticas inestables pueden inducir sobre/infra-provisión, violar SLOs o introducir fluctuaciones innecesarias[^5][^2].

Las limitaciones prácticas se manifiestan en escasez de datasets públicos de workloads cloud con etiquetas y granularidad suficientes, falta de benchmarks estandarizados para CI/CD y en la dificultad de generalizar hallazgos de dominios energéticos a cargas cloud genéricas. También existen lagunas específicas: evidencia cuantitativa comparativa directa y reciente entre ARIMA y Prophet sobre métricas cloud homogéneas; series históricas multi-tenant en nubes públicas para validar forecasting multi-serie y RL; análisis de coste total (cloud TCO) de ML de forecasting/autoscaling en producción; y guías de MLOps para drift en series de carga (p. ej., perfiles de distribución y reentrenamiento). Estas brechas deben abordarse mediante campañas de medición interna, experimentación controlada y documentación transparente.

Desde una perspectiva ética y de cumplimiento, es crítico asegurar privacidad en telemetría (logs y métricas), minimización de datos, auditorías de sesgo y transparencia de decisiones automatizadas. La gobernanza de modelos —versionado, trazabilidad, justificación— debe ser parte del contrato operacional.

## Conclusiones y líneas futuras

La combinación de pronóstico, detección de anomalías y RL para control produce beneficios operativos: mayor eficiencia de recursos, reducción de latencia y coste, y resiliencia ante variabilidad y bursts. El siguiente paso es la adopción incremental con validación rigurosa, integración en plataformas existentes y guardrails operacionales. Entre las líneas futuras destacan: algoritmos RL explicables y estables para producción, aprendizaje federado para escenarios multi-tenant, y enfoques “carbon-aware” que integren intensidad de carbono en las decisiones de scheduling y autoscaling, alineando coste, rendimiento y sostenibilidad[^4][^25].

La madurez de LSTM e híbridos —respaldada por resultados empíricos en contextos de carga— sugiere que el pronóstico de series seguirá siendo el cimiento de la planificación proactiva; la integración con arquitecturas de atención y Transformers puede capturar mejor dependencias largas y cambios abruptos[^13][^14]. En paralelo, la detección de anomalías basada en logs y telemetría de red, con arquitecturas híbridas reglas+ML y modelos con atención, se consolidará como línea de defensa esencial en plataformas distribuidas[^6][^7][^8][^9].

---

## Referencias

[^1]: Enhanced LSTM-based robotic agent for load forecasting in low-voltage distributed photovoltaic power distribution networks. Frontiers in Neurorobotics. https://www.frontiersin.org/journals/neurorobotics/articles/10.3389/fnbot.2024.1431643/full

[^2]: Auto-Scaling Techniques in Cloud Computing: Issues and Research Directions. Sensors (Basilea). https://pmc.ncbi.nlm.nih.gov/articles/PMC11398277/

[^3]: RL-based Adaptive Batch Size Optimization in Distributed Machine Learning (DYNAMIX). arXiv. https://arxiv.org/html/2510.08522

[^4]: Deep reinforcement learning-based methods for resource scheduling of Cloud computing. Springer. https://link.springer.com/article/10.1007/s10462-024-10756-9

[^5]: Deep reinforcement learning-based methods for resource scheduling of Cloud computing (formulaciones y mapeos). Springer. https://link.springer.com/article/10.1007/s10462-024-10756-9

[^6]: Distributed System Log Anomaly Detection Method Based on LSTM and Process State Checks. Wiley QRE. https://onlinelibrary.wiley.com/doi/10.1002/qre.3793?af=R

[^7]: Intelligent anomaly detection system for Internet of Things (ITU-T YSTR-IADIoT, 07/2024). ITU. https://www.itu.int/epublications/zh/publication/itu-t-ystr-iadiot-2024-07-intelligent-anomaly-detection-system-for-internet-of-things

[^8]: Anomaly Detection in Large-Scale Cloud Systems. arXiv 2411.09047. https://arxiv.org/pdf/2411.09047

[^9]: Temporal Logical Attention Network for Log-Based Anomaly Detection. MDPI Sensors. https://www.mdpi.com/1424-8220/24/24/7949

[^10]: Machine learning application in batch scheduling for multi-product pipelines. ScienceDirect. https://www.sciencedirect.com/science/article/pii/S2667143324000076

[^11]: CI/CD for Machine Learning in 2024: Best Practices to Build, Test and Deploy. Medium (JFrog/Qwak). https://medium.com/infer-qwak/ci-cd-for-machine-learning-in-2024-best-practices-to-build-test-and-deploy-c4ad869824d2

[^12]: MLOps: Continuous delivery and automation pipelines in machine learning. Google Cloud. https://docs.cloud.google.com/architecture/mlops-continuous-delivery-and-automation-pipelines-in-machine-learning

[^13]: Time series prediction model using LSTM-Transformer neural architecture. Nature Scientific Reports. https://www.nature.com/articles/s41598-024-69418-z

[^14]: Unlocking the Power of LSTM for Long Term Time Series Forecasting. AAAI. https://ojs.aaai.org/index.php/AAAI/article/view/33303/35458

[^15]: Optimizing multi-time series forecasting for enhanced cloud resource optimization. ScienceDirect. https://www.sciencedirect.com/science/article/pii/S0950705124011237

[^16]: A state-of-the-art comparative review of load forecasting methods. ScienceDirect. https://www.sciencedirect.com/science/article/pii/S2590174525000546

[^17]: Empowering data-driven load forecasting by leveraging long short-term memory networks. ScienceDirect. https://www.sciencedirect.com/science/article/pii/S2405844024169654

[^18]: Gwydion: Efficient auto-scaling for complex containerized microservice architectures. ACM JNCA. https://dl.acm.org/doi/10.1016/j.jnca.2024.104067

[^19]: A deep reinforcement learning-based scheduling framework for real-time workflows. ScienceDirect. https://www.sciencedirect.com/science/article/abs/pii/S0957417424017123

[^20]: Efficient deep reinforcement learning based task scheduler in multi-cloud. Nature Scientific Reports. https://www.nature.com/articles/s41598-024-72774-5

[^21]: Multi-agent Reinforcement Learning-based In-place Scaling Engine for Cloud-native Services. arXiv. https://arxiv.org/html/2507.07671v1

[^22]: Dynamic Load Balancing and Distribution Algorithm in Distributed Cloud Computing. IEEE Xplore. https://ieeexplore.ieee.org/document/10515775/

[^23]: Predictive Load Balancing in Cloud Computing: A Comparative Study. ACM DL. https://dl.acm.org/doi/10.1145/3659677.3659713

[^24]: Load Balancing with Network Latencies via Distributed Gradient Algorithms. arXiv 2504.10693. https://arxiv.org/pdf/2504.10693

[^25]: AI Forecasting Based Carbon Neutral Cloud Resource Optimization (Ainet0). Wiley TETT. https://onlinelibrary.wiley.com/doi/10.1002/ett.70166?af=R

[^26]: Enhancing the output of time series forecasting algorithms for cloud resource sharing. ScienceDirect. https://www.sciencedirect.com/science/article/pii/S0167739X25001281

[^27]: Kubeflow Training Operator. https://www.kubeflow.org/docs/components/training/

[^28]: Shadow Deployment vs. Canary Release of Machine Learning Models. Qwak. https://www.qwak.com/post/shadow-deployment-vs-canary-release-of-machine-learning-models

[^29]: AI-Powered CI/CD: How Machine Learning is Optimizing Build Pipelines. EM360. https://em360tech.com/tech-articles/ai-powered-cicd-how-machine-learning-optimizing-build-pipelines

[^30]: DVC (Data Version Control). https://dvc.org/

[^31]: Weights & Biases. https://wandb.ai/site

[^32]: MLflow. https://mlflow.org/

[^33]: How it Works: Training — Amazon SageMaker. https://docs.aws.amazon.com/sagemaker/latest/dg/how-it-works-training.html

[^34]: Vertex AI Training Overview. https://cloud.google.com/vertex-ai/docs/training/overview