import React from 'react';
import { useNavigate } from 'react-router-dom';
import { useBackupStatus } from '../../hooks/useBackupStatus';
import { formatAgo } from '../../utils/backup';
import './BackupStatusBadge.css';

export const BackupStatusBadge: React.FC = () => {
  const { status } = useBackupStatus();
  const navigate = useNavigate();
  if (!status) return null;

  const text = status.running
    ? 'Backup en curso…'
    : status.last_success_at
      ? `Backup ${formatAgo(status.hours_since_success)}`
      : 'Sin backups';

  return (
    <button
      type="button"
      className={`backup-badge-btn backup-badge-btn--${status.running ? 'running' : status.level}`}
      onClick={() => navigate('/settings', { state: { section: 'backup' } })}
      title="Ir a Ajustes → Backup"
    >
      <span className="backup-badge-dot" aria-hidden="true" />
      {text}
    </button>
  );
};
