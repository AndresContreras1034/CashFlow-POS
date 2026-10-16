import React, { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { notifyBackupChanged, useBackupStatus } from '../../hooks/useBackupStatus';
import { runBackupNow } from '../../services/backup.service';
import { errorText, needsBackupEncryptionSetup } from '../../utils/backup';
import { getOperatorOrNull } from '../../utils/preferences';
import './BackupAlertBanner.css';

export const BackupAlertBanner: React.FC = () => {
  const { status } = useBackupStatus();
  const navigate = useNavigate();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  if (!status) return null;
  const encryptionSetupRequired = needsBackupEncryptionSetup(
    status.auto_enabled,
    status.encrypt_enabled,
    status.encryption_password_set,
  );
  const backupNeedsAttention = status.level === 'warning' || status.level === 'critical';
  if (!encryptionSetupRequired && !backupNeedsAttention) return null;

  const backupMessage =
    status.days_since_success == null
      ? 'Todavía no hay ningún backup de tus datos.'
      : `Llevas ${status.days_since_success} ${status.days_since_success === 1 ? 'día' : 'días'} sin backup.`;
  const encryptionMessage = !status.encrypt_enabled
    ? 'El backup automático está activo sin cifrado. Activa el cifrado y define una contraseña en Ajustes → Backup.'
    : 'Define una contraseña de cifrado en Ajustes → Backup antes del primer backup automático.';

  const handleNow = async () => {
    setBusy(true);
    setError(null);
    try {
      await runBackupNow(getOperatorOrNull());
    } catch (e) {
      setError(errorText(e, 'El backup falló'));
    } finally {
      setBusy(false);
      notifyBackupChanged();
    }
  };

  const detail = error ?? (status.last_attempt_failed ? status.last_error : null);

  return (
    <div
      className={`backup-banner backup-banner--${encryptionSetupRequired ? 'security' : status.level}`}
      role="alert"
    >
      <div className="backup-banner-text">
        <strong>{encryptionSetupRequired ? encryptionMessage : backupMessage}</strong>
        {encryptionSetupRequired && backupNeedsAttention && (
          <span className="backup-banner-detail"> {backupMessage}</span>
        )}
        {detail && <span className="backup-banner-detail"> Último error: {detail}</span>}
      </div>
      {encryptionSetupRequired ? (
        <button
          type="button"
          className="btn btn-primary"
          onClick={() => navigate('/settings', { state: { section: 'backup' } })}
        >
          Configurar cifrado
        </button>
      ) : (
        <button
          type="button"
          className="btn btn-primary"
          onClick={handleNow}
          disabled={busy || status.running}
        >
          {busy || status.running ? 'Backup en curso…' : 'Hacer ahora'}
        </button>
      )}
    </div>
  );
};
