import type { HeatmapCell } from '../../types/reports';
import { formatMoney } from '../../utils/format';
import { fillHeatmap } from '../../utils/reportFormat';

const WEEKDAYS = ['Lun', 'Mar', 'Mié', 'Jue', 'Vie', 'Sáb', 'Dom'];

interface SalesHeatmapProps {
  cells: HeatmapCell[];
  currency: string;
  decimals: number;
}

function intensityLevel(value: number, maximum: number): number {
  if (value <= 0 || maximum <= 0) return 0;
  const scaledValue = BigInt(Math.trunc(value));
  const scaledMaximum = BigInt(Math.trunc(maximum));
  if (scaledValue * 4n >= scaledMaximum * 3n) return 4;
  if (scaledValue * 4n >= scaledMaximum * 2n) return 3;
  if (scaledValue * 4n >= scaledMaximum) return 2;
  return 1;
}

export function SalesHeatmap({ cells, currency, decimals }: SalesHeatmapProps) {
  const matrix = fillHeatmap(cells);
  const maxSales = cells.reduce(
    (maximum, cell) => Math.max(maximum, cell.gross_sales),
    0,
  );

  return (
    <>
      <div className="reports-heatmap-scroll">
        <div className="reports-heatmap" role="grid" aria-label="Ventas por día y hora">
          <div className="reports-heatmap-corner" aria-hidden="true" />
          {Array.from({ length: 24 }, (_, hour) => (
            <span className="reports-heatmap-hour" key={hour}>
              {String(hour).padStart(2, '0')}
            </span>
          ))}
          {matrix.map((row, weekday) => (
            <div className="reports-heatmap-row" role="row" key={WEEKDAYS[weekday]}>
              <span className="reports-heatmap-day" role="rowheader">
                {WEEKDAYS[weekday]}
              </span>
              {row.map(cell => {
                const title = `${WEEKDAYS[weekday]} ${String(cell.hour).padStart(2, '0')}:00 · ${
                  cell.tickets
                } tickets · ${formatMoney(cell.gross_sales, currency, decimals)}`;
                return (
                  <span
                    className={`reports-heat reports-heat--${intensityLevel(
                      cell.gross_sales,
                      maxSales,
                    )}`}
                    role="gridcell"
                    aria-label={title}
                    title={title}
                    key={cell.hour}
                  />
                );
              })}
            </div>
          ))}
        </div>
      </div>
      <p className="reports-heatmap-legend">Intensidad según ventas brutas en centésimas.</p>
    </>
  );
}
