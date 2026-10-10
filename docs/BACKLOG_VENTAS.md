# Backlog de ventas

Elementos pendientes basados en la interfaz y los servicios inspeccionados.

1. **Imprimir al confirmar la venta**
   - **Estado:** pendiente.
   - **Situación actual:** `Sales.tsx` registra la venta sin llamar a impresión.
     La acción `printSaleTicket` se ejecuta desde el detalle del historial.
   - **Trabajo propuesto:** ofrecer la impresión al terminar de confirmar la
     venta, conservando el manejo visible de errores de impresión.

2. **Atajos de teclado para el punto de venta**
   - **Estado:** pendiente.
   - **Atajos solicitados:** F2 enfoca el escáner; Enter confirma; Esc limpia;
     `+`/`-` modifican la cantidad; Supr quita la línea seleccionada.
   - **Situación actual:** no se encontraron estos atajos específicos en
     `Sales.tsx`; la pantalla dispone de controles con clic e inputs.

3. **Aviso cuando la cantidad supera el stock**
   - **Estado:** pendiente.
   - **Situación actual:** cada línea de carrito conserva `stock`, proveniente de
     la variante, pero la tabla del carrito no lo muestra ni compara visualmente
     con `quantity`. La validación definitiva de stock se realiza en el backend
     al confirmar.
   - **Trabajo propuesto:** mostrar stock disponible y advertir antes de
     confirmar cuando la cantidad solicitada lo supere, sin sustituir la
     validación del servidor.

4. **Nombre del producto en el detalle del historial**
   - **Estado:** pendiente.
   - **Situación actual:** el detalle muestra `#variant_id`; las filas de ítems
     devueltas por el servicio de ventas no incluyen el nombre del producto.
   - **Trabajo propuesto:** devolver y mostrar un nombre legible junto a los
     atributos de la variante, cuando estén disponibles.

5. **Ampliar filtros y mostrar el total filtrado**
   - **Estado:** pendiente.
   - **Situación actual:** el historial filtra por estado y rango de fechas y
     pagina los resultados. El valor `total` de la respuesta se usa para contar
     páginas; no es la suma monetaria de las ventas filtradas. No se encontraron
     presets Hoy/Ayer/Semana, búsqueda por número, filtro por método de pago ni
     filtro por turno de caja.
   - **Trabajo propuesto:** añadir esos presets y filtros, y mostrar la suma
     monetaria del conjunto que cumpla los filtros, separada de la cantidad de
     resultados.

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
