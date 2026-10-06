-- ============================================================
-- MIGRACIÓN 005: Caja (sesiones de turno + movimientos)
-- ============================================================

CREATE TYPE cash_movement_type AS ENUM ('manual_in', 'manual_out', 'sale_in', 'sale_out');

CREATE TABLE cash_sessions (
    id               SERIAL PRIMARY KEY,
    opening_amount   BIGINT NOT NULL CHECK (opening_amount >= 0),
    expected_amount  BIGINT,               -- se calcula al cerrar
    counted_amount   BIGINT,               -- arqueo físico al cerrar
    difference       BIGINT,               -- counted - expected
    status           VARCHAR(10) NOT NULL DEFAULT 'open'
                     CHECK (status IN ('open', 'closed')),
    opened_by        VARCHAR(100) NOT NULL DEFAULT 'system',
    closed_by        VARCHAR(100),
    opening_notes    TEXT,
    closing_notes    TEXT,
    opened_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    closed_at        TIMESTAMPTZ
);

-- Solo una sesión abierta a la vez (constraint real, no solo validación en Rust)
CREATE UNIQUE INDEX uq_one_open_session
    ON cash_sessions (status)
    WHERE (status = 'open');

CREATE TABLE cash_movements (
    id             SERIAL PRIMARY KEY,
    session_id     INT NOT NULL REFERENCES cash_sessions(id) ON DELETE RESTRICT,
    movement_type  cash_movement_type NOT NULL,
    amount         BIGINT NOT NULL CHECK (amount > 0),
    sale_id        INT,          -- FK futura cuando exista modules::sales
    notes          TEXT,
    created_by     VARCHAR(100) NOT NULL DEFAULT 'system',
    created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_cash_movements_session ON cash_movements (session_id);
CREATE INDEX idx_cash_sessions_status   ON cash_sessions (status);