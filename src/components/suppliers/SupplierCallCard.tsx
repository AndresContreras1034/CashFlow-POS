import React, { useMemo, useState } from 'react';
import type { RestockGroup } from '../../types/suppliers';
import {
  buildRestockOrderMessage,
  callNumber,
  contactActions,
  groupNeedsAttention,
  splitItems,
  urgencySummary,
  type ContactAction,
} from '../../utils/callPanel';
import { formatAttributes, formatMoney } from '../../utils/format';
import {
  RESTOCK_URGENCY_LABELS,
  formatSupplierConditions,
  priorityLabel,
} from '../../utils/suppliers';
import { formatCover, restockUrgencyBadgeClass } from '../../utils/supplierTabs';
import './SuppliersBase.css';
import './SupplierCallCard.css';

interface SupplierCallCardProps {
  group: RestockGroup;
  businessName: string;
  onAction: (action: ContactAction, group: RestockGroup) => void;
  onCopy: (text: string, what: string) => void;
  onOpenSupplier: (supplierId: number) => void;
}

const ITEM_LIMIT = 5;

export const SupplierCallCard: React.FC<SupplierCallCardProps> = ({
  group,
  businessName,
  onAction,
  onCopy,
  onOpenSupplier,
}) => {
  const [expanded, setExpanded] = useState(false);
  const supplier = group.supplier;
  const name = supplier?.name ?? 'Sin proveedor asignado';

  const message = useMemo(
    () => buildRestockOrderMessage(supplier?.name ?? null, group.items, businessName),
    [supplier?.name, group.items, businessName],
  );
  const actions = useMemo(
    () => (supplier ? contactActions(supplier, message, businessName) : []),
    [supplier, message, businessName],
  );
  const number = supplier ? callNumber(supplier) : null;
  const conditions = supplier
    ? formatSupplierConditions(supplier.lead_time_days, supplier.payment_terms_days)
    : '—';
  const { visible, hiddenCount } = splitItems(group.items, ITEM_LIMIT, expanded);

  const className = [
    'supplier-call-card',
    groupNeedsAttention(group) ? 'supplier-call-card--attention' : '',
    supplier ? '' : 'supplier-call-card--unassigned',
  ]
    .filter(Boolean)
    .join(' ');

  return (
    <article className={className} aria-label={`Reposición: ${name}`}>
      <header className="supplier-call-head">
        <div className="supplier-call-title">
          <h3>{name}</h3>
          {supplier && supplier.priority !== 'normal' && (
            <span
              className={
                supplier.priority === 'urgent'
                  ? 'suppliers-badge suppliers-badge-urgent'
                  : 'suppliers-badge suppliers-badge-preferred'
              }
            >
              {priorityLabel(supplier.priority)}
            </span>
          )}
        </div>
        <div className="supplier-call-totals">
          <strong>{group.item_count === 1 ? '1 producto' : `${group.item_count} productos`}</strong>
          <span>
            Pedido estimado {formatMoney(group.estimated_cost)}
            {group.items_without_cost > 0 && ` (+${group.items_without_cost} sin costo)`}
          </span>
        </div>
      </header>

      {supplier ? (
        <p className="supplier-call-meta">
          {[
            supplier.contact_name,
            conditions !== '—' ? conditions : null,
            supplier.visit_days ? `Visita: ${supplier.visit_days}` : null,
          ]
            .filter(Boolean)
            .join(' · ') || 'Sin datos de contacto adicionales'}
        </p>
      ) : (
        <p className="supplier-call-meta">
          Estos productos necesitan reposición pero no tienen un proveedor activo enlazado. Enlázalos
          desde la pestaña Productos de un proveedor.
        </p>
      )}

      <p className="supplier-call-summary">{urgencySummary(group)}</p>

      <ul className="supplier-call-items">
        {visible.map(item => {
          const attributes = formatAttributes(item.attributes);
          return (
            <li key={item.variant_id} className="supplier-call-item">
              <div className="supplier-call-item-main">
                <span className="supplier-call-item-name">{item.product_name}</span>
                {attributes && <span className="supplier-call-sub">{attributes}</span>}
                {item.days_of_cover !== null && (
                  <span className="supplier-call-sub">Alcanza: {formatCover(item.days_of_cover)}</span>
                )}
              </div>
              <span className={restockUrgencyBadgeClass(item.urgency)}>
                {RESTOCK_URGENCY_LABELS[item.urgency]}
              </span>
              <span className="supplier-call-item-nums tabular">
                Stock {item.stock} / mín. {item.stock_min} · Pedir {item.suggested_qty}
              </span>
            </li>
          );
        })}
      </ul>

      {group.items.length > ITEM_LIMIT && (
        <button
          type="button"
          className="supplier-call-toggle"
          aria-expanded={expanded}
          onClick={() => setExpanded(current => !current)}
        >
          {expanded ? 'Ver menos' : `Ver ${hiddenCount} más`}
        </button>
      )}

      <div className="supplier-call-actions">
        {actions.map(action => (
          <button
            key={action.channel}
            type="button"
            className="suppliers-btn"
            disabled={action.url === null}
            title={action.disabledReason ?? undefined}
            aria-label={`${action.label} a ${name}`}
            onClick={() => onAction(action, group)}
          >
            {action.label}
          </button>
        ))}
        {number && (
          <button
            type="button"
            className="suppliers-btn"
            aria-label={`Copiar número de ${name}`}
            onClick={() => onCopy(number, 'Número')}
          >
            Copiar número
          </button>
        )}
        <button
          type="button"
          className="suppliers-btn"
          aria-label={`Copiar pedido de ${name}`}
          onClick={() => onCopy(message, supplier ? 'Pedido' : 'Lista')}
        >
          {supplier ? 'Copiar pedido' : 'Copiar lista'}
        </button>
        {supplier && (
          <button
            type="button"
            className="suppliers-btn"
            aria-label={`Ver ficha de ${name}`}
            onClick={() => onOpenSupplier(supplier.id)}
          >
            Ver ficha
          </button>
        )}
      </div>
    </article>
  );
};
