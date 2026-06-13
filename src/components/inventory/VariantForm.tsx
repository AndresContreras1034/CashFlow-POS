import React, { useEffect, useState } from 'react';
import type { ProductVariant, CreateVariantDto, UpdateVariantDto } from '../../types';
import './ProductForm.css';

interface VariantFormProps {
  productId: number;
  variant?: ProductVariant;
  onSubmit: (dto: CreateVariantDto | UpdateVariantDto) => Promise<void>;
  onCancel: () => void;
  loading?: boolean;
}

const parseAttributes = (text: string) => {
  return text
    .split('\n')
    .map(line => line.trim())
    .filter(Boolean)
    .reduce<Record<string, string>>((acc, line) => {
      const [key, ...rest] = line.split(':');
      const value = rest.join(':').trim();
      if (key && value) acc[key.trim()] = value;
      return acc;
    }, {});
};

const formatAttributes = (attributes: Record<string, string>) => {
  return Object.entries(attributes)
    .map(([key, value]) => `${key}: ${value}`)
    .join('\n');
};

export const VariantForm: React.FC<VariantFormProps> = ({
  productId,
  variant,
  onSubmit,
  onCancel,
  loading = false,
}) => {
  const isEdit = Boolean(variant);

  const [form, setForm] = useState({
    sku: variant?.sku ?? '',
    barcode: variant?.barcode ?? '',
    price: variant?.price.toString() ?? '0',
    cost: variant?.cost.toString() ?? '0',
    stock: variant?.stock.toString() ?? '0',
    stock_min: variant?.stock_min.toString() ?? '1',
    allow_negative: variant?.allow_negative ?? false,
    is_active: variant?.is_active ?? true,
    attributes: formatAttributes(variant?.attributes ?? {}),
  });

  const [errors, setErrors] = useState<Record<string, string>>({});

  useEffect(() => {
    if (variant) {
      setForm({
        sku: variant.sku ?? '',
        barcode: variant.barcode ?? '',
        price: variant.price.toString(),
        cost: variant.cost.toString(),
        stock: variant.stock.toString(),
        stock_min: variant.stock_min.toString(),
        allow_negative: variant.allow_negative,
        is_active: variant.is_active,
        attributes: formatAttributes(variant.attributes),
      });
    }
  }, [variant]);

  const setValue = (field: string) =>
    (e: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement>) => {
      const value = e.target.type === 'checkbox' ? String((e.target as HTMLInputElement).checked) : e.target.value;
      setForm(prev => ({ ...prev, [field]: value }));
      setErrors(prev => ({ ...prev, [field]: '' }));
    };

  const validate = () => {
    const errs: Record<string, string> = {};
    if (!form.price.trim() || Number(form.price) <= 0) errs.price = 'El precio debe ser mayor a 0';
    if (form.cost.trim() && Number(form.cost) < 0) errs.cost = 'El costo no puede ser negativo';
    if (!form.stock.trim() || Number(form.stock) < 0) errs.stock = 'El stock no puede ser negativo';
    if (!form.stock_min.trim() || Number(form.stock_min) < 0) errs.stock_min = 'El stock mínimo no puede ser negativo';
    setErrors(errs);
    return Object.keys(errs).length === 0;
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!validate()) return;

    const dto: CreateVariantDto | UpdateVariantDto = {
      product_id: productId,
      attributes: parseAttributes(form.attributes),
      sku: form.sku.trim() || undefined,
      barcode: form.barcode.trim() || undefined,
      price: Number(form.price),
      cost: Number(form.cost) || 0,
      stock: Number(form.stock),
      stock_min: Number(form.stock_min),
      allow_negative: form.allow_negative,
      ...(isEdit && { is_active: form.is_active }),
    };

    await onSubmit(dto);
  };

  return (
    <form className="product-form" onSubmit={handleSubmit} noValidate>
      <div className="form-row">
        <div className="form-field">
          <label className="form-label">SKU</label>
          <input
            className="form-input"
            type="text"
            value={form.sku}
            onChange={setValue('sku')}
            placeholder="Ej: SHA-001"
          />
        </div>

        <div className="form-field">
          <label className="form-label">Código de barras</label>
          <input
            className="form-input"
            type="text"
            value={form.barcode}
            onChange={setValue('barcode')}
            placeholder="Ej: 7501234000011"
          />
        </div>
      </div>

      <div className="form-row">
        <div className="form-field">
          <label className="form-label">Precio <span className="required">*</span></label>
          <input
            className={`form-input ${errors.price ? 'form-input--error' : ''}`}
            type="number"
            min="0"
            value={form.price}
            onChange={setValue('price')}
            placeholder="Ej: 2800000"
          />
          {errors.price && <span className="form-error">{errors.price}</span>}
        </div>

        <div className="form-field">
          <label className="form-label">Costo</label>
          <input
            className={`form-input ${errors.cost ? 'form-input--error' : ''}`}
            type="number"
            min="0"
            value={form.cost}
            onChange={setValue('cost')}
            placeholder="Ej: 1500000"
          />
          {errors.cost && <span className="form-error">{errors.cost}</span>}
        </div>
      </div>

      <div className="form-row">
        <div className="form-field">
          <label className="form-label">Stock</label>
          <input
            className={`form-input ${errors.stock ? 'form-input--error' : ''}`}
            type="number"
            min="0"
            value={form.stock}
            onChange={setValue('stock')}
            placeholder="0"
          />
          {errors.stock && <span className="form-error">{errors.stock}</span>}
        </div>

        <div className="form-field">
          <label className="form-label">Stock mínimo</label>
          <input
            className={`form-input ${errors.stock_min ? 'form-input--error' : ''}`}
            type="number"
            min="0"
            value={form.stock_min}
            onChange={setValue('stock_min')}
            placeholder="5"
          />
          {errors.stock_min && <span className="form-error">{errors.stock_min}</span>}
        </div>
      </div>

      <div className="form-field">
        <label className="form-label">Atributos</label>
        <textarea
          className="form-input form-textarea"
          value={form.attributes}
          onChange={setValue('attributes')}
          placeholder="Tamaño: 400 ml\nColor: Rojo"
          rows={4}
        />
      </div>

      <div className="form-field form-field--inline">
        <label className="form-label">Permitir stock negativo</label>
        <label className="toggle">
          <input
            type="checkbox"
            checked={form.allow_negative}
            onChange={e => setForm(prev => ({ ...prev, allow_negative: e.target.checked }))}
          />
          <span className="toggle-track" />
          <span className="toggle-label">{form.allow_negative ? 'Sí' : 'No'}</span>
        </label>
      </div>

      {isEdit && (
        <div className="form-field form-field--inline">
          <label className="form-label">Estado</label>
          <label className="toggle">
            <input
              type="checkbox"
              checked={form.is_active}
              onChange={e => setForm(prev => ({ ...prev, is_active: e.target.checked }))}
            />
            <span className="toggle-track" />
            <span className="toggle-label">{form.is_active ? 'Activo' : 'Inactivo'}</span>
          </label>
        </div>
      )}

      <div className="form-actions">
        <button type="button" className="btn btn-ghost" onClick={onCancel} disabled={loading}>
          Cancelar
        </button>
        <button type="submit" className="btn btn-primary" disabled={loading}>
          {loading ? 'Guardando…' : isEdit ? 'Guardar variante' : 'Crear variante'}
        </button>
      </div>
    </form>
  );
};
