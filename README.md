# Proxy IA en Rust

Proyecto para construir un proxy de inspección y manipulación de tráfico similar a Charles Proxy y Proxyman, escrito en Rust y orientado a funcionar en Windows, macOS y Linux. Se prioriza la facilidad de configuración para aplicaciones móviles (Android/iOS) y web, con un agente de IA que permita operar la herramienta mediante lenguaje natural.

## Objetivos clave
- **Multiplataforma**: binarios nativos para Windows, macOS y Linux.
- **Compatibilidad móvil y web**: configuración sencilla de certificados y perfiles para Android, iOS y navegadores.
- **Experiencia tipo Proxyman/Charles**: interceptar, inspeccionar, modificar y repetir tráfico HTTP(S) y websockets.
- **Agente de IA**: aprendizaje del uso y capacidad de generar mapeos locales/remotos, automatizar flujos y responder a órdenes en lenguaje natural.
- **Extensibilidad**: reglas personalizadas, scripts y filtros por dominio/ruta/metodología.

## Arquitectura propuesta (alto nivel)
- **Proxy core**: servicio Rust asíncrono (Tokio/Hyper) con soporte HTTP(S), HTTP/2 y websockets.
- **Certificados**: autoridad raíz generada/local gestionada, instalación guiada y exportable a móviles/navegadores.
- **Panel UI**: interfaz (pendiente de tecnología) para inspección, búsqueda, edición y reproducción de requests/responses.
- **Motor de reglas**: mapeos locales/remotos, reescritura de cabeceras y cuerpo, mocks y redirecciones.
- **Agente IA**: componente que observa acciones, genera sugerencias y ejecuta tareas declaradas en lenguaje natural.
- **Registro/auditoría**: capturas exportables (HAR/JSON), historial y perfiles de sesión.

## Roadmap inicial
- **v0.1 - Fundamentos**
  - Repositorio Rust inicial, CLI mínima y configuración base.
  - Proxy HTTP/HTTPS básico con captura y guardado de tráfico.
  - Autoridad raíz y generación de certificados por host.
- **v0.2 - Experiencia de usuario**
  - Flujo guiado para configurar Android/iOS y navegadores.
  - UI inicial para inspección y replay.
  - Exportación/importación de sesiones (HAR) y filtros rápidos.
- **v0.3 - Motor de reglas**
  - Mapeos locales/remotos, mocks y redirecciones.
  - Scripts/rules declarativas (YAML/JSON) con recarga en caliente.
- **v0.4 - Agente IA**
  - Comandos en lenguaje natural ("mapea esta ruta a localhost", "mockea este endpoint").
  - Aprendizaje de patrones de uso y sugerencias contextuales.
- **v0.5 - Pulido multiplataforma**
  - Empaquetado y distribución para Windows/macOS/Linux.
  - Documentación avanzada, performance tuning y métricas.

## Estado actual
- Proyecto Rust inicializado con binario `proxy-ia`.
- Proxy HTTP/HTTPS básico usando Hyper/Tokio:
  - Soporta requests HTTP con URI absoluta (modo proxy) y reenvío de respuestas.
  - Maneja `CONNECT` para túneles TCP (HTTPS) creando un canal bidireccional.
  - Limpia headers hop-by-hop para evitar inconsistencias.
- CLI inicial con opciones de escucha y nivel de log.
- Pruebas automáticas que validan rechazo de URIs relativas y reenvío a un backend local.

### Cómo ejecutar
1. Instalar Rust (toolchain 1.74+ recomendado).
2. Ejecutar el proxy en el puerto por defecto 8888: `cargo run --release`.
3. Configurar tu cliente/OS para usar `http://<host>:8888` como proxy.
4. Para cambiar puerto: `cargo run -- --listen 0.0.0.0:8080`.
5. Ajustar logs: `--log-level debug` o `-q` para silencioso.

### Pruebas
- Ejecutar el suite: `cargo test`.
- Las pruebas levantan servidores locales ligeros para validar reenvío y túneles.

## Flujo de contribución
- Todas las nuevas features deben integrarse mediante Merge Requests (MRs) descriptivos.
- Incluir descripciones claras, pruebas relevantes y capturas cuando haya cambios de UI.
- Se promoverán ramas por feature y revisiones cortas y frecuentes.

## Próximos pasos sugeridos
- [x] Inicializar workspace Rust con binario principal y crate compartido.
- [x] Configurar toolchain y formato (rustfmt, clippy) y pipelines de CI básicos. *(formato/clippy pendientes de CI, binario ya configurado).* 
- [ ] Implementar proxy HTTP mínimo con logging y exportación simple de sesiones.
- [ ] Diseñar estructura de datos para reglas y preparar hooks para el agente de IA.
- [ ] Añadir captura estructurada de requests/responses y almacenamiento en disco (HAR/JSON).
- [ ] Esbozar API para agente IA (comandos de mapeo, mocks, redirecciones).
- [ ] Incluir guía rápida para Android/iOS y navegadores.
