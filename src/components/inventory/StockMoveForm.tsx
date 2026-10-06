import React, { useState } from 'react';
import type {
  ProductVariant,
  StockEntryDto,
  StockOutDto,
  StockAdjustmentDto,
  MovementReason,
} from '../../types';
import { ALL_REASONS, INCREASE_REASONS, REASON_LABELS } from '../../utils/movementReasons';
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
  const [reason, setReason] = useState<MovementReason | ''>('');
  const [notes, setNotes] = useState('');
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [confirming, setConfirming] = useState(false);

  const actionLabel =
    mode === 'entry'
      ? 'Registrar entrada'
      : mode === 'out'
        ? 'Registrar salida'
        : 'Ajustar stock';

  const qty = Number(quantity);
  const actual = Number(actualStock);
  const diff =
    mode === 'adjustment' && actualStock.trim() !== '' && Number.isInteger(actual)
      ? actual - variant.stock
      : null;
  const needsReason = mode === 'out' || (mode === 'adjustment' && diff !== null && diff !== 0);
  const allowedReasons: MovementReason[] =
    mode === 'adjustment' && diff !== null && diff > 0 ? INCREASE_REASONS : ALL_REASONS;
  const effectiveReason: MovementReason | '' =
    reason && allowedReasons.includes(reason) ? reason : '';
  const resultingStock =
    mode === 'out' && Number.isInteger(qty) && qty > 0 ? variant.stock - qty : null;

  const clearError = (field: string) =>
    setErrors(prev => ({ ...prev, [field]: '' }));

  const validate = () => {
    const errs: Record<string, string> = {};

    if (mode !== 'adjustment') {
      if (!quantity.trim() || !Number.isInteger(qty) || qty <= 0) {
        errs.quantity = 'La cantidad debe ser un entero mayor a 0';
      } else if (mode === 'out' && qty > variant.stock && !variant.allow_negative) {
        errs.quantity = `Stock insuficiente. Disponible: ${variant.stock}`;
      }
    }

    if (mode === 'entry' && unitCost.trim() && Number(unitCost) < 0) {
      errs.unitCost = 'El costo unitario no puede ser negativo';
    }

    if (mode === 'adjustment') {
      if (actualStock.trim() === '' || !Number.isInteger(actual) || actual < 0) {
        errs.actualStock = 'El stock debe ser un entero de 0 o mayor';
      } else if (diff === 0) {
        errs.actualStock = 'El stock real es igual al actual: no hay nada que ajustar';
      }
    }

    if (needsReason && !effectiveReason) {
      errs.reason = 'Selecciona el motivo';
    }

    if (effectiveReason === 'other' && !notes.trim()) {
      errs.notes = 'Si el motivo es «Otro», escribe una nota que lo explique';
    }

    setErrors(errs);
    return Object.keys(errs).length === 0;
  };

  const buildDto = (): StockEntryDto | StockOutDto | StockAdjustmentDto => {
    const base = { variant_id: variant.id };
    const cleanNotes = notes.trim() || undefined;

    if (mode === 'entry') {
      return {
        ...base,
        quantity: qty,
        unit_cost: unitCost.trim() ? Number(unitCost) : undefined,
        notes: cleanNotes,
      };
    }
    if (mode === 'out') {
      return {
        ...base,
        quantity: qty,
        reason: effectiveReason || undefined,
        notes: cleanNotes,
      };
    }
    return {
      ...base,
      actual_stock: actual,
      reason: effectiveReason || undefined,
      notes: cleanNotes,
    };
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!validate()) return;

    if (mode !== 'entry' && !confirming) {
      setConfirming(true);
      return;
    }

    await onSubmit(buildDto());
  };

  if (confirming) {
    const newStock = mode === 'out' ? variant.stock - qty : actual;
    return (
      <div className="product-form">
        <div className="form-field">
          <label className="form-label">Confirma el movimiento</label>
          <div className="form-static">
            {actionLabel}: stock {variant.stock} → {newStock}
            {mode === 'out' && ` (−${qty})`}
            {mode === 'adjustment' && diff !== null && ` (${diff > 0 ? '+' : ''}${diff})`}
          </div>
        </div>
        {effectiveReason && (
          <div className="form-field">
            <label className="form-label">Motivo</label>
            <div className="form-static">{REASON_LABELS[effectiveReason]}</div>
          </div>
        )}
        {notes.trim() && (
          <div className="form-field">
            <label className="form-label">Notas</label>
            <div className="form-static">{notes.trim()}</div>
          </div>
        )}
        <span className="form-hint">Este movimiento quedará registrado en el kardex.</span>
        <div className="form-actions">
          <button
            type="button"
            className="btn btn-ghost"
            onClick={() => setConfirming(false)}
            disabled={loading}
          >
            Volver
          </button>
          <button
            type="button"
            className="btn btn-primary"
            onClick={() => onSubmit(buildDto())}
            disabled={loading}
          >
            {loading ? 'Guardando…' : 'Confirmar'}
          </button>
        </div>
      </div>
    );
  }

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
              clearError('quantity');
            }}
            placeholder="Ej: 10"
          />
          {errors.quantity && <span className="form-error">{errors.quantity}</span>}
          {resultingStock !== null && (
            <span className="form-hint">
              El stock quedará en {resultingStock}
              {resultingStock < 0 && ' (stock negativo permitido en esta variante)'}
            </span>
          )}
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
              clearError('unitCost');
            }}
            placeholder="Opcional"
          />
          {errors.unitCost && <span className="form-error">{errors.unitCost}</span>}
        </div>
      )}

      {mode === 'adjustment' && (
        <div className="form-field">
          <label className="form-label">Stock real (conteo físico)</label>
          <input
            className={`form-input ${errors.actualStock ? 'form-input--error' : ''}`}
            type="number"
            min="0"
            value={actualStock}
            onChange={e => {
              setActualStock(e.target.value);
              clearError('actualStock');
              clearError('reason');
            }}
          />
          {errors.actualStock && <span className="form-error">{errors.actualStock}</span>}
          {diff !== null && diff !== 0 && (
            <span className="form-hint">
              Diferencia: {diff > 0 ? '+' : ''}{diff} unidades ({variant.stock} → {actual})
            </span>
          )}
        </div>
      )}

      {needsReason && (
        <div className="form-field">
          <label className="form-label">Motivo <span className="required">*</span></label>
          <select
            className={`form-input form-select ${errors.reason ? 'form-input--error' : ''}`}
            value={effectiveReason}
            onChange={e => {
              setReason(e.target.value as MovementReason | '');
              clearError('reason');
              clearError('notes');
            }}
          >
            <option value="">Selecciona un motivo…</option>
            {allowedReasons.map(item => (
              <option key={item} value={item}>{REASON_LABELS[item]}</option>
            ))}
          </select>
          {errors.reason && <span className="form-error">{errors.reason}</span>}
          {mode === 'adjustment' && diff !== null && diff > 0 && (
            <span className="form-hint">
              Al aumentar el stock solo se permite «Corrección de conteo» u «Otro».
            </span>
          )}
        </div>
      )}

      <div className="form-field">
        <label className="form-label">
          Notas {effectiveReason === 'other' && <span className="required">*</span>}
        </label>
        <textarea
          className={`form-input form-textarea ${errors.notes ? 'form-input--error' : ''}`}
          value={notes}
          onChange={e => {
            setNotes(e.target.value);
            clearError('notes');
          }}
          placeholder={effectiveReason === 'other' ? 'Explica el motivo' : 'Opcional'}
          rows={3}
        />
        {errors.notes && <span className="form-error">{errors.notes}</span>}
      </div>

      <div className="form-actions">
        <button type="button" className="btn btn-ghost" onClick={onCancel} disabled={loading}>
          Cancelar
        </button>
        <button type="submit" className="btn btn-primary" disabled={loading}>
          {mode === 'entry' ? actionLabel : 'Continuar'}
        </button>
      </div>
    </form>
  );
};
