import React, { useCallback, useEffect, useState } from 'react';
import { ToastContainer, ToastData } from '../../components/ui/Toast';
import { useSettings } from '../../context/AppContext';
import { updateSettings } from '../../services/settings.service';
import type { AppSettings, UpdateSettingsDto } from '../../types';
import { formatMoney } from '../../utils/format';
import { getOperatorOrNull, setOperator } from '../../utils/preferences';
import {
  formatShortcutKey,
  isSupportedShortcut,
  NAVIGATION_SHORTCUTS,
  readNavigationShortcuts,
  saveNavigationShortcuts,
  type NavigationPath,
  type NavigationShortcuts,
} from '../../utils/shortcuts';
import { APP_THEMES, applyTheme, loadSavedTheme, type AppTheme } from '../../utils/theme';
import './Settings.css';

type Section = 'business' | 'money' | 'regional' | 'inventory' | 'ticket' | 'appearance' | 'shortcuts';

const SECTIONS: { id: Section; label: string }[] = [
  { id: 'business', label: 'Negocio' },
  { id: 'money', label: 'Dinero e impuestos' },
  { id: 'regional', label: 'Regional' },
  { id: 'inventory', label: 'Inventario' },
  { id: 'ticket', label: 'Ticket' },
  { id: 'appearance', label: 'Temas' },
  { id: 'shortcuts', label: 'Atajos' },
];

const TICKET_TOGGLES = [
  ['show_logo', 'Mostrar logo'],
  ['show_tax_id', 'Mostrar NIT / RUT'],
  ['show_address', 'Mostrar dirección'],
  ['show_phone', 'Mostrar teléfono'],
  ['show_cashier', 'Mostrar vendedor'],
  ['show_tax_breakdown', 'Mostrar desglose de impuesto'],
  ['show_discounts', 'Mostrar descuentos'],
  ['show_payment_method', 'Mostrar forma de pago'],
] as const;

type ToggleKey = (typeof TICKET_TOGGLES)[number][0];

type FormState = Record<ToggleKey, boolean> & {
  business_name: string;
  tax_id: string;
  address: string;
  phone: string;
  email: string;
  logo_url: string;
  currency: string;
  currency_decimals: string;
  tax_rate_percent: string;
  tax_name: string;
  timezone: string;
  low_stock_default_threshold: string;
  ticket_header: string;
  ticket_footer: string;
};

type TextField = Exclude<keyof FormState, ToggleKey>;

const FIELD_SECTION: Partial<Record<keyof FormState, Section>> = {
  business_name: 'business',
  email: 'business',
  currency: 'money',
  currency_decimals: 'money',
  tax_rate_percent: 'money',
  tax_name: 'money',
  timezone: 'regional',
  low_stock_default_threshold: 'inventory',
};

const COMMON_TIMEZONES = [
  'America/Bogota',
  'America/Lima',
  'America/Guayaquil',
  'America/Caracas',
  'America/Mexico_City',
  'America/Santiago',
  'America/Argentina/Buenos_Aires',
  'America/Asuncion',
  'America/La_Paz',
  'America/Montevideo',
  'America/Sao_Paulo',
  'America/New_York',
  'America/Chicago',
  'America/Denver',
  'America/Los_Angeles',
  'America/Toronto',
  'America/Vancouver',
  'America/Panama',
  'America/Costa_Rica',
  'America/El_Salvador',
  'America/Guatemala',
  'America/Havana',
  'America/Puerto_Rico',
  'America/Santo_Domingo',
  'America/Tijuana',
  'America/Anchorage',
  'America/Phoenix',
  'America/Adak',
  'America/St_Johns',
  'Atlantic/Bermuda',
  'Atlantic/Azores',
  'Atlantic/Reykjavik',
  'Europe/London',
  'Europe/Paris',
  'Europe/Berlin',
  'Europe/Rome',
  'Europe/Amsterdam',
  'Europe/Brussels',
  'Europe/Lisbon',
  'Europe/Madrid',
  'Europe/Zurich',
  'Europe/Stockholm',
  'Europe/Athens',
  'Europe/Istanbul',
  'Europe/Moscow',
  'Africa/Cairo',
  'Africa/Johannesburg',
  'Africa/Lagos',
  'Africa/Nairobi',
  'Asia/Dubai',
  'Asia/Jerusalem',
  'Asia/Kolkata',
  'Asia/Bangkok',
  'Asia/Singapore',
  'Asia/Hong_Kong',
  'Asia/Shanghai',
  'Asia/Seoul',
  'Asia/Tokyo',
  'Asia/Manila',
  'Asia/Jakarta',
  'Asia/Karachi',
  'Asia/Dhaka',
  'Asia/Kathmandu',
  'Asia/Almaty',
  'Asia/Tbilisi',
  'Asia/Tehran',
  'Australia/Perth',
  'Australia/Adelaide',
  'Australia/Sydney',
  'Australia/Melbourne',
  'Australia/Brisbane',
  'Pacific/Auckland',
  'Pacific/Fiji',
  'Pacific/Honolulu',
  'UTC',
];

type IntlWithTimezoneSupport = typeof Intl & {
  supportedValuesOf?: (key: 'timeZone') => string[];
};

const AVAILABLE_TIMEZONES = [...new Set([
  ...((Intl as IntlWithTimezoneSupport).supportedValuesOf?.('timeZone') ?? []),
  ...COMMON_TIMEZONES,
])].sort((left, right) => left.localeCompare(right));

const CURRENCIES = [
  ['COP', 'Peso colombiano'],
  ['USD', 'Dólar estadounidense'],
  ['EUR', 'Euro'],
  ['GBP', 'Libra esterlina'],
  ['MXN', 'Peso mexicano'],
  ['PEN', 'Sol peruano'],
  ['CLP', 'Peso chileno'],
  ['ARS', 'Peso argentino'],
  ['BRL', 'Real brasileño'],
  ['CAD', 'Dólar canadiense'],
  ['AUD', 'Dólar australiano'],
  ['JPY', 'Yen japonés'],
  ['CNY', 'Yuan chino'],
  ['KRW', 'Won surcoreano'],
  ['CHF', 'Franco suizo'],
  ['INR', 'Rupia india'],
  ['UYU', 'Peso uruguayo'],
  ['BOB', 'Boliviano'],
  ['CRC', 'Colón costarricense'],
  ['DOP', 'Peso dominicano'],
  ['GTQ', 'Quetzal guatemalteco'],
  ['PAB', 'Balboa panameño'],
  ['PYG', 'Guaraní paraguayo'],
  ['VES', 'Bolívar venezolano'],
  ['AED', 'Dírham de los Emiratos Árabes Unidos'],
  ['NZD', 'Dólar neozelandés'],
  ['SEK', 'Corona sueca'],
  ['NOK', 'Corona noruega'],
  ['DKK', 'Corona danesa'],
  ['ZAR', 'Rand sudafricano'],
] as const;

const ZERO_DECIMAL_CURRENCIES = new Set(['COP', 'CLP', 'JPY', 'KRW', 'PYG']);

const toForm = (settings: AppSettings): FormState => ({
  business_name: settings.business_name,
  tax_id: settings.tax_id ?? '',
  address: settings.address ?? '',
  phone: settings.phone ?? '',
  email: settings.email ?? '',
  logo_url: settings.logo_url ?? '',
  currency: settings.currency,
  currency_decimals: String(settings.currency_decimals),
  tax_rate_percent: (settings.tax_rate_bps / 100).toString(),
  tax_name: settings.tax_name,
  timezone: settings.timezone,
  low_stock_default_threshold: settings.low_stock_default_threshold.toString(),
  ticket_header: settings.ticket_header ?? '',
  ticket_footer: settings.ticket_footer ?? '',
  show_logo: settings.show_logo,
  show_tax_id: settings.show_tax_id,
  show_address: settings.show_address,
  show_phone: settings.show_phone,
  show_cashier: settings.show_cashier,
  show_tax_breakdown: settings.show_tax_breakdown,
  show_discounts: settings.show_discounts,
  show_payment_method: settings.show_payment_method,
});

let pendingToast: { message: string; type: ToastData['type'] } | null = null;
let rememberedSection: Section = 'business';

export const Settings: React.FC = () => {
  const { settings, applySettings } = useSettings();
  const [section, setSection] = useState<Section>(rememberedSection);
  const [theme, setTheme] = useState<AppTheme>(() => loadSavedTheme());
  const [shortcuts, setShortcuts] = useState<NavigationShortcuts>({});
  const [capturingShortcut, setCapturingShortcut] = useState<NavigationPath | null>(null);
  const [form, setForm] = useState<FormState>(() => toForm(settings));
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [saving, setSaving] = useState(false);
  const [dirty, setDirty] = useState(false);
  const [toasts, setToasts] = useState<ToastData[]>([]);
  const [savedOperator, setSavedOperator] = useState(() => getOperatorOrNull() ?? '');
  const [operator, setOperatorInput] = useState(savedOperator);

  const addToast = useCallback((message: string, type: ToastData['type']) => {
    setToasts(current => [
      ...current,
      { id: Math.random().toString(36).slice(2), message, type },
    ]);
  }, []);

  const dismissToast = useCallback((id: string) => {
    setToasts(current => current.filter(toast => toast.id !== id));
  }, []);

  const operatorDirty = operator.trim() !== savedOperator;

  const saveOperator = () => {
    const next = operator.trim();
    try {
      setOperator(next);
      setSavedOperator(next);
      setOperatorInput(next);
      addToast(next ? 'Operador guardado en este equipo' : 'Operador borrado', 'success');
    } catch (error) {
      addToast(
        error instanceof Error
          ? `No se pudo guardar el operador: ${error.message}`
          : 'No se pudo guardar el operador',
        'error',
      );
    }
  };

  useEffect(() => {
    if (!pendingToast) return;
    addToast(pendingToast.message, pendingToast.type);
    pendingToast = null;
  }, [addToast]);

  useEffect(() => {
    try {
      setShortcuts(readNavigationShortcuts());
    } catch (error) {
      addToast(
        error instanceof Error ? error.message : 'No se pudieron cargar los atajos guardados',
        'error',
      );
    }
  }, [addToast]);

  const goTo = (nextSection: Section) => {
    rememberedSection = nextSection;
    setSection(nextSection);
  };

  const set = (field: TextField) => (
    event: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement>,
  ) => {
    setForm(current => ({ ...current, [field]: event.target.value }));
    setErrors(current => ({ ...current, [field]: '' }));
    setDirty(true);
  };

  const setToggle = (key: ToggleKey) => (event: React.ChangeEvent<HTMLInputElement>) => {
    setForm(current => ({ ...current, [key]: event.target.checked }));
    setDirty(true);
  };

  const setCurrency = (event: React.ChangeEvent<HTMLSelectElement>) => {
    const currency = event.target.value;
    setForm(current => ({
      ...current,
      currency,
      currency_decimals: String(ZERO_DECIMAL_CURRENCIES.has(currency) ? 0 : 2),
    }));
    setErrors(current => ({ ...current, currency: '', currency_decimals: '' }));
    setDirty(true);
  };

  const selectTheme = (nextTheme: AppTheme) => {
    try {
      applyTheme(nextTheme);
      setTheme(nextTheme);
    } catch (error) {
      addToast(
        error instanceof Error ? `No se pudo guardar el tema: ${error.message}` : 'No se pudo guardar el tema',
        'error',
      );
    }
  };

  const updateShortcut = (path: NavigationPath, key: string | null) => {
    const normalized = key?.toUpperCase() ?? null;
    if (normalized && !isSupportedShortcut(normalized)) {
      addToast('Usa una tecla A-Z, 0-9 o una tecla de función F1-F12.', 'warning');
      return;
    }
    if (normalized && Object.entries(shortcuts).some(
      ([assignedPath, assignedKey]) => assignedPath !== path && assignedKey === normalized,
    )) {
      addToast(`La tecla ${normalized} ya está asignada a otra sección.`, 'warning');
      return;
    }

    const nextShortcuts = { ...shortcuts };
    if (normalized) {
      nextShortcuts[path] = normalized;
    } else {
      delete nextShortcuts[path];
    }

    try {
      saveNavigationShortcuts(nextShortcuts);
      setShortcuts(nextShortcuts);
    } catch (error) {
      addToast(
        error instanceof Error
          ? `No se pudo guardar el atajo: ${error.message}`
          : 'No se pudo guardar el atajo',
        'error',
      );
    }
  };

  const validate = (): boolean => {
    const nextErrors: Record<string, string> = {};

    if (!form.business_name.trim()) {
      nextErrors.business_name = 'El nombre del negocio es obligatorio';
    }
    if (form.email.trim() && !/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(form.email.trim())) {
      nextErrors.email = 'Correo inválido';
    }
    if (!/^[A-Za-z]{3}$/.test(form.currency.trim())) {
      nextErrors.currency = 'Usa un código de 3 letras (ej: COP, USD)';
    }
    if (form.currency_decimals !== '0' && form.currency_decimals !== '2') {
      nextErrors.currency_decimals = 'Elige 0 o 2 decimales';
    }

    const taxRate = Number(form.tax_rate_percent);
    if (
      form.tax_rate_percent.trim() === '' ||
      Number.isNaN(taxRate) ||
      taxRate < 0 ||
      taxRate > 100
    ) {
      nextErrors.tax_rate_percent = 'El impuesto debe estar entre 0 y 100';
    }

    const taxName = form.tax_name.trim();
    if (!taxName || taxName.length > 30) {
      nextErrors.tax_name = 'Entre 1 y 30 caracteres';
    }
    if (!form.timezone.trim()) {
      nextErrors.timezone = 'La zona horaria es obligatoria';
    }

    const threshold = Number(form.low_stock_default_threshold);
    if (
      form.low_stock_default_threshold.trim() === '' ||
      !Number.isInteger(threshold) ||
      threshold < 0
    ) {
      nextErrors.low_stock_default_threshold = 'Debe ser un entero de 0 o mayor';
    }

    setErrors(nextErrors);
    const firstInvalidField = Object.keys(nextErrors).find(
      field => FIELD_SECTION[field as keyof FormState],
    );
    const invalidSection = firstInvalidField
      ? FIELD_SECTION[firstInvalidField as keyof FormState]
      : undefined;
    if (invalidSection && invalidSection !== section) goTo(invalidSection);

    return Object.keys(nextErrors).length === 0;
  };

  const handleSubmit = async (event: React.FormEvent) => {
    event.preventDefault();
    if (!validate()) return;

    const dto: UpdateSettingsDto = {
      business_name: form.business_name.trim(),
      tax_id: form.tax_id.trim() || null,
      address: form.address.trim() || null,
      phone: form.phone.trim() || null,
      email: form.email.trim() || null,
      logo_url: form.logo_url.trim() || null,
      currency: form.currency.trim().toUpperCase(),
      currency_decimals: Number(form.currency_decimals),
      tax_rate_bps: Math.round(Number(form.tax_rate_percent) * 100),
      tax_name: form.tax_name.trim(),
      timezone: form.timezone.trim(),
      low_stock_default_threshold: Number(form.low_stock_default_threshold),
      ticket_header: form.ticket_header.trim() || null,
      ticket_footer: form.ticket_footer.trim() || null,
      show_logo: form.show_logo,
      show_tax_id: form.show_tax_id,
      show_address: form.show_address,
      show_phone: form.show_phone,
      show_cashier: form.show_cashier,
      show_tax_breakdown: form.show_tax_breakdown,
      show_discounts: form.show_discounts,
      show_payment_method: form.show_payment_method,
    };

    setSaving(true);
    try {
      const updated = await updateSettings(dto);
      const moneySettingsChanged =
        updated.currency !== settings.currency ||
        updated.currency_decimals !== settings.currency_decimals;

      if (moneySettingsChanged) {
        pendingToast = { message: 'Ajustes guardados correctamente', type: 'success' };
      }
      applySettings(updated);
      setForm(toForm(updated));
      setDirty(false);

      if (moneySettingsChanged) {
        rememberedSection = 'money';
      } else {
        addToast('Ajustes guardados correctamente', 'success');
      }
    } catch (error) {
      pendingToast = null;
      addToast(
        typeof error === 'string'
          ? error
          : error instanceof Error
            ? error.message
            : 'No se pudieron guardar los ajustes',
        'error',
      );
    } finally {
      setSaving(false);
    }
  };

  const handleReset = () => {
    setForm(toForm(settings));
    setErrors({});
    setDirty(false);
  };

  const decimalsChanged = Number(form.currency_decimals) !== settings.currency_decimals;
  const currencyChanged = form.currency.trim().toUpperCase() !== settings.currency;
  const moneyPreview = (() => {
    if (!/^[A-Za-z]{3}$/.test(form.currency.trim())) return null;
    if (form.currency_decimals !== '0' && form.currency_decimals !== '2') return null;
    try {
      return formatMoney(
        2_800_000,
        form.currency.trim().toUpperCase(),
        Number(form.currency_decimals),
      );
    } catch {
      return null;
    }
  })();

  const fieldClass = (name: string) =>
    `form-input ${errors[name] ? 'form-input--error' : ''}`;

  return (
    <div className="settings-page">
      <div className="settings-header">
        <h1 className="settings-title">Ajustes</h1>
        <p className="settings-subtitle">Configuración general del negocio</p>
      </div>

      <div className="settings-tabs" role="tablist" aria-label="Secciones de ajustes">
        {SECTIONS.map(item => (
          <button
            key={item.id}
            type="button"
            role="tab"
            aria-selected={section === item.id}
            className={`settings-tab ${section === item.id ? 'settings-tab--active' : ''}`}
            onClick={() => goTo(item.id)}
          >
            {item.label}
          </button>
        ))}
      </div>

      <form className="settings-form" onSubmit={handleSubmit} noValidate>
        {section === 'business' && (
          <section className="settings-section">
            <h2 className="settings-section-title">Datos del negocio</h2>
            <div className="form-row">
              <div className="form-field">
                <label className="form-label">
                  Nombre del negocio <span className="required">*</span>
                </label>
                <input
                  className={fieldClass('business_name')}
                  type="text"
                  value={form.business_name}
                  onChange={set('business_name')}
                  placeholder="Ej: Detalles y Regalos La 15"
                />
                {errors.business_name && (
                  <span className="form-error">{errors.business_name}</span>
                )}
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
                  className={fieldClass('email')}
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

            <div className="form-field">
              <label className="form-label" htmlFor="settings-operator">Operador</label>
              <div style={{ display: 'flex', gap: 8 }}>
                <input
                  id="settings-operator"
                  className="form-input"
                  type="text"
                  maxLength={100}
                  value={operator}
                  onChange={event => setOperatorInput(event.target.value)}
                  onKeyDown={event => {
                    if (event.key === 'Enter') {
                      event.preventDefault();
                      if (operatorDirty) saveOperator();
                    }
                  }}
                  placeholder="Nombre de quien opera la caja"
                />
                <button
                  type="button"
                  className="btn btn-ghost"
                  onClick={saveOperator}
                  disabled={!operatorDirty}
                >
                  Guardar operador
                </button>
              </div>
              <span className="settings-hint">
                Se guarda solo en este equipo, no en el servidor. Se registra como autor en la
                auditoría. Déjalo vacío para no registrar autor. Cuando exista inicio de sesión,
                este campo dejará de usarse.
              </span>
            </div>
          </section>
        )}

        {section === 'money' && (
          <>
            <section className="settings-section">
              <h2 className="settings-section-title">Moneda</h2>
              <div className="form-row">
                <div className="form-field">
                  <label className="form-label">Moneda</label>
                  <select
                    className={fieldClass('currency')}
                    value={form.currency}
                    onChange={setCurrency}
                  >
                    {CURRENCIES.map(([code, name]) => (
                      <option key={code} value={code}>{code} — {name}</option>
                    ))}
                    {!CURRENCIES.some(([code]) => code === form.currency) && (
                      <option value={form.currency}>{form.currency} — moneda actual</option>
                    )}
                  </select>
                  {errors.currency && <span className="form-error">{errors.currency}</span>}
                </div>
                <div className="form-field">
                  <label className="form-label">Decimales</label>
                  <select
                    className={fieldClass('currency_decimals')}
                    value={form.currency_decimals}
                    onChange={set('currency_decimals')}
                  >
                    <option value="0">0 (ej: $ 28.000)</option>
                    <option value="2">2 (ej: $ 28.000,00)</option>
                  </select>
                  {errors.currency_decimals && (
                    <span className="form-error">{errors.currency_decimals}</span>
                  )}
                </div>
              </div>

              {moneyPreview && (
                <span className="settings-hint">Vista previa: {moneyPreview}</span>
              )}

              {(decimalsChanged || currencyChanged) && (
                <div className="settings-notice">
                  Cambiar la moneda o los decimales no modifica los importes guardados, solo cómo
                  se muestran. Al guardar, la aplicación se recarga para actualizar todas las
                  pantallas.
                  {Number(form.currency_decimals) === 0 &&
                    ' Con 0 decimales, un precio que tenga centavos se verá redondeado.'}
                </div>
              )}
            </section>

            <section className="settings-section">
              <h2 className="settings-section-title">Impuesto</h2>
              <div className="form-row">
                <div className="form-field">
                  <label className="form-label">Nombre del impuesto</label>
                  <input
                    className={fieldClass('tax_name')}
                    type="text"
                    maxLength={30}
                    value={form.tax_name}
                    onChange={set('tax_name')}
                    placeholder="IVA"
                  />
                  {errors.tax_name && <span className="form-error">{errors.tax_name}</span>}
                </div>
                <div className="form-field">
                  <label className="form-label">Tasa (%)</label>
                  <input
                    className={fieldClass('tax_rate_percent')}
                    type="number"
                    min="0"
                    max="100"
                    step="0.01"
                    value={form.tax_rate_percent}
                    onChange={set('tax_rate_percent')}
                    placeholder="19"
                  />
                  {errors.tax_rate_percent && (
                    <span className="form-error">{errors.tax_rate_percent}</span>
                  )}
                </div>
              </div>
              <span className="settings-hint">
                Los precios de los productos ya incluyen el impuesto: la tasa se usa para calcular
                cuánto del total corresponde a impuesto, sin cambiar lo que paga el cliente.
              </span>
            </section>
          </>
        )}

        {section === 'regional' && (
          <section className="settings-section">
            <h2 className="settings-section-title">Zona horaria</h2>
            <div className="form-field">
              <label className="form-label">Zona horaria</label>
              <select
                className={fieldClass('timezone')}
                value={form.timezone}
                onChange={set('timezone')}
              >
                {!AVAILABLE_TIMEZONES.includes(form.timezone) && (
                  <option value={form.timezone}>{form.timezone} — actual</option>
                )}
                {AVAILABLE_TIMEZONES.map(timezone => (
                  <option key={timezone} value={timezone}>{timezone}</option>
                ))}
              </select>
              {errors.timezone && <span className="form-error">{errors.timezone}</span>}
            </div>
            <span className="settings-hint">
              Elige una de la lista o escribe otra en formato Región/Ciudad. Se valida al guardar.
              Define cuándo empieza y termina el día para cortes de caja y reportes.
            </span>
          </section>
        )}

        {section === 'inventory' && (
          <section className="settings-section">
            <h2 className="settings-section-title">Inventario</h2>
            <div className="form-field">
              <label className="form-label">Umbral de stock bajo</label>
              <input
                className={fieldClass('low_stock_default_threshold')}
                type="number"
                min="0"
                step="1"
                value={form.low_stock_default_threshold}
                onChange={set('low_stock_default_threshold')}
                placeholder="5"
              />
              {errors.low_stock_default_threshold && (
                <span className="form-error">{errors.low_stock_default_threshold}</span>
              )}
            </div>
          </section>
        )}

        {section === 'ticket' && (
          <>
            <section className="settings-section">
              <h2 className="settings-section-title">Textos del ticket</h2>
              <div className="form-field">
                <label className="form-label">Encabezado</label>
                <textarea
                  className="form-input form-textarea"
                  rows={2}
                  value={form.ticket_header}
                  onChange={set('ticket_header')}
                  placeholder="Texto que aparece arriba del ticket impreso"
                />
              </div>
              <div className="form-field">
                <label className="form-label">Pie</label>
                <textarea
                  className="form-input form-textarea"
                  rows={2}
                  value={form.ticket_footer}
                  onChange={set('ticket_footer')}
                  placeholder="Ej: ¡Gracias por su compra!"
                />
              </div>
            </section>

            <section className="settings-section">
              <h2 className="settings-section-title">Qué se imprime</h2>
              <div className="settings-switch-list">
                {TICKET_TOGGLES.map(([key, label]) => (
                  <label className="settings-switch" key={key}>
                    <span>{label}</span>
                    <input
                      type="checkbox"
                      checked={form[key]}
                      onChange={setToggle(key)}
                    />
                    <span className="settings-switch-track" />
                  </label>
                ))}
              </div>
              <span className="settings-hint">
                Estas opciones aplican al próximo ticket impreso.
              </span>
            </section>
          </>
        )}

        {section === 'appearance' && (
          <section className="settings-section">
            <h2 className="settings-section-title">Personalización de colores</h2>
            <p className="settings-hint">
              El tema se guarda en este equipo y se aplica inmediatamente. No modifica los datos
              del negocio ni afecta a otras instalaciones.
            </p>
            <div className="settings-theme-grid" role="radiogroup" aria-label="Tema de la aplicación">
              {APP_THEMES.map(item => (
                <button
                  key={item.id}
                  type="button"
                  role="radio"
                  aria-checked={theme === item.id}
                  className={`settings-theme-option ${theme === item.id ? 'settings-theme-option--active' : ''}`}
                  onClick={() => selectTheme(item.id)}
                >
                  <span
                    className={`settings-theme-preview ${item.dark ? 'settings-theme-preview--dark' : ''}`}
                    style={{ '--theme-accent': item.color } as React.CSSProperties}
                    aria-hidden="true"
                  >
                    <span />
                    <span />
                    <span />
                  </span>
                  <span className="settings-theme-name">{item.name}</span>
                  {theme === item.id && <span className="settings-theme-selected">Seleccionado</span>}
                </button>
              ))}
            </div>
          </section>
        )}

        {section === 'shortcuts' && (
          <section className="settings-section">
            <h2 className="settings-section-title">Atajos del teclado</h2>
            <p className="settings-hint">
              Asigna una tecla para abrir cada sección desde cualquier pantalla. Haz clic en
              “Asignar tecla” y luego presiona una letra, un número o F1-F12. No se activa
              mientras escribes en un campo de texto.
            </p>
            <div className="settings-shortcuts-list">
              {NAVIGATION_SHORTCUTS.map(item => (
                <div className="settings-shortcut-row" key={item.path}>
                  <span className="settings-shortcut-label">{item.label}</span>
                  <div className="settings-shortcut-actions">
                    <button
                      type="button"
                      className={`settings-shortcut-key ${capturingShortcut === item.path ? 'settings-shortcut-key--capturing' : ''}`}
                      onClick={() => setCapturingShortcut(item.path)}
                      onKeyDown={event => {
                        if (capturingShortcut !== item.path) return;
                        event.preventDefault();
                        event.stopPropagation();
                        if (event.key === 'Escape') {
                          setCapturingShortcut(null);
                          return;
                        }
                        if (event.key === 'Backspace' || event.key === 'Delete') {
                          updateShortcut(item.path, null);
                          setCapturingShortcut(null);
                          return;
                        }
                        if (event.altKey || event.ctrlKey || event.metaKey || event.shiftKey) return;
                        const key = event.key.toUpperCase();
                        if (isSupportedShortcut(key)) {
                          updateShortcut(item.path, key);
                          setCapturingShortcut(null);
                        }
                      }}
                    >
                      {capturingShortcut === item.path
                        ? 'Presiona una tecla…'
                        : formatShortcutKey(shortcuts[item.path])}
                    </button>
                    {shortcuts[item.path] && (
                      <button
                        type="button"
                        className="settings-shortcut-clear"
                        onClick={() => updateShortcut(item.path, null)}
                        aria-label={`Quitar atajo de ${item.label}`}
                      >
                        Quitar
                      </button>
                    )}
                  </div>
                </div>
              ))}
            </div>
          </section>
        )}

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
