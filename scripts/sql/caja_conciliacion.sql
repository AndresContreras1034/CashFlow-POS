-- Conciliación por turno de los pagos en efectivo de ventas completadas
-- frente a los movimientos sale_in. Ventas con cash_session_id NULL quedan
-- fuera de la atribución histórica y deben revisarse por separado.
WITH cash_payments AS (
    SELECT s.cash_session_id AS session_id,
           COALESCE(SUM(sp.amount), 0)::BIGINT AS payment_cash
    FROM sales s
    JOIN sale_payments sp ON sp.sale_id = s.id AND sp.method = 'cash'
    WHERE s.status = 'completed' AND s.cash_session_id IS NOT NULL
    GROUP BY s.cash_session_id
),
sale_in AS (
    SELECT session_id,
           COALESCE(SUM(amount), 0)::BIGINT AS movement_cash
    FROM cash_movements
    WHERE movement_type = 'sale_in'
    GROUP BY session_id
)
SELECT cs.id AS cash_session_id,
       COALESCE(cp.payment_cash, 0)::BIGINT AS cash_payments,
       COALESCE(si.movement_cash, 0)::BIGINT AS sale_in,
       (COALESCE(cp.payment_cash, 0) - COALESCE(si.movement_cash, 0))::BIGINT AS difference
FROM cash_sessions cs
LEFT JOIN cash_payments cp ON cp.session_id = cs.id
LEFT JOIN sale_in si ON si.session_id = cs.id
ORDER BY cs.id;
-- Conciliación de caja por venta (solo lectura).
-- Completada: neto en caja (sale_in - sale_out) = efectivo de sus pagos. Anulada o devuelta: neto = 0.
WITH pagos AS (
    SELECT sale_id, SUM(amount)::BIGINT AS efectivo
    FROM sale_payments WHERE method = 'cash' GROUP BY sale_id
), caja AS (
    SELECT sale_id,
           SUM(CASE movement_type WHEN 'sale_in' THEN amount
                                  WHEN 'sale_out' THEN -amount ELSE 0 END)::BIGINT AS neto
    FROM cash_movements
    WHERE sale_id IS NOT NULL AND movement_type IN ('sale_in', 'sale_out')
    GROUP BY sale_id
)
SELECT s.id AS venta, s.status, s.cash_session_id AS turno_venta,
       COALESCE(p.efectivo, 0) AS efectivo_pagos,
       COALESCE(c.neto, 0) AS neto_en_caja,
       CASE WHEN s.status = 'completed' THEN COALESCE(c.neto, 0) - COALESCE(p.efectivo, 0)
            ELSE COALESCE(c.neto, 0) END AS diferencia,
       CASE WHEN (s.status = 'completed' AND COALESCE(c.neto, 0) = COALESCE(p.efectivo, 0))
              OR (s.status <> 'completed' AND COALESCE(c.neto, 0) = 0)
            THEN 'ok' ELSE 'REVISAR' END AS estado
FROM sales s
LEFT JOIN pagos p ON p.sale_id = s.id
LEFT JOIN caja c ON c.sale_id = s.id
ORDER BY s.id;