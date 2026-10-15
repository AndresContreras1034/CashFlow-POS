import React, { useEffect, useRef } from 'react';
import { createPortal } from 'react-dom';
import { formatMoney } from '../../utils/format';
import { describeTotals, type PrintModel } from '../../utils/purchasePrint';
import './PurchasePrint.css';

interface PurchasePrintJobProps {
  model: PrintModel;
  documentTitle: string;
  onDone: () => void;
}

const PRINT_CLASS = 'purchase-printing';

const PrintDocument: React.FC<{ model: PrintModel }> = ({ model }) => (
  <div className="purchase-print-root" aria-hidden="true">
    <header className="pp-header">
      <p className="pp-business">{model.businessName}</p>
      <h1>{model.title}</h1>
      <p className="pp-meta">
        {model.statusLabel} · {model.day}
      </p>
      <p className="pp-meta">{describeTotals(model.totals, formatMoney)}</p>
      {model.notes && <p className="pp-notes">{model.notes}</p>}
    </header>

    {model.groups.map(group => (
      <section key={group.key ?? 'sin-proveedor'} className="pp-group">
        <div className="pp-group-head">
          <h2>{group.name}</h2>
          <span className="pp-group-sum">
            {group.itemCount === 1 ? '1 producto' : `${group.itemCount} productos`} ·{' '}
            {group.units === 1 ? '1 unidad' : `${group.units} unidades`} ·{' '}
            {formatMoney(group.estimatedCost)}
            {group.itemsWithoutCost > 0 && ` (+${group.itemsWithoutCost} sin costo)`}
          </span>
        </div>
        {group.contact && <p className="pp-contact">{group.contact}</p>}

        <table className="pp-table">
          <thead>
            <tr>
              <th className="pp-check" scope="col">✓</th>
              <th scope="col">Producto</th>
              <th className="pp-num" scope="col">Cant.</th>
              <th className="pp-num" scope="col">Costo unit.</th>
              <th className="pp-num" scope="col">Total</th>
            </tr>
          </thead>
          <tbody>
            {group.lines.map(line => (
              <tr key={line.key}>
                <td className="pp-check"><span className="pp-box" /></td>
                <td>
                  <span className="pp-product">{line.product}</span>
                  {line.detail && <span className="pp-sub">{line.detail}</span>}
                  {line.supplierSku && <span className="pp-sub">Ref. proveedor: {line.supplierSku}</span>}
                  {line.note && <span className="pp-sub pp-note">{line.note}</span>}
                </td>
                <td className="pp-num pp-qty">{line.quantity}</td>
                <td className="pp-num">{line.unitCost === null ? '—' : formatMoney(line.unitCost)}</td>
                <td className="pp-num">{line.lineCost === null ? '—' : formatMoney(line.lineCost)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </section>
    ))}

    <footer className="pp-footer">
      <strong>Total estimado: {formatMoney(model.totals.estimatedCost)}</strong>
      <span>Los costos son estimados con el costo actual del inventario.</span>
    </footer>
  </div>
);

export const PurchasePrintJob: React.FC<PurchasePrintJobProps> = ({
  model,
  documentTitle,
  onDone,
}) => {
  const doneRef = useRef(onDone);
  doneRef.current = onDone;

  useEffect(() => {
    const previousTitle = document.title;
    document.title = documentTitle;
    document.documentElement.classList.add(PRINT_CLASS);

    const finish = () => doneRef.current();
    window.addEventListener('afterprint', finish);
    const timer = window.setTimeout(() => window.print(), 100);

    return () => {
      window.clearTimeout(timer);
      window.removeEventListener('afterprint', finish);
      document.documentElement.classList.remove(PRINT_CLASS);
      document.title = previousTitle;
    };
  }, [documentTitle]);

  return createPortal(<PrintDocument model={model} />, document.body);
};
