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

- **Descripción:** los eventos previos a M2 usan `subtotal` neto de línea y
  `discount` global. Los eventos posteriores usan `subtotal` bruto y `discount`
  total, con `discount_lines` y `discount_global` en metadata. La auditoría es
  append-only y los eventos existentes no se recalculan.
- **Impacto:** comparar directamente eventos históricos con ventas calculadas
  bajo la semántica posterior puede producir interpretaciones o conciliaciones
  incorrectas.
- **Mitigación propuesta:** documentar la semántica por periodo y, si se requiere
  comparar, presentar una normalización derivada que no modifique los eventos
  originales.
- **Estado:** diferido (A5).
- **Referencias:** `src-tauri/src/modules/sales/repository.rs`,
  `src-tauri/migrations/012_audit.sql`.

### F1-01 — Anulación y devolución sin autorización de supervisor

- **Descripción:** la operación de anulación/devolución valida estado y motivo,
  pero no requiere permiso o autorización de supervisor. La identidad
  `performed_by` sigue siendo declarada por el cliente.
- **Impacto:** un operador puede revertir una venta sin una autorización
  diferenciada.
- **Mitigación propuesta:** añadir autorización en el backend y auditoría de
  quién aprobó la operación.
- **Estado:** abierto; permisos de supervisor quedan para F3.

### F1-02 — No hay devolución parcial

- **Descripción:** la operación actual cambia el estado de la venta completa,
  repone todas sus líneas y devuelve el importe neto de todos sus pagos en
  efectivo; no permite devolver una parte de los artículos o del efectivo.
- **Impacto:** no se puede representar una devolución parcial usando este
  flujo.
- **Mitigación propuesta:** diseñar una entidad/operación de devolución parcial
  que conserve cantidades, pagos y movimientos acumulados por venta.
- **Estado:** abierto; alcance actual limitado a devolución total.

### F1-03 — Constraint histórico creado NOT VALID

- **Descripción:** `sales_void_consistency_chk` se crea `NOT VALID`; PostgreSQL
  no verifica las filas existentes al crearla, aunque comprueba las nuevas y
  modificadas.
- **Impacto:** datos históricos inconsistentes pueden permanecer hasta validar
  la restricción.
- **Mitigación propuesta:** revisar el histórico y ejecutar
  `VALIDATE CONSTRAINT sales_void_consistency_chk` cuando sea seguro.
- **Estado:** pendiente de validación histórica.

### XZ-01 — Informes Z no inmutables por base de datos

- **Descripción:** `cash_session_reports` guarda el informe Z como JSONB, pero
  la migración 022 no instala un trigger que impida actualizar o eliminar ese
  registro.
- **Impacto:** un cambio directo en la base de datos podría alterar el informe
  histórico que se presenta como reproducible.
- **Mitigación propuesta:** en F4, añadir una protección append-only para los
  informes Z y una ruta explícita de corrección auditada si se requiere.
- **Estado:** abierto, pendiente F4.

### XZ-02 — Hora del ticket de venta en UTC

- **Descripción:** la impresión del ticket de venta usa directamente
  `created_at` en UTC; no aplica `app_settings.timezone`.
- **Impacto:** la hora impresa puede diferir de la hora local del negocio.
- **Mitigación propuesta:** formatear la hora de venta con la zona horaria
  configurada y cubrir la conversión con pruebas.
- **Estado:** hallazgo pendiente; no corregido en el cambio de informes X/Z.

### XZ-03 — Turnos históricos sin informe Z

- **Descripción:** los turnos cerrados antes de instalar la migración 022 no
  tienen una fila en `cash_session_reports`.
- **Impacto:** no se puede recuperar el snapshot Z de esos turnos mediante la
  nueva consulta.
- **Mitigación propuesta:** conservar el mensaje explícito de no disponibilidad;
  generar retrospectivamente informes solo si los datos existentes permiten
  reconstruirlos con fidelidad, sin presentarlos como snapshots contemporáneos.
- **Estado:** conocido; histórico anterior a 022.

### A5-09 — Ventas en espera locales a un equipo

- **Descripción:** las ventas en espera se guardan en `localStorage` del equipo
  con la clave `pos.heldCarts.v1`; no hay sincronización con otras cajas.
- **Impacto:** un carrito guardado en un equipo no estará disponible en otra
  caja y puede perderse si se limpia el almacenamiento local.
- **Mitigación propuesta:** evaluar sincronización autenticada entre cajas o
  exportación controlada, manteniendo claro que estos carritos no son ventas
  registradas.
- **Estado:** riesgo abierto; implementación local limitada a cinco carritos.

### A5-10 — Producto eliminado en una venta en espera

- **Descripción:** al retomar se consulta cada variante. Si ya no existe, está
  inactiva o la consulta falla, la línea permanece marcada y bloquea la
  confirmación hasta que el operador la revise o la elimine.
- **Impacto:** una venta en espera con productos eliminados requiere revisión
  manual y no puede confirmarse tal cual.
- **Mitigación propuesta:** ofrecer una resolución administrativa explícita que
  permita sustituir o quitar el producto con registro de la decisión.
- **Estado:** riesgo abierto; revalidación manual en la pantalla de ventas.