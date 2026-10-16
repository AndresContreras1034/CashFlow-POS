import { useEffect, useMemo, useState } from 'react';
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

function createPdf(text: string): jsPDF {
  const pdf = new jsPDF({ unit: 'mm', format: 'a4' });
  const pageWidth = pdf.internal.pageSize.getWidth();
  const pageHeight = pdf.internal.pageSize.getHeight();
  const margin = 18;
  const contentWidth = pageWidth - margin * 2;
  const lines = text.split(/\r?\n/);
  let y = 22;

  const ensureSpace = (height: number) => {
    if (y + height > pageHeight - margin) {
      pdf.addPage();
      y = margin;
    }
  };

  const writeWrapped = (value: string, fontSize: number, color: [number, number, number]) => {
    pdf.setFont('helvetica', 'normal');
    pdf.setFontSize(fontSize);
    pdf.setTextColor(...color);
    const wrapped = pdf.splitTextToSize(value, contentWidth);
    ensureSpace(wrapped.length * 5 + 2);
    pdf.text(wrapped, margin, y);
    y += wrapped.length * 5 + 2;
  };

  const title = lines.shift()?.trim() || 'Pedido de compra';
  pdf.setFont('helvetica', 'bold');
  pdf.setFontSize(19);
  pdf.setTextColor(30, 41, 59);
  const wrappedTitle = pdf.splitTextToSize(title, contentWidth);
  pdf.text(wrappedTitle, margin, y);
  y += wrappedTitle.length * 8 + 2;
  pdf.setDrawColor(37, 99, 235);
  pdf.setLineWidth(1.1);
  pdf.line(margin, y, pageWidth - margin, y);
  y += 9;

  for (const line of lines) {
    const trimmed = line.trim();
    if (!trimmed) {
      y += 3;
      continue;
    }
    if (trimmed.startsWith('Proveedor:')) {
      ensureSpace(12);
      pdf.setFillColor(239, 246, 255);
      pdf.roundedRect(margin, y - 5, contentWidth, 10, 1.5, 1.5, 'F');
      pdf.setFont('helvetica', 'bold');
      pdf.setFontSize(11);
      pdf.setTextColor(30, 64, 175);
      pdf.text(trimmed.slice('Proveedor:'.length).trim() || 'Proveedor', margin + 3, y + 1);
      y += 13;
      continue;
    }
    if (trimmed.startsWith('- ')) {
      const item = trimmed.slice(2);
      const match = item.match(/^(.*)\s×\s(\d+)$/);
      if (match) {
        pdf.setFont('helvetica', 'normal');
        pdf.setFontSize(10);
        const quantity = `${match[2]} uds.`;
        const productWidth = contentWidth - pdf.getTextWidth(quantity) - 8;
        const productLines = pdf.splitTextToSize(match[1], productWidth);
        ensureSpace(Math.max(productLines.length * 5, 6) + 5);
        pdf.setTextColor(51, 65, 85);
        pdf.text(productLines, margin + 2, y);
        pdf.setFont('helvetica', 'bold');
        pdf.setTextColor(30, 41, 59);
        pdf.text(quantity, pageWidth - margin - pdf.getTextWidth(quantity), y);
        y += Math.max(productLines.length * 5, 6) + 3;
        pdf.setDrawColor(226, 232, 240);
        pdf.setLineWidth(0.2);
        pdf.line(margin + 2, y, pageWidth - margin - 2, y);
        y += 3;
        continue;
      }
    }

    writeWrapped(trimmed, 10, [51, 65, 85]);
  }

  return pdf;
}

export function PurchasePdfDialog({ model, documentTitle, onClose, onSaved }: PurchasePdfDialogProps) {
  const [text, setText] = useState(() => draftText(model));
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [previewUrl, setPreviewUrl] = useState<string | null>(null);
  const pdf = useMemo(() => createPdf(text), [text]);

  useEffect(() => {
    const url = URL.createObjectURL(pdf.output('blob'));
    setPreviewUrl(url);
    return () => URL.revokeObjectURL(url);
  }, [pdf]);

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
      const pdfBytes = new Uint8Array(pdf.output('arraybuffer'));
      if (pdfBytes.length === 0 || pdfBytes.length > 20 * 1024 * 1024) {
        setError('El PDF está vacío o supera el límite de 20 MB.');
        return;
      }
      const filePath = /\.pdf$/i.test(path) ? path : `${path}.pdf`;
      await savePurchasePdf(filePath, Array.from(pdfBytes));
      onSaved();
    } catch (cause) {
      setError(`No se pudo guardar el PDF: ${errorMessage(cause)}`);
    } finally {
      setSaving(false);
    }
  };

  return (
    <Modal title="Editar pedido" onClose={saving ? () => undefined : onClose} width={1160}>
      <div className="purchase-pdf-editor-dialog">
        <p>Edita el contenido y revisa la vista previa del PDF antes de guardarlo. No incluye precios.</p>
        <div className="purchase-pdf-workspace">
          <div className="purchase-pdf-edit-pane">
            <label className="purchase-pdf-editor-label" htmlFor="purchase-pdf-text">
              Contenido del pedido
            </label>
            <textarea
              id="purchase-pdf-text"
              className="suppliers-input suppliers-textarea purchase-pdf-editor"
              value={text}
              disabled={saving}
              onChange={event => setText(event.target.value)}
              spellCheck
            />
          </div>
          <section className="purchase-pdf-preview-pane" aria-label="Vista previa del PDF">
            <div className="purchase-pdf-preview-heading">
              <strong>Vista previa</strong>
              <span>A4 · {documentTitle}.pdf</span>
            </div>
            {previewUrl ? (
              <iframe
                className="purchase-pdf-preview"
                title="Vista previa del pedido en PDF"
                src={`${previewUrl}#toolbar=0&navpanes=0`}
              />
            ) : (
              <p className="purchase-muted" role="status">Preparando vista previa…</p>
            )}
          </section>
        </div>
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