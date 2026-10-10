# Riesgos de ventas

## A5 — diferido

Los siguientes riesgos quedan diferidos para A5. Las descripciones reflejan el
comportamiento verificado en el código actual; las mitigaciones son propuestas,
no funcionalidades ya implementadas.

### A5-01 — Actor de venta declarado por el cliente

- **Descripción:** `CreateSaleDto.created_by` es opcional y el handler deriva el
  actor de auditoría de ese valor mediante `declared_actor`. El código de
  auditoría indica expresamente que no existe login.
- **Impacto:** el nombre del vendedor no constituye una identidad autenticada y
  puede no permitir atribuir una operación a una persona real.
- **Mitigación propuesta:** incorporar autenticación y obtener el actor desde la
  identidad de la sesión en el backend, sin confiar en el nombre enviado por el
  cliente.
- **Estado:** diferido (A5).
- **Referencias:** `src-tauri/src/modules/sales/dto.rs`,
  `src-tauri/src/modules/sales/handlers.rs`,
  `src-tauri/src/modules/audit/actor.rs`.

### A5-02 — Descuentos y cortesías sin autorización de supervisor

- **Descripción:** el servicio valida límites numéricos del descuento y exige
  motivo para una venta de total cero, pero no comprueba una autorización o rol
  de supervisor.
- **Impacto:** un operador sin autorización diferenciada puede registrar
  descuentos o cortesías.
- **Mitigación propuesta:** aplicar autorización de supervisor en el backend,
  registrar quién autorizó y conservar el motivo en auditoría.
- **Estado:** diferido (A5).
- **Referencias:** `src-tauri/src/modules/sales/repository.rs`,
  `src-tauri/src/modules/sales/service.rs`.

### A5-03 — Una cortesía descuenta existencias

- **Descripción:** las líneas de una venta se procesan con el movimiento de
  salida de inventario también cuando el total queda en cero y la venta se
  registra como cortesía.
- **Impacto:** las unidades salen del inventario sin que exista un pago asociado
  a esa venta.
- **Mitigación propuesta:** definir explícitamente la política de inventario de
  cortesías y representar su motivo/tipo en los movimientos; mantener cualquier
  excepción bajo autorización.
- **Estado:** diferido (A5).
- **Referencias:** `src-tauri/src/modules/sales/repository.rs`.

### A5-04 — Posible abuso de descuentos globales y cortesías

- **Descripción:** el backend limita el descuento global a un importe no mayor
  que el subtotal y exige un motivo de hasta 200 caracteres para una cortesía,
  pero el flujo inspeccionado no aplica límites por operador, frecuencia ni
  aprobación.
- **Impacto:** el uso repetido de descuentos o cortesías puede reducir ingresos
  y afectar la confiabilidad de los registros de venta.
- **Mitigación propuesta:** establecer límites y reglas de aprobación del lado
  del servidor, y habilitar revisión de auditoría por operador, importe y
  frecuencia.
- **Estado:** diferido (A5).
- **Referencias:** `src-tauri/src/modules/sales/repository.rs`,
  `src-tauri/src/modules/sales/handlers.rs`.

### A5-05 — Ventas históricas sin turno de caja asociado

- **Descripción:** `cash_session_id` se añadió como columna nullable; la
  migración indica que las ventas anteriores a ella no tienen sesión asociada.
- **Impacto:** esas ventas no se pueden atribuir a un turno mediante esa columna,
  lo que limita la conciliación y los filtros por caja.
- **Mitigación propuesta:** investigar si existe evidencia fiable para un
  backfill; mantener explícitamente desconocidas las asociaciones que no puedan
  probarse y contemplar `NULL` en consultas e informes.
- **Estado:** diferido (A5).
- **Referencias:** `src-tauri/migrations/016_sales_cash_session.sql`.

### A5-06 — La idempotencia no compara el contenido del DTO

- **Descripción:** al encontrar una `idempotency_key` ya registrada, el
  repositorio devuelve esa venta antes de procesar o comparar el resto del DTO.
  Por tanto, reutilizar la misma clave con un carrito o importe diferente
  devuelve la venta original.
- **Impacto:** un cliente que reutilice accidentalmente una clave puede recibir
  una respuesta correspondiente a un contenido distinto del que acaba de enviar.
- **Mitigación propuesta:** guardar una huella del contenido normalizado y
  rechazar la reutilización de la clave cuando la huella no coincida.
- **Estado:** diferido (A5).
- **Referencias:** `src-tauri/src/modules/sales/repository.rs`,
  `src-tauri/migrations/015_sales_idempotency.sql`.

### A5-07 — Reportes de ventas sin implementación

- **Descripción:** `src-tauri/src/modules/reports/sales_report.rs` y
  `src/pages/Reports/Reports.tsx` están vacíos en el árbol inspeccionado; no hay
  una implementación de reportes de ventas en esos archivos.
- **Impacto:** esos módulos no proporcionan resultados de ventas para análisis
  operativo o financiero.
- **Mitigación propuesta:** implementar consultas y presentación de reportes,
  validando sus totales contra ventas, pagos y movimientos de caja.
- **Estado:** pendiente de implementación, diferido (A5).
- **Referencias:** `src-tauri/src/modules/reports/sales_report.rs`,
  `src/pages/Reports/Reports.tsx`.

### A5-08 — Semántica histórica de `subtotal` y `discount` en auditoría

- **Descripción:** los eventos de negocio de ventas guardan `subtotal` y
  `discount` como valores de la venta al momento de crearla. Los eventos
  anteriores a M2 conservan la semántica anterior de esos campos; la tabla de
  auditoría es append-only y el código actual no recalcula eventos existentes.
- **Impacto:** comparar directamente eventos históricos con ventas calculadas
  bajo la semántica posterior puede producir interpretaciones o conciliaciones
  incorrectas.
- **Mitigación propuesta:** documentar la semántica por periodo y, si se requiere
  comparar, presentar una normalización derivada que no modifique los eventos
  originales.
- **Estado:** diferido (A5).
- **Referencias:** `src-tauri/src/modules/sales/repository.rs`,
  `src-tauri/migrations/012_audit.sql`.