import { useState } from 'react';
import { save } from '@tauri-apps/plugin-dialog';
import { saveReportCsv } from '../../services/reports.service';
import type { CsvTable } from '../../utils/supplierExport';
import { toCsv } from '../../utils/csv';
import { errorMessage } from '../../utils/audit';

export interface SupplierCsvReport {
  table: CsvTable;
  notice?: string;
}

interface SupplierCsvButtonProps {
  disabled: boolean;
  filename: string;
  buildReport: () => SupplierCsvReport | Promise<SupplierCsvReport>;
  onSuccess: (notice?: string) => void;
  onError: (message: string) => void;
}

export const SupplierCsvButton: React.FC<SupplierCsvButtonProps> = ({
  disabled,
  filename,
  buildReport,
  onSuccess,
  onError,
}) => {
  const [saving, setSaving] = useState(false);

  const exportCsv = async () => {
    setSaving(true);
    try {
      const report = await buildReport();
      const filePath = await save({
        defaultPath: filename,
        filters: [{ name: 'CSV', extensions: ['csv'] }],
      });
      if (!filePath) return;
      await saveReportCsv(filePath, toCsv(report.table.headers, report.table.rows));
      onSuccess(report.notice);
    } catch (cause) {
      onError(`No se pudo guardar el CSV: ${errorMessage(cause)}`);
    } finally {
      setSaving(false);
    }
  };

  return (
    <button
      type="button"
      className="suppliers-btn"
      disabled={disabled || saving}
      onClick={() => void exportCsv()}
    >
      {saving ? 'Exportando…' : 'Exportar CSV'}
    </button>
  );
};
