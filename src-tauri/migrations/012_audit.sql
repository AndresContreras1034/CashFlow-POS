-- ============================================================
-- MIGRACIÓN 012: Auditoría (eventos append-only)
-- ============================================================
--
-- Una sola tabla para auditoría de negocio y errores registrados,
-- diferenciados por `category`. Los logs técnicos (debug, trazas)
-- siguen en `tracing` y NO se guardan aquí.
--
-- Decisiones:
--   * Sin FK hacia ventas, productos, caja, etc.: el evento debe
--     sobrevivir aunque la entidad cambie o se archive.
--   * `actor` es NULL mientras no exista login/cajeros. Sin FK.
--   * `entity_id` es TEXT para no atarlo a INT.
--   * `correlation_id` lo genera Rust (Uuid::new_v4()); agrupa los
--     eventos de una misma operación ("Ver operación completa").
--   * La tabla es append-only: INSERT sí; UPDATE, DELETE y TRUNCATE
--     se bloquean con triggers.
-- ============================================================

CREATE TYPE audit_category AS ENUM (
    'business',     -- acción de negocio (venta, cambio de precio, cierre de caja)
    'error'         -- error registrado (falló algo que el usuario intentó)
);

CREATE TYPE audit_module AS ENUM (
    'sales',
    'inventory',
    'cash',
    'stocktake',
    'settings',
    'import',
    'licensing',
    'billing',      -- impresión de tickets y etiquetas
    'system'        -- errores que no pertenecen a un módulo concreto
);

CREATE TYPE audit_outcome AS ENUM (
    'success',      -- la acción ocurrió
    'failure'       -- se intentó, pero no ocurrió
);

CREATE TABLE IF NOT EXISTS audit_events (
    id              BIGSERIAL PRIMARY KEY,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    correlation_id  UUID,

    category        audit_category NOT NULL,
    module          audit_module   NOT NULL,
    action          TEXT           NOT NULL CHECK (char_length(btrim(action)) > 0),
    outcome         audit_outcome  NOT NULL,

    actor           TEXT,

    entity_type     TEXT,
    entity_id       TEXT,

    summary         TEXT NOT NULL CHECK (char_length(btrim(summary)) > 0),

    -- {"campo": {"from": ..., "to": ...}}
    changes         JSONB,
    -- Detalle libre: resumen de importación, contexto de un error, etc.
    metadata        JSONB,
    error_message   TEXT,

    -- Un evento de categoría 'error' siempre es un fallo. Así la vista
    -- "Errores" se resuelve con outcome = 'failure'.
    CHECK (category <> 'error' OR outcome = 'failure'),

    -- Si hay entity_id debe haber entity_type.
    CHECK (entity_id IS NULL OR entity_type IS NOT NULL)
);

CREATE INDEX idx_audit_events_created   ON audit_events (created_at DESC, id DESC);
CREATE INDEX idx_audit_events_module    ON audit_events (module, created_at DESC);
CREATE INDEX idx_audit_events_corr      ON audit_events (correlation_id) WHERE correlation_id IS NOT NULL;
CREATE INDEX idx_audit_events_entity    ON audit_events (entity_type, entity_id) WHERE entity_type IS NOT NULL;
CREATE INDEX idx_audit_events_failures  ON audit_events (created_at DESC) WHERE outcome = 'failure';

-- ------------------------------------------------------------
-- Inmutabilidad (append-only)
-- ------------------------------------------------------------
CREATE OR REPLACE FUNCTION prevent_audit_events_modification()
RETURNS TRIGGER AS $$
BEGIN
    RAISE EXCEPTION 'audit_events es append-only: % no está permitido', TG_OP
        USING ERRCODE = 'restrict_violation';
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_audit_events_no_update_delete
    BEFORE UPDATE OR DELETE ON audit_events
    FOR EACH ROW EXECUTE FUNCTION prevent_audit_events_modification();

CREATE TRIGGER trg_audit_events_no_truncate
    BEFORE TRUNCATE ON audit_events
    FOR EACH STATEMENT EXECUTE FUNCTION prevent_audit_events_modification();
