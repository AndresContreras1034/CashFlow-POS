export type BlockStatus = 'ok' | 'insufficient';

export interface CommercialContext {
  fingerprint?: string;
  as_of: string;
  pace: { status: BlockStatus; days: number; vs_baseline_bps: number | null };
  calendar: {
    name: string;
    kind: string;
    in_days: number;
    last_year: { status: BlockStatus };
  }[];
  inventory: {
    coverage: { alias: string; days_cover: number | null; risk: boolean }[];
    stale: { total_value_cents: number; rows: number };
  };
  promo_candidates: {
    alias: string;
    scenarios: {
      kind: string;
      profit: number;
      allowed: boolean;
      breakeven_x100: number | null;
    }[];
  }[];
  data_sufficiency: { history_days: number; uncosted_pct_bps: number };
}

export interface AiAdvice {
  insufficient_data: boolean;
  advice: {
    observation: string;
    context: string;
    proposal: string;
    inventory_action: string | null;
    promo: {
      alias: string;
      kind: string;
      second_off_pct: number | null;
      direct_off_pct: number | null;
    } | null;
    cited: { path: string; value: unknown }[];
  }[];
}

export interface StoredAdvice {
  id: number;
  local_date: string;
  scope: string;
  regen_index: number;
  content: AiAdvice;
  model: string;
  generated_at: string;
}

export interface AiSettings {
  enabled: boolean;
  model: string;
  daily_token_cap: number;
  has_key: boolean;
  tokens_today: number;
}