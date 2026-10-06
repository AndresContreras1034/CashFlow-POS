import React, { useState, useEffect, useCallback } from 'react';
import { ToastContainer, ToastData } from '../../components/ui/Toast';
import type { AppSettings, UpdateSettingsDto } from '../../types';
import { getSettings, updateSettings } from '../../services/settings.service';
import './Settings.css';

type FormState = {
  business_name: string;
  tax_id: string;
  address: string;
  phone: string;
  email: string;
  currency: string;
  tax_rate_percent: string; // se muestra en % al usuario, se convierte a bps al guardar
  ticket_header: string;
  ticket_footer: string;
  low_stock_default_threshold: string;
  logo_url: string;
};

const emptyForm: FormState = {
  business_name: '',
  tax_id: '',
  address: '',
  phone: '',
  email: '',
  currency: 'COP',
  tax_rate_percent: '19',
  ticket_header: '',
  ticket_footer: '',
  low_stock_default_threshold: '5',
  logo_url: '',
};

const toForm = (s: AppSettings): FormState => ({
  business_name: s.business_name,
  tax_id: s.tax_id ?? '',
  address: s.address ?? '',
  phone: s.phone ?? '',
  email: s.email ?? '',
  currency: s.currency,
  tax_rate_percent: (s.tax_rate_bps / 100).toString(),
  ticket_header: s.ticket_header ?? '',
  ticket_footer: s.ticket_footer ?? '',
  low_stock_default_threshold: s.low_stock_default_threshold.toString(),
  logo_url: s.logo_url ?? '',
});

export const Settings: React.FC = () => {
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [form, setForm] = useState<FormState>(emptyForm);
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [dirty, setDirty] = useState(false);
  const [toasts, setToasts] = useState<ToastData[]>([]);

  const addToast = useCallback((message: string, type: ToastData['type']) => {
    setToasts(t => [...t, { id: Math.random().toString(36).slice(2), message, type }]);
  }, []);

  const dismissToast = useCallback((id: string) => {
    setToasts(t => t.filter(x => x.id !== id));
  }, []);

  useEffect(() => {
    const load = async () => {
      try {
        setLoading(true);
        const data = await getSettings();
        setSettings(data);
        setForm(toForm(data));
      } catch (error) {
        addToast('No se pudieron cargar los ajustes', 'error');
      } finally {
        setLoading(false);
      }
    };
    load();
  }, [addToast]);

  const set = (field: keyof FormState) => (
    e: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement>
  ) => {
    setForm(f => ({ ...f, [field]: e.target.value }));
    setErrors(err => ({ ...err, [field]: '' }));
    setDirty(true);
  };

  const validate = (): boolean => {
    const e: Record<string, string> = {};

    if (!form.business_name.trim()) e.business_name = 'El nombre del negocio es obligatorio';

    if (form.currency.trim().length !== 3) e.currency = 'Usa un código de 3 letras (ej: COP, USD)';

    const taxRate = Number(form.tax_rate_percent);
    if (form.tax_rate_percent.trim() === '' || Number.isNaN(taxRate) || taxRate < 0 || taxRate > 100) {
      e.tax_rate_percent = 'El IVA debe estar entre 0 y 100';
    }

    const threshold = Number(form.low_stock_default_threshold);
    if (form.low_stock_default_threshold.trim() === '' || Number.isNaN(threshold) || threshold < 0) {
      e.low_stock_default_threshold = 'El umbral no puede ser negativo';
    }

    if (form.email.trim() && !/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(form.email.trim())) {
      e.email = 'Correo inválido';
    }

    setErrors(e);
    return Object.keys(e).length === 0;
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!validate()) return;

    const dto: UpdateSettingsDto = {
      business_name: form.business_name.trim(),
      tax_id: form.tax_id.trim() || undefined,
      address: form.address.trim() || undefined,
      phone: form.phone.trim() || undefined,
      email: form.email.trim() || undefined,
      currency: form.currency.trim().toUpperCase(),
      tax_rate_bps: Math.round(Number(form.tax_rate_percent) * 100),
      ticket_header: form.ticket_header.trim() || undefined,
      ticket_footer: form.ticket_footer.trim() || undefined,
      low_stock_default_threshold: Number(form.low_stock_default_threshold),
      logo_url: form.logo_url.trim() || undefined,
    };

    setSaving(true);
    try {
      const updated = await updateSettings(dto);
      setSettings(updated);
      setForm(toForm(updated));
      setDirty(false);
      addToast('Ajustes guardados correctamente', 'success');
    } catch (error) {
      addToast('No se pudieron guardar los ajustes', 'error');
    } finally {
      setSaving(false);
    }
  };

  const handleReset = () => {
    if (!settings) return;
    setForm(toForm(settings));
    setErrors({});
    setDirty(false);
  };

  if (loading) {
    return (
      <div className="settings-page">
        <div className="settings-loading">Cargando ajustes...</div>
      </div>
    );
  }

  return (
    <div className="settings-page">
      <div className="settings-header">
        <h1 className="settings-title">Ajustes</h1>
        <p className="settings-subtitle">Configuración general del negocio</p>
      </div>

      <form className="settings-form" onSubmit={handleSubmit} noValidate>
        <section className="settings-section">
          <h2 className="settings-section-title">Datos del negocio</h2>

          <div className="form-row">
            <div className="form-field">
              <label className="form-label">Nombre del negocio <span className="required">*</span></label>
              <input
                className={`form-input ${errors.business_name ? 'form-input--error' : ''}`}
                type="text"
                value={form.business_name}
                onChange={set('business_name')}
                placeholder="Ej: Detalles y Regalos La 15"
              />
              {errors.business_name && <span className="form-error">{errors.business_name}</span>}
            </div>

            <div className="form-field">
              <label className="form-label">NIT / RUT</label>
              <input
                className="form-input"
                type="text"
                value={form.tax_id}
                onChange={set('tax_id')}
                placeholder="Ej: 900123456-7"
              />
            </div>
          </div>

          <div className="form-field">
            <label className="form-label">Dirección</label>
            <input
              className="form-input"
              type="text"
              value={form.address}
              onChange={set('address')}
              placeholder="Dirección del establecimiento"
            />
          </div>

          <div className="form-row">
            <div className="form-field">
              <label className="form-label">Teléfono</label>
              <input
                className="form-input"
                type="text"
                value={form.phone}
                onChange={set('phone')}
                placeholder="Ej: 3001234567"
              />
            </div>

            <div className="form-field">
              <label className="form-label">Correo</label>
              <input
                className={`form-input ${errors.email ? 'form-input--error' : ''}`}
                type="text"
                value={form.email}
                onChange={set('email')}
                placeholder="contacto@negocio.com"
              />
              {errors.email && <span className="form-error">{errors.email}</span>}
            </div>
          </div>

          <div className="form-field">
            <label className="form-label">URL del logo</label>
            <input
              className="form-input"
              type="text"
              value={form.logo_url}
              onChange={set('logo_url')}
              placeholder="https://..."
            />
          </div>
        </section>

        <section className="settings-section">
          <h2 className="settings-section-title">Facturación e inventario</h2>

          <div className="form-row">
            <div className="form-field">
              <label className="form-label">Moneda</label>
              <input
                className={`form-input ${errors.currency ? 'form-input--error' : ''}`}
                type="text"
                value={form.currency}
                onChange={set('currency')}
                placeholder="COP"
                maxLength={3}
              />
              {errors.currency && <span className="form-error">{errors.currency}</span>}
            </div>

            <div className="form-field">
              <label className="form-label">IVA (%)</label>
              <input
                className={`form-input ${errors.tax_rate_percent ? 'form-input--error' : ''}`}
                type="number"
                min="0"
                max="100"
                step="0.01"
                value={form.tax_rate_percent}
                onChange={set('tax_rate_percent')}
                placeholder="19"
              />
              {errors.tax_rate_percent && <span className="form-error">{errors.tax_rate_percent}</span>}
            </div>

            <div className="form-field">
              <label className="form-label">Umbral de stock bajo</label>
              <input
                className={`form-input ${errors.low_stock_default_threshold ? 'form-input--error' : ''}`}
                type="number"
                min="0"
                value={form.low_stock_default_threshold}
                onChange={set('low_stock_default_threshold')}
                placeholder="5"
              />
              {errors.low_stock_default_threshold && (
                <span className="form-error">{errors.low_stock_default_threshold}</span>
              )}
            </div>
          </div>
        </section>

        <section className="settings-section">
          <h2 className="settings-section-title">Ticket de venta</h2>

          <div className="form-field">
            <label className="form-label">Encabezado del ticket</label>
            <textarea
              className="form-input form-textarea"
              value={form.ticket_header}
              onChange={set('ticket_header')}
              placeholder="Texto que aparece arriba del ticket impreso"
              rows={2}
            />
          </div>

          <div className="form-field">
            <label className="form-label">Pie del ticket</label>
            <textarea
              className="form-input form-textarea"
              value={form.ticket_footer}
              onChange={set('ticket_footer')}
              placeholder="Ej: ¡Gracias por su compra!"
              rows={2}
            />
          </div>
        </section>

        <div className="form-actions">
          <button
            type="button"
            className="btn btn-ghost"
            onClick={handleReset}
            disabled={saving || !dirty}
          >
            Descartar cambios
          </button>
          <button type="submit" className="btn btn-primary" disabled={saving || !dirty}>
            {saving ? 'Guardando…' : 'Guardar ajustes'}
          </button>
        </div>
      </form>

      <ToastContainer toasts={toasts} onDismiss={dismissToast} />
    </div>
  );
};