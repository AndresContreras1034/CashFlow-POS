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

## Anulación y devolución

- **Regla:** **DECISIÓN PENDIENTE: ver sección final**.
- **Dónde se aplica actualmente:** `src-tauri/src/modules/sales/models.rs`
  define estados `cancelled` y `refunded`; `src-tauri/src/modules/cash/models.rs`
  define `SaleOut`, y `src-tauri/src/modules/cash/service.rs` restringe el
  comando manual a movimientos manuales. Estas definiciones no constituyen por
  sí solas un flujo de anulación o devolución de venta.
- **Prueba:** no se identificó en las pruebas de ventas consultadas una prueba
  que cubra un flujo completo de anulación/devolución.

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
| 10 | Existe una decisión y cobertura para un flujo completo de anulación/devolución. | **Pendiente:** decisión y prueba de flujo no identificadas; DECISIÓN PENDIENTE: ver sección final. |

### Sección final — decisión pendiente sobre anulación/devolución

**DECISIÓN PENDIENTE:** definir la regla de negocio para anulación y devolución,
incluidos los estados permitidos, el efecto sobre pagos y caja, la restitución
de inventario, la autorización y la auditoría. No se documenta una decisión
como implementada porque no se encontró un flujo completo que la materialice.
