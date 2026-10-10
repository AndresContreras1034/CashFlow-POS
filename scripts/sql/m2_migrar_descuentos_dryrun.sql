-- M2 dry-run: no modifica nada. Una fila por venta con la semántica antigua.
SELECT s.id, s.subtotal AS subtotal_viejo, s.discount AS descuento_viejo,
       COALESCE(SUM(si.discount),0)::BIGINT AS desc_lineas,
       s.subtotal + COALESCE(SUM(si.discount),0) AS subtotal_nuevo,
       s.discount + COALESCE(SUM(si.discount),0) AS descuento_nuevo,
       s.total,
       CASE WHEN s.subtotal = COALESCE(SUM(si.subtotal),0)
            THEN 'ok' ELSE 'REVISAR: subtotal viejo != suma neta de ítems' END AS inv_previo,
       CASE WHEN s.subtotal + COALESCE(SUM(si.discount),0)
                 = COALESCE(SUM(si.unit_price * si.quantity),0)
            THEN 'ok' ELSE 'REVISAR: bruto != suma precio*cantidad' END AS inv_bruto,
       CASE WHEN (s.subtotal + COALESCE(SUM(si.discount),0))
                 - (s.discount + COALESCE(SUM(si.discount),0)) = s.total
            THEN 'ok' ELSE 'REVISAR: el total cambiaría' END AS inv_total
FROM sales s LEFT JOIN sale_items si ON si.sale_id = s.id
WHERE NOT s.gross_semantics
GROUP BY s.id, s.subtotal, s.discount, s.total
ORDER BY s.id;