export type CommercialEventKind = 'payday' | 'bonus' | 'holiday' | 'special';
export type CommercialEventRule = 'once' | 'yearly' | 'monthly' | 'nth_weekday';

export interface CommercialEvent {
  id: number;
  name: string;
  kind: CommercialEventKind;
  rule: CommercialEventRule;
  event_date: string | null;
  month: number | null;
  day: number | null;
  weekday: number | null;
  nth: number | null;
  prep_days: number;
  is_active: boolean;
}

export type CommercialEventInput = Omit<CommercialEvent, 'id'> & {
  actor: string | null;
};

export interface UpcomingCommercialEvent {
  id: number;
  name: string;
  kind: CommercialEventKind;
  date: string;
  in_days: number;
  prep_days: number;
}

export interface SalesPace {
  as_of: string;
  days: number;
  actual_cents: number;
  baseline_cents: number | null;
  vs_baseline_bps: number | null;
  status: 'ok' | 'insufficient';
  year_ago_date: string | null;
}
