import React, { useState } from 'react';
import { open, save } from '@tauri-apps/plugin-dialog';
import { Modal } from '../ui/Modal';
import { ImportSummaryDto } from '../../types';
import {
  previewImportInventory,
  executeImportInventory,
  exportInventoryTemplate,
} from '../../services/inventory.service';

interface Props {
  onClose: () => void;
  onImported: () => void;
  addToast: (message: string, type: 'success' | 'error' | 'info') => void;
}

const importStatusLabel: Record<string, string> = {
  new_product: 'Producto nuevo',
  existing_product: 'Producto existente',
  skipped_duplicate_sku: 'SKU duplicado',
  error: 'Error',
};

const errorMessage = (error: unknown): string =>
  typeof error === 'string'
    ? error
    : error instanceof Error
      ? error.message
      : 'Error desconocido';

export const ImportInventoryModal: React.FC<Props> = ({ onClose, onImported, addToast }) => {
  const [filePath, setFilePath] = useState<string | null>(null);
  const [summary, setSummary] = useState<ImportSummaryDto | null>(null);
  const [loading, setLoading] = useState(false);
  const [executing, setExecuting] = useState(false);
  const [savingTemplate, setSavingTemplate] = useState(false);

  const handlePickFile = async () => {
    setLoading(true);
    try {
      const selected = await open({
        multiple: false,
        filters: [{ name: 'Excel', extensions: ['xlsx'] }],
      });
      if (!selected || Array.isArray(selected)) return;

      setFilePath(selected);
      setSummary(null);
      const res = await previewImportInventory(selected);
      setSummary(res);
    } catch (error) {
      addToast(`No se pudo leer el archivo: ${errorMessage(error)}`, 'error');
    } finally {
      setLoading(false);
    }
  };

  const handleTemplate = async () => {
    try {
      const path = await save({
        defaultPath: 'plantilla_inventario.xlsx',
        filters: [{ name: 'Excel', extensions: ['xlsx'] }],
      });
      if (!path) return;

      setSavingTemplate(true);
      await exportInventoryTemplate(path);
      addToast('Plantilla guardada', 'success');
    } catch (error) {
      addToast(`No se pudo guardar la plantilla: ${errorMessage(error)}`, 'error');
    } finally {
      setSavingTemplate(false);
    }
  };

  const handleConfirm = async () => {
    if (!filePath) return;
    setExecuting(true);
    try {
      const res = await executeImportInventory(filePath);
      if (!res.can_execute) {
        setSummary(res);
        addToast('No se importó nada: el archivo tiene errores', 'error');
        return;
      }

      const skipped = res.skipped > 0 ? `, ${res.skipped} omitidas` : '';
      addToast(
        `Importación completa: ${res.new_products} productos nuevos, ${res.new_variants} variantes${skipped}`,
        'success'
      );
      onImported();
      onClose();
    } catch (error) {
      addToast(`Error al importar: ${errorMessage(error)}`, 'error');
    } finally {
      setExecuting(false);
    }
  };

  return (
    <Modal title="Importar inventario desde Excel" onClose={onClose}>
      <div className="import-modal">
        <div className="import-actions">
          <button className="btn btn-ghost" onClick={handleTemplate} disabled={savingTemplate}>
            {savingTemplate ? 'Guardando plantilla...' : 'Descargar plantilla'}
          </button>
          {!filePath && (
            <button className="btn btn-primary" onClick={handlePickFile} disabled={loading}>
              Seleccionar archivo .xlsx
            </button>
          )}
        </div>

        {filePath && (
          <div className="import-file-row">
            <p className="import-filepath">{filePath}</p>
            <button className="btn btn-ghost" onClick={handlePickFile} disabled={loading}>
              Cambiar archivo
            </button>
          </div>
        )}

        {loading && <p className="import-loading">Analizando archivo...</p>}

        {summary && (
          <>
            <div className="import-summary-grid">
              <div><span>Categorías nuevas</span><strong>{summary.new_categories}</strong></div>
              <div><span>Productos nuevos</span><strong>{summary.new_products}</strong></div>
              <div><span>Variantes nuevas</span><strong>{summary.new_variants}</strong></div>
              <div><span>Omitidas (SKU duplicado)</span><strong>{summary.skipped}</strong></div>
              <div className={summary.errors > 0 ? 'import-summary-error' : ''}>
                <span>Errores</span><strong>{summary.errors}</strong>
              </div>
            </div>

            <div className="import-rows-table-wrap">
              <table className="import-rows-table">
                <thead>
                  <tr>
                    <th>Fila</th>
                    <th>Producto</th>
                    <th>SKU</th>
                    <th>Estado</th>
                    <th>Mensaje</th>
                  </tr>
                </thead>
                <tbody>
                  {summary.rows.map((row) => (
                    <tr key={row.row_number} className={`import-row import-row--${row.status}`}>
                      <td>{row.row_number}</td>
                      <td>{row.product_name}</td>
                      <td>{row.sku ?? '—'}</td>
                      <td>{importStatusLabel[row.status] ?? row.status}</td>
                      <td>{row.message ?? '—'}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>

            <div className="import-actions">
              <button className="btn btn-ghost" onClick={onClose}>
                Cancelar
              </button>
              <button
                className="btn btn-primary"
                disabled={!summary.can_execute || executing}
                onClick={handleConfirm}
              >
                {executing ? 'Importando...' : `Confirmar importación (${summary.new_variants} variantes)`}
              </button>
            </div>

            {!summary.can_execute && (
              <p className="import-blocked-note">
                Corrige los errores en el archivo antes de poder confirmar la importación.
              </p>
            )}
          </>
        )}
      </div>
    </Modal>
  );
};