import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useSearchParams } from 'react-router-dom';
import { DebtCategoryStats, DebtSummaryCards } from '../../components/debts/DebtSummaryCards';
import { DebtDetail } from '../../components/debts/DebtDetail';
import { DebtFilters } from '../../components/debts/DebtFilters';
import { DebtForm } from '../../components/debts/DebtForm';
import { DebtList } from '../../components/debts/DebtList';
import { Modal } from '../../components/ui/Modal';
import { ToastContainer, type ToastData, type ToastType } from '../../components/ui/Toast';
import { useSettings } from '../../context/AppContext';
import {
  getDebtCategoryStats,
  getDebtOptions,
  getDebtSummary,
  listDebts,
} from '../../services/debts.service';
import type { PaginatedResponse } from '../../types';
import type {
  DebtCategoryStat,
  DebtOptions,
  DebtOverview,
  DebtSummary,
} from '../../types/debts';
import { errorMessage } from '../../utils/audit';
import {
  DEFAULT_DEBT_PAGE_SIZE,
  EMPTY_DEBT_FILTER,
  buildDebtFilter,
  debtFilterToSearchParams,
  hasActiveDebtFilters,
  pivotCategoryStats,
  searchParamsToDebtFilter,
  toPanelFilter,
  todayInTimeZone,
  validateDebtFilterRanges,
  type DebtFilterState,
  type DueGroup,
} from '../../utils/debts';
import './Debts.css';

type Dialog =
  | { kind: 'none' }
  | { kind: 'create' }
  /** `backToDetail`: al guardar o cancelar se vuelve al detalle de la deuda. */
  | { kind: 'edit'; debt: DebtOverview; backToDetail: boolean }
  | { kind: 'detail'; id: number; startWithPayment: boolean };

export const Debts: React.FC = () => {
  const { settings } = useSettings();
  const [searchParams, setSearchParams] = useSearchParams();
  const today = todayInTimeZone(settings.timezone);

  // ---------------- Catálogo (etiquetas y opciones: única fuente, el backend) ----------------
  const [options, setOptions] = useState<DebtOptions | null>(null);
  const [optionsError, setOptionsError] = useState<string | null>(null);
  const loadOptions = useCallback(() => {
    setOptionsError(null);
    getDebtOptions()
      .then(setOptions)
      .catch(cause => setOptionsError(errorMessage(cause)));
  }, []);
  useEffect(() => {
    loadOptions();
  }, [loadOptions]);

  // ---------------- Filtros (viven en la URL) ----------------
  const queryKey = searchParams.toString();
  const filter = useMemo(() => searchParamsToDebtFilter(new URLSearchParams(queryKey)), [queryKey]);
  const rangeError = useMemo(() => validateDebtFilterRanges(filter), [filter]);

  /** Cambia filtros en la URL. Salvo que el patch traiga `page`, vuelve a la página 1. */
  const updateFilter = useCallback(
    (patch: Partial<DebtFilterState>, replace = true) => {
      setSearchParams(
        previous => {
          const current = searchParamsToDebtFilter(previous);
          return debtFilterToSearchParams({ ...current, ...patch, page: patch.page ?? 1 });
        },
        { replace },
      );
    },
    [setSearchParams],
  );

  const resetFilters = useCallback(() => {
    setSearchParams(
      previous => {
        const current = searchParamsToDebtFilter(previous);
        return debtFilterToSearchParams({
          ...EMPTY_DEBT_FILTER,
          sortBy: current.sortBy,
          sortDir: current.sortDir,
        });
      },
      { replace: true },
    );
  }, [setSearchParams]);

  // ---------------- Listado ----------------
  const [result, setResult] = useState<PaginatedResponse<DebtOverview> | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [reloadToken, setReloadToken] = useState(0);
  const listRequest = useRef(0);
  const refresh = useCallback(() => setReloadToken(token => token + 1), []);

  useEffect(() => {
    if (rangeError) {
      listRequest.current += 1;
      setLoading(false);
      setError(null);
      setResult(null);
      return;
    }

    const requestId = ++listRequest.current;
    setLoading(true);
    setError(null);
    listDebts(buildDebtFilter(filter, DEFAULT_DEBT_PAGE_SIZE))
      .then(response => {
        if (requestId !== listRequest.current) return;
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
      })
      .catch(cause => {
        if (requestId !== listRequest.current) return;
        setResult(null);
        setError(errorMessage(cause));
        setLoading(false);
      });

    return () => {
      listRequest.current += 1;
    };
  }, [filter, rangeError, reloadToken, updateFilter]);

  // ---------------- Panel: totales e indicadores por categoría ----------------
  // Usa el filtro del listado SIN estado ni urgencia, para que las tarjetas
  // sigan mostrando «vencido» y «próximo a vencer» aunque el listado ya esté filtrado.
  const panelKey = useMemo(() => JSON.stringify(toPanelFilter(buildDebtFilter(filter))), [filter]);
  const [summary, setSummary] = useState<DebtSummary | null>(null);
  const [stats, setStats] = useState<DebtCategoryStat[] | null>(null);
  const [panelError, setPanelError] = useState<string | null>(null);
  const panelRequest = useRef(0);

  useEffect(() => {
    if (rangeError) return;
    const requestId = ++panelRequest.current;
    const panelFilter = JSON.parse(panelKey) as ReturnType<typeof toPanelFilter>;
    setPanelError(null);
    Promise.all([getDebtSummary(panelFilter), getDebtCategoryStats(panelFilter)])
      .then(([nextSummary, nextStats]) => {
        if (requestId !== panelRequest.current) return;
        setSummary(nextSummary);
        setStats(nextStats);
      })
      .catch(cause => {
        if (requestId !== panelRequest.current) return;
        setPanelError(errorMessage(cause));
      });
    return () => {
      panelRequest.current += 1;
    };
  }, [panelKey, rangeError, reloadToken]);

  const categoryRows = useMemo(() => (stats ? pivotCategoryStats(stats) : null), [stats]);

  // ---------------- Avisos ----------------
  const [toasts, setToasts] = useState<ToastData[]>([]);
  const dismissToast = useCallback(
    (id: string) => setToasts(current => current.filter(toast => toast.id !== id)),
    [],
  );
  const notify = useCallback((message: string, type: ToastType = 'success') => {
    const id = `${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
    setToasts(current => [...current, { id, message, type }]);
  }, []);

  // ---------------- Diálogos ----------------
  const [dialog, setDialog] = useState<Dialog>({ kind: 'none' });
  const closeDialog = useCallback(() => setDialog({ kind: 'none' }), []);

  const debts = result?.data ?? [];
  const totalPages = Math.max(result?.total_pages ?? 0, 1);
  const filtersActive = hasActiveDebtFilters(filter);
  const goToPage = (page: number) => updateFilter({ page }, false);

  let status = '';
  if (loading) status = 'Cargando…';
  else if (error) status = 'No se pudieron cargar las deudas';
  else if (result) {
    status = `${result.total.toLocaleString('es-CO')} deuda${result.total !== 1 ? 's' : ''}`;
  }

  return (
    <div className="debts-page">
      <header className="debts-header">
        <div>
          <h1 className="debts-title">Deudas</h1>
          <p className="debts-subtitle">Cuentas por pagar del negocio</p>
        </div>
        <button
          type="button"
          className="btn btn-primary"
          disabled={!options}
          onClick={() => setDialog({ kind: 'create' })}
        >
          Nueva deuda
        </button>
      </header>

      {optionsError && (
        <div className="debts-error-block" role="alert">
          <p>No se pudo cargar el catálogo de deudas: {optionsError}</p>
          <button type="button" className="btn btn-primary" onClick={loadOptions}>
            Reintentar
          </button>
        </div>
      )}

      {options && (
        <>
          <DebtSummaryCards
            summary={summary}
            error={panelError}
            activeGroup={filter.due as DueGroup}
            onSelectGroup={group => updateFilter({ due: group })}
            onRetry={refresh}
          />
          {filtersActive && (
            <p className="debts-muted">
              Los totales respetan la búsqueda, la categoría, la naturaleza y las fechas
              elegidas; no el estado ni el vencimiento.
            </p>
          )}

          <DebtCategoryStats rows={categoryRows} options={options} />

          <DebtFilters
            filter={filter}
            options={options}
            rangeError={rangeError}
            hasFilters={filtersActive}
            onChange={updateFilter}
            onReset={resetFilters}
          />

          <p className="debts-status-line" role="status" aria-live="polite">{status}</p>

          {error && (
            <div className="debts-error-block" role="alert">
              <p>{error}</p>
              <button type="button" className="btn btn-primary" onClick={refresh}>
                Reintentar
              </button>
            </div>
          )}

          {!error && !rangeError && !loading && debts.length === 0 && (
            <div className="debts-empty">
              {filtersActive ? (
                'Ninguna deuda coincide con los filtros.'
              ) : (
                <>
                  <p>Aún no hay deudas registradas.</p>
                  <button
                    type="button"
                    className="btn btn-primary"
                    onClick={() => setDialog({ kind: 'create' })}
                  >
                    Registrar la primera deuda
                  </button>
                </>
              )}
            </div>
          )}

          {debts.length > 0 && (
            <div className={loading ? 'debts-results debts-results--loading' : 'debts-results'}>
              <DebtList
                debts={debts}
                options={options}
                onOpen={debt => setDialog({ kind: 'detail', id: debt.id, startWithPayment: false })}
                onPay={debt => setDialog({ kind: 'detail', id: debt.id, startWithPayment: true })}
              />
            </div>
          )}

          <nav className="debts-pagination" aria-label="Paginación">
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
        </>
      )}

      {options && dialog.kind === 'create' && (
        <Modal title="Nueva deuda" onClose={closeDialog} width={720}>
          <DebtForm
            initial={null}
            options={options}
            today={today}
            decimals={settings.currency_decimals}
            onCancel={closeDialog}
            onSaved={saved => {
              closeDialog();
              refresh();
              notify(`Deuda #${saved.id} registrada`);
            }}
          />
        </Modal>
      )}

      {options && dialog.kind === 'edit' && (
        <Modal title={`Editar deuda #${dialog.debt.id}`} onClose={closeDialog} width={720}>
          <DebtForm
            initial={dialog.debt}
            options={options}
            today={today}
            decimals={settings.currency_decimals}
            onCancel={() =>
              setDialog(
                dialog.backToDetail
                  ? { kind: 'detail', id: dialog.debt.id, startWithPayment: false }
                  : { kind: 'none' },
              )
            }
            onSaved={saved => {
              refresh();
              notify(`Deuda #${saved.id} actualizada`);
              setDialog(
                dialog.backToDetail
                  ? { kind: 'detail', id: saved.id, startWithPayment: false }
                  : { kind: 'none' },
              );
            }}
          />
        </Modal>
      )}

      {options && dialog.kind === 'detail' && (
        <Modal title={`Deuda #${dialog.id}`} onClose={closeDialog} width={900}>
          <DebtDetail
            key={dialog.id}
            debtId={dialog.id}
            options={options}
            today={today}
            decimals={settings.currency_decimals}
            timeZone={settings.timezone}
            startWithPayment={dialog.startWithPayment}
            onEdit={debt => setDialog({ kind: 'edit', debt, backToDetail: true })}
            onChanged={refresh}
            onNotify={notify}
          />
        </Modal>
      )}

      <ToastContainer toasts={toasts} onDismiss={dismissToast} />
    </div>
  );
};