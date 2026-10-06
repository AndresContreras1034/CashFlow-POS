import { useEffect, useState, useCallback } from 'react';
import type {
  CashSessionWithTotals,
  CashMovement,
  CashMovementType,
} from '../../types';
import {
  getCurrentCashSession,
  openCashSession,
  closeCashSession,
  registerCashMovement,
  listCashMovements,
} from '../../services/cash.service';
import './Cash.css';

const formatMoney = (cents: number) =>
  (cents / 100).toLocaleString('es-CO', { style: 'currency', currency: 'COP' });

export default function Cash() {
  const [session, setSession] = useState<CashSessionWithTotals | null>(null);
  const [movements, setMovements] = useState<CashMovement[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Form: apertura
  const [openingAmount, setOpeningAmount] = useState('');
  const [openingNotes, setOpeningNotes] = useState('');

  // Form: movimiento manual
  const [movementType, setMovementType] = useState<CashMovementType>('manual_in');
  const [movementAmount, setMovementAmount] = useState('');
  const [movementNotes, setMovementNotes] = useState('');

  // Form: cierre
  const [showCloseForm, setShowCloseForm] = useState(false);
  const [countedAmount, setCountedAmount] = useState('');
  const [closingNotes, setClosingNotes] = useState('');

  const loadSession = useCallback(async () => {
    setError(null);
    try {
      const current = await getCurrentCashSession();
      setSession(current);

      if (current) {
        const res = await listCashMovements({ session_id: current.id, page_size: 50 });
        setMovements(res.data);
      } else {
        setMovements([]);
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadSession();
  }, [loadSession]);

  async function handleOpenSession(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    try {
      const amountCents = Math.round(parseFloat(openingAmount || '0') * 100);
      await openCashSession({
        opening_amount: amountCents,
        opening_notes: openingNotes || null,
      });
      setOpeningAmount('');
      setOpeningNotes('');
      await loadSession();
    } catch (e) {
      setError(String(e));
    }
  }

  async function handleAddMovement(e: React.FormEvent) {
    e.preventDefault();
    if (!session) return;
    setError(null);
    try {
      const amountCents = Math.round(parseFloat(movementAmount || '0') * 100);
      await registerCashMovement(session.id, {
        movement_type: movementType,
        amount: amountCents,
        notes: movementNotes || null,
      });
      setMovementAmount('');
      setMovementNotes('');
      await loadSession();
    } catch (e) {
      setError(String(e));
    }
  }

  async function handleCloseSession(e: React.FormEvent) {
    e.preventDefault();
    if (!session) return;
    setError(null);
    try {
      const countedCents = Math.round(parseFloat(countedAmount || '0') * 100);
      await closeCashSession(session.id, {
        counted_amount: countedCents,
        closing_notes: closingNotes || null,
      });
      setShowCloseForm(false);
      setCountedAmount('');
      setClosingNotes('');
      await loadSession();
    } catch (e) {
      setError(String(e));
    }
  }

  if (loading) {
    return <div className="cash-page cash-loading">Cargando caja...</div>;
  }

  return (
    <div className="cash-page">
      <h1>Caja</h1>

      {error && <div className="form-error">{error}</div>}

      {!session ? (
        <div className="cash-card">
          <h2>No hay un turno abierto</h2>
          <form onSubmit={handleOpenSession} className="cash-form">
            <div className="form-field">
              <label>Monto inicial</label>
              <input
                className="form-input"
                type="number"
                step="0.01"
                min="0"
                required
                value={openingAmount}
                onChange={(e) => setOpeningAmount(e.target.value)}
                placeholder="0.00"
              />
            </div>
            <div className="form-field">
              <label>Notas de apertura (opcional)</label>
              <input
                className="form-input"
                type="text"
                value={openingNotes}
                onChange={(e) => setOpeningNotes(e.target.value)}
              />
            </div>
            <button type="submit" className="btn btn-primary">
              Abrir turno
            </button>
          </form>
        </div>
      ) : (
        <>
          <div className="cash-card cash-summary">
            <h2>Turno abierto</h2>
            <div className="cash-summary-grid">
              <div>
                <span className="cash-label">Apertura</span>
                <span className="cash-value">{formatMoney(session.opening_amount)}</span>
              </div>
              <div>
                <span className="cash-label">Ingresos</span>
                <span className="cash-value cash-in">{formatMoney(session.total_in)}</span>
              </div>
              <div>
                <span className="cash-label">Egresos</span>
                <span className="cash-value cash-out">{formatMoney(session.total_out)}</span>
              </div>
              <div>
                <span className="cash-label">Saldo actual</span>
                <span className="cash-value cash-balance">
                  {formatMoney(session.current_balance)}
                </span>
              </div>
            </div>
            <p className="cash-meta">
              Abierto por {session.opened_by} —{' '}
              {new Date(session.opened_at).toLocaleString('es-CO')}
            </p>
            <button
              className="btn btn-danger"
              onClick={() => setShowCloseForm((v) => !v)}
            >
              {showCloseForm ? 'Cancelar cierre' : 'Cerrar turno'}
            </button>
          </div>

          {showCloseForm && (
            <div className="cash-card">
              <h2>Arqueo de cierre</h2>
              <form onSubmit={handleCloseSession} className="cash-form">
                <div className="form-field">
                  <label>Monto contado</label>
                  <input
                    className="form-input"
                    type="number"
                    step="0.01"
                    min="0"
                    required
                    value={countedAmount}
                    onChange={(e) => setCountedAmount(e.target.value)}
                    placeholder="0.00"
                  />
                </div>
                <div className="form-field">
                  <label>Notas de cierre (opcional)</label>
                  <input
                    className="form-input"
                    type="text"
                    value={closingNotes}
                    onChange={(e) => setClosingNotes(e.target.value)}
                  />
                </div>
                <button type="submit" className="btn btn-primary">
                  Confirmar cierre
                </button>
              </form>
            </div>
          )}

          <div className="cash-card">
            <h2>Registrar movimiento manual</h2>
            <form onSubmit={handleAddMovement} className="cash-form cash-form-row">
              <div className="form-field">
                <label>Tipo</label>
                <select
                  className="form-input"
                  value={movementType}
                  onChange={(e) => setMovementType(e.target.value as CashMovementType)}
                >
                  <option value="manual_in">Ingreso</option>
                  <option value="manual_out">Egreso</option>
                </select>
              </div>
              <div className="form-field">
                <label>Monto</label>
                <input
                  className="form-input"
                  type="number"
                  step="0.01"
                  min="0"
                  required
                  value={movementAmount}
                  onChange={(e) => setMovementAmount(e.target.value)}
                  placeholder="0.00"
                />
              </div>
              <div className="form-field">
                <label>Notas (opcional)</label>
                <input
                  className="form-input"
                  type="text"
                  value={movementNotes}
                  onChange={(e) => setMovementNotes(e.target.value)}
                />
              </div>
              <button type="submit" className="btn btn-primary">
                Agregar
              </button>
            </form>
          </div>

          <div className="cash-card">
            <h2>Movimientos del turno</h2>
            {movements.length === 0 ? (
              <p className="cash-meta">Sin movimientos registrados aún.</p>
            ) : (
              <table className="cash-table">
                <thead>
                  <tr>
                    <th>Fecha</th>
                    <th>Tipo</th>
                    <th>Monto</th>
                    <th>Notas</th>
                  </tr>
                </thead>
                <tbody>
                  {movements.map((m) => (
                    <tr key={m.id}>
                      <td>{new Date(m.created_at).toLocaleString('es-CO')}</td>
                      <td>
                        <span
                          className={
                            m.movement_type.includes('in') ? 'cash-in' : 'cash-out'
                          }
                        >
                          {m.movement_type === 'manual_in' && 'Ingreso manual'}
                          {m.movement_type === 'manual_out' && 'Egreso manual'}
                          {m.movement_type === 'sale_in' && 'Venta (ingreso)'}
                          {m.movement_type === 'sale_out' && 'Venta (devolución)'}
                        </span>
                      </td>
                      <td>{formatMoney(m.amount)}</td>
                      <td>{m.notes ?? '—'}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </div>
        </>
      )}
    </div>
  );
}