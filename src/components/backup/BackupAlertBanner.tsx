import React, { useState } from 'react';
import { notifyBackupChanged, useBackupStatus } from '../../hooks/useBackupStatus';
import { runBackupNow } from '../../services/backup.service';
import { errorText } from '../../utils/backup';
import { getOperatorOrNull } from '../../utils/preferences';
import './BackupAlertBanner.css';

export const BackupAlertBanner: React.FC = () => {
  const { status } = useBackupStatus();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  if (!status || (status.level !== 'warning' && status.level !== 'critical')) return null;

  const message =
    status.days_since_success == null
      ? 'Todavía no hay ningún backup de tus datos.'
      : `Llevas ${status.days_since_success} ${status.days_since_success === 1 ? 'día' : 'días'} sin backup.`;

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
    <div className={`backup-banner backup-banner--${status.level}`} role="alert">
      <div className="backup-banner-text">
        <strong>{message}</strong>
        {detail && <span className="backup-banner-detail"> Último error: {detail}</span>}
      </div>
      <button
        type="button"
        className="btn btn-primary"
        onClick={handleNow}
        disabled={busy || status.running}
      >
        {busy || status.running ? 'Backup en curso…' : 'Hacer ahora'}
      </button>
    </div>
  );
};
