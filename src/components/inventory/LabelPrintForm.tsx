import React, { useState } from 'react';
import type { ProductVariant } from '../../types';
import { formatAttributes, formatMoney } from '../../utils/format';
import './ProductForm.css';

interface LabelPrintFormProps {
  productName: string;
  variant: ProductVariant;
  onPrint: (copies: number) => Promise<void>;
  onCancel: () => void;
  loading?: boolean;
}

const MAX_COPIES = 100;

export const LabelPrintForm: React.FC<LabelPrintFormProps> = ({
  productName,
  variant,
  onPrint,
  onCancel,
  loading = false,
}) => {
  const [copies, setCopies] = useState('1');
  const [error, setError] = useState('');

  const hasBarcode = Boolean(variant.barcode?.trim());
  const attrs = formatAttributes(variant.attributes);

  const handleSubmit = async (event: React.FormEvent) => {
    event.preventDefault();
    const count = Number(copies);
    if (!Number.isInteger(count) || count < 1 || count > MAX_COPIES) {
      setError(`Las copias deben ser un entero entre 1 y ${MAX_COPIES}`);
      return;
    }
    await onPrint(count);
  };

  return (
    <form className="product-form label-print-form" onSubmit={handleSubmit} noValidate>
      <div className="form-field">
        <label className="form-label">Etiqueta</label>
        <div className="label-print-summary">
          <div className="label-print-summary__name">{productName}</div>
          {attrs && <div className="label-print-summary__attributes">{attrs}</div>}
          <div className="label-print-summary__price">{formatMoney(variant.price)}</div>
          <div className="label-print-summary__barcode">
            {hasBarcode ? variant.barcode : 'Sin código de barras'}
          </div>
        </div>
      </div>

      {!hasBarcode && (
        <span className="form-error">
          Esta variante no tiene código de barras. Edítala y usa «Generar código interno».
        </span>
      )}

      <div className="form-field">
        <label className="form-label" htmlFor="label-copies">Copias</label>
        <input
          id="label-copies"
          className={`form-input ${error ? 'form-input--error' : ''}`}
          type="number"
          min="1"
          max={MAX_COPIES}
          value={copies}
          onChange={event => {
            setCopies(event.target.value);
            setError('');
          }}
        />
        {error && <span className="form-error">{error}</span>}
        <span className="form-hint">
          Cada etiqueta sale como una tira de la impresora de tickets y se corta sola.
        </span>
      </div>

      <div className="form-actions">
        <button type="button" className="btn btn-ghost" onClick={onCancel} disabled={loading}>
          Cancelar
        </button>
        <button type="submit" className="btn btn-primary" disabled={loading || !hasBarcode}>
          {loading ? 'Imprimiendo…' : 'Imprimir'}
        </button>
      </div>
    </form>
  );
};
