import React, { useCallback, useEffect, useRef, useState } from 'react';
import { getSupplier, setSupplierActive } from '../../services/suppliers.service';
import type { SupplierOverview } from '../../types/suppliers';
import { errorMessage } from '../../utils/audit';
import { formatDate } from '../../utils/format';
import { actorField } from '../../utils/preferences';
import {
  callNumber,
  contactActions,
  type ContactAction,
} from '../../utils/callPanel';
import { priorityBadgeClass, priorityLabel } from '../../utils/suppliers';
import { SupplierCategoriesTab } from './SupplierCategoriesTab';
import { SupplierProductsTab } from './SupplierProductsTab';
import { SupplierRestockTab } from './SupplierRestockTab';
import './SuppliersBase.css';
import './SupplierDetail.css';

const TABS = [
  { key: 'info', label: 'Datos' },
  { key: 'categories', label: 'Categorías' },
  { key: 'products', label: 'Productos' },
  { key: 'restock', label: 'Reposición' },
] as const;

type TabKey = typeof TABS[number]['key'];

interface SupplierDetailProps {
  supplierId: number;
  reloadKey?: number;
  initialTab?: 'data' | 'categories' | 'products' | 'restock';
  businessName: string;
  onEdit: (supplier: SupplierOverview) => void;
  onChanged: () => void;
  onNotify: (message: string, type?: 'success' | 'error') => void;
  onAction: (action: ContactAction) => void;
  onCopy: (text: string, what: string) => void;
}

export const SupplierDetail: React.FC<SupplierDetailProps> = ({
  supplierId,
  reloadKey = 0,
  initialTab = 'data',
  businessName,
  onEdit,
  onChanged,
  onNotify,
  onAction,
  onCopy,
}) => {
  const [supplier, setSupplier] = useState<SupplierOverview | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [confirmingToggle, setConfirmingToggle] = useState(false);
  const [toggling, setToggling] = useState(false);
  const [activeTab, setActiveTab] = useState<TabKey>(
    initialTab === 'data' ? 'info' : initialTab,
  );
  const requestRef = useRef(0);
  const tabListRef = useRef<HTMLDivElement>(null);

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

  const handleTabKeyDown = (event: React.KeyboardEvent) => {
    const currentIndex = TABS.findIndex(t => t.key === activeTab);
    if (event.key === 'ArrowRight') {
      event.preventDefault();
      const nextIndex = (currentIndex + 1) % TABS.length;
      setActiveTab(TABS[nextIndex].key);
    } else if (event.key === 'ArrowLeft') {
      event.preventDefault();
      const prevIndex = (currentIndex - 1 + TABS.length) % TABS.length;
      setActiveTab(TABS[prevIndex].key);
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
  const quickActions = contactActions(supplier, '', businessName);
  const number = callNumber(supplier);

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

      {/* Pestañas accesibles */}
      <div
        ref={tabListRef}
        className="supplier-detail-tabs"
        role="tablist"
        aria-label="Pestañas de detalle del proveedor"
        onKeyDown={handleTabKeyDown}
      >
        {TABS.map(tab => {
          const isSelected = activeTab === tab.key;
          return (
            <button
              key={tab.key}
              id={`supplier-tab-${tab.key}`}
              type="button"
              role="tab"
              className="supplier-tab-btn"
              aria-selected={isSelected}
              aria-controls={`supplier-tabpanel-${tab.key}`}
              tabIndex={isSelected ? 0 : -1}
              onClick={() => setActiveTab(tab.key)}
            >
              {tab.label}
            </button>
          );
        })}
      </div>

      {/* Pestaña: Datos */}
      {activeTab === 'info' && (
        <div
          id="supplier-tabpanel-info"
          role="tabpanel"
          aria-labelledby="supplier-tab-info"
          className="supplier-tab-panel"
        >
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

          <div className="supplier-contact-actions" aria-label="Acciones de contacto">
            {quickActions.map(action => (
              <button
                key={action.channel}
                type="button"
                className="suppliers-btn"
                disabled={action.url === null}
                title={action.disabledReason ?? undefined}
                aria-label={`${action.label} a ${supplier.name}`}
                onClick={() => onAction(action)}
              >
                {action.label}
              </button>
            ))}
            {number && (
              <button
                type="button"
                className="suppliers-btn"
                aria-label={`Copiar número de ${supplier.name}`}
                onClick={() => onCopy(number, 'Número')}
              >
                Copiar número
              </button>
            )}
          </div>

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
      )}

      {/* Pestaña: Categorías */}
      {activeTab === 'categories' && (
        <div
          id="supplier-tabpanel-categories"
          role="tabpanel"
          aria-labelledby="supplier-tab-categories"
          className="supplier-tab-panel"
        >
          <SupplierCategoriesTab
            supplierId={supplier.id}
            isActive={supplier.is_active}
            onChanged={() => {
              loadSupplier();
              onChanged();
            }}
            onNotify={onNotify}
          />
        </div>
      )}

      {/* Pestaña: Productos */}
      {activeTab === 'products' && (
        <div
          id="supplier-tabpanel-products"
          role="tabpanel"
          aria-labelledby="supplier-tab-products"
          className="supplier-tab-panel"
        >
          <SupplierProductsTab
            supplierId={supplier.id}
            isActive={supplier.is_active}
            onChanged={() => {
              loadSupplier();
              onChanged();
            }}
            onNotify={onNotify}
          />
        </div>
      )}

      {/* Pestaña: Reposición */}
      {activeTab === 'restock' && (
        <div
          id="supplier-tabpanel-restock"
          role="tabpanel"
          aria-labelledby="supplier-tab-restock"
          className="supplier-tab-panel"
        >
          <SupplierRestockTab
            supplierId={supplier.id}
            isActive={supplier.is_active}
            supplier={supplier}
            businessName={businessName}
            onAction={onAction}
            onCopy={onCopy}
          />
        </div>
      )}
    </div>
  );
};
