import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useSettings } from '../../context/AppContext';
import {
  clearDeveloperEvents,
  getDeveloperEvents,
  getDeveloperStatus,
  getHealth,
} from '../../services/developer.service';
import type { DeveloperEvent, DeveloperStatus, HealthSnapshot } from '../../types/developer';
import './Developer.css';

const REFRESH_MS = 2000;

export const Developer: React.FC = () => {
  const { settings } = useSettings();
  const [status, setStatus] = useState<DeveloperStatus | null>(null);
  const [events, setEvents] = useState<DeveloperEvent[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [health, setHealth] = useState<HealthSnapshot | null>(null);
  const [healthError, setHealthError] = useState<string | null>(null);
  const [healthLoading, setHealthLoading] = useState(false);
  const [copyState, setCopyState] = useState<'idle' | 'ok' | 'fail'>('idle');
  const healthRequest = useRef(0);

  const refresh = useCallback(async () => {
    try {
      const [nextStatus, nextEvents] = await Promise.all([
        getDeveloperStatus(),
        getDeveloperEvents(),
      ]);
      setStatus(nextStatus);
      setEvents(nextEvents);
      setError(null);
    } catch (cause) {
      setError(typeof cause === 'string' ? cause : 'No se pudo leer Developer Mode');
    }
  }, []);

  useEffect(() => {
    void refresh();
    const timer = window.setInterval(() => void refresh(), REFRESH_MS);
    return () => window.clearInterval(timer);
  }, [refresh]);

  const refreshHealth = useCallback(async () => {
    const requestId = ++healthRequest.current;
    setHealthLoading(true);
    setHealthError(null);
    try {
      const next = await getHealth();
      if (requestId !== healthRequest.current) return;
      setHealth(next);
    } catch (cause) {
      if (requestId !== healthRequest.current) return;
      setHealth(null);
      setHealthError(typeof cause === 'string' ? cause : 'No se pudo consultar la salud');
    } finally {
      if (requestId === healthRequest.current) setHealthLoading(false);
    }
  }, []);

  useEffect(() => {
    void refreshHealth();
  }, [refreshHealth]);

  const errorCount = useMemo(
    () => events.filter(event => event.level === 'error').length,
    [events],
  );
  const selected = events.find(event => event.id === selectedId) ?? null;
  const timezoneMismatch = health !== null && health.db_timezone !== settings.timezone;

  const healthSummary = health
    ? [
        `App: v${health.app_version}, ${health.profile}, log: ${health.log_filter}`,
        `Base de datos: ${health.database}, ${health.server_version}, ${health.latency_ms.toFixed(1)} ms`,
        `Pool: ${health.pool_size} abiertas, ${health.pool_idle} libres, max ${health.pool_max}`,
        `Hora del servidor (UTC): ${health.db_time}`,
        `Zona PostgreSQL: ${health.db_timezone} | Zona de Ajustes: ${settings.timezone}`,
        `Errores recientes: ${errorCount}`,
      ].join('\n')
    : '';

  const copySummary = async () => {
    try {
      await navigator.clipboard.writeText(healthSummary);
      setCopyState('ok');
    } catch {
      setCopyState('fail');
    }
    window.setTimeout(() => setCopyState('idle'), 2500);
  };

  const formatTime = (iso: string, withDate = false) => {
    try {
      return new Intl.DateTimeFormat('es-CO', {
        ...(withDate ? { dateStyle: 'medium' as const } : {}),
        timeStyle: 'medium',
        timeZone: settings.timezone,
      }).format(new Date(iso));
    } catch {
      return iso;
    }
  };

  const handleClear = async () => {
    try {
      await clearDeveloperEvents();
      setSelectedId(null);
      await refresh();
    } catch (cause) {
      setError(typeof cause === 'string' ? cause : 'No se pudo limpiar el historial');
    }
  };

  return (
    <div className="developer-page">
      <header className="developer-header">
        <div>
          <h1 className="developer-title">Developer Mode</h1>
          <p className="developer-subtitle">
            Diagnóstico técnico temporal. Solo en memoria; no es la auditoría.
          </p>
        </div>
        <span className={`developer-badge ${status?.enabled ? 'developer-badge--on' : ''}`}>
          {status?.enabled ? 'Activo' : 'Inactivo'}
        </span>
        <button
          type="button"
          className="btn btn-ghost"
          onClick={handleClear}
          disabled={events.length === 0}
        >
          Limpiar
        </button>
      </header>

      {error && <div className="developer-error" role="alert">{error}</div>}

      {status && !status.enabled && (
        <div className="settings-notice">
          El modo desarrollador está apagado. Actívalo en Ajustes → Desarrollador para
          capturar eventos técnicos.
        </div>
      )}

      <section className="developer-health" aria-label="Salud del sistema">
        <div className="developer-health-head">
          <h2 className="developer-panel-title">Salud</h2>
          <div className="developer-health-actions">
            <button
              type="button"
              className="btn btn-primary"
              onClick={() => void refreshHealth()}
              disabled={healthLoading}
            >
              {healthLoading ? 'Actualizando…' : 'Actualizar'}
            </button>
            <button
              type="button"
              className="btn btn-ghost"
              onClick={() => void copySummary()}
              disabled={!health}
            >
              Copiar resumen
            </button>
            <span className="developer-muted" role="status" aria-live="polite">
              {copyState === 'ok' && 'Copiado'}
              {copyState === 'fail' && 'No se pudo copiar'}
            </span>
          </div>
        </div>

        {healthError && (
          <div className="developer-error" role="alert">
            PostgreSQL no responde: {healthError}
          </div>
        )}

        {health && (
          <dl className="developer-detail">
            <dt>PostgreSQL</dt>
            <dd>{health.database}, {health.server_version}, {health.latency_ms.toFixed(1)} ms</dd>
            <dt>Pool</dt>
            <dd>{health.pool_size} abiertas, {health.pool_idle} libres, máx. {health.pool_max}</dd>
            <dt>Hora del servidor</dt>
            <dd>{formatTime(health.db_time, true)} ({settings.timezone})</dd>
            <dt>Zona PostgreSQL</dt>
            <dd>{health.db_timezone}</dd>
            <dt>App</dt>
            <dd>v{health.app_version}, {health.profile}, log: {health.log_filter}</dd>
          </dl>
        )}

        {timezoneMismatch && health && (
          <div className="settings-notice" role="alert">
            La zona de PostgreSQL ({health.db_timezone}) difiere de la de Ajustes
            ({settings.timezone}). Los filtros de fecha de Auditoría pueden desplazarse un día
            en eventos nocturnos.
          </div>
        )}
      </section>

      <section className="developer-summary" aria-label="Resumen">
        <div><strong>{events.length}</strong> eventos</div>
        <div><strong>{errorCount}</strong> errores</div>
        <span className="developer-muted">
          Máximo {status?.capacity ?? 500} eventos; se pierden al cerrar la aplicación.
        </span>
      </section>

      <div className="developer-body">
        <section className="developer-panel">
          <h2 className="developer-panel-title">Timeline</h2>
          {events.length === 0 ? (
            <p className="developer-muted">Sin eventos.</p>
          ) : (
            <ul className="developer-list">
              {events.map(event => (
                <li key={event.id}>
                  <button
                    type="button"
                    className={`developer-row ${event.id === selectedId ? 'developer-row--selected' : ''}`}
                    onClick={() => setSelectedId(event.id)}
                  >
                    <span className="developer-time">{formatTime(event.timestamp)}</span>
                    <span className={`developer-level developer-level--${event.level}`}>
                      {event.level.toUpperCase()}
                    </span>
                    <span className="developer-op">{event.operation}</span>
                    <span className="developer-msg">{event.message}</span>
                  </button>
                </li>
              ))}
            </ul>
          )}
        </section>

        <section className="developer-panel">
          <h2 className="developer-panel-title">Detalle</h2>
          {!selected ? (
            <p className="developer-muted">Selecciona un evento.</p>
          ) : (
            <dl className="developer-detail">
              <dt>Operación</dt>
              <dd>{selected.operation}</dd>
              <dt>Origen</dt>
              <dd>{selected.target}</dd>
              <dt>Fecha</dt>
              <dd>{formatTime(selected.timestamp, true)}</dd>
              <dt>Resultado</dt>
              <dd>{selected.outcome}</dd>
              <dt>Mensaje</dt>
              <dd>{selected.message}</dd>
              {selected.correlation_id && (
                <>
                  <dt>ID de correlación (Auditoría)</dt>
                  <dd className="developer-mono">{selected.correlation_id}</dd>
                </>
              )}
            </dl>
          )}
        </section>
      </div>
    </div>
  );
};