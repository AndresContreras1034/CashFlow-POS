# Reportes

## Pestañas y comandos

- **Resumen:** `get_dashboard_kpis`, `get_sales_series`,
  `get_sales_by_payment_method` y `get_sales_heatmap`.
- **Productos:** `get_top_products`, `get_sales_by_category` y
  `get_stale_products`.
- **Inventario:** `get_inventory_valuation`, `get_inventory_turnover` y
  `get_stock_alerts`.
- **Obligaciones:** `get_payables_summary`, `get_payables_aging`,
  `get_top_creditors` y `get_upcoming_dues`.

La exportación de cualquiera de las pestañas usa `save_report_csv`.

## Reglas de negocio

- Una cortesía tiene total cero y cuenta como ticket. Su costo reduce la
  utilidad, pero no entra en los rankings de productos ni en el cálculo del
  ticket promedio.
- La utilidad del reporte de ventas se calcula únicamente sobre tickets cuyos
  ítems tienen todos costo registrado. Los ítems sin costo se informan aparte
  y no forman parte de esa base de utilidad.
- El descuento global de una venta y su IVA se distribuyen entre los ítems de
  forma proporcional a sus subtotales; el redondeo asigna los residuos en un
  orden determinista para conservar los totales de la venta.
- Las ventas anuladas y reembolsadas se muestran en conteos separados y no se
  suman a las métricas de ventas completadas.

## Fechas y vigencia

- La valorización de inventario, las existencias y las alertas de stock
  describen el estado actual. Las unidades vendidas, cobertura y reposición
  sugerida de las alertas dependen del rango seleccionado.
- Los saldos, antigüedad, acreedores y próximos vencimientos de obligaciones
  son una foto actual. Los pagos y las deudas emitidas en el resumen dependen
  del rango.
- Las ventas y la rotación dependen del rango. La rotación usa el inventario
  actual a costo, no el inventario promedio del período; no es comparable
  directamente entre rangos de distinta duración.
- Los días de los reportes se interpretan en la zona horaria configurada en
  `app_settings.timezone`.

## Limitaciones de exportación e interpretación

- Los acreedores se agrupan por coincidencia exacta del nombre; diferencias
  de escritura se muestran como acreedores distintos.
- Los archivos CSV usan punto y coma (`;`) como separador e incluyen BOM UTF-8
  para facilitar su apertura en hojas de cálculo.
