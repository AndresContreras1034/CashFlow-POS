import React, { useState } from 'react';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import { notifyBackupChanged, setBackupPolling } from '../../hooks/useBackupStatus';
import { inspectBackupFile, restoreBackup } from '../../services/backup.service';
import type { BackupInspection, RestoreOutcome } from '../../types/backup';
import { errorText, formatSize } from '../../utils/backup';
import { getOperatorOrNull } from '../../utils/preferences';
import { Modal } from '../ui/Modal';
import './RestoreModal.css';

interface Props {
  onClose: () => void;
}

const CONFIRM_WORD = 'RESTAURAR';
const COUNT_LABELS = ['Ventas', 'Productos', 'Clientes', 'Deudas'];

export const RestoreModal: React.FC<Props> = ({ onClose }) => {
  const [path, setPath] = useState<string | null>(null);
  const [password, setPassword] = useState('');
  const [inspection, setInspection] = useState<BackupInspection | null>(null);
  const [confirm, setConfirm] = useState('');
  const [busy, setBusy] = useState<'inspect' | 'restore' | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [outcome, setOutcome] = useState<RestoreOutcome | null>(null);

  const analyze = async (file: string, pass: string) => {
    setBusy('inspect');
    setError(null);
    try {
      setInspection(await inspectBackupFile(file, pass || null));
    } catch (cause) {
      setInspection(current =>
        current?.encrypted
          ? { ...current, needs_password: true, details: null }
          : null,
      );
      setError(errorText(cause, 'No se pudo leer el backup'));
    } finally {
      setBusy(null);
    }
  };

  const pick = async () => {
    try {
      const selected = await openDialog({
        multiple: false,
        directory: false,
        filters: [{ name: 'Backups de CashFlow POS', extensions: ['dump', 'enc'] }],
      });
      if (typeof selected !== 'string') return;
      setPath(selected);
      setPassword('');
      setConfirm('');
      setInspection(null);
      setError(null);
      await analyze(selected, '');
    } catch (cause) {
      setError(errorText(cause, 'No se pudo abrir el selector de archivos'));
    }
  };

  const run = async () => {
    if (!path || !inspection?.details) return;
    setBusy('restore');
    setError(null);
    setBackupPolling(false);
    try {
      setOutcome(await restoreBackup(path, password || null, getOperatorOrNull()));
    } catch (cause) {
      setBackupPolling(true);
      notifyBackupChanged();
      setError(errorText(cause, 'La restauración falló'));
    } finally {
      setBusy(null);
    }
  };

  const details = inspection?.details ?? null;
  const countOf = (items: NonNullable<typeof details>['backup_counts'], label: string) =>
    items.find(item => item.label === label)?.count ?? 0;
  const locked = busy === 'restore' || outcome !== null;

  return (
    <Modal
      title="Restaurar desde un backup"
      onClose={locked ? () => {} : onClose}
      width={640}
    >
      <div className="restore-modal">
        {outcome ? (
          <>
            <div className="settings-notice">
              <strong>Restauración completada.</strong> La aplicación se reiniciará en unos
              segundos.
              {outcome.pre_restore_file && (
                <p>Se creó una copia de seguridad previa: {outcome.pre_restore_file}</p>
              )}
            </div>
            {outcome.warnings.map(warning => (
              <div className="settings-notice" key={warning}>{warning}</div>
            ))}
          </>
        ) : (
          <>
            <div className="settings-notice">
              Restaurar reemplaza todos los datos actuales por los del backup. Antes se hará una
              copia de seguridad de lo que hay ahora. La aplicación se reiniciará al terminar.
            </div>

            <div className="restore-modal-file">
              <button
                type="button"
                className="btn btn-ghost"
                onClick={() => void pick()}
                disabled={busy !== null}
              >
                {path ? 'Elegir otro archivo…' : 'Elegir archivo de backup…'}
              </button>
              <span className="settings-hint" title={path ?? undefined}>
                {path ?? 'Selecciona un archivo .dump o .dump.enc'}
              </span>
            </div>

            {busy === 'inspect' && (
              <p className="settings-hint" role="status">Leyendo y verificando el backup…</p>
            )}
            {error && <div className="restore-modal-error" role="alert">{error}</div>}

            {inspection?.needs_password && path && (
              <div className="form-field">
                <label className="form-label" htmlFor="restore-pass">
                  Contraseña de cifrado
                </label>
                <div className="restore-modal-password">
                  <input
                    id="restore-pass"
                    className="form-input"
                    type="password"
                    autoComplete="off"
                    value={password}
                    onChange={event => setPassword(event.target.value)}
                    onKeyDown={event => {
                      if (event.key === 'Enter' && password && busy === null) {
                        event.preventDefault();
                        void analyze(path, password);
                      }
                    }}
                  />
                  <button
                    type="button"
                    className="btn btn-primary"
                    disabled={!password || busy !== null}
                    onClick={() => void analyze(path, password)}
                  >
                    Continuar
                  </button>
                </div>
              </div>
            )}

            {details && inspection && (
              <>
                <ul className="restore-modal-metadata">
                  <li>
                    Archivo: {inspection.file_name} ({formatSize(inspection.size_bytes)})
                    {inspection.encrypted ? ', cifrado' : ''}
                  </li>
                  {details.business_name && <li>Negocio: {details.business_name}</li>}
                  {details.archive_created_at && <li>Creado: {details.archive_created_at}</li>}
                  {details.dumped_server_version && (
                    <li>PostgreSQL del backup: {details.dumped_server_version}</li>
                  )}
                  <li>
                    Versión de datos: migración {details.last_migration} de{' '}
                    {details.app_last_migration}
                  </li>
                </ul>

                <div className="restore-modal-table-wrap">
                  <table className="backup-history-table restore-modal-table">
                    <thead>
                      <tr><th></th><th>En el backup</th><th>Ahora</th></tr>
                    </thead>
                    <tbody>
                      {COUNT_LABELS.map(label => (
                        <tr key={label}>
                          <td>{label}</td>
                          <td>{countOf(details.backup_counts, label).toLocaleString('es-CO')}</td>
                          <td>{countOf(details.current_counts, label).toLocaleString('es-CO')}</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>

                {details.last_migration < details.app_last_migration && (
                  <p className="settings-hint">
                    Este backup es de una versión anterior; se actualizará automáticamente.
                  </p>
                )}

                <div className="form-field">
                  <label className="form-label" htmlFor="restore-confirm">
                    Escribe {CONFIRM_WORD} para confirmar
                  </label>
                  <input
                    id="restore-confirm"
                    className="form-input"
                    type="text"
                    value={confirm}
                    onChange={event => setConfirm(event.target.value)}
                    disabled={busy !== null}
                    autoComplete="off"
                  />
                </div>

                <div className="form-actions">
                  <button
                    type="button"
                    className="btn btn-ghost"
                    onClick={onClose}
                    disabled={busy !== null}
                  >
                    Cancelar
                  </button>
                  <button
                    type="button"
                    className="btn btn-primary"
                    disabled={confirm.trim().toUpperCase() !== CONFIRM_WORD || busy !== null}
                    onClick={() => void run()}
                  >
                    {busy === 'restore' ? 'Restaurando… no cierres la aplicación' : 'Restaurar'}
                  </button>
                </div>
              </>
            )}
          </>
        )}
      </div>
    </Modal>
  );
};