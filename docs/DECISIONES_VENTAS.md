# Decisiones de ventas

Registro de reglas observadas en las migraciones y el código. Los nombres de
pruebas indican cobertura existente; esta documentación no implica que las
pruebas se hayan ejecutado en esta ronda.

## M3 — Turno de caja obligatorio

- **Regla:** toda venta nueva requiere que exista un turno abierto y se guarda
  su identificador en `sales.cash_session_id`, incluso cuando el pago no es en
  efectivo. La columna permite `NULL` para conservar ventas anteriores a la
  migración, que no tenían turno asociado.
- **Dónde se aplica:** `src-tauri/migrations/016_sales_cash_session.sql`;
  `src-tauri/src/modules/sales/repository.rs` (búsqueda y bloqueo del turno
  abierto, asociación al insertar la venta); `src-tauri/src/modules/cash/repository.rs`
  (cálculo del esperado mediante movimientos de caja).
- **Pruebas:** `a01_venta_en_efectivo_valida`,
  `a02_venta_con_tarjeta_exige_turno`,
  `a03_efectivo_sin_turno_revierte_todo`,
  `a15_venta_con_tarjeta_guarda_turno_sin_movimiento_de_caja`,
  `d11_cortesia_sin_turno_abierto_se_rechaza`.

## M5 — Cortesía

- **Regla:** la cortesía se deriva de que el total calculado sea cero; no es un
  estado que envíe el cliente. Requiere un motivo no vacío de hasta 200
  caracteres, no admite pagos y se rechaza si el importe bruto también es cero.
  La migración añade la restricción del motivo según el total; el rechazo de
  bruto cero se aplica en el repositorio.
- **Dónde se aplica:** `src-tauri/migrations/017_sales_courtesy_reason.sql`;
  `src-tauri/src/modules/sales/repository.rs`.
- **Pruebas:** `a16_venta_de_cortesia_guarda_motivo_y_no_registra_pago`,
  `a17_total_cero_sin_motivo_de_cortesia_se_rechaza`,
  `d03_total_cero_con_pago_se_rechaza`,
  `d05_motivo_de_200_caracteres_se_acepta`,
  `d06_motivo_de_201_caracteres_se_rechaza_sin_evento_de_fallo`,
  `d08_bruto_cero_se_rechaza_como_cortesia`,
  `d09_descuento_de_linea_total_permite_cortesia`.

## A4 — Efectivo recibido y vuelto

- **Regla:** `cash_received` y `change_given` son opcionales y deben estar
  presentes juntos cuando se registran. El efectivo recibido requiere una
  venta con pago en efectivo y no puede ser menor que la suma neta de los pagos
  en efectivo. El vuelto es recibido menos ese neto; un recibido exacto produce
  vuelto cero. Las ventas sin esos valores conservan ambos campos nulos.
- **Dónde se aplica:** `src-tauri/migrations/018_sales_cash_received.sql`;
  `src-tauri/src/modules/sales/repository.rs` (validación y cálculo);
  `src-tauri/src/modules/cash` (el movimiento `sale_in` usa el efectivo neto,
  no el efectivo recibido).
- **Pruebas:** `e01_efectivo_con_vuelto_guarda_neto_y_turno`,
  `e02_pago_mixto_calcula_vuelto_sobre_neto_en_efectivo`,
  `e03_recibido_menor_que_efectivo_revierte_todo`,
  `e04_recibido_sin_pago_en_efectivo_se_rechaza`,
  `e07_sin_recibido_los_dos_campos_se_guardan_null`,
  `e08_recibido_exacto_guarda_cambio_cero`,
  `e09_check_de_bd_rechaza_recibido_inconsistente`.

## M2 — Subtotal bruto y descuento total

- **Regla:** para ventas con `gross_semantics = TRUE`, `sales.subtotal` es la
  suma bruta de precio por cantidad; `sales.discount` suma descuentos de línea
  y global; `sales.total` conserva la regla subtotal menos descuento. La
  migración deja las ventas existentes en `FALSE`. Los scripts históricos
  incluyen dry-run de solo lectura, apply transaccional con copia de respaldo y
  rollback documentado.
- **Dónde se aplica:** `src-tauri/migrations/019_sales_gross_semantics.sql`;
  `src-tauri/src/modules/sales/repository.rs`;
  `scripts/sql/m2_migrar_descuentos_dryrun.sql`,
  `scripts/sql/m2_migrar_descuentos_apply.sql` y
  `scripts/sql/m2_migrar_descuentos_rollback.sql`.
- **Pruebas:** `f01_descuento_de_linea_guarda_subtotal_bruto`,
  `f02_descuentos_de_linea_y_global_se_suman`,
  `f03_invariantes_sql_de_venta_bruta_y_pagos`,
  `f04_cortesia_guarda_descuento_igual_al_subtotal_bruto`,
  `f05_auditoria_separa_descuentos_de_linea_y_global`.

## M1 — Costo de venta como snapshot

- **Regla:** cada línea guarda el costo tomado de la variante al confirmar la
  venta. `sale_items.unit_cost` admite `NULL` para costo desconocido y tiene
  restricción de no negatividad. El costo histórico no se completa mediante
  backfill. En el código, `product_variants.cost` es obligatorio y usa cero
  como valor predeterminado; al construir el snapshot, cero se representa como
  `NULL`. El movimiento de inventario conserva la columna preexistente
  `inventory_movements.unit_cost`.
- **Dónde se aplica:** `src-tauri/migrations/020_sales_unit_cost.sql`;
  `src-tauri/src/modules/sales/repository.rs`;
  `src-tauri/src/modules/inventory/repository.rs`.
- **Pruebas:** `h01_guarda_snapshot_del_costo_en_la_linea`,
  `h02_cambiar_costo_no_modifica_snapshot`,
  `h03_movimiento_de_venta_registra_costo`,
  `h04_costo_no_definido_se_guarda_null`,
  `h05_cortesia_guarda_costo_snapshot`.

## N7 — Precisión monetaria

- **Regla:** los importes se almacenan como enteros de 64 bits en centésimas;
  `currency_decimals` determina la precisión visible (entre cero y dos
  decimales). Los cálculos monetarios se redondean a esa precisión y el efectivo
  recibido debe ser múltiplo del paso de moneda configurado.
- **Dónde se aplica:** `src-tauri/src/types/money.rs`;
  `src-tauri/src/modules/sales/repository.rs`;
  `src-tauri/src/modules/settings`.
- **Pruebas:** `cop_rounds_to_whole_pesos`,
  `tax_is_rounded_once_to_currency_precision`,
  `tax_included_in_gross_is_rounded_once`,
  `e05_precision_de_efectivo_recibido_sigue_la_moneda`.

## Idempotencia por clave

- **Regla:** la clave UUID es opcional y tiene un índice único parcial. Al
  recibir una clave ya registrada, el repositorio devuelve la venta existente
  sin volver a aplicar sus efectos. El código no compara el contenido del DTO
  nuevo con el original; por tanto, reutilizar una clave con contenido distinto
  devuelve la venta original.
- **Dónde se aplica:** `src-tauri/migrations/015_sales_idempotency.sql`;
  `src-tauri/src/modules/sales/repository.rs`.
- **Pruebas:** `c01_reintento_secuencial_devuelve_la_misma_venta`,
  `c02_reintentos_simultaneos_crean_una_sola_venta`,
  `c06_reintento_idempotente_tras_cerrar_turno_devuelve_venta_original`,
  `e10_idempotencia_con_recibido_retorna_los_mismos_valores`.

## Auditoría de rechazos de validación

- **Regla:** los errores de validación y de recurso no encontrado no generan
  eventos de fallo de auditoría. El wrapper de auditoría registra fallos
  técnicos, como errores de base de datos o internos.
- **Dónde se aplica:** `src-tauri/src/modules/audit/failure.rs`
  (`technical_details` y `finish`); handlers que delegan el resultado a
  `finish`.
- **Prueba:** `d06_motivo_de_201_caracteres_se_rechaza_sin_evento_de_fallo`.

## Actor declarado por el cliente — A5, diferido

- **Regla:** `created_by` es opcional en el DTO y el handler obtiene el actor
  declarado desde ese valor, que puede normalizarse y descartarse si está en
  blanco. En este flujo no se observa que la identidad proceda de una sesión
  autenticada; el nombre declarado no demuestra por sí solo quién realizó la
  operación. La incorporación de autenticación queda diferida como riesgo A5.
- **Dónde se aplica:** `src-tauri/src/modules/sales/dto.rs`;
  `src-tauri/src/modules/sales/handlers.rs`;
  `src-tauri/src/modules/audit/actor.rs`;
  `docs/RIESGOS_VENTAS.md` (A5-01).
- **Prueba:** `declared_actor_trims_and_drops_blank_names` prueba la
  normalización, no la autenticación de la identidad.

## F1 — Anulación y devolución total

- **Regla:** solo se permite cambiar una venta `completed`. La anulación la
  marca `cancelled`; la devolución total la marca `refunded`. Ambas reponen
  todas las cantidades vendidas y registran únicamente la suma neta de pagos
  `cash` como `sale_out`; `cash_received` no se usa para calcular el reembolso.
  Si hay efectivo, el movimiento se registra en el turno abierto actual; sin
  efectivo no se requiere un turno abierto. La operación, sus movimientos,
  estado y auditoría son atómicos. Se exige motivo de 1 a 200 caracteres.
- **Orden de bloqueos:** fila de venta, turno abierto si corresponde y después
  variantes ordenadas por identificador.
- **Dónde se aplica:** `src-tauri/migrations/021_sales_void.sql`;
  `src-tauri/src/modules/sales/repository.rs`;
  `src-tauri/src/modules/inventory/repository.rs`;
  `src-tauri/src/modules/cash/repository.rs`.
- **Pruebas:** `v01_efectivo_con_vuelto_anula_y_cierra_en_cero`,
  `v02_pago_mixto_devuelve_solo_efectivo_neto`,
  `v06_venta_de_turno_cerrado_se_devuelve_en_turno_actual`,
  `v09_anulaciones_concurrentes_reponen_una_sola_vez`,
  `v10_fallo_de_auditoria_revierte_operacion_completa`.
- **CHECK pendiente:** `sales_void_consistency_chk` se crea `NOT VALID` para
  no validar filas históricas al instalar la migración; sí comprueba
  inserciones y actualizaciones nuevas. Ejecutar `VALIDATE CONSTRAINT`
  posteriormente, tras revisar que las filas históricas satisfacen la regla.
- **Alcance:** no hay devolución parcial en este flujo.

## Informes de caja X y Z

- **Regla:** el informe X se calcula en una transacción de lectura repetible
  para un turno abierto y no lo cierra. El informe Z se construye dentro de la
  transacción de cierre y se guarda como snapshot JSONB en
  `cash_session_reports`; no se agrega una columna a `cash_sessions` porque el
  informe contiene varios bloques y debe poder recuperarse como un documento
  reproducible.
- **Ventas y anulaciones:** las ventas se atribuyen por
  `sales.cash_session_id` y solo cuentan mientras su estado sea `completed`.
  Las anulaciones y devoluciones se atribuyen al turno en que ocurrió
  `voided_at`, aunque la venta se originara en otro turno; no se restan de los
  totales de ventas completadas.
- **Efectivo:** el esperado conserva la fórmula del cierre: apertura más
  `sale_in` y `manual_in`, menos `sale_out` y `manual_out`. El informe X/Z
  separa importes de pagos por método, cortesías y movimientos de efectivo.
- **Dónde se aplica:** `src-tauri/migrations/022_cash_session_reports.sql`;
  `src-tauri/src/modules/cash/models.rs`;
  `src-tauri/src/modules/cash/repository.rs`;
  `src-tauri/src/modules/cash/service.rs`;
  `src-tauri/src/modules/billing/ticket.rs`.
- **Pruebas añadidas:** `r01`–`r09` en
  `src-tauri/tests/cash_report_integration.rs`; pruebas unitarias de reimpresión
  y ancho ESC/POS en `billing/ticket.rs`.
- **Histórico y riesgos pendientes:** los turnos cerrados antes de la migración
  022 no tienen un Z guardado. La tabla no es inmutable por trigger (pendiente
  F4). La hora del ticket de venta existente se imprime en UTC; el hallazgo se
  deja pendiente y no se corrige en este cambio.

## Estado de la auditoría

La tabla resume la correspondencia de los diez criterios con las reglas
documentadas y las pruebas encontradas en el repositorio. No certifica que las
pruebas hayan sido ejecutadas durante la elaboración de este documento.

| # | Criterio de aceptación | Resultado |
|---|---|---|
| 1 | M3: las ventas nuevas requieren turno y guardan `cash_session_id`; las históricas pueden quedar sin turno. | Verificado en migración, repositorio y pruebas `a01`, `a02`, `a03`, `a15`, `d11`. |
| 2 | M5: cortesía derivada de total cero, motivo obligatorio de hasta 200 caracteres y bruto cero rechazado. | Verificado en migración, repositorio y pruebas `a16`, `a17`, `d05`, `d06`, `d08`, `d09`. |
| 3 | A4: recibido y vuelto opcionales y coherentes, calculados sobre pagos netos en efectivo. | Verificado en migración, repositorio y pruebas `e01`–`e09`. |
| 4 | M2: semántica bruta señalada por `gross_semantics`, descuentos agregados y scripts históricos. | Verificado en migración, repositorio, scripts dry-run/apply/rollback y pruebas `f01`–`f05`. |
| 5 | M1: costo persistido como snapshot en la línea y movimiento, nullable para desconocido. | Verificado en migración, repositorios y pruebas `h01`–`h05`. |
| 6 | N7: importes en centésimas, redondeo según precisión y restricción de fracciones en efectivo recibido. | Verificado en `money.rs`, repositorio y pruebas monetarias y `e05`. |
| 7 | Idempotencia mediante clave UUID con devolución de venta existente. | Verificado en migración, repositorio y pruebas `c01`, `c02`, `c06`, `e10`; el payload no se compara. |
| 8 | Los rechazos de validación no se registran como eventos de fallo de auditoría. | Verificado en wrapper de auditoría y prueba `d06`. |
| 9 | El actor se deriva del campo declarado por el cliente; autenticación pendiente A5. | Verificado en DTO, handler, helper y A5-01; la prueba del helper no verifica autenticación. |
| 10 | Anulación y devolución total de venta completada son atómicas, restituyen stock y devuelven el neto efectivo en el turno actual. | Implementado; pruebas `v01`–`v11` añadidas. Validación del constraint histórico queda pendiente. |

## Fase 5.4 — Ventas en espera locales

- **Regla:** los carritos se guardan en `localStorage` bajo
  `pos.heldCarts.v1`, con un máximo de cinco. Se asocian al identificador del
  turno abierto, no reservan stock y se eliminan al cambiar de turno o quedar
  sin turno. Retomarlos revalida precios y stock y exige que el operador revise
  el contenido; los pagos se vuelven a capturar.
- **Estado de negocio:** una venta en espera no es una venta registrada y no
  genera pagos ni movimientos de inventario.
- **Lógica de carrito:** los cálculos puros viven en `src/utils/cart.ts`;
  almacenamiento, validación y poda por turno en `src/utils/heldSales.ts`.
- **Pruebas:** `src/utils/cart.test.ts` y
  `src/utils/heldSales.test.ts`.
