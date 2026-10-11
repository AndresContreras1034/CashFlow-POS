export type ReportRangePreset =
  | 'today'
  | 'yesterday'
  | 'last_7_days'
  | 'last_30_days'
  | 'this_month'
  | 'previous_month';

export interface ReportDateRange {
  date_from: string;
  date_to: string;
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
  const month = currentDay.getUTCMonth();
  let from: Date;
  let to: Date;

  switch (preset) {
    case 'today':
      from = currentDay;
      to = currentDay;
      break;
    case 'yesterday':
      from = addDays(currentDay, -1);
      to = from;
      break;
    case 'last_7_days':
      from = addDays(currentDay, -6);
      to = currentDay;
      break;
    case 'last_30_days':
      from = addDays(currentDay, -29);
      to = currentDay;
      break;
    case 'this_month':
      from = new Date(Date.UTC(year, month, 1));
      to = new Date(Date.UTC(year, month + 1, 0));
      break;
    case 'previous_month':
      from = new Date(Date.UTC(year, month - 1, 1));
      to = new Date(Date.UTC(year, month, 0));
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
