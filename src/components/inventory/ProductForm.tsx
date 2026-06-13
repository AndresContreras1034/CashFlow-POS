import React, { useEffect, useState } from 'react';
import type { Category, CreateProductDto, UpdateProductDto, ProductWithCategory } from '../../types';
import './ProductForm.css';

interface ProductFormProps {
  categories: Category[];
  product?: ProductWithCategory;
  onSubmit: (dto: CreateProductDto | UpdateProductDto) => Promise<void>;
  onCancel: () => void;
  loading?: boolean;
}

export const ProductForm: React.FC<ProductFormProps> = ({
  categories,
  product,
  onSubmit,
  onCancel,
  loading = false,
}) => {
  const isEdit = !!product;

  const [form, setForm] = useState({
    category_id: product?.category_id ?? (categories[0]?.id ?? 0),
    name:        product?.name ?? '',
    description: product?.description ?? '',
    brand:       product?.brand ?? '',
    image_url:   product?.image_url ?? '',
    is_active:   product?.is_active ?? true,
  });

  const [errors, setErrors] = useState<Record<string, string>>({});

  useEffect(() => {
    if (!form.category_id && categories.length > 0) {
      setForm(f => ({ ...f, category_id: categories[0].id }));
    }
  }, [categories]);

  const set = (field: string) => (e: React.ChangeEvent<HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement>) => {
    setForm(f => ({ ...f, [field]: e.target.value }));
    setErrors(err => ({ ...err, [field]: '' }));
  };

  const validate = (): boolean => {
    const e: Record<string, string> = {};
    if (!form.name.trim()) e.name = 'El nombre es obligatorio';
    if (!form.category_id) e.category_id = 'Selecciona una categoría';
    setErrors(e);
    return Object.keys(e).length === 0;
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!validate()) return;

    const dto: CreateProductDto | UpdateProductDto = {
      category_id: Number(form.category_id),
      name: form.name.trim(),
      description: form.description.trim() || undefined,
      brand: form.brand.trim() || undefined,
      image_url: form.image_url.trim() || undefined,
      ...(isEdit && { is_active: form.is_active }),
    };

    await onSubmit(dto);
  };

  return (
    <form className="product-form" onSubmit={handleSubmit} noValidate>
      <div className="form-row">
        <div className="form-field">
          <label className="form-label">Nombre <span className="required">*</span></label>
          <input
            className={`form-input ${errors.name ? 'form-input--error' : ''}`}
            type="text"
            value={form.name}
            onChange={set('name')}
            placeholder="Ej: Shampoo Anticaída"
            autoFocus
          />
          {errors.name && <span className="form-error">{errors.name}</span>}
        </div>

        <div className="form-field">
          <label className="form-label">Categoría <span className="required">*</span></label>
          <select
            className={`form-input form-select ${errors.category_id ? 'form-input--error' : ''}`}
            value={form.category_id}
            onChange={set('category_id')}
          >
            {categories.map(c => (
              <option key={c.id} value={c.id}>{c.name}</option>
            ))}
          </select>
          {errors.category_id && <span className="form-error">{errors.category_id}</span>}
        </div>
      </div>

      <div className="form-row">
        <div className="form-field">
          <label className="form-label">Marca</label>
          <input
            className="form-input"
            type="text"
            value={form.brand}
            onChange={set('brand')}
            placeholder="Ej: L'Oréal"
          />
        </div>

        <div className="form-field">
          <label className="form-label">URL de imagen</label>
          <input
            className="form-input"
            type="text"
            value={form.image_url}
            onChange={set('image_url')}
            placeholder="https://..."
          />
        </div>
      </div>

      <div className="form-field">
        <label className="form-label">Descripción</label>
        <textarea
          className="form-input form-textarea"
          value={form.description}
          onChange={set('description')}
          placeholder="Descripción opcional del producto"
          rows={3}
        />
      </div>

      {isEdit && (
        <div className="form-field form-field--inline">
          <label className="form-label">Estado</label>
          <label className="toggle">
            <input
              type="checkbox"
              checked={form.is_active}
              onChange={e => setForm(f => ({ ...f, is_active: e.target.checked }))}
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
          {loading ? 'Guardando…' : isEdit ? 'Guardar cambios' : 'Crear producto'}
        </button>
      </div>
    </form>
  );
};
