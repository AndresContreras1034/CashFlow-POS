import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useSearchParams } from 'react-router-dom';
import { AuditDetail } from '../../components/audit/AuditDetail';
import { AuditFilters } from '../../components/audit/AuditFilters';
import { AuditTable } from '../../components/audit/AuditTable';
import { useSettings } from '../../context/AppContext';
import { listAuditEvents } from '../../services/audit.service';
import type { AuditEvent, PaginatedResponse } from '../../types';
import {
  buildAuditFilter,
  EMPTY_FILTER,
  errorMessage,
  filterToSearchParams,
  hasActiveFilters,
  searchParamsToFilter,
  validateDateRange,
  type AuditFilterState,
} from '../../utils/audit';
import './Audit.css';

export const Audit: React.FC = () => {
  const { settings } = useSettings();
  const [searchParams, setSearchParams] = useSearchParams();

  const queryKey = searchParams.toString();
  const filter = useMemo(() => searchParamsToFilter(new URLSearchParams(queryKey)), [queryKey]);
  const dateError = useMemo(
    () => validateDateRange(filter.dateFrom, filter.dateTo),
    [filter.dateFrom, filter.dateTo],
  );

  const [result, setResult] = useState<PaginatedResponse<AuditEvent> | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [reloadToken, setReloadToken] = useState(0);
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const requestRef = useRef(0);
  const closeDetail = useCallback(() => setSelectedId(null), []);

  /** Cambia filtros en la URL. Salvo que el patch traiga `page`, vuelve a la página 1. */
  const updateFilter = useCallback(
    (patch: Partial<AuditFilterState>, replace = true) => {
      setSearchParams(
        previous => {
          const current = searchParamsToFilter(previous);
          return filterToSearchParams({ ...current, ...patch, page: patch.page ?? 1 });
        },
        { replace },
      );
    },
    [setSearchParams],
  );

  const resetFilters = useCallback(() => {
    setSearchParams(
      previous => {
        const current = searchParamsToFilter(previous);
        return filterToSearchParams({ ...EMPTY_FILTER, sortDir: current.sortDir });
      },
      { replace: true },
    );
  }, [setSearchParams]);

  useEffect(() => {
    if (dateError) {
      requestRef.current += 1;
      setLoading(false);
      setError(null);
      setResult(null);
      return;
    }

    const requestId = ++requestRef.current;
    const load = async () => {
      setLoading(true);
      setError(null);
      try {
        const response = await listAuditEvents(buildAuditFilter(filter));
        if (requestId !== requestRef.current) return;

        if (
          response.data.length === 0 &&
          response.page > 1 &&
          response.total_pages > 0 &&
          response.page > response.total_pages
        ) {
          updateFilter({ page: response.total_pages });
          return;
        }
        setResult(response);
        setLoading(false);
      } catch (cause) {
        if (requestId !== requestRef.current) return;
        setResult(null);
        setError(errorMessage(cause));
        setLoading(false);
      }
    };
    void load();

    return () => {
      requestRef.current += 1;
    };
  }, [filter, dateError, reloadToken, updateFilter]);

  const events = result?.data ?? [];
  const totalPages = Math.max(result?.total_pages ?? 0, 1);
  const goToPage = (page: number) => updateFilter({ page }, false);

  let status = '';
  if (loading) status = 'Cargando…';
  else if (error) status = 'No se pudieron cargar los eventos';
  else if (result) {
    status = `${result.total.toLocaleString('es-CO')} evento${result.total !== 1 ? 's' : ''}`;
  }

  return (
    <div className="audit-page">
      <header className="audit-header">
        <h1 className="audit-title">Auditoría</h1>
        <p className="audit-subtitle">Registro de solo lectura de las operaciones del sistema</p>
      </header>

      <AuditFilters
        filter={filter}
        dateError={dateError}
        hasFilters={hasActiveFilters(filter)}
        onChange={updateFilter}
        onReset={resetFilters}
      />

      <p className="audit-status" role="status" aria-live="polite">{status}</p>

      {error && (
        <div className="audit-error-block" role="alert">
          <p>{error}</p>
          <button
            type="button"
            className="btn btn-primary"
            onClick={() => setReloadToken(token => token + 1)}
          >
            Reintentar
          </button>
        </div>
      )}

      {!error && !dateError && !loading && events.length === 0 && (
        <div className="audit-empty">
          {hasActiveFilters(filter)
            ? 'Ningún evento coincide con los filtros.'
            : 'Aún no hay eventos registrados.'}
        </div>
      )}

      {events.length > 0 && (
        <div className={loading ? 'audit-results audit-results--loading' : 'audit-results'}>
          <AuditTable
            events={events}
            timeZone={settings.timezone}
            onSelect={event => setSelectedId(event.id)}
          />
        </div>
      )}

      <nav className="audit-pagination" aria-label="Paginación">
        <button
          type="button"
          className="btn btn-ghost"
          disabled={loading || filter.page <= 1}
          onClick={() => goToPage(filter.page - 1)}
        >
          ← Anterior
        </button>
        <span>Página {filter.page} de {totalPages}</span>
        <button
          type="button"
          className="btn btn-ghost"
          disabled={loading || !result || filter.page >= totalPages}
          onClick={() => goToPage(filter.page + 1)}
        >
          Siguiente →
        </button>
      </nav>

      {selectedId !== null && (
        <AuditDetail eventId={selectedId} timeZone={settings.timezone} onClose={closeDetail} />
      )}
    </div>
  );
};