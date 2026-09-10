# Historial de Actualizaciones / Changelog

Todas las novedades, correcciones y cambios notables en el **Arknights Analytical Tier List Engine** están documentados en este archivo.

El formato se basa en [Keep a Changelog](https://keepachangelog.com/es-ES/1.1.0/) y este proyecto se adhiere a [Semantic Versioning](https://semver.org/).

---

## [1.0.0] - Versión Inicial / Stable Core Release (2026-09-09)

> [!WARNING]
> **Aviso de Versión 1.0.0**:  
> Esta es la versión oficial inicial (1.0.0) del motor analítico. Debido a la inmensa cantidad de combinaciones simuladas (más de **426 operadores**, **3,000+ configuraciones** de habilidades y módulos, y **1,698 enemigos** evaluados matemáticamente en rotaciones discretas de 300 segundos), es de esperar que aún puedan presentarse pequeños fallos (*bugs*), desajustes numéricos en interacciones poco comunes o casos borde.  
> 
> Agradecemos enormemente cualquier reporte de discrepancias o comportamientos anómalos para seguir refinando el motor en próximas revisiones.

---

### ✨ Características Principales

#### 1. Motor de Simulación Determinista en Rust (`rust_engine`)
- **Simulación Granular a 300 Segundos (5 Minutos)**: Modela el ciclo de rotación completo, tiempos de carga de SP (automático, por golpe ofensivo y por daño recibido), ventanas de ráfaga y tiempos muertos en lugar de aproximaciones teóricas infinitas.
- **Espacio de Estados Exhaustivo**: Evalúa a cada operador en todas sus combinaciones posibles de habilidades (`RAW / Sin Habilidad`, `S1`, `S2`, `S3`) y módulos (`NO MOD`, `MOD-X`, `MOD-Y`, `MOD-D`, `MOD-RA`, `MOD-IS`).
- **Paralelismo Multihilo de Alto Rendimiento**: Implementación con **Rayon**, permitiendo procesar y clasificar los más de 426 operadores y 3,000 configuraciones en menos de **150 milisegundos**.
- **Regla Canónica del Piso de Daño del 5%**: Aplicación estricta de la regla hardcoded de *Arknights*, asegurando que los ataques físicos y de artes nunca inflijan menos del 5% del ATK final tras el cálculo de DEF y RES.

#### 2. Escenarios y Perfiles de Combate Especializados
- **General**: Promedio estadístico compuesto del conjunto total de enemigos.
- **Enemigos Normales (Normal)**: Escenarios de hordas con baja defensa y alta densidad de enemigos.
- **Élites (Elite)**: Enemigos blindados de resistencia media-alta que demandan daño sostenido o penetración.
- **Jefes (Boss)**: Enemigos de gran amenaza (1,000 DEF, 50 RES, 80,000 HP y alto poder ofensivo).
- **Reclamation Algorithm (RA)**: Escenario de defensa de bases y control de masas en hordas masivas.
- **Integrated Strategies (IS)**: Escenario roguelike centrado en escalado de ráfagas críticas y control de multitudes.
- **Generadores de DP (DP)**: Benchmark especializado para medir confiabilidad y velocidad de producción de Puntos de Despliegue.

#### 3. Tier List de Amenaza de Enemigos (1,698+ Enemigos)
- Clasificación de toda la base de datos de enemigos en categorías *Normal*, *Elite* y *Boss*.
- Modelado de **Threat Score**, EHP físico y de artes, DPS directo y por habilidades, esquiva y detección de mecánicas especiales (revivir, fases invulnerables, taunt).

#### 4. Suite de Exportación y Generación de Informes
- **Descarga en CSV**: Exportación inmediata de datos tabulares desde la interfaz web.
- **Matriz Analítica de 116 PDFs**: Generación automatizada con ReportLab (`Scripts/generate_tierlist_pdfs.py`) empaquetada en `data/Arknights_Tier_Lists_PDF.zip`.
  - 84 PDFs de Operadores (6 escenarios × 14 métricas analíticas con 426 filas cada uno).
  - 32 PDFs de Enemigos (4 categorías × 8 métricas de amenaza con hasta 1,698 filas).

#### 5. Interfaz Web y Herramientas Sandbox
- **Diseño Glassmorphism Oscuro**: Gráficas SVG interactivas con curvas de distribución acumulada (CDF), filtros rápidos de clase y rareza, e insignias de tiers (`OP`, `S`, `A`, `B`, `C`, `D`).
- **Editor de Operadores y Buffs**: Sandbox interactivo para ajustar estadísticas, inyectar buffs de equipo y simular curvas de daño segundo a segundo.
- **Editor de Enemigos**: Interfaz para crear, ajustar y balancear estadísticas de objetivos de prueba.

---

### 🐛 Correcciones y Ajustes Críticos en v1.0.0

- **Habilidades Instantáneas vs Infinitas**: Se corrigió el error en el que habilidades instantáneas ofensivas con `duration: -1.0` (como S1/S3 de Necrass, S2 de Ch'en, S2 de Eyjafjalla) se interpretaban incorrectamente como habilidades continuas de duración infinita, inflando desproporcionadamente su daño a lo largo de los 300 segundos.
- **Arquetipo Trapmaster (Dorothy, Wang, Ela, Robin, Frost)**: Se implementó la lógica completa de despliegue dinámico y detonación de trampas según el intervalo de recarga de SP, aplicando el daño de área real (físico y artes) y reducciones de DEF/RES enemiga.
- **Generación Canónica de Puntos de Despliegue (DP)**:
  - Se corrigió la prioridad de lectura en *Flagbearers* (Myrtle, Elysium, Saileach) para evitar que el costo por tick sobreescribiera la ganancia total de DP del ciclo (Elysium genera 162 DP y Myrtle 140 DP en 300s).
  - Se resolvió la sobreacumulación de auras pasivas de SP que inflaban artificialmente los números de SilverAsh the Reignfrost y Siege.
- **Métricas de Supervivencia y Golpes para Derrota (HTK)**:
  - Se integró el modelo de *Hits-to-Kill (HTK)* que evalúa cuántos golpes puede soportar el operador antes de caer, considerando la regeneración pasiva y mitigaciones de santuario.
  - Se ajustó la puntuación para recompensar la resistencia de los *Defenders* y penalizar a operadores con bloqueo unitario (como Eunectes) en escenarios de enjambre.
- **Ciclo de Retirada y Redespliegue para Ejecutores (*Fast-Redeploy*)**: Se implementó el ciclo de campo/banquillo para especialistas como Texas Alter y Yato Alter, simulando su ventana de ráfaga en campo y su tiempo de recarga en banca.
- **Penalización por Derrota en Combate**: Los operadores cuya supervivencia no alcanza a sostener el DPS enemigo sufren derrotas que reducen su factor de actividad en combate (*combat uptime*), reflejando la necesidad de soporte defensivo.
