import React, { useCallback, useEffect, useState } from 'react';
import {
  addBackupRecipient,
  deleteBackupRecipient,
  listBackupDeliveries,
  listBackupRecipients,
  sendBackupTestEmail,
  updateBackupRecipient,
} from '../../services/backup.service';
import type { BackupDelivery, BackupRecipient } from '../../types/backup';
import { errorText } from '../../utils/backup';
import './BackupRecipients.css';

interface Props {
  onToast: (message: string, type: 'success' | 'error' | 'warning' | 'info') => void;
}

const EMAIL_RE = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
const NOTICE_LABELS: Record<string, string> = {
  success: 'Backup',
  failure: 'Aviso de fallo',
  test: 'Prueba',
};

const formatDate = (iso: string) =>
  new Date(iso).toLocaleString('es-CO', { dateStyle: 'short', timeStyle: 'short' });

export const BackupRecipients: React.FC<Props> = ({ onToast }) => {
  const [items, setItems] = useState<BackupRecipient[]>([]);
  const [deliveries, setDeliveries] = useState<BackupDelivery[]>([]);
  const [email, setEmail] = useState('');
  const [name, setName] = useState('');
  const [busy, setBusy] = useState(false);
  const [testing, setTesting] = useState(false);

  const refreshDeliveries = useCallback(async () => {
    try {
      setDeliveries(await listBackupDeliveries(10));
    } catch (error) {
      console.error('No se pudieron cargar los envíos:', error);
    }
  }, []);

  const refresh = useCallback(async () => {
    try {
      setItems(await listBackupRecipients());
    } catch (error) {
      onToast(errorText(error, 'No se pudieron cargar los destinatarios'), 'error');
    }
    await refreshDeliveries();
  }, [onToast, refreshDeliveries]);

  useEffect(() => {
    void refresh();
    const id = window.setInterval(() => void refreshDeliveries(), 15_000);
    return () => window.clearInterval(id);
  }, [refresh, refreshDeliveries]);

  const handleAdd = async () => {
    const value = email.trim();
    if (!EMAIL_RE.test(value)) {
      onToast('Escribe un correo válido', 'warning');
      return;
    }
    setBusy(true);
    try {
      await addBackupRecipient({ email: value, name: name.trim() || null, send_attachment: true });
      setEmail('');
      setName('');
      await refresh();
    } catch (error) {
      onToast(errorText(error, 'No se pudo añadir el destinatario'), 'error');
    } finally {
      setBusy(false);
    }
  };

  const patch = async (item: BackupRecipient, change: Partial<Pick<BackupRecipient, 'active' | 'send_attachment'>>) => {
    try {
      await updateBackupRecipient(item.id, {
        name: item.name,
        active: change.active ?? item.active,
        send_attachment: change.send_attachment ?? item.send_attachment,
      });
      await refresh();
    } catch (error) {
      onToast(errorText(error, 'No se pudo actualizar el destinatario'), 'error');
    }
  };

  const handleDelete = async (item: BackupRecipient) => {
    if (!window.confirm(`¿Quitar a ${item.email} de la lista?`)) return;
    try {
      await deleteBackupRecipient(item.id);
      await refresh();
    } catch (error) {
      onToast(errorText(error, 'No se pudo quitar el destinatario'), 'error');
    }
  };

  const handleTest = async () => {
    setTesting(true);
    try {
      const results = await sendBackupTestEmail();
      const failed = results.filter(r => !r.ok);
      if (failed.length === 0) {
        onToast(`Correo de prueba enviado a ${results.length} destinatario(s)`, 'success');
      } else {
        onToast(`${failed[0].email}: ${failed[0].error ?? 'falló el envío'}`, 'error');
      }
    } catch (error) {
      onToast(errorText(error, 'No se pudo enviar el correo de prueba'), 'error');
    } finally {
      setTesting(false);
      await refreshDeliveries();
    }
  };

  return (
    <>
      <div className="backup-recipients">
        {items.length === 0 && (
          <p className="settings-hint">Aún no hay destinatarios. Añade al menos un correo.</p>
        )}
        {items.map(item => (
          <div className="backup-recipient-row" key={item.id}>
            <div className="backup-recipient-info">
              <strong>{item.email}</strong>
              {item.name && <span className="settings-hint"> — {item.name}</span>}
            </div>
            <label className="backup-recipient-check">
              <input type="checkbox" checked={item.active} onChange={e => void patch(item, { active: e.target.checked })} />
              Activo
            </label>
            <label className="backup-recipient-check">
              <input
                type="checkbox"
                checked={item.send_attachment}
                onChange={e => void patch(item, { send_attachment: e.target.checked })}
              />
              Con archivo
            </label>
            <button type="button" className="btn btn-ghost" onClick={() => void handleDelete(item)}>
              Quitar
            </button>
          </div>
        ))}
      </div>

      <div className="form-row">
        <div className="form-field">
          <label className="form-label" htmlFor="recipient-email">Correo</label>
          <input
            id="recipient-email"
            className="form-input"
            type="text"
            value={email}
            onChange={e => setEmail(e.target.value)}
            onKeyDown={e => {
              if (e.key === 'Enter') {
                e.preventDefault();
                void handleAdd();
              }
            }}
            placeholder="dueno@correo.com"
          />
        </div>
        <div className="form-field">
          <label className="form-label" htmlFor="recipient-name">Nombre (opcional)</label>
          <input
            id="recipient-name"
            className="form-input"
            type="text"
            maxLength={100}
            value={name}
            onChange={e => setName(e.target.value)}
          />
        </div>
      </div>

      <div style={{ display: 'flex', gap: 8 }}>
        <button type="button" className="btn btn-primary" onClick={() => void handleAdd()} disabled={busy}>
          Añadir destinatario
        </button>
        <button
          type="button"
          className="btn btn-ghost"
          onClick={() => void handleTest()}
          disabled={testing || items.every(i => !i.active)}
        >
          {testing ? 'Enviando…' : 'Enviar correo de prueba'}
        </button>
      </div>
      <span className="settings-hint">
        “Con archivo” envía el backup cifrado adjunto (si pesa menos de 18 MB); sin marcar, solo el
        aviso. La prueba usa los ajustes ya guardados.
      </span>

      {deliveries.length > 0 && (
        <div className="backup-history">
          <table className="backup-history-table">
            <thead>
              <tr><th>Fecha</th><th>Tipo</th><th>Destinatario</th><th>Archivo</th><th>Estado</th></tr>
            </thead>
            <tbody>
              {deliveries.map(d => (
                <tr key={d.id}>
                  <td>{formatDate(d.created_at)}</td>
                  <td>{NOTICE_LABELS[d.notice] ?? d.notice}</td>
                  <td>{d.recipient_email}</td>
                  <td>{d.attached ? 'Adjunto' : 'No'}</td>
                  <td>
                    {d.status === 'sent' ? (
                      <span className="backup-badge backup-badge--ok">Enviado</span>
                    ) : (
                      <span className="backup-badge backup-badge--fail" title={d.error_message ?? ''}>Falló</span>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </>
  );
};
