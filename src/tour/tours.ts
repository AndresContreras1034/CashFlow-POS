export interface TourStep {
  /** Selector CSS. Si no existe en pantalla, el paso se muestra centrado (sin resaltar). */
  element?: string
  title: string
  description: string
  side?: 'top' | 'right' | 'bottom' | 'left'
  /** Selector a pulsar ANTES de mostrar este paso (por ejemplo, una pestaña). */
  click?: string
}

export interface TourDef {
  id: string
  label: string
  description: string
  route: string
  /** Selector que debe existir antes de empezar. */
  waitFor?: string
  /** Si es true: no se lanza desde Ajustes ni en el tour completo (solo con el botón "?"). */
  standalone?: boolean
  /** Para rutas con parámetros (ej: /inventory/12). */
  match?: RegExp
  steps: TourStep[]
}

export const TOURS: TourDef[] = [
  /* ------------------------------------------------------------------ VENTAS */
  {
    id: 'sales',
    label: 'Ventas',
    description: 'Cómo cobrar una venta paso a paso.',
    route: '/sales',
    waitFor: '.sales-scan-form',
    steps: [
      {
        title: 'Pantalla de ventas',
        description:
          'Aquí se registran las ventas. El flujo es: agregar productos al carrito, elegir cliente (opcional), definir el pago y confirmar. Para vender necesitas un turno de caja abierto.',
      },
      {
        element: '.sales-shortcuts',
        title: 'Atajos de teclado',
        description:
          'F2 lleva el cursor al escáner, F4 al campo de efectivo recibido, F9 confirma la venta y Esc cierra avisos.',
        side: 'bottom',
      },
      {
        element: '.sales-scan-form',
        title: 'Escáner / búsqueda',
        description:
          'Escanea el código de barras o escribe el nombre, SKU o código y pulsa Enter. Si hay varias coincidencias aparece una lista para elegir.',
        side: 'bottom',
      },
      {
        element: '.sales-held-section',
        title: 'Ventas en espera',
        description:
          'Puedes guardar un carrito para atender a otro cliente y retomarlo después (máximo 5 por turno). Las ventas en espera NO están registradas ni reservan stock.',
        side: 'bottom',
      },
      {
        element: '.sales-cart-heading',
        title: 'Carrito',
        description:
          'Cuando hay productos aparecen los botones “Guardar en espera” y “Vaciar carrito” (pide confirmar con un segundo clic).',
        side: 'bottom',
      },
      {
        element: '.sales-cart-heading + .form-field',
        title: 'Cliente (opcional)',
        description:
          'Asocia la venta a un cliente registrado. Si lo dejas vacío, queda como “Consumidor final”.',
        side: 'bottom',
      },
      {
        element: '.sales-table',
        title: 'Líneas del carrito',
        description:
          'Cada línea muestra precio, cantidad (con − y +), descuento por línea, subtotal y el stock disponible. Si la cantidad supera el stock se resalta en aviso.',
        side: 'top',
      },
      {
        element: '.sales-auto-print',
        title: 'Imprimir al confirmar',
        description:
          'Si lo activas, el ticket se imprime automáticamente al registrar la venta. Esta sección aparece cuando hay productos en el carrito.',
        side: 'top',
      },
      {
        element: '.sales-courtesy-toggle',
        title: 'Venta de cortesía',
        description:
          'Para regalar productos: el total queda en $0 y es obligatorio escribir el motivo. Queda marcada como cortesía en el historial.',
        side: 'top',
      },
      {
        element: '.sales-total-row',
        title: 'Total a pagar',
        description: 'Suma de las líneas menos los descuentos.',
        side: 'top',
      },
      {
        element: '.sales-payment-row',
        title: 'Formas de pago',
        description:
          'Elige efectivo, tarjeta o transferencia y el monto. Puedes dividir el pago en varias formas con “+ Agregar forma de pago”, o usar “Llenar con el total”.',
        side: 'top',
      },
      {
        element: '.sales-cash-tender',
        title: 'Efectivo recibido y vuelto',
        description:
          'Escribe cuánto entrega el cliente o usa los botones rápidos de billetes. El sistema calcula el vuelto y avisa si falta dinero.',
        side: 'top',
      },
      {
        element: '.sales-confirm-btn',
        title: 'Confirmar venta',
        description:
          'Se habilita cuando los pagos cubren exactamente el total. Antes de registrar, el sistema revisa si algún precio cambió.',
        side: 'top',
      },
    ],
  },

  /* ------------------------------------------------------- HISTORIAL VENTAS */
  {
    id: 'sales-history',
    label: 'Historial de ventas',
    description: 'Buscar ventas, reimprimir, anular o devolver.',
    route: '/sales-history',
    waitFor: '.sales-history-filters',
    steps: [
      {
        title: 'Historial de ventas',
        description:
          'Todas las ventas registradas, de la más reciente a la más antigua. Haz clic en una fila para ver su detalle.',
      },
      {
        element: '.sales-history-presets',
        title: 'Períodos rápidos',
        description: 'Hoy, Ayer o los últimos 7 días con un solo clic.',
        side: 'bottom',
      },
      {
        element: '.sales-history-filters',
        title: 'Filtros',
        description:
          'Filtra por estado, fechas, número de venta (#12 o T-000012), método de pago, número de turno o cliente. Pulsa “Filtrar” para aplicar.',
        side: 'bottom',
      },
      {
        element: '.sales-history-filter-actions',
        title: 'Filtrar y limpiar',
        description: '“Filtrar” aplica los criterios; “Limpiar” los quita todos.',
        side: 'bottom',
      },
      {
        element: '.sales-history-card',
        title: 'Resultados y total',
        description:
          'Arriba ves cuántas ventas hay y el total de las completadas. Las anuladas y reembolsadas no suman. Las ventas de cortesía llevan una etiqueta.',
        side: 'top',
      },
      {
        element: '.sales-history-table',
        title: 'Detalle de una venta',
        description:
          'Al abrir una venta puedes: reimprimir el ticket (sale marcado como COPIA), asignar o cambiar el cliente, y anular o devolver. Anular/devolver repone el stock y, si hubo efectivo, lo devuelve del turno actual; exige escribir un motivo.',
        side: 'top',
      },
    ],
  },

  /* -------------------------------------------------------------- INVENTARIO */
  {
    id: 'inventory',
    label: 'Inventario',
    description: 'Productos, stock, filtros, Excel y toma de inventario.',
    route: '/inventory',
    waitFor: '.inv-filters',
    steps: [
      {
        title: 'Inventario',
        description:
          'Aquí administras tus productos. Cada producto tiene variantes (por ejemplo talla o color) y el stock se lleva por variante.',
      },
      {
        element: '.inv-header-actions button:nth-child(1)',
        title: 'Toma de inventario',
        description: 'Abre el conteo físico para comparar lo que hay con lo que dice el sistema.',
        side: 'bottom',
      },
      {
        element: '.inv-header-actions button:nth-child(2)',
        title: 'Exportar a Excel',
        description:
          'Descarga todo el inventario en un .xlsx: variantes activas, inactivas y las de stock bajo.',
        side: 'bottom',
      },
      {
        element: '.inv-header-actions button:nth-child(3)',
        title: 'Importar desde Excel',
        description:
          'Carga o actualiza productos en bloque usando la plantilla de inventario.',
        side: 'bottom',
      },
      {
        element: '.inv-header-actions button:nth-child(4)',
        title: 'Nuevo producto',
        description:
          'Crea un producto. Luego, desde su detalle, agregas las variantes con precio, costo, SKU y código de barras.',
        side: 'bottom',
      },
      {
        element: '.inv-low-stock-banner',
        title: 'Alerta de stock bajo',
        description:
          'Aparece cuando hay variantes con poca existencia o agotadas, y muestra algunas de ellas.',
        side: 'bottom',
      },
      {
        element: '.search-wrap',
        title: 'Búsqueda',
        description: 'Busca por nombre, marca, SKU o código. Filtra sobre TODO el inventario, no solo la página visible.',
        side: 'bottom',
      },
      {
        element: '.inv-filters',
        title: 'Filtros y orden',
        description:
          'Filtra por categoría, nivel de stock (normal, bajo, agotado) y estado de variantes, y ordena por nombre, marca, categoría o fecha.',
        side: 'bottom',
      },
      {
        element: '.filter-tabs',
        title: 'Activos / Inactivos',
        description:
          'Por defecto ves solo productos activos. Los inactivos no se venden pero conservan su historial.',
        side: 'bottom',
      },
      {
        element: '.inv-table-wrap',
        title: 'Tabla de productos',
        description:
          'Muestra categoría, número de variantes, stock total con su indicador de color y estado. Haz clic en una fila para abrir el detalle del producto.',
        side: 'top',
      },
      {
        element: '.inv-row .cell-actions',
        title: 'Editar',
        description: 'El lápiz edita los datos del producto sin entrar al detalle.',
        side: 'left',
      },
      {
        element: '.inv-pagination',
        title: 'Paginación',
        description: 'Navega entre páginas y elige cuántos productos ver por página.',
        side: 'top',
      },
    ],
  },

  /* ------------------------------------------------------ DETALLE PRODUCTO */
  {
    id: 'product-detail',
    label: 'Detalle de producto',
    description: 'Variantes, movimientos de stock y kardex.',
    route: '/inventory',
    match: /^\/inventory\/\d+/,
    standalone: true,
    waitFor: '.detail-hero',
    steps: [
      {
        element: '.detail-hero',
        title: 'Ficha del producto',
        description: 'Categoría, estado, nombre, marca y descripción del producto.',
        side: 'bottom',
      },
      {
        element: '.detail-stats',
        title: 'Resumen',
        description:
          'Stock total, variantes activas, el precio más bajo y cuántos movimientos de kardex tiene.',
        side: 'bottom',
      },
      {
        element: '.detail-tabs',
        title: 'Variantes y Kardex',
        description: 'Alterna entre las variantes del producto y su historial de movimientos.',
        side: 'bottom',
      },
      {
        element: '.section-header .btn-primary',
        title: 'Nueva variante',
        description:
          'Una variante es una versión vendible (talla, color…). Lleva precio, costo, stock mínimo, SKU y código de barras.',
        side: 'left',
      },
      {
        element: '.variant-card .variant-prices',
        title: 'Precio, costo y margen',
        description: 'El margen se calcula con el precio y el costo de la variante.',
        side: 'top',
      },
      {
        element: '.variant-card .variant-actions',
        title: 'Acciones de stock',
        description:
          'Entrada suma stock, Salida lo descuenta y Ajuste lo corrige a un valor. Etiqueta imprime códigos de barras. Todo queda en el kardex con su motivo.',
        side: 'top',
      },
      {
        element: '.detail-section',
        click: '.detail-tabs .detail-tab:nth-child(2)',
        title: 'Kardex',
        description:
          'Historial de todos los movimientos del producto: entradas, salidas, ventas y ajustes, con fecha, motivo y quién los hizo.',
        side: 'top',
      },
    ],
  },

  /* -------------------------------------------------------------- STOCKTAKE */
  {
    id: 'stocktake',
    label: 'Toma de inventario',
    description: 'Conteo físico y ajuste de diferencias.',
    route: '/stocktake',
    waitFor: '.st-title',
    steps: [
      {
        title: 'Toma de inventario',
        description:
          'Cuentas el stock físico sin ver el del sistema; al terminar revisas las diferencias y las aplicas todas juntas. Lo que no cuentes no se modifica.',
      },
      {
        element: '#stocktake-category',
        title: 'Alcance',
        description: 'Cuenta todo el inventario o solo una categoría.',
        side: 'bottom',
      },
      {
        element: '#stocktake-notes',
        title: 'Notas',
        description: 'Opcional, por ejemplo “Conteo de fin de mes”.',
        side: 'bottom',
      },
      {
        element: '.st-warn',
        title: 'Evita mover stock',
        description:
          'Mientras cuentas, evita vender esas variantes. Si hay movimientos, la revisión te lo avisa.',
        side: 'top',
      },
      {
        element: '.st-progress',
        title: 'Progreso',
        description: 'Cuántas variantes llevas contadas del total. (Visible con una toma iniciada.)',
        side: 'bottom',
      },
      {
        element: '.st-scan',
        title: 'Escáner',
        description: 'Cada lectura de un código o SKU suma 1 al conteo de esa variante.',
        side: 'bottom',
      },
      {
        element: '.st-filters',
        title: 'Buscar y pendientes',
        description: 'Busca una variante o muestra solo las que aún no has contado.',
        side: 'bottom',
      },
      {
        element: '.st-table-wrap',
        title: 'Conteo manual',
        description:
          'Escribe la cantidad contada y pulsa Enter o sal del campo para guardarla. Vacío significa “sin contar”.',
        side: 'top',
      },
      {
        element: '.st-actions',
        title: 'Finalizar y revisar',
        description:
          'Muestra sobrantes, faltantes y variantes con movimientos durante el conteo. Al aplicar, cada diferencia queda en el Kardex como “Ajuste: corrección de conteo”. No se puede deshacer.',
        side: 'top',
      },
    ],
  },

  /* -------------------------------------------------------------------- CAJA */
  {
    id: 'cash',
    label: 'Caja',
    description: 'Turnos, movimientos manuales, arqueo e informes X / Z.',
    route: '/cash',
    waitFor: '.cash-card',
    steps: [
      {
        title: 'Caja',
        description:
          'Controla el dinero por turnos: abres con un monto inicial, registras movimientos, y al final cuentas el efectivo (arqueo) y cierras.',
      },
      {
        element: '#cash-operator',
        title: 'Operador de caja',
        description:
          'Nombre de quien opera. Se guarda solo en este equipo y se registra como autor en los movimientos y la auditoría.',
        side: 'bottom',
      },
      {
        element: '.cash-form',
        title: 'Abrir turno',
        description:
          'Si no hay turno abierto, ingresa el monto inicial en caja. Sin turno abierto no se pueden registrar ventas.',
        side: 'top',
      },
      {
        element: '.cash-summary-grid',
        title: 'Resumen del turno',
        description:
          'Apertura, ingresos, egresos y saldo actual. Las ventas en efectivo suman automáticamente.',
        side: 'bottom',
      },
      {
        element: '.cash-summary .btn-ghost',
        title: 'Informe X',
        description:
          'Foto del turno ABIERTO: no lo cierra. Sirve para revisar cómo va la caja en cualquier momento.',
        side: 'bottom',
      },
      {
        element: '.cash-summary .btn-danger',
        title: 'Cerrar turno (arqueo)',
        description:
          'Ingresa el efectivo contado. El sistema lo compara con lo esperado y genera el Informe Z.',
        side: 'bottom',
      },
      {
        element: '.cash-form-row',
        title: 'Movimiento manual',
        description:
          'Registra dinero que entra o sale sin ser una venta (por ejemplo, pagar un domicilio). Indica una nota para recordar el motivo.',
        side: 'top',
      },
      {
        element: '.cash-table',
        title: 'Movimientos del turno',
        description: 'Lista de ingresos y egresos, manuales y por ventas o devoluciones.',
        side: 'top',
      },
      {
        element: '.cash-closed-sessions',
        title: 'Turnos cerrados',
        description:
          'Consulta el Informe Z de turnos anteriores, o reimprímelo.',
        side: 'top',
      },
    ],
  },

  /* ---------------------------------------------------------------- CLIENTES */
  {
    id: 'customers',
    label: 'Clientes',
    description: 'Directorio de clientes y sus compras.',
    route: '/customers',
    waitFor: '.customers-filters',
    steps: [
      {
        title: 'Clientes',
        description:
          'Directorio de clientes y cuánto han comprado. Puedes asociarlos a las ventas para llevar su historial.',
      },
      {
        element: '.customers-header .customers-btn-primary',
        title: 'Nuevo cliente',
        description: 'Registra nombre, documento, teléfono y correo.',
        side: 'left',
      },
      {
        element: '#customers-search',
        title: 'Búsqueda',
        description: 'Por nombre, documento, teléfono o correo.',
        side: 'bottom',
      },
      {
        element: '#customers-active',
        title: 'Estado',
        description:
          'Ver clientes activos, inactivos o todos. Un cliente inactivo no se puede asignar a nuevas ventas.',
        side: 'bottom',
      },
      {
        element: '.customers-table-wrap',
        title: 'Listado',
        description:
          'Muestra número de compras, total comprado y fecha de la última compra. Haz clic en un cliente para ver su detalle o editarlo.',
        side: 'top',
      },
      {
        element: '.customers-pagination',
        title: 'Paginación',
        description: 'Navega entre páginas de resultados.',
        side: 'top',
      },
    ],
  },

  /* ------------------------------------------------------------------ DEUDAS */
  {
    id: 'debts',
    label: 'Deudas',
    description: 'Obligaciones por pagar del negocio y sus abonos.',
    route: '/debts',
    waitFor: '.debts-summary',
    steps: [
      {
        title: 'Deudas',
        description:
          'Aquí llevas las obligaciones por pagar del negocio: a quién le debes, cuánto queda pendiente, cuándo vence y qué abonos has hecho.',
      },
      {
        element: '.debts-header .btn-primary',
        title: 'Nueva deuda',
        description:
          'Registra una obligación con su categoría, naturaleza, monto, fechas y datos del acreedor.',
        side: 'left',
      },
      {
        element: '.debts-card--primary',
        title: 'Total pendiente',
        description: 'Suma del saldo que aún debes y en cuántas obligaciones activas está repartido.',
        side: 'bottom',
      },
      {
        element: '.debts-summary button:nth-of-type(1)',
        title: 'Vencido',
        description:
          'Lo que ya pasó su fecha de vencimiento. Haz clic para filtrar el listado solo por las vencidas; vuelve a pulsar para quitar el filtro.',
        side: 'bottom',
      },
      {
        element: '.debts-summary button:nth-of-type(2)',
        title: 'Próximo a vencer',
        description: 'Obligaciones que vencen pronto. También filtra el listado al pulsarla.',
        side: 'bottom',
      },
      {
        element: '.debts-categories',
        title: 'Indicadores por categoría',
        description:
          'Despliégalo para ver, por categoría, cuántas hay activas, saldo pendiente, vencido, pagadas y anuladas.',
        side: 'bottom',
      },
      {
        element: '#debts-search',
        title: 'Búsqueda',
        description: 'Por acreedor, concepto o referencia.',
        side: 'bottom',
      },
      {
        element: '#debts-category',
        title: 'Categoría y naturaleza',
        description:
          'Filtra por el tipo de obligación (categoría) y por su naturaleza. Las opciones vienen del sistema.',
        side: 'bottom',
      },
      {
        element: '#debts-status',
        title: 'Estado y vencimiento',
        description:
          'Estado de la deuda (por ejemplo pendiente o pagada) y grupo de vencimiento. “Requieren atención” resume lo urgente.',
        side: 'bottom',
      },
      {
        element: '.debts-dates',
        title: 'Filtro por fechas',
        description:
          'Despliégalo para filtrar por fecha de emisión, de vencimiento o de pago. Si el rango es inválido se avisa.',
        side: 'top',
      },
      {
        element: '.debts-filters-actions',
        title: 'Atención y limpiar',
        description:
          'Acceso rápido a lo que requiere atención y botón para limpiar todos los filtros. Los filtros quedan en la dirección de la página.',
        side: 'top',
      },
      {
        element: '.debts-results',
        title: 'Listado de deudas',
        description:
          'Abre una deuda para ver su detalle e historial de pagos, editarla o registrar un abono (también hay un botón directo de pago en cada fila).',
        side: 'top',
      },
      {
        element: '.debts-pagination',
        title: 'Paginación',
        description: 'Navega entre páginas de resultados.',
        side: 'top',
      },
    ],
  },

  /* ------------------------------------------------------------- PROVEEDORES */
  {
    id: 'suppliers',
    label: 'Proveedores',
    description: 'Directorio, reposición de stock y panel “Para llamar”.',
    route: '/suppliers',
    waitFor: '.suppliers-view-tabs',
    steps: [
      {
        title: 'Proveedores',
        description:
          'Directorio de proveedores y reposición de stock: sabes a quién pedirle cada producto y qué te hace falta.',
      },
      {
        element: '.suppliers-header .suppliers-btn-primary',
        title: 'Nuevo proveedor',
        description:
          'Registra contacto, documento, prioridad y condiciones (plazo de entrega y de pago).',
        side: 'left',
      },
      {
        element: '.suppliers-card--primary',
        title: 'Proveedores activos',
        description: 'Cuántos proveedores activos tienes en el catálogo.',
        side: 'bottom',
      },
      {
        element: '.suppliers-summary button:nth-of-type(1)',
        title: 'Por contactar',
        description:
          'Proveedores con productos por reponer. Al pulsarla se abre el panel “Para llamar”.',
        side: 'bottom',
      },
      {
        element: '.suppliers-summary button:nth-of-type(2)',
        title: 'Urgentes',
        description:
          'Suma de productos agotados y críticos. Abre el panel “Para llamar” mostrando solo lo urgente.',
        side: 'bottom',
      },
      {
        element: '.suppliers-summary button:nth-of-type(3)',
        title: 'Sin proveedor',
        description:
          'Productos que no tienen a quién pedirles. Conviene vincularlos a un proveedor desde su ficha.',
        side: 'bottom',
      },
      {
        element: '.suppliers-view-tabs',
        title: 'Directorio / Para llamar',
        description:
          'Alterna entre el directorio completo y el panel de proveedores a los que conviene llamar hoy.',
        side: 'bottom',
      },
      {
        element: '.suppliers-export-row',
        title: 'Exportar CSV',
        description: 'Descarga el directorio (con los filtros aplicados) para abrirlo en Excel.',
        side: 'bottom',
      },
      {
        element: '#suppliers-search',
        title: 'Búsqueda',
        description: 'Por nombre, contacto, documento, teléfono o correo.',
        side: 'bottom',
      },
      {
        element: '#suppliers-priority',
        title: 'Prioridad',
        description: 'Normal, Preferido o Urgente. Sirve para decidir a quién contactar primero.',
        side: 'bottom',
      },
      {
        element: '.suppliers-table-wrap',
        title: 'Listado',
        description:
          'Cuántas variantes y categorías maneja, cuántos productos suyos están agotados o bajos, y sus condiciones. Haz clic para abrir su ficha: datos, categorías, productos, reposición y cuentas por pagar.',
        side: 'top',
      },
      {
        element: '#suppliers-view-panel',
        click: '#suppliers-view-call',
        title: 'Panel “Para llamar”',
        description:
          'Lista de proveedores con productos por reponer. Desde cada tarjeta puedes llamar, escribir por WhatsApp o correo, y copiar el pedido ya armado.',
        side: 'top',
      },
    ],
  },

  /* ---------------------------------------------------------------- REPORTES */
  {
    id: 'reports',
    label: 'Reportes',
    description: 'Resumen, productos, inventario y obligaciones.',
    route: '/reports',
    waitFor: '.reports-tabs',
    steps: [
      {
        title: 'Reportes',
        description:
          'Ventas y comportamiento del negocio. Primero eliges el período y luego la pestaña que quieres revisar.',
      },
      {
        element: '.reports-presets',
        title: 'Períodos rápidos',
        description: 'Hoy, Ayer, últimos 7 o 30 días, este mes o el mes anterior.',
        side: 'bottom',
      },
      {
        element: '.reports-filter-controls',
        title: 'Rango personalizado',
        description:
          'Elige fechas “Desde” y “Hasta”, agrupa por día, semana o mes, y marca “Comparar con período anterior” para ver la variación.',
        side: 'bottom',
      },
      {
        element: '.reports-tabs',
        title: 'Secciones',
        description: 'Resumen, Productos, Inventario y Obligaciones. Veámoslas una por una.',
        side: 'bottom',
      },
      {
        element: '#reports-panel-summary',
        click: '#reports-tab-summary',
        title: 'Resumen',
        description:
          'Vista general del período: ventas, evolución por fecha, métodos de pago y mapa de calor de los días y horas con más movimiento.',
        side: 'top',
      },
      {
        element: '#reports-panel-products',
        click: '#reports-tab-products',
        title: 'Productos',
        description:
          'Ranking de productos por unidades, ingresos o ganancia. Filtra por categoría y elige cuántos mostrar.',
        side: 'top',
      },
      {
        element: '#reports-panel-inventory',
        click: '#reports-tab-inventory',
        title: 'Inventario',
        description:
          'Valor del stock y alertas de productos con poca existencia o agotados, con filtro por categoría.',
        side: 'top',
      },
      {
        element: '#reports-panel-payables',
        click: '#reports-tab-payables',
        title: 'Obligaciones',
        description:
          'Lo que el negocio debe: principales acreedores y próximos vencimientos.',
        side: 'top',
      },
    ],
  },

  /* --------------------------------------------------------------- AUDITORÍA */
  {
    id: 'audit',
    label: 'Auditoría',
    description: 'Registro de solo lectura de las operaciones.',
    route: '/audit',
    waitFor: '.audit-header',
    steps: [
      {
        element: '.audit-header',
        title: 'Auditoría',
        description:
          'Registro de solo lectura de las operaciones del sistema: ventas, anulaciones, cambios de stock, caja y más. Nadie puede editarlo.',
        side: 'bottom',
      },
      {
        element: '#audit-search',
        title: 'Búsqueda',
        description: 'Por resumen, acción, id o texto del error.',
        side: 'bottom',
      },
      {
        element: '#audit-module',
        title: 'Módulo y categoría',
        description:
          'Limita los eventos a un módulo (por ejemplo Ventas o Caja) y a un tipo de operación.',
        side: 'bottom',
      },
      {
        element: '#audit-outcome',
        title: 'Resultado',
        description: 'Filtra por éxito o fallo de la operación.',
        side: 'bottom',
      },
      {
        element: '#audit-action',
        title: 'Acción y entidad',
        description:
          'Filtros avanzados: acción exacta (ej. stock_adjust), tipo de entidad (ej. sale) y su id, para seguir la historia de un registro concreto.',
        side: 'bottom',
      },
      {
        element: '#audit-date-from',
        title: 'Fechas',
        description: 'Rango “Desde / Hasta”. Si el rango es inválido, el listado no se carga.',
        side: 'bottom',
      },
      {
        element: '.audit-filters-actions',
        title: 'Solo errores y limpiar',
        description:
          '“Solo errores” muestra únicamente operaciones fallidas, útil para detectar problemas. Los filtros quedan en la dirección de la página.',
        side: 'top',
      },
      {
        element: '.audit-status',
        title: 'Conteo de eventos',
        description: 'Cuántos eventos coinciden con los filtros actuales.',
        side: 'bottom',
      },
      {
        element: '.audit-results',
        title: 'Tabla de eventos',
        description:
          'Haz clic en un evento para ver su detalle: quién lo hizo, cuándo y qué cambió.',
        side: 'top',
      },
      {
        element: '.audit-pagination',
        title: 'Paginación',
        description: 'Navega entre páginas de eventos.',
        side: 'top',
      },
    ],
  },

  /* ----------------------------------------------------------------- AJUSTES */
  {
    id: 'settings',
    label: 'Ajustes',
    description: 'Negocio, dinero, ticket, temas, atajos y copias de seguridad.',
    route: '/settings',
    waitFor: '.settings-tabs',
    steps: [
      {
        element: '.settings-tabs',
        title: 'Ajustes',
        description: 'Cada pestaña agrupa un tipo de configuración. Recorrámoslas.',
        side: 'bottom',
      },
      {
        element: '.settings-tab:nth-child(1)',
        title: 'Negocio',
        description:
          'Nombre, NIT/RUT, dirección, teléfono, correo, logo y el operador (quien aparece como autor en la auditoría).',
        side: 'bottom',
      },
      {
        element: '.settings-tab:nth-child(2)',
        title: 'Dinero e impuestos',
        description:
          'Moneda, decimales, nombre y tasa del impuesto. Cambiar la moneda o decimales no modifica los importes guardados, solo cómo se muestran.',
        side: 'bottom',
      },
      {
        element: '.settings-tab:nth-child(3)',
        title: 'Regional',
        description:
          'Zona horaria. Define cuándo empieza y termina el día para cortes de caja y reportes.',
        side: 'bottom',
      },
      {
        element: '.settings-tab:nth-child(4)',
        title: 'Inventario',
        description: 'Umbral por defecto para considerar que un producto tiene stock bajo.',
        side: 'bottom',
      },
      {
        element: '.settings-tab:nth-child(5)',
        title: 'Ticket',
        description:
          'Encabezado y pie del ticket, y qué datos se imprimen (logo, NIT, vendedor, impuestos, descuentos, forma de pago).',
        side: 'bottom',
      },
      {
        element: '.settings-tab:nth-child(6)',
        title: 'Temas',
        description: 'Cambia los colores de la aplicación. Se guarda solo en este equipo.',
        side: 'bottom',
      },
      {
        element: '.settings-tab:nth-child(7)',
        title: 'Atajos',
        description:
          'Asigna una tecla (letra, número o F1–F12) para abrir cada sección desde cualquier pantalla. No se activa mientras escribes en un campo.',
        side: 'bottom',
      },
      {
        element: '.settings-tab:nth-child(8)',
        title: 'Backup',
        description:
          'Copias de seguridad programadas, su historial y la restauración. Revisa que estén funcionando: es lo que protege tus datos.',
        side: 'bottom',
      },
      {
        element: '.settings-tab:nth-child(9)',
        title: 'Desarrollador',
        description:
          'Modo de diagnóstico técnico temporal. No es la auditoría y normalmente no hace falta activarlo.',
        side: 'bottom',
      },
      {
        element: '.settings-tab:nth-child(10)',
        title: 'Tutoriales',
        description: 'Aquí puedes repetir cualquier tutorial cuando quieras.',
        side: 'bottom',
      },
    ],
  },
]

export const getTour = (id: string) => TOURS.find((t) => t.id === id)

export const getTourByRoute = (pathname: string): TourDef | undefined =>
  TOURS.find((t) => t.match?.test(pathname)) ??
  TOURS.find((t) => !t.match && pathname === t.route)

/** Tours que se muestran en Ajustes y en el tour completo. */
export const LISTED_TOURS = TOURS.filter((t) => !t.standalone)