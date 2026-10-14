import React, { useCallback, useEffect, useRef, useState } from 'react';
import { getSupplier, setSupplierActive } from '../../services/suppliers.service';
import type { SupplierOverview } from '../../types/suppliers';
import { errorMessage } from '../../utils/audit';
import { formatDate } from '../../utils/format';
import { actorField } from '../../utils/preferences';
import { priorityBadgeClass, priorityLabel } from '../../utils/suppliers';
import './SuppliersBase.css';
import './SupplierDetail.css';

interface SupplierDetailProps {
  supplierId: number;
  reloadKey?: number;
  onEdit: (supplier: SupplierOverview) => void;
  onChanged: () => void;
  onNotify: (message: string, type?: 'success' | 'error') => void;
}

export const SupplierDetail: React.FC<SupplierDetailProps> = ({
  supplierId,
  reloadKey = 0,
  onEdit,
  onChanged,
  onNotify,
}) => {
  const [supplier, setSupplier] = useState<SupplierOverview | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [confirmingToggle, setConfirmingToggle] = useState(false);
  const [toggling, setToggling] = useState(false);
  const requestRef = useRef(0);

  const loadSupplier = useCallback(() => {
    const requestId = ++requestRef.current;
    setLoading(true);
    setError(null);
    getSupplier(supplierId)
      .then(data => {
        if (requestId !== requestRef.current) return;
        setSupplier(data);
        setLoading(false);
      })
      .catch(cause => {
        if (requestId !== requestRef.current) return;
        setError(errorMessage(cause));
        setLoading(false);
      });
  }, [supplierId]);

  useEffect(() => {
    loadSupplier();
    return () => {
      requestRef.current += 1;
    };
  }, [loadSupplier, reloadKey]);

  const handleToggleActive = async () => {
    if (!supplier || toggling) return;
    setToggling(true);
    try {
      await setSupplierActive(supplier.id, {
        is_active: !supplier.is_active,
        ...actorField('updated_by'),
      });
      setConfirmingToggle(false);
      loadSupplier();
      onChanged();
      onNotify(
        supplier.is_active ? 'Proveedor desactivado' : 'Proveedor reactivado',
        'success',
      );
    } catch (cause) {
      onNotify(errorMessage(cause), 'error');
    } finally {
      setToggling(false);
    }
  };

  if (error) {
    return (
      <div className="supplier-detail-error" role="alert">
        {error}
      </div>
    );
  }

  if (loading || !supplier) {
    return <p className="supplier-detail-muted">Cargando proveedor…</p>;
  }

  const documentText =
    supplier.document_type && supplier.document_number
      ? `${supplier.document_type} ${supplier.document_number}`
      : supplier.document_number ?? '—';

  const paymentText =
    supplier.payment_terms_days !== null
      ? supplier.payment_terms_days === 0
        ? '0 días (contado)'
        : `${supplier.payment_terms_days} días`
      : '—';

  const leadText =
    supplier.lead_time_days !== null ? `${supplier.lead_time_days} días` : '—';

  return (
    <div className="supplier-detail">
      <div className="supplier-detail-head">
        <div className="supplier-detail-title-wrap">
          <h3>{supplier.name}</h3>
          <div className="supplier-detail-badges">
            <span className={priorityBadgeClass(supplier.priority)}>
              {priorityLabel(supplier.priority)}
            </span>
            <span
              className={
                supplier.is_active
                  ? 'suppliers-badge suppliers-badge-ok'
                  : 'suppliers-badge'
              }
            >
              {supplier.is_active ? 'Activo' : 'Inactivo'}
            </span>
          </div>
        </div>
      </div>

      <dl className="supplier-detail-dl">
        <dt>Persona de contacto</dt>
        <dd>{supplier.contact_name ?? '—'}</dd>

        <dt>Documento</dt>
        <dd>{documentText}</dd>

        <dt>Teléfono</dt>
        <dd>{supplier.phone ?? '—'}</dd>

        <dt>WhatsApp</dt>
        <dd>{supplier.whatsapp ?? '—'}</dd>

        <dt>Email</dt>
        <dd>{supplier.email ?? '—'}</dd>

        <dt>Dirección</dt>
        <dd>{supplier.address ?? '—'}</dd>

        <dt>Ciudad</dt>
        <dd>{supplier.city ?? '—'}</dd>

        <dt>Plazo de pago</dt>
        <dd>{paymentText}</dd>

        <dt>Tiempo de entrega</dt>
        <dd>{leadText}</dd>

        <dt>Días de visita o pedido</dt>
        <dd>{supplier.visit_days ?? '—'}</dd>

        <dt>Notas</dt>
        <dd>{supplier.notes ?? '—'}</dd>

        <dt>Registro</dt>
        <dd>
          Registrado el {formatDate(supplier.created_at)} por {supplier.created_by}
        </dd>
      </dl>

      <div className="supplier-detail-actions">
        <button
          type="button"
          className="suppliers-btn"
          onClick={() => onEdit(supplier)}
        >
          Editar
        </button>

        {!confirmingToggle ? (
          <button
            type="button"
            className={
              supplier.is_active
                ? 'suppliers-btn suppliers-btn-danger'
                : 'suppliers-btn'
            }
            onClick={() => setConfirmingToggle(true)}
          >
            {supplier.is_active ? 'Desactivar' : 'Reactivar'}
          </button>
        ) : (
          <div className="supplier-detail-confirm">
            <span className="supplier-detail-confirm-text">
              {supplier.is_active
                ? 'Dejará de aparecer en reposición. Su historial y enlaces se conservan.'
                : '¿Reactivar este proveedor?'}
            </span>
            <div className="supplier-detail-confirm-btns">
              <button
                type="button"
                className="suppliers-btn"
                onClick={() => setConfirmingToggle(false)}
                disabled={toggling}
              >
                Cancelar
              </button>
              <button
                type="button"
                className={
                  supplier.is_active
                    ? 'suppliers-btn suppliers-btn-danger'
                    : 'suppliers-btn suppliers-btn-primary'
                }
                onClick={() => void handleToggleActive()}
                disabled={toggling}
              >
                {toggling
                  ? 'Procesando…'
                  : supplier.is_active
                  ? 'Confirmar desactivación'
                  : 'Reactivar'}
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};
