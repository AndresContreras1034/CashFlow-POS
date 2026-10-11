import { useState } from 'react';
import { save } from '@tauri-apps/plugin-dialog';
import { saveReportCsv } from '../../services/reports.service';
import { toCsv } from '../../utils/csv';
import type { ReportCsv } from '../../utils/reportExport';
import './ReportCsvButton.css';

interface ReportCsvButtonProps {
  disabled: boolean;
  buildReport: () => ReportCsv | null;
}

function errorMessage(error: unknown): string {
  return error instanceof Error
    ? error.message
    : typeof error === 'string'
      ? error
      : 'Error desconocido';
}

export function ReportCsvButton({ disabled, buildReport }: ReportCsvButtonProps) {
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const exportCsv = async () => {
    setError(null);
    setSaving(true);
    try {
      const report = buildReport();
      if (!report) return;
      const filePath = await save({
        defaultPath: report.filename,
        filters: [{ name: 'CSV', extensions: ['csv'] }],
      });
      if (!filePath) return;
      await saveReportCsv(filePath, toCsv(report.headers, report.rows));
    } catch (cause) {
      setError(`No se pudo guardar el CSV: ${errorMessage(cause)}`);
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="reports-export">
      <button
        type="button"
        className="btn btn-ghost"
        disabled={disabled || saving}
        onClick={exportCsv}
      >
        {saving ? 'Exportando…' : 'Exportar CSV'}
      </button>
      {error && <p className="reports-export-error" role="alert">{error}</p>}
    </div>
  );
}
