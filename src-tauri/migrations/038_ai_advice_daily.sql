CREATE TABLE IF NOT EXISTS ai_advice_daily (
  id BIGSERIAL PRIMARY KEY,
  local_date DATE NOT NULL,
  scope TEXT NOT NULL CHECK (scope IN ('dashboard','products','inventory','payables')),
  regen_index INT NOT NULL DEFAULT 0,
  input_hash TEXT NOT NULL,
  content JSONB NOT NULL,
  model TEXT NOT NULL,
  prompt_tokens INT NOT NULL DEFAULT 0,
  completion_tokens INT NOT NULL DEFAULT 0,
  status TEXT NOT NULL DEFAULT 'ok' CHECK (status IN ('ok','failed')),
  feedback TEXT CHECK (feedback IN ('useful','not_applicable')),
  generated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE (local_date, scope, regen_index)
);

CREATE TABLE IF NOT EXISTS ai_settings (
  id SMALLINT PRIMARY KEY DEFAULT 1 CHECK (id = 1),
  enabled BOOLEAN NOT NULL DEFAULT FALSE,
  model TEXT NOT NULL DEFAULT 'deepseek-v4-flash',
  daily_token_cap INT NOT NULL DEFAULT 60000 CHECK (daily_token_cap > 0)
);
INSERT INTO ai_settings (id) VALUES (1) ON CONFLICT DO NOTHING;