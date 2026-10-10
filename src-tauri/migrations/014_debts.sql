-- ============================================================
-- MIGRACIÓN 014: Gestión de deudas (cuentas por pagar)
-- ============================================================
--
-- Decisiones:
--   * Sin FK hacia ventas, caja, inventario, clientes ni proveedores.
--   * Importes en BIGINT, centésimas de la unidad principal (igual que
--     el resto del proyecto).
--   * Saldo, estado y urgencia NO se almacenan: se derivan en la vista
--     `debt_overview`, que es su única definición.
--   * Nada se borra: las deudas y los pagos se anulan (DELETE y
--     TRUNCATE bloqueados). Corregir un pago = anularlo y registrar otro.
--   * Los triggers son un respaldo en la base de datos; el repository
--     valida antes y traduce los errores a mensajes claros.
-- ============================================================

CREATE TYPE debt_category AS ENUM (
    'maintenance',          -- Mantenimiento
    'business_operation',   -- Operación del negocio
    'fixed_services',       -- Servicios y gastos fijos
    'other_obligations'     -- Otras obligaciones
);

CREATE TYPE debt_nature AS ENUM (
    'operating',
    'financial_loan',
    'tax',
    'other'
);

CREATE TYPE debt_payment_method AS ENUM (
    'cash', 'card', 'transfer', 'check', 'other'
);

-- Tipos de salida de la vista debt_overview (no se almacenan).
CREATE TYPE debt_status AS ENUM (
    'voided', 'paid', 'overdue', 'partially_paid', 'pending'
);

CREATE TYPE debt_urgency AS ENUM (
    'none', 'ok', 'alert', 'due_today', 'overdue'
);

-- ------------------------------------------------------------
-- Obligaciones
-- ------------------------------------------------------------
CREATE TABLE debts (
    id               SERIAL PRIMARY KEY,

    creditor_name    VARCHAR(150)  NOT NULL,
    category         debt_category NOT NULL,
    subcategory      VARCHAR(60),
    nature           debt_nature   NOT NULL,
    concept          VARCHAR(200)  NOT NULL,

    original_amount  BIGINT        NOT NULL,
    issued_on        DATE          NOT NULL,
    due_on           DATE,

    document_ref     VARCHAR(100),
    notes            TEXT,

    idempotency_key  UUID          NOT NULL,

    voided_at        TIMESTAMPTZ,
    voided_by        VARCHAR(100),
    void_reason      TEXT,

    created_by       VARCHAR(100)  NOT NULL DEFAULT 'system',
    created_at       TIMESTAMPTZ   NOT NULL DEFAULT NOW(),
    updated_at       TIMESTAMPTZ   NOT NULL DEFAULT NOW(),

    CONSTRAINT uq_debts_idempotency_key  UNIQUE (idempotency_key),
    CONSTRAINT ck_debts_amount_positive  CHECK (original_amount > 0),
    CONSTRAINT ck_debts_creditor_not_blank
        CHECK (char_length(btrim(creditor_name)) > 0),
    CONSTRAINT ck_debts_concept_not_blank
        CHECK (char_length(btrim(concept)) > 0),
    CONSTRAINT ck_debts_subcategory_not_blank
        CHECK (subcategory IS NULL OR char_length(btrim(subcategory)) > 0),
    CONSTRAINT ck_debts_due_after_issue
        CHECK (due_on IS NULL OR due_on >= issued_on),
    CONSTRAINT ck_debts_void_consistent
        CHECK (
            (voided_at IS NULL AND voided_by IS NULL AND void_reason IS NULL)
            OR
            (voided_at IS NOT NULL
             AND void_reason IS NOT NULL
             AND char_length(btrim(void_reason)) > 0)
        )
);

CREATE INDEX idx_debts_category   ON debts (category);
CREATE INDEX idx_debts_due_on     ON debts (due_on) WHERE voided_at IS NULL;
CREATE INDEX idx_debts_issued_on  ON debts (issued_on);
CREATE INDEX idx_debts_created_at ON debts (created_at DESC);

CREATE TRIGGER trg_debts_updated_at
    BEFORE UPDATE ON debts
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

-- ------------------------------------------------------------
-- Pagos / abonos
-- ------------------------------------------------------------
CREATE TABLE debt_payments (
    id               SERIAL PRIMARY KEY,
    debt_id          INT           NOT NULL REFERENCES debts(id) ON DELETE RESTRICT,

    amount           BIGINT        NOT NULL,
    paid_on          DATE          NOT NULL,
    method           debt_payment_method NOT NULL,
    reference        VARCHAR(100),
    notes            TEXT,

    idempotency_key  UUID          NOT NULL,

    voided_at        TIMESTAMPTZ,
    voided_by        VARCHAR(100),
    void_reason      TEXT,

    created_by       VARCHAR(100)  NOT NULL DEFAULT 'system',
    created_at       TIMESTAMPTZ   NOT NULL DEFAULT NOW(),

    CONSTRAINT uq_debt_payments_idempotency_key UNIQUE (idempotency_key),
    CONSTRAINT ck_debt_payments_amount_positive CHECK (amount > 0),
    CONSTRAINT ck_debt_payments_void_consistent
        CHECK (
            (voided_at IS NULL AND voided_by IS NULL AND void_reason IS NULL)
            OR
            (voided_at IS NOT NULL
             AND void_reason IS NOT NULL
             AND char_length(btrim(void_reason)) > 0)
        )
);

CREATE INDEX idx_debt_payments_debt  ON debt_payments (debt_id);
CREATE INDEX idx_debt_payments_paid  ON debt_payments (paid_on) WHERE voided_at IS NULL;

-- ------------------------------------------------------------
-- Nada se borra
-- ------------------------------------------------------------
CREATE OR REPLACE FUNCTION prevent_debt_deletion()
RETURNS TRIGGER AS $$
BEGIN
    RAISE EXCEPTION '% es de solo anulación: % no está permitido', TG_TABLE_NAME, TG_OP
        USING ERRCODE = 'restrict_violation';
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_debts_no_delete
    BEFORE DELETE ON debts
    FOR EACH ROW EXECUTE FUNCTION prevent_debt_deletion();
CREATE TRIGGER trg_debts_no_truncate
    BEFORE TRUNCATE ON debts
    FOR EACH STATEMENT EXECUTE FUNCTION prevent_debt_deletion();
CREATE TRIGGER trg_debt_payments_no_delete
    BEFORE DELETE ON debt_payments
    FOR EACH ROW EXECUTE FUNCTION prevent_debt_deletion();
CREATE TRIGGER trg_debt_payments_no_truncate
    BEFORE TRUNCATE ON debt_payments
    FOR EACH STATEMENT EXECUTE FUNCTION prevent_debt_deletion();

-- ------------------------------------------------------------
-- Pagos: reglas al insertar
--   Bloquea la fila de la deuda, así dos pagos simultáneos se
--   serializan aunque el llamador no haya tomado el bloqueo.
-- ------------------------------------------------------------
CREATE OR REPLACE FUNCTION enforce_debt_payment_insert()
RETURNS TRIGGER AS $$
DECLARE
    d    debts%ROWTYPE;
    paid BIGINT;
BEGIN
    SELECT * INTO d FROM debts WHERE id = NEW.debt_id FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'La deuda % no existe', NEW.debt_id
            USING ERRCODE = 'foreign_key_violation';
    END IF;

    IF NEW.voided_at IS NOT NULL THEN
        RAISE EXCEPTION 'Un pago no puede crearse ya anulado'
            USING ERRCODE = 'restrict_violation';
    END IF;

    IF d.voided_at IS NOT NULL THEN
        RAISE EXCEPTION 'La deuda % está anulada y no admite pagos', d.id
            USING ERRCODE = 'restrict_violation';
    END IF;

    IF NEW.paid_on < d.issued_on THEN
        RAISE EXCEPTION 'La fecha de pago no puede ser anterior a la emisión de la deuda'
            USING ERRCODE = 'restrict_violation';
    END IF;

    SELECT COALESCE(SUM(amount), 0) INTO paid
    FROM debt_payments
    WHERE debt_id = NEW.debt_id AND voided_at IS NULL;

    IF paid + NEW.amount > d.original_amount THEN
        RAISE EXCEPTION 'El pago supera el saldo pendiente de la deuda %', d.id
            USING ERRCODE = 'restrict_violation';
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_debt_payments_before_insert
    BEFORE INSERT ON debt_payments
    FOR EACH ROW EXECUTE FUNCTION enforce_debt_payment_insert();

-- ------------------------------------------------------------
-- Pagos: lo único que se puede hacer es anular uno vigente, una vez
-- ------------------------------------------------------------
CREATE OR REPLACE FUNCTION enforce_debt_payment_update()
RETURNS TRIGGER AS $$
BEGIN
    IF OLD.voided_at IS NOT NULL THEN
        RAISE EXCEPTION 'Un pago anulado no se puede modificar'
            USING ERRCODE = 'restrict_violation';
    END IF;

    IF NEW.voided_at IS NULL THEN
        RAISE EXCEPTION 'Un pago solo puede anularse; para corregirlo, anúlalo y regístralo de nuevo'
            USING ERRCODE = 'restrict_violation';
    END IF;

    IF ROW(NEW.id, NEW.debt_id, NEW.amount, NEW.paid_on, NEW.method,
           NEW.reference, NEW.notes, NEW.idempotency_key,
           NEW.created_by, NEW.created_at)
       IS DISTINCT FROM
       ROW(OLD.id, OLD.debt_id, OLD.amount, OLD.paid_on, OLD.method,
           OLD.reference, OLD.notes, OLD.idempotency_key,
           OLD.created_by, OLD.created_at)
    THEN
        RAISE EXCEPTION 'Los datos de un pago son inmutables'
            USING ERRCODE = 'restrict_violation';
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_debt_payments_before_update
    BEFORE UPDATE ON debt_payments
    FOR EACH ROW EXECUTE FUNCTION enforce_debt_payment_update();

-- ------------------------------------------------------------
-- Deudas: reglas al actualizar
-- ------------------------------------------------------------
CREATE OR REPLACE FUNCTION enforce_debt_update()
RETURNS TRIGGER AS $$
DECLARE
    live_count BIGINT;
    first_paid DATE;
BEGIN
    IF OLD.voided_at IS NOT NULL THEN
        RAISE EXCEPTION 'Una deuda anulada no se puede modificar'
            USING ERRCODE = 'restrict_violation';
    END IF;

    IF NEW.id <> OLD.id
       OR NEW.idempotency_key <> OLD.idempotency_key
       OR NEW.created_by <> OLD.created_by
       OR NEW.created_at <> OLD.created_at THEN
        RAISE EXCEPTION 'Los datos de origen de una deuda son inmutables'
            USING ERRCODE = 'restrict_violation';
    END IF;

    SELECT COUNT(*), MIN(paid_on) INTO live_count, first_paid
    FROM debt_payments
    WHERE debt_id = OLD.id AND voided_at IS NULL;

    IF live_count > 0 THEN
        IF NEW.voided_at IS NOT NULL THEN
            RAISE EXCEPTION 'No se puede anular una deuda con pagos vigentes; reversa los pagos primero'
                USING ERRCODE = 'restrict_violation';
        END IF;
        IF NEW.original_amount <> OLD.original_amount THEN
            RAISE EXCEPTION 'No se puede cambiar el importe de una deuda con pagos vigentes'
                USING ERRCODE = 'restrict_violation';
        END IF;
        IF NEW.issued_on > first_paid THEN
            RAISE EXCEPTION 'La emisión no puede ser posterior al primer pago vigente'
                USING ERRCODE = 'restrict_violation';
        END IF;
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_debts_before_update
    BEFORE UPDATE ON debts
    FOR EACH ROW EXECUTE FUNCTION enforce_debt_update();

-- ------------------------------------------------------------
-- debt_today(): «hoy» según la zona de Ajustes
--   Misma regla que el filtro de Auditoría: app_settings.timezone,
--   con la zona de la sesión como respaldo. La usan la vista y el
--   repository, para que nunca haya dos definiciones de «hoy».
-- ------------------------------------------------------------
CREATE OR REPLACE FUNCTION debt_today()
RETURNS DATE
LANGUAGE sql STABLE AS $$
    SELECT (NOW() AT TIME ZONE COALESCE(
               (SELECT timezone FROM app_settings WHERE id = 1),
               current_setting('TimeZone')
           ))::DATE
$$;

-- ------------------------------------------------------------
-- debt_overview: ÚNICA definición de saldo, estado y urgencia
--
--   * Estado: anulada > pagada > vencida > pago parcial > pendiente.
--   * Urgencia (independiente del estado): más de 7 días = ok;
--     de 1 a 7 = alert; 0 = due_today; negativo = overdue.
--     El umbral de 7 días vive SOLO aquí.
-- ------------------------------------------------------------
CREATE VIEW debt_overview AS
WITH base AS (
    SELECT
        d.id, d.creditor_name, d.category, d.subcategory, d.nature,
        d.concept, d.original_amount, d.issued_on, d.due_on,
        d.document_ref, d.notes,
        d.voided_at, d.voided_by, d.void_reason,
        d.created_by, d.created_at, d.updated_at,
        COALESCE(p.paid_amount, 0)::BIGINT                       AS paid_amount,
        (d.original_amount - COALESCE(p.paid_amount, 0))::BIGINT AS balance,
        COALESCE(p.payment_count, 0)::BIGINT                     AS payment_count,
        p.last_paid_on,
        debt_today()                                             AS today,
        (d.due_on - debt_today())::INT                           AS days_until_due
    FROM debts d
    LEFT JOIN LATERAL (
        SELECT SUM(amount)  AS paid_amount,
               COUNT(*)     AS payment_count,
               MAX(paid_on) AS last_paid_on
        FROM debt_payments
        WHERE debt_id = d.id AND voided_at IS NULL
    ) p ON TRUE
)
SELECT
    b.*,
    (CASE
        WHEN b.voided_at IS NOT NULL                     THEN 'voided'
        WHEN b.balance <= 0                              THEN 'paid'
        WHEN b.due_on IS NOT NULL AND b.due_on < b.today THEN 'overdue'
        WHEN b.payment_count > 0                         THEN 'partially_paid'
        ELSE 'pending'
    END)::debt_status AS status,
    (CASE
        WHEN b.voided_at IS NOT NULL OR b.balance <= 0 OR b.due_on IS NULL THEN 'none'
        WHEN b.days_until_due < 0                        THEN 'overdue'
        WHEN b.days_until_due = 0                        THEN 'due_today'
        WHEN b.days_until_due <= 7                       THEN 'alert'
        ELSE 'ok'
    END)::debt_urgency AS urgency
FROM base b;