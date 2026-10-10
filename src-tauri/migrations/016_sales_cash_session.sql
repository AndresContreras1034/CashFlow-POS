-- Sesión de caja en la que se registró la venta. Nullable: las ventas
-- anteriores a esta migración no tienen sesión asociada.
ALTER TABLE sales
    ADD COLUMN cash_session_id INT REFERENCES cash_sessions(id) ON DELETE RESTRICT;

CREATE INDEX idx_sales_cash_session ON sales (cash_session_id)
    WHERE cash_session_id IS NOT NULL;