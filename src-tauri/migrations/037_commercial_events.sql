CREATE TABLE commercial_events (
    id          BIGSERIAL PRIMARY KEY,
    name        TEXT NOT NULL CHECK (char_length(btrim(name)) BETWEEN 1 AND 100),
    kind        TEXT NOT NULL CHECK (kind IN ('payday', 'bonus', 'holiday', 'special')),
    rule        TEXT NOT NULL CHECK (rule IN ('once', 'yearly', 'monthly', 'nth_weekday')),
    event_date  DATE,
    month       SMALLINT,
    day         SMALLINT,
    weekday     SMALLINT,
    nth         SMALLINT,
    prep_days   SMALLINT NOT NULL DEFAULT 7 CHECK (prep_days BETWEEN 0 AND 60),
    is_active   BOOLEAN NOT NULL DEFAULT TRUE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT commercial_events_rule_fields_chk CHECK (
        (rule = 'once'
            AND event_date IS NOT NULL
            AND month IS NULL AND day IS NULL AND weekday IS NULL AND nth IS NULL)
        OR
        (rule = 'yearly'
            AND event_date IS NULL
            AND month IS NOT NULL
            AND month BETWEEN 1 AND 12
            AND day IS NOT NULL
            AND (day BETWEEN 1 AND 31 OR day = -1)
            AND weekday IS NULL AND nth IS NULL)
        OR
        (rule = 'monthly'
            AND event_date IS NULL AND month IS NULL
            AND day IS NOT NULL
            AND (day BETWEEN 1 AND 31 OR day = -1)
            AND weekday IS NULL AND nth IS NULL)
        OR
        (rule = 'nth_weekday'
            AND event_date IS NULL
            AND month IS NOT NULL
            AND month BETWEEN 1 AND 12
            AND day IS NULL
            AND weekday IS NOT NULL
            AND weekday BETWEEN 0 AND 6
            AND nth IS NOT NULL
            AND (nth BETWEEN 1 AND 4 OR nth = -1))
    )
);

CREATE INDEX idx_commercial_events_active ON commercial_events (is_active, id);

INSERT INTO commercial_events (name, kind, rule, day, prep_days)
SELECT 'Quincena - día 15', 'payday', 'monthly', 15, 7
WHERE NOT EXISTS (
    SELECT 1 FROM commercial_events
    WHERE name = 'Quincena - día 15' AND rule = 'monthly' AND day = 15
);

INSERT INTO commercial_events (name, kind, rule, day, prep_days)
SELECT 'Quincena - fin de mes', 'payday', 'monthly', -1, 7
WHERE NOT EXISTS (
    SELECT 1 FROM commercial_events
    WHERE name = 'Quincena - fin de mes' AND rule = 'monthly' AND day = -1
);
