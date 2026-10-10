# Backlog de ventas

Elementos pendientes y completados basados en la interfaz y los servicios inspeccionados.

1. **Imprimir al confirmar la venta**
   - **Estado:** implementado en punto de venta (impresión manual y preferencia de impresión automática).
   - **Situación actual:** el banner permite imprimir manualmente; la opción
     `pos.autoPrint` habilita impresión tras confirmar y muestra errores sin
     revertir la venta.

2. **Atajos de teclado para el punto de venta**
   - **Estado:** parcialmente implementado: F2, F4, F9 y Esc; controles +/- por línea y vaciado con doble confirmación.
   - **Atajos:** F2 enfoca el escáner, F4 enfoca efectivo, F9 confirma y Esc cierra resultados/aviso. No hay acciones destructivas globales con Enter/Esc/Supr.
   - **Situación actual:** F2 enfoca el escáner, F4 el efectivo, F9 confirma si
     está habilitado y Esc cierra resultados/aviso. Hay controles +/- por línea
     y vaciado de carrito con doble clic dentro de 3 segundos.

3. **Aviso cuando la cantidad supera el stock**
   - **Estado:** implementado con aviso no bloqueante; el servidor conserva la validación final.
   - **Situación actual:** la línea muestra el stock recibido y un aviso no
     bloqueante advierte cantidades superiores; la validación definitiva sigue
     en backend.

4. **Nombre del producto en el detalle del historial**
   - **Estado:** implementado con nombre y atributos; conserva fallback al identificador de variante.
   - **Situación actual:** `get_sale` devuelve nombre y atributos de cada
     producto; la interfaz conserva `#variant_id` como fallback.

5. **Ampliar filtros y mostrar el total filtrado**
   - **Estado:** implementado (presets, número, método, turno y total de ventas completadas).
   - **Situación actual:** incluye Hoy/Ayer/7 días, número de venta, método y
     turno; informa cantidad de resultados y total monetario de ventas
     completadas conforme a los filtros.

6. **Venta en espera**
   - **Estado:** pendiente.
   - **Situación actual:** no se encontró una acción para guardar y recuperar
     carritos pendientes en la pantalla de ventas.
   - **Trabajo propuesto:** permitir suspender una venta y retomarla sin
     registrarla como venta ni descontar existencias mientras siga pendiente.

7. **Asociar un cliente a la venta**
   - **Estado:** pendiente en la interfaz.
   - **Situación actual:** el DTO admite `customer_id`, pero `Sales.tsx` no ofrece
     selección de cliente ni incluye ese campo en el payload de creación.
   - **Trabajo propuesto:** permitir buscar y asociar un cliente, y enviar su
     identificador al crear la venta.

8. **Descuento en porcentaje**
   - **Estado:** pendiente.
   - **Situación actual:** los descuentos expuestos por el formulario de ventas
     se capturan como importes monetarios; no se encontró una opción porcentual.
   - **Trabajo propuesto:** permitir ingresar un porcentaje, calcular el importe
     correspondiente con la precisión monetaria configurada y presentar con
     claridad el valor aplicado.

## Caja, conciliación y devoluciones

- **Defecto — gravedad media:** el cierre deriva el esperado de movimientos
  (`sale_in` neto y `sale_out`) más apertura; no contrasta sus pagos por venta
  contra esos movimientos. La consulta de conciliación de solo lectura está en
  `scripts/sql/caja_conciliacion.sql`.
- **Anulación/devolución total — implementada:** desde el historial se puede
  anular una venta completada o marcarla como devuelta. Se reponen las líneas
  completas y el efectivo neto se registra como `sale_out` en el turno abierto
  actual (nunca se usa el importe de efectivo recibido); el proceso es atómico
  y auditado. El orden de bloqueos es venta, turno si aplica y variantes por id.
- **Pendiente F3:** autorización de supervisor. El flujo no admite devolución
  parcial.
- **Migración 021:** `sales_void_consistency_chk` se crea `NOT VALID`; revisar
  los datos históricos y ejecutar `VALIDATE CONSTRAINT` posteriormente.
- **Ventas históricas sin turno — gravedad media:** `cash_session_id` es NULL
  en ventas anteriores a su incorporación y no se atribuyen a un turno en la
  conciliación. No se debe inventar un backfill; conservarlas como no
  conciliadas salvo evidencia fiable para asignarlas.
- **Informes X/Z — implementados:** X es una vista en vivo del turno abierto;
  Z se genera y persiste al cerrar. Ventas completadas se agrupan por
  `cash_session_id`, mientras que anulaciones/devoluciones se agrupan por el
  turno donde ocurrió `voided_at`. El snapshot se guarda en
  `cash_session_reports` como JSONB, no como columna del turno.
- **Informes históricos — limitación:** los turnos cerrados antes de 022 no
  tienen un Z guardado y la interfaz informa que no está disponible.
- **Riesgos pendientes:** la tabla del informe no es inmutable por trigger
  (F4); además, el ticket de venta imprime su hora en UTC. Este hallazgo no se
  corrige en esta fase.
