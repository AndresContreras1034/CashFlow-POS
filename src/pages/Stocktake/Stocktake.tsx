import React, { useCallback, useEffect, useRef, useState } from 'react';
import { Modal } from '../../components/ui/Modal';
import { ToastContainer, ToastData } from '../../components/ui/Toast';
import type {
  Category,
  Stocktake,
  StocktakeApplyResultDto,
  StocktakeCountLineDto,
  StocktakeReviewDto,
} from '../../types';
import { formatAttributes, formatDate } from '../../utils/format';
import { listCategories } from '../../services/inventory.service';
import {
  applyStocktake,
  cancelStocktake,
  findStocktakeLineByCode,
  getCurrentStocktake,
  getStocktakeReview,
  listStocktakeLines,
  setStocktakeCount,
  startStocktake,
} from '../../services/stocktake.service';
import './Stocktake.css';

const PAGE_SIZE = 50;

const errorMessage = (error: unknown): string =>
  typeof error === 'string'
    ? error
    : error instanceof Error
      ? error.message
      : 'Error desconocido';

const variantLabel = (name: string, attributes: Record<string, string>): string =>
  Object.keys(attributes).length > 0
    ? `${name} · ${formatAttributes(attributes)}`
    : name;

export const StocktakePage: React.FC = () => {
  const [toasts, setToasts] = useState<ToastData[]>([]);
  const [loading, setLoading] = useState(true);
  const [session, setSession] = useState<Stocktake | null>(null);
  const [view, setView] = useState<'count' | 'review'>('count');
  const [result, setResult] = useState<StocktakeApplyResultDto | null>(null);

  const [categories, setCategories] = useState<Category[]>([]);
  const [startCategory, setStartCategory] = useState<number | ''>('');
  const [startNotes, setStartNotes] = useState('');
  const [starting, setStarting] = useState(false);

  const [lines, setLines] = useState<StocktakeCountLineDto[]>([]);
  const [total, setTotal] = useState(0);
  const [totalPages, setTotalPages] = useState(0);
  const [page, setPage] = useState(1);
  const [search, setSearch] = useState('');
  const [debouncedSearch, setDebouncedSearch] = useState('');
  const [onlyPending, setOnlyPending] = useState(false);
  const [loadingLines, setLoadingLines] = useState(false);
  const [drafts, setDrafts] = useState<Record<number, string>>({});
  const [scanValue, setScanValue] = useState('');
  const [scanMsg, setScanMsg] = useState<{ text: string; ok: boolean } | null>(null);
  const scanRef = useRef<HTMLInputElement>(null);
  const pendingCounts = useRef(new Map<number, Promise<boolean>>());
  const sessionId = session?.id ?? null;

  const [review, setReview] = useState<StocktakeReviewDto | null>(null);
  const [reviewLoading, setReviewLoading] = useState(false);
  const [confirmApply, setConfirmApply] = useState(false);
  const [confirmCancel, setConfirmCancel] = useState(false);
  const [busy, setBusy] = useState(false);

  const addToast = useCallback((message: string, type: ToastData['type']) => {
    setToasts(current => [...current, { id: Math.random().toString(36).slice(2), message, type }]);
  }, []);

  const dismissToast = useCallback((id: string) => {
    setToasts(current => current.filter(toast => toast.id !== id));
  }, []);

  const refreshSession = useCallback(async () => {
    try {
      setSession(await getCurrentStocktake());
    } catch (error) {
      addToast(`No se pudo actualizar la toma: ${errorMessage(error)}`, 'error');
    }
  }, [addToast]);

  useEffect(() => {
    const initialize = async () => {
      try {
        const [current, loadedCategories] = await Promise.all([
          getCurrentStocktake(),
          listCategories(),
        ]);
        setSession(current);
        setCategories(loadedCategories);
      } catch (error) {
        addToast(`No se pudo cargar la toma de inventario: ${errorMessage(error)}`, 'error');
      } finally {
        setLoading(false);
      }
    };
    void initialize();
  }, [addToast]);

  useEffect(() => {
    const timer = window.setTimeout(() => setDebouncedSearch(search.trim()), 300);
    return () => window.clearTimeout(timer);
  }, [search]);

  useEffect(() => {
    setPage(1);
  }, [debouncedSearch, onlyPending]);

  const loadLines = useCallback(async () => {
    if (sessionId === null || view !== 'count') return;
    setLoadingLines(true);
    try {
      const response = await listStocktakeLines(sessionId, {
        search: debouncedSearch || null,
        only_pending: onlyPending,
        page,
        page_size: PAGE_SIZE,
      });
      setLines(response.data);
      setTotal(response.total);
      setTotalPages(response.total_pages);
      setDrafts({});
    } catch (error) {
      addToast(`No se pudieron cargar las variantes: ${errorMessage(error)}`, 'error');
    } finally {
      setLoadingLines(false);
    }
  }, [sessionId, view, debouncedSearch, onlyPending, page, addToast]);

  useEffect(() => {
    void loadLines();
  }, [loadLines]);

  useEffect(() => {
    if (sessionId !== null && view === 'count') scanRef.current?.focus();
  }, [sessionId, view]);

  const handleStart = async () => {
    setStarting(true);
    try {
      const created = await startStocktake({
        category_id: startCategory === '' ? null : startCategory,
        notes: startNotes.trim() || null,
      });
      setSession(created);
      setView('count');
      setStartNotes('');
      setSearch('');
      setOnlyPending(false);
      setPage(1);
      addToast('Toma de inventario iniciada', 'success');
    } catch (error) {
      addToast(errorMessage(error), 'error');
    } finally {
      setStarting(false);
    }
  };

  const saveCount = async (
    line: StocktakeCountLineDto,
    rawValue: string
  ): Promise<boolean> => {
    if (sessionId === null) return false;
    const clearDraft = () => {
      setDrafts(current => {
        const next = { ...current };
        delete next[line.variant_id];
        return next;
      });
    };
    const trimmed = rawValue.trim();
    let value: number | null;
    if (trimmed === '') {
      value = null;
    } else {
      const parsed = Number(trimmed);
      if (!Number.isSafeInteger(parsed) || parsed < 0 || parsed > 2_147_483_647) {
        addToast('El conteo debe ser un entero de 0 o mayor', 'error');
        clearDraft();
        return false;
      }
      value = parsed;
    }
    if (value === line.counted_stock) {
      clearDraft();
      return true;
    }
    try {
      await setStocktakeCount(sessionId, line.variant_id, value);
      setLines(current =>
        current.map(item =>
          item.variant_id === line.variant_id ? { ...item, counted_stock: value } : item
        )
      );
      clearDraft();
      await refreshSession();
      return true;
    } catch (error) {
      addToast(`No se pudo guardar el conteo: ${errorMessage(error)}`, 'error');
      return false;
    }
  };

  const queueCountSave = (line: StocktakeCountLineDto, rawValue: string) => {
    const savePromise = saveCount(line, rawValue);
    pendingCounts.current.set(line.variant_id, savePromise);
    void savePromise.then(() => {
      if (pendingCounts.current.get(line.variant_id) === savePromise) {
        pendingCounts.current.delete(line.variant_id);
      }
    });
  };

  const handleScan = async (event: React.FormEvent) => {
    event.preventDefault();
    const code = scanValue.trim();
    if (!code || sessionId === null) return;
    setScanValue('');
    try {
      const exactMatches = await findStocktakeLineByCode(sessionId, code);
      if (exactMatches.length === 0) {
        setScanMsg({ text: `No está en esta toma: ${code}`, ok: false });
      } else if (exactMatches.length > 1) {
        setScanMsg({ text: `El código ${code} coincide con varias variantes`, ok: false });
      } else {
        const target = exactMatches[0];
        if ((target.counted_stock ?? 0) >= 2_147_483_647) {
          setScanMsg({ text: 'El conteo llegó al máximo permitido', ok: false });
          return;
        }
        const nextCount = (target.counted_stock ?? 0) + 1;
        await setStocktakeCount(sessionId, target.variant_id, nextCount);
        setLines(current =>
          current.map(line =>
            line.variant_id === target.variant_id
              ? { ...line, counted_stock: nextCount }
              : line
          )
        );
        setScanMsg({
          text: `${variantLabel(target.product_name, target.attributes)}: ${nextCount}`,
          ok: true,
        });
        await refreshSession();
      }
    } catch (error) {
      setScanMsg({ text: errorMessage(error), ok: false });
    } finally {
      scanRef.current?.focus();
    }
  };

  const openReview = async () => {
    if (sessionId === null) return;
    if (Object.keys(drafts).length > 0) {
      addToast('Guarda o corrige el conteo pendiente antes de finalizar', 'warning');
      return;
    }
    setReviewLoading(true);
    try {
      const pendingResults = await Promise.all(pendingCounts.current.values());
      if (pendingResults.some(saved => !saved)) return;
      if (Object.keys(drafts).length > 0) {
        addToast('Guarda o corrige el conteo pendiente antes de finalizar', 'warning');
        return;
      }
      setReview(await getStocktakeReview(sessionId));
      setView('review');
    } catch (error) {
      addToast(`No se pudo preparar la revisión: ${errorMessage(error)}`, 'error');
    } finally {
      setReviewLoading(false);
    }
  };

  const handleApply = async () => {
    if (sessionId === null) return;
    setBusy(true);
    try {
      const applied = await applyStocktake(sessionId);
      setConfirmApply(false);
      setResult(applied);
      setSession(null);
      setReview(null);
      setView('count');
    } catch (error) {
      setConfirmApply(false);
      addToast(`No se aplicó nada: ${errorMessage(error)}`, 'error');
    } finally {
      setBusy(false);
    }
  };

  const handleCancel = async () => {
    if (sessionId === null) return;
    setBusy(true);
    try {
      await cancelStocktake(sessionId);
      setConfirmCancel(false);
      setSession(null);
      setReview(null);
      setView('count');
      addToast('Toma de inventario cancelada. No se modificó ningún stock', 'info');
    } catch (error) {
      setConfirmCancel(false);
      addToast(errorMessage(error), 'error');
    } finally {
      setBusy(false);
    }
  };

  const toastBox = <ToastContainer toasts={toasts} onDismiss={dismissToast} />;
  if (loading) {
    return <div className="st-page"><div className="st-loading">Cargando…</div>{toastBox}</div>;
  }

  if (result) {
    return (
      <div className="st-page">
        <div className="st-card st-result">
          <h1 className="st-title">Toma de inventario aplicada</h1>
          <div className="st-summary">
            <div className="st-stat"><span>Ajustadas</span><strong>{result.adjusted}</strong></div>
            <div className="st-stat"><span>Sin diferencia</span><strong>{result.unchanged}</strong></div>
            <div className="st-stat"><span>Sin contar (no se tocaron)</span><strong>{result.uncounted}</strong></div>
          </div>
          <p className="st-hint">
            Cada diferencia quedó en el Kardex como «Ajuste» con motivo «Corrección de conteo».
          </p>
          <div className="st-actions">
            <button className="btn btn-primary" onClick={() => setResult(null)}>Aceptar</button>
          </div>
        </div>
        {toastBox}
      </div>
    );
  }

  if (!session) {
    return (
      <div className="st-page">
        <h1 className="st-title">Toma de inventario</h1>
        <div className="st-card">
          <p className="st-hint">
            Cuenta el stock físico sin ver el del sistema. Al terminar revisas las diferencias y,
            si todo está bien, las aplicas de una sola vez. Las variantes que no cuentes no se modifican.
          </p>
          <div className="st-field">
            <label htmlFor="stocktake-category">Alcance</label>
            <select
              id="stocktake-category"
              className="st-input"
              value={startCategory}
              onChange={event =>
                setStartCategory(event.target.value === '' ? '' : Number(event.target.value))
              }
            >
              <option value="">Todo el inventario</option>
              {categories.map(category => (
                <option key={category.id} value={category.id}>{category.name}</option>
              ))}
            </select>
          </div>
          <div className="st-field">
            <label htmlFor="stocktake-notes">Notas (opcional)</label>
            <input
              id="stocktake-notes"
              className="st-input"
              value={startNotes}
              onChange={event => setStartNotes(event.target.value)}
              placeholder="Ej: Conteo de fin de mes"
            />
          </div>
          <p className="st-warn">
            Mientras cuentas, evita vender o mover stock de esas variantes. Si hay movimientos, la
            revisión te avisará; el conteo es más fiable sin ellos.
          </p>
          <div className="st-actions">
            <button className="btn btn-primary" onClick={handleStart} disabled={starting}>
              {starting ? 'Iniciando…' : 'Iniciar toma de inventario'}
            </button>
          </div>
        </div>
        {toastBox}
      </div>
    );
  }

  const progress = session.total_lines > 0
    ? Math.round((session.counted_lines / session.total_lines) * 100)
    : 0;
  const header = (
    <div className="st-header">
      <div>
        <h1 className="st-title">Toma de inventario #{session.id}</h1>
        <p className="st-subtitle">
          {session.category_name ?? 'Todo el inventario'} · iniciada {formatDate(session.created_at)}
          {session.notes ? ` · ${session.notes}` : ''}
        </p>
      </div>
      <div className="st-header-actions">
        <button className="btn btn-ghost" onClick={() => setConfirmCancel(true)} disabled={busy}>
          Cancelar toma
        </button>
      </div>
    </div>
  );
  const cancelModal = confirmCancel && (
    <Modal title="Cancelar toma de inventario" onClose={() => setConfirmCancel(false)} width={440}>
      <p>Se descartarán los conteos de esta sesión. No se modificará ningún stock.</p>
      <div className="st-actions">
        <button className="btn btn-ghost" onClick={() => setConfirmCancel(false)} disabled={busy}>
          Volver
        </button>
        <button className="btn btn-primary" onClick={handleCancel} disabled={busy}>
          {busy ? 'Cancelando…' : 'Sí, cancelar'}
        </button>
      </div>
    </Modal>
  );

  if (view === 'review' && review) {
    const summary = review.summary;
    return (
      <div className="st-page">
        {header}
        <div className="st-summary">
          <div className="st-stat"><span>Con diferencia</span><strong>{summary.with_difference}</strong></div>
          <div className="st-stat"><span>Coinciden</span><strong>{summary.matching}</strong></div>
          <div className="st-stat"><span>Sobrantes (unid.)</span><strong>+{summary.surplus_units}</strong></div>
          <div className="st-stat"><span>Faltantes (unid.)</span><strong>−{summary.shortage_units}</strong></div>
          <div className={`st-stat ${summary.uncounted > 0 ? 'st-stat--warn' : ''}`}>
            <span>Sin contar</span><strong>{summary.uncounted}</strong>
          </div>
        </div>
        {summary.uncounted > 0 && (
          <p className="st-warn">
            {summary.uncounted} variante{summary.uncounted !== 1 ? 's' : ''} sin contar: no se
            modificarán ni se pondrán en 0. Vuelve a contar si quieres incluirlas.
          </p>
        )}
        {summary.moved_during_count > 0 && (
          <p className="st-warn">
            ⚠ {summary.moved_during_count} variante{summary.moved_during_count !== 1 ? 's' : ''} tuvo
            movimientos durante el conteo. Al aplicar, el stock quedará igual a lo contado; revisa
            las marcadas.
          </p>
        )}
        {review.lines.length === 0 ? (
          <div className="st-empty">No hay diferencias que aplicar.</div>
        ) : (
          <div className="st-table-wrap">
            <table className="st-table">
              <thead>
                <tr>
                  <th>Producto / variante</th>
                  <th className="st-right">Stock al iniciar</th>
                  <th className="st-right">Stock actual</th>
                  <th className="st-right">Contado</th>
                  <th className="st-right">Diferencia</th>
                </tr>
              </thead>
              <tbody>
                {review.lines.map(line => (
                  <tr key={line.variant_id}>
                    <td>
                      {variantLabel(line.product_name, line.attributes)}
                      {line.moved_during_count && <span className="st-flag"> ⚠ con movimientos</span>}
                    </td>
                    <td className="st-right">{line.expected_stock}</td>
                    <td className="st-right">{line.current_stock}</td>
                    <td className="st-right">{line.counted_stock}</td>
                    <td className={`st-right ${line.difference > 0 ? 'st-pos' : 'st-neg'}`}>
                      {line.difference > 0 ? '+' : ''}{line.difference}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
        <div className="st-actions">
          <button className="btn btn-ghost" onClick={() => setView('count')} disabled={busy}>
            ← Volver a contar
          </button>
          <button className="btn btn-primary" onClick={() => setConfirmApply(true)} disabled={busy}>
            Aplicar ajustes
          </button>
        </div>
        {confirmApply && (
          <Modal title="Aplicar ajustes" onClose={() => setConfirmApply(false)} width={460}>
            <p>
              Se ajustarán <strong>{summary.with_difference}</strong> variante
              {summary.with_difference !== 1 ? 's' : ''} al stock contado, todas juntas. Si algo falla,
              no se aplica ninguna. Esta acción no se puede deshacer.
            </p>
            <div className="st-actions">
              <button className="btn btn-ghost" onClick={() => setConfirmApply(false)} disabled={busy}>
                Volver
              </button>
              <button className="btn btn-primary" onClick={handleApply} disabled={busy}>
                {busy ? 'Aplicando…' : 'Confirmar y aplicar'}
              </button>
            </div>
          </Modal>
        )}
        {cancelModal}
        {toastBox}
      </div>
    );
  }

  return (
    <div className="st-page">
      {header}
      <div className="st-progress">
        <div className="st-progress-text">
          Contadas <strong>{session.counted_lines}</strong> de <strong>{session.total_lines}</strong> ({progress}%)
        </div>
        <div className="st-progress-bar">
          <div className="st-progress-fill" style={{ width: `${progress}%` }} />
        </div>
      </div>
      <form className="st-scan" onSubmit={handleScan}>
        <input
          ref={scanRef}
          className="st-input"
          value={scanValue}
          onChange={event => setScanValue(event.target.value)}
          placeholder="Escanea un código de barras o SKU (cada lectura suma 1)"
          aria-label="Escanear código de barras o SKU"
        />
      </form>
      {scanMsg && <p className={scanMsg.ok ? 'st-scan-ok' : 'st-scan-err'}>{scanMsg.text}</p>}
      <div className="st-filters">
        <input
          className="st-input"
          value={search}
          onChange={event => setSearch(event.target.value)}
          placeholder="Buscar por nombre, marca, SKU o código…"
          aria-label="Buscar variantes del conteo"
        />
        <label className="st-check">
          <input
            type="checkbox"
            checked={onlyPending}
            onChange={event => setOnlyPending(event.target.checked)}
          />
          Solo pendientes
        </label>
      </div>
      {lines.length === 0 ? (
        <div className="st-empty">
          {loadingLines ? 'Cargando…' : 'No hay variantes con ese filtro'}
        </div>
      ) : (
        <div className="st-table-wrap">
          <table className="st-table">
            <thead>
              <tr>
                <th>Producto / variante</th>
                <th>SKU</th>
                <th>Código</th>
                <th className="st-right">Contado</th>
              </tr>
            </thead>
            <tbody>
              {lines.map(line => (
                <tr key={line.variant_id} className={line.counted_stock === null ? '' : 'st-row-done'}>
                  <td>{variantLabel(line.product_name, line.attributes)}</td>
                  <td>{line.sku ?? '—'}</td>
                  <td>{line.barcode ?? '—'}</td>
                  <td className="st-right">
                    <input
                      className="st-count"
                      type="number"
                      min="0"
                      step="1"
                      value={drafts[line.variant_id] ?? (line.counted_stock?.toString() ?? '')}
                      placeholder="—"
                      aria-label={`Conteo para ${variantLabel(line.product_name, line.attributes)}`}
                      onChange={event =>
                        setDrafts(current => ({ ...current, [line.variant_id]: event.target.value }))
                      }
                      onBlur={event => {
                        if (drafts[line.variant_id] !== undefined) {
                          queueCountSave(line, event.target.value);
                        }
                      }}
                      onKeyDown={event => {
                        if (event.key === 'Enter') event.currentTarget.blur();
                      }}
                    />
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
      <div className="st-pagination">
        <span>{total.toLocaleString()} variantes</span>
        <div>
          <button
            className="btn btn-ghost"
            disabled={page <= 1 || loadingLines}
            onClick={() => setPage(current => current - 1)}
          >
            ← Anterior
          </button>
          <span className="st-page-num">Página {page} de {Math.max(totalPages, 1)}</span>
          <button
            className="btn btn-ghost"
            disabled={loadingLines || totalPages === 0 || page >= totalPages}
            onClick={() => setPage(current => current + 1)}
          >
            Siguiente →
          </button>
        </div>
      </div>
      <div className="st-actions">
        <button
          className="btn btn-primary"
          onClick={openReview}
          disabled={reviewLoading || loadingLines}
        >
          {reviewLoading ? 'Preparando…' : 'Finalizar y revisar'}
        </button>
      </div>
      {cancelModal}
      {toastBox}
    </div>
  );
};
