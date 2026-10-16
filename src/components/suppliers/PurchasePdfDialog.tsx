import { useState } from 'react';
import { save } from '@tauri-apps/plugin-dialog';
import { jsPDF } from 'jspdf';
import { errorMessage } from '../../utils/audit';
import type { PrintModel } from '../../utils/purchasePrint';
import { savePurchasePdf } from '../../services/purchases.service';
import { Modal } from '../ui/Modal';
import './PurchasePdfDialog.css';

interface PurchasePdfDialogProps {
  model: PrintModel;
  documentTitle: string;
  onClose: () => void;
  onSaved: () => void;
}

function draftText(model: PrintModel): string {
  const groups = model.groups.map(group => {
    const lines = group.lines.map(line => `- ${line.product} × ${line.quantity}`);
    return [`Proveedor: ${group.name}`, ...lines].join('\n');
  });

  return [
    model.title,
    `Fecha: ${model.day}`,
    model.businessName ? `De: ${model.businessName}` : '',
    model.notes?.trim() ? `Nota: ${model.notes.trim()}` : '',
    ...groups,
  ]
    .filter(Boolean)
    .join('\n\n');
}

function createPdf(text: string): number[] {
  const pdf = new jsPDF({ unit: 'mm', format: 'a4' });
  const margin = 17;
  const pageHeight = pdf.internal.pageSize.getHeight();
  const lines = pdf.splitTextToSize(text, pdf.internal.pageSize.getWidth() - margin * 2);
  let y = margin;

  pdf.setFont('helvetica', 'normal');
  pdf.setFontSize(11);
  for (const line of lines) {
    if (y > pageHeight - margin) {
      pdf.addPage();
      y = margin;
    }
    pdf.text(line, margin, y);
    y += 6;
  }

  return Array.from(new Uint8Array(pdf.output('arraybuffer')));
}

export function PurchasePdfDialog({ model, documentTitle, onClose, onSaved }: PurchasePdfDialogProps) {
  const [text, setText] = useState(() => draftText(model));
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const savePdf = async () => {
    if (!text.trim()) {
      setError('Escribe el contenido del pedido antes de guardar.');
      return;
    }

    setSaving(true);
    setError(null);
    try {
      const path = await save({
        defaultPath: `${documentTitle}.pdf`,
        filters: [{ name: 'PDF', extensions: ['pdf'] }],
      });
      if (!path) return;
      await savePurchasePdf(path, createPdf(text));
      onSaved();
    } catch (cause) {
      setError(`No se pudo guardar el PDF: ${errorMessage(cause)}`);
    } finally {
      setSaving(false);
    }
  };

  return (
    <Modal title="Editar pedido" onClose={saving ? () => undefined : onClose} width={760}>
      <div className="purchase-pdf-editor-dialog">
        <p>Este texto se guardará tal como lo edites. No incluye precios ni formato de factura.</p>
        <label className="purchase-pdf-editor-label" htmlFor="purchase-pdf-text">
          Texto del pedido
        </label>
        <textarea
          id="purchase-pdf-text"
          className="suppliers-input suppliers-textarea purchase-pdf-editor"
          value={text}
          disabled={saving}
          onChange={event => setText(event.target.value)}
          spellCheck
        />
        {error && <p className="purchase-pdf-error" role="alert">{error}</p>}
        <div className="purchase-pdf-actions">
          <button type="button" className="suppliers-btn" disabled={saving} onClick={onClose}>
            Cancelar
          </button>
          <button type="button" className="suppliers-btn suppliers-btn-primary" disabled={saving} onClick={() => void savePdf()}>
            {saving ? 'Guardando PDF…' : 'Guardar PDF'}
          </button>
        </div>
      </div>
    </Modal>
  );
}