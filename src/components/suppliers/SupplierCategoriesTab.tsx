import React, { useCallback, useEffect, useRef, useState } from 'react';
import { listCategories } from '../../services/inventory.service';
import {
  listSupplierCategories,
  setSupplierCategories,
} from '../../services/suppliers.service';
import type { Category } from '../../types';
import { errorMessage } from '../../utils/audit';
import { actorField } from '../../utils/preferences';
import { sameIdSet, toggleId } from '../../utils/supplierTabs';
import './SuppliersBase.css';
import './SupplierCategoriesTab.css';

interface SupplierCategoriesTabProps {
  supplierId: number;
  isActive: boolean;
  onChanged: () => void;
  onNotify: (message: string, type?: 'success' | 'error') => void;
}

export const SupplierCategoriesTab: React.FC<SupplierCategoriesTabProps> = ({
  supplierId,
  isActive,
  onChanged,
  onNotify,
}) => {
  const [categories, setCategories] = useState<Category[]>([]);
  const [initialIds, setInitialIds] = useState<number[]>([]);
  const [selectedIds, setSelectedIds] = useState<number[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const submittingRef = useRef(false);
  const requestRef = useRef(0);

  const loadData = useCallback(() => {
    const requestId = ++requestRef.current;
    setLoading(true);
    setError(null);

    Promise.all([listCategories(), listSupplierCategories(supplierId)])
      .then(([allCategories, assignedCategories]) => {
        if (requestId !== requestRef.current) return;
        setCategories(allCategories);
        const assignedIds = assignedCategories.map(c => c.category_id);
        setInitialIds(assignedIds);
        setSelectedIds(assignedIds);
        setLoading(false);
      })
      .catch(cause => {
        if (requestId !== requestRef.current) return;
        setError(errorMessage(cause));
        setLoading(false);
      });
  }, [supplierId]);

  useEffect(() => {
    loadData();
    return () => {
      requestRef.current += 1;
    };
  }, [loadData]);

  const hasChanges = !sameIdSet(initialIds, selectedIds);

  const handleToggle = (id: number) => {
    if (!isActive) return;
    setSelectedIds(current => toggleId(current, id));
  };

  const handleDiscard = () => {
    setSelectedIds(initialIds);
  };

  const handleSave = async () => {
    if (!isActive || !hasChanges || submittingRef.current) return;
    submittingRef.current = true;
    setSaving(true);
    try {
      const actor = actorField('updated_by').updated_by;
      await setSupplierCategories(supplierId, selectedIds, actor);
      setInitialIds(selectedIds);
      onNotify('Categorías actualizadas', 'success');
      onChanged();
    } catch (cause) {
      onNotify(errorMessage(cause), 'error');
    } finally {
      submittingRef.current = false;
      setSaving(false);
    }
  };

  if (loading) {
    return <p className="supplier-tab-muted">Cargando categorías…</p>;
  }

  if (error) {
    return (
      <div className="supplier-tab-error" role="alert">
        <span>No se pudieron cargar las categorías: {error}</span>
        <button
          type="button"
          className="suppliers-btn suppliers-btn-primary"
          onClick={loadData}
        >
          Reintentar
        </button>
      </div>
    );
  }

  return (
    <div className="supplier-categories-tab">
      {!isActive && (
        <div className="supplier-tab-notice" role="note">
          Reactiva el proveedor para modificar sus enlaces.
        </div>
      )}

      <p className="supplier-categories-hint">
        Indica de qué tipo de cosas provee. No enlaza productos concretos; eso se hace
        en la pestaña Productos.
      </p>

      {categories.length === 0 ? (
        <p className="supplier-tab-muted">No hay categorías registradas en el sistema.</p>
      ) : (
        <div className="supplier-categories-grid" role="group" aria-label="Categorías">
          {categories.map(cat => {
            const checked = selectedIds.includes(cat.id);
            return (
              <label key={cat.id} className="supplier-category-item">
                <input
                  type="checkbox"
                  checked={checked}
                  disabled={!isActive || saving}
                  onChange={() => handleToggle(cat.id)}
                />
                <span>{cat.name}</span>
              </label>
            );
          })}
        </div>
      )}

      <div className="supplier-categories-actions">
        <button
          type="button"
          className="suppliers-btn"
          disabled={!hasChanges || saving || !isActive}
          onClick={handleDiscard}
        >
          Descartar cambios
        </button>
        <button
          type="button"
          className="suppliers-btn suppliers-btn-primary"
          disabled={!hasChanges || saving || !isActive}
          onClick={() => void handleSave()}
        >
          {saving ? 'Guardando…' : 'Guardar categorías'}
        </button>
      </div>
    </div>
  );
};
