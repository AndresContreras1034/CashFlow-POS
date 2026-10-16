export type ReportRangePreset = 'last_7_days' | 'last_30_days' | 'last_90_days'
  | 'last_12_months' | 'year_to_date' | 'previous_year';

export const MAX_REPORT_RANGE_DAYS = 732;

export interface ReportDateRange {
  date_from: string;
  date_to: string;
}

export function todayInTimezone(timeZone: string): string | null {
  try {
    const parts = new Intl.DateTimeFormat('en-CA', {
      timeZone,
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
    }).formatToParts(new Date());
    const part = (type: string) => parts.find(value => value.type === type)?.value ?? '';
    return `${part('year')}-${part('month')}-${part('day')}`;
  } catch {
    return null;
  }
}

export function validateReportRange(from: string, to: string): string | null {
  if (!from || !to) return 'Selecciona las dos fechas del período.';

  const isValidDate = (value: string): boolean => {
    if (!/^\d{4}-\d{2}-\d{2}$/.test(value)) return false;
    const parsed = new Date(`${value}T00:00:00Z`);
    return Number.isFinite(parsed.getTime()) && parsed.toISOString().slice(0, 10) === value;
  };
  if (!isValidDate(from) || !isValidDate(to)) return 'Ingresa fechas válidas.';
  if (from > to) return 'La fecha Desde no puede ser posterior a Hasta.';

  const inclusiveDays = Math.round(
    (parseDate(to).getTime() - parseDate(from).getTime()) / 86_400_000,
  ) + 1;
  if (inclusiveDays > MAX_REPORT_RANGE_DAYS) return 'El rango no puede superar 732 días.';
  return null;
}

const parseDate = (value: string): Date => {
  const [year, month, day] = value.split('-').map(Number);
  return new Date(Date.UTC(year, month - 1, day));
};

const formatDate = (value: Date): string =>
  `${value.getUTCFullYear()}-${String(value.getUTCMonth() + 1).padStart(2, '0')}-${String(
    value.getUTCDate(),
  ).padStart(2, '0')}`;

const addDays = (value: Date, days: number): Date => {
  const result = new Date(value);
  result.setUTCDate(result.getUTCDate() + days);
  return result;
};

export function getReportRange(
  preset: ReportRangePreset,
  today: string,
): ReportDateRange {
  const currentDay = parseDate(today);
  const year = currentDay.getUTCFullYear();
  const addDays = (date: Date, days: number) => {
    const result = new Date(date);
    result.setUTCDate(result.getUTCDate() + days);
    return result;
  };
  let from: Date = currentDay;
  let to: Date = currentDay;
  switch (preset) {
    case 'last_7_days':
      from = addDays(currentDay, -6);
      break;
    case 'last_30_days':
      from = addDays(currentDay, -29);
      break;
    case 'last_90_days':
      from = addDays(currentDay, -89);
      break;
    case 'last_12_months':
      from = new Date(Date.UTC(year - 1, currentDay.getUTCMonth(), currentDay.getUTCDate()));
      if (from.getUTCMonth() !== currentDay.getUTCMonth()) {
        from = new Date(Date.UTC(year - 1, currentDay.getUTCMonth() + 1, 0));
      }
      from = addDays(from, 1);
      break;
    case 'year_to_date':
      from = new Date(Date.UTC(year, 0, 1));
      break;
    case 'previous_year':
      from = new Date(Date.UTC(year - 1, 0, 1));
      to = new Date(Date.UTC(year, 0, 0));
      break;
  }

  return { date_from: formatDate(from), date_to: formatDate(to) };
}

export function getPreviousReportRange(
  range: ReportDateRange,
): ReportDateRange {
  const from = parseDate(range.date_from);
  const to = parseDate(range.date_to);
  const dayCount = Math.round((to.getTime() - from.getTime()) / 86_400_000) + 1;
  const previousTo = addDays(from, -1);
  const previousFrom = addDays(previousTo, -(dayCount - 1));

  return {
    date_from: formatDate(previousFrom),
    date_to: formatDate(previousTo),
  };
}

export function getPreviousYearReportRange(
  range: ReportDateRange,
): ReportDateRange | null {
  const minusOneYear = (value: string): string | null => {
    const current = parseDate(value);
    const year = current.getUTCFullYear() - 1;
    const month = current.getUTCMonth();
    const day = current.getUTCDate();
    const shifted = new Date(Date.UTC(year, month, day));
    if (shifted.getUTCMonth() !== month) {
      shifted.setUTCDate(0);
    }
    const result = formatDate(shifted);
    return validateReportRange(result, result) === null ? result : null;
  };
  const from = minusOneYear(range.date_from);
  const to = minusOneYear(range.date_to);
  if (!from || !to) return null;
  const shifted = { date_from: from, date_to: to };
  return validateReportRange(from, to) === null ? shifted : null;
}

export function getComparisonReportRange(
  range: ReportDateRange,
  yearOverYear: boolean,
): ReportDateRange | null {
  const comparison = yearOverYear ? getPreviousYearReportRange(range) : getPreviousReportRange(range);
  if (!comparison) return null;
  return validateReportRange(comparison.date_from, comparison.date_to) === null ? comparison : null;
}
