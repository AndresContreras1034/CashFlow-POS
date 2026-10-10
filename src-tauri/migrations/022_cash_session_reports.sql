-- Los turnos cerrados antes de esta migración no tienen un informe Z guardado.
CREATE TABLE cash_session_reports (
    session_id INT PRIMARY KEY REFERENCES cash_sessions(id) ON DELETE RESTRICT,
    report     JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
