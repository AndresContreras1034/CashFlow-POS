import React, { useState } from 'react';
import type {
  ProductVariant,
  StockEntryDto,
  StockOutDto,
  StockAdjustmentDto,
} from '../../types';
import './ProductForm.css';

interface StockMoveFormProps {
  mode: 'entry' | 'out' | 'adjustment';
  variant: ProductVariant;
  onSubmit: (dto: StockEntryDto | StockOutDto | StockAdjustmentDto) => Promise<void>;
  onCancel: () => void;
  loading?: boolean;
}

export const StockMoveForm: React.FC<StockMoveFormProps> = ({
  mode,
  variant,
  onSubmit,
  onCancel,
  loading = false,
}) => {
  const [quantity, setQuantity] = useState('');
  const [unitCost, setUnitCost] = useState('');
  const [actualStock, setActualStock] = useState(variant.stock.toString());
  const [notes, setNotes] = useState('');
  const [errors, setErrors] = useState<Record<string, string>>({});

  const actionLabel =
    mode === 'entry'
      ? 'Registrar entrada'
      : mode === 'out'
      ? 'Registrar salida'
      : 'Ajustar stock';

  const validate = () => {
    const errs: Record<string, string> = {};
    if (mode !== 'adjustment') {
      if (!quantity.trim() || Number(quantity) <= 0) {
        errs.quantity = 'La cantidad debe ser mayor a 0';
      }
    }
    if (mode === 'entry' && unitCost.trim() && Number(unitCost) < 0) {
      errs.unitCost = 'El costo unitario no puede ser negativo';
    }
    if (mode === 'adjustment' && (actualStock.trim() === '' || Number(actualStock) < 0)) {
      errs.actualStock = 'El stock debe ser 0 o mayor';
    }
    setErrors(errs);
    return Object.keys(errs).length === 0;
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!validate()) return;

    const base = { variant_id: variant.id };
    let dto: StockEntryDto | StockOutDto | StockAdjustmentDto;

    if (mode === 'entry') {
      dto = {
        ...base,
        quantity: Number(quantity),
        unit_cost: unitCost.trim() ? Number(unitCost) : undefined,
        notes: notes.trim() || undefined,
      };
    } else if (mode === 'out') {
      dto = {
        ...base,
        quantity: Number(quantity),
        notes: notes.trim() || undefined,
      };
    } else {
      dto = {
        ...base,
        actual_stock: Number(actualStock),
        notes: notes.trim() || undefined,
      };
    }

    await onSubmit(dto);
  };

  return (
    <form className="product-form" onSubmit={handleSubmit} noValidate>
      <div className="form-row">
        <div className="form-field">
          <label className="form-label">Variante</label>
          <div className="form-static">{variant.sku || variant.barcode || 'ID ' + variant.id}</div>
        </div>
        <div className="form-field">
          <label className="form-label">Stock actual</label>
          <div className="form-static">{variant.stock}</div>
        </div>
      </div>

      {(mode === 'entry' || mode === 'out') && (
        <div className="form-field">
          <label className="form-label">Cantidad</label>
          <input
            className={`form-input ${errors.quantity ? 'form-input--error' : ''}`}
            type="number"
            min="1"
            value={quantity}
            onChange={e => {
              setQuantity(e.target.value);
              setErrors(prev => ({ ...prev, quantity: '' }));
            }}
            placeholder="Ej: 10"
          />
          {errors.quantity && <span className="form-error">{errors.quantity}</span>}
        </div>
      )}

      {mode === 'entry' && (
        <div className="form-field">
          <label className="form-label">Costo unitario</label>
          <input
            className={`form-input ${errors.unitCost ? 'form-input--error' : ''}`}
            type="number"
            min="0"
            value={unitCost}
            onChange={e => {
              setUnitCost(e.target.value);
              setErrors(prev => ({ ...prev, unitCost: '' }));
            }}
            placeholder="Opcional"
          />
          {errors.unitCost && <span className="form-error">{errors.unitCost}</span>}
        </div>
      )}

      {mode === 'adjustment' && (
        <div className="form-field">
          <label className="form-label">Stock real</label>
          <input
            className={`form-input ${errors.actualStock ? 'form-input--error' : ''}`}
            type="number"
            min="0"
            value={actualStock}
            onChange={e => {
              setActualStock(e.target.value);
              setErrors(prev => ({ ...prev, actualStock: '' }));
            }}
          />
          {errors.actualStock && <span className="form-error">{errors.actualStock}</span>}
        </div>
      )}

      <div className="form-field">
        <label className="form-label">Notas</label>
        <textarea
          className="form-input form-textarea"
          value={notes}
          onChange={e => setNotes(e.target.value)}
          placeholder="Opcional"
          rows={3}
        />
      </div>

      <div className="form-actions">
        <button type="button" className="btn btn-ghost" onClick={onCancel} disabled={loading}>
          Cancelar
        </button>
        <button type="submit" className="btn btn-primary" disabled={loading}>
          {loading ? 'Guardando…' : actionLabel}
        </button>
      </div>
    </form>
  );
};
