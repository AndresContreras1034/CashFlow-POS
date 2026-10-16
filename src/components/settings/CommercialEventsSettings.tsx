import React, { useCallback, useEffect, useState } from 'react';
import {
  createCommercialEvent,
  deleteCommercialEvent,
  listCommercialEvents,
  updateCommercialEvent,
} from '../../services/commercialCalendar.service';
import type {
  CommercialEvent,
  CommercialEventInput,
  CommercialEventKind,
  CommercialEventRule,
} from '../../types/commercialCalendar';
import { getOperatorOrNull } from '../../utils/preferences';

interface Props {
  onToast: (message: string, type: 'success' | 'error' | 'warning' | 'info') => void;
}

interface EventForm {
  name: string;
  kind: CommercialEventKind;
  rule: CommercialEventRule;
  event_date: string;
  month: string;
  day: string;
  weekday: string;
  nth: string;
  prep_days: string;
  is_active: boolean;
}

const freshForm = (): EventForm => ({
  name: '',
  kind: 'special',
  rule: 'once',
  event_date: '',
  month: '',
  day: '',
  weekday: '',
  nth: '',
  prep_days: '7',
  is_active: true,
});

const toForm = (event: CommercialEvent): EventForm => ({
  name: event.name,
  kind: event.kind,
  rule: event.rule,
  event_date: event.event_date ?? '',
  month: event.month?.toString() ?? '',
  day: event.day?.toString() ?? '',
  weekday: event.weekday?.toString() ?? '',
  nth: event.nth?.toString() ?? '',
  prep_days: event.prep_days.toString(),
  is_active: event.is_active,
});

const parseOptionalNumber = (value: string): number | null =>
  value === '' ? null : Number(value);

function toInput(form: EventForm): CommercialEventInput {
  return {
    name: form.name,
    kind: form.kind,
    rule: form.rule,
    event_date: form.rule === 'once' ? form.event_date || null : null,
    month: ['yearly', 'nth_weekday'].includes(form.rule)
      ? parseOptionalNumber(form.month)
      : null,
    day: ['yearly', 'monthly'].includes(form.rule)
      ? parseOptionalNumber(form.day)
      : null,
    weekday: form.rule === 'nth_weekday' ? parseOptionalNumber(form.weekday) : null,
    nth: form.rule === 'nth_weekday' ? parseOptionalNumber(form.nth) : null,
    prep_days: Number(form.prep_days),
    is_active: form.is_active,
    actor: getOperatorOrNull(),
  };
}

const errorMessage = (error: unknown) =>
  typeof error === 'string'
    ? error
    : error instanceof Error
      ? error.message
      : 'Ocurrió un error inesperado';

export const CommercialEventsSettings: React.FC<Props> = ({ onToast }) => {
  const [events, setEvents] = useState<CommercialEvent[]>([]);
  const [form, setForm] = useState<EventForm>(freshForm);
  const [editingId, setEditingId] = useState<number | null>(null);
  const [deletingId, setDeletingId] = useState<number | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);

  const refresh = useCallback(async () => {
    try {
      setEvents(await listCommercialEvents());
    } catch (error) {
      onToast(`No se pudieron cargar los eventos: ${errorMessage(error)}`, 'error');
    } finally {
      setLoading(false);
    }
  }, [onToast]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const setField = <K extends keyof EventForm>(key: K, value: EventForm[K]) => {
    setForm(current => ({ ...current, [key]: value }));
  };

  const cancelEdit = () => {
    setEditingId(null);
    setForm(freshForm());
  };

  const saveEvent = async () => {
    setSaving(true);
    try {
      const dto = toInput(form);
      if (editingId === null) {
        await createCommercialEvent(dto);
        onToast('Evento comercial creado', 'success');
      } else {
        await updateCommercialEvent(editingId, dto);
        onToast('Evento comercial actualizado', 'success');
      }
      cancelEdit();
      await refresh();
    } catch (error) {
      onToast(`No se pudo guardar el evento: ${errorMessage(error)}`, 'error');
    } finally {
      setSaving(false);
    }
  };

  const removeEvent = async (id: number) => {
    try {
      await deleteCommercialEvent(id, getOperatorOrNull());
      setDeletingId(null);
      onToast('Evento comercial eliminado', 'success');
      await refresh();
    } catch (error) {
      onToast(`No se pudo eliminar el evento: ${errorMessage(error)}`, 'error');
    }
  };

  return (
    <section className="settings-section">
      <h2 className="settings-section-title">Calendario comercial</h2>
      <p>Registra quincenas, festivos y fechas especiales para excluirlas de las comparaciones normales.</p>

      <div className="form-row">
        <div className="form-field">
          <label className="form-label" htmlFor="commercial-event-name">Nombre</label>
          <input
            id="commercial-event-name"
            className="form-input"
            value={form.name}
            maxLength={100}
            onChange={event => setField('name', event.target.value)}
          />
        </div>
        <div className="form-field">
          <label className="form-label" htmlFor="commercial-event-kind">Tipo</label>
          <select
            id="commercial-event-kind"
            className="form-input"
            value={form.kind}
            onChange={event => setField('kind', event.target.value as CommercialEventKind)}
          >
            <option value="payday">Quincena / pago</option>
            <option value="bonus">Prima / bono</option>
            <option value="holiday">Festivo</option>
            <option value="special">Especial</option>
          </select>
        </div>
        <div className="form-field">
          <label className="form-label" htmlFor="commercial-event-rule">Repetición</label>
          <select
            id="commercial-event-rule"
            className="form-input"
            value={form.rule}
            onChange={event => setField('rule', event.target.value as CommercialEventRule)}
          >
            <option value="once">Una vez</option>
            <option value="yearly">Cada año</option>
            <option value="monthly">Cada mes</option>
            <option value="nth_weekday">Día de semana del mes</option>
          </select>
        </div>
      </div>

      {form.rule === 'once' && (
        <div className="form-field">
          <label className="form-label" htmlFor="commercial-event-date">Fecha</label>
          <input
            id="commercial-event-date"
            className="form-input"
            type="date"
            value={form.event_date}
            onChange={event => setField('event_date', event.target.value)}
          />
        </div>
      )}

      {['yearly', 'nth_weekday'].includes(form.rule) && (
        <div className="form-field">
          <label className="form-label" htmlFor="commercial-event-month">Mes</label>
          <select
            id="commercial-event-month"
            className="form-input"
            value={form.month}
            onChange={event => setField('month', event.target.value)}
          >
            <option value="">Selecciona un mes</option>
            {['Enero', 'Febrero', 'Marzo', 'Abril', 'Mayo', 'Junio', 'Julio', 'Agosto', 'Septiembre', 'Octubre', 'Noviembre', 'Diciembre'].map((month, index) => (
              <option key={month} value={index + 1}>{month}</option>
            ))}
          </select>
        </div>
      )}

      {['yearly', 'monthly'].includes(form.rule) && (
        <div className="form-field">
          <label className="form-label" htmlFor="commercial-event-day">Día del mes</label>
          <input
            id="commercial-event-day"
            className="form-input"
            type="number"
            min={-1}
            max={31}
            value={form.day}
            onChange={event => setField('day', event.target.value)}
          />
          <span className="form-hint">Usa -1 para el último día del mes.</span>
        </div>
      )}

      {form.rule === 'nth_weekday' && (
        <div className="form-row">
          <div className="form-field">
            <label className="form-label" htmlFor="commercial-event-weekday">Día de semana</label>
            <select
              id="commercial-event-weekday"
              className="form-input"
              value={form.weekday}
              onChange={event => setField('weekday', event.target.value)}
            >
              <option value="">Selecciona un día</option>
              {['Lunes', 'Martes', 'Miércoles', 'Jueves', 'Viernes', 'Sábado', 'Domingo'].map((weekday, index) => (
                <option key={weekday} value={index}>{weekday}</option>
              ))}
            </select>
          </div>
          <div className="form-field">
            <label className="form-label" htmlFor="commercial-event-nth">Ocurrencia</label>
            <select
              id="commercial-event-nth"
              className="form-input"
              value={form.nth}
              onChange={event => setField('nth', event.target.value)}
            >
              <option value="">Selecciona</option>
              <option value="1">Primero</option>
              <option value="2">Segundo</option>
              <option value="3">Tercero</option>
              <option value="4">Cuarto</option>
              <option value="-1">Último</option>
            </select>
          </div>
        </div>
      )}

      <div className="form-row">
        <div className="form-field">
          <label className="form-label" htmlFor="commercial-event-prep">Días de preparación</label>
          <input
            id="commercial-event-prep"
            className="form-input"
            type="number"
            min={0}
            max={60}
            value={form.prep_days}
            onChange={event => setField('prep_days', event.target.value)}
          />
        </div>
        <label className="settings-switch">
          <span>Evento activo</span>
          <input
            type="checkbox"
            checked={form.is_active}
            onChange={event => setField('is_active', event.target.checked)}
          />
          <span className="settings-switch-track" />
        </label>
      </div>

      <div className="settings-event-actions">
        <button
          type="button"
          className="btn btn-primary"
          disabled={saving}
          onClick={() => void saveEvent()}
        >
          {saving ? 'Guardando…' : editingId === null ? 'Agregar evento' : 'Guardar cambios'}
        </button>
        {editingId !== null && (
          <button type="button" className="btn" onClick={cancelEdit}>Cancelar</button>
        )}
      </div>

      {loading ? <p>Cargando eventos…</p> : (
        <div className="settings-event-list">
          {events.map(event => (
            <article className="settings-event-row" key={event.id}>
              <div>
                <strong>{event.name}</strong>
                <span>{event.kind} · {event.rule} · preparación {event.prep_days} días{!event.is_active ? ' · inactivo' : ''}</span>
              </div>
              <div className="settings-event-actions">
                {deletingId === event.id ? (
                  <>
                    <span>¿Eliminar?</span>
                    <button type="button" className="btn btn-danger" onClick={() => void removeEvent(event.id)}>Sí</button>
                    <button type="button" className="btn" onClick={() => setDeletingId(null)}>No</button>
                  </>
                ) : (
                  <>
                    <button
                      type="button"
                      className="btn"
                      onClick={() => {
                        setEditingId(event.id);
                        setForm(toForm(event));
                      }}
                    >
                      Editar
                    </button>
                    <button type="button" className="btn btn-danger" onClick={() => setDeletingId(event.id)}>
                      Eliminar
                    </button>
                  </>
                )}
              </div>
            </article>
          ))}
        </div>
      )}
    </section>
  );
};
