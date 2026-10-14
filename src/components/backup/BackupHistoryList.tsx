import React from 'react';
import type { BackupRecord } from '../../types/backup';
import { BACKUP_KIND_LABELS, formatDuration, formatSize } from '../../utils/backup';
import './BackupHistoryList.css';

interface Props {
  records: BackupRecord[];
}

const formatDate = (iso: string) =>
  new Date(iso).toLocaleString('es-CO', { dateStyle: 'short', timeStyle: 'short' });

export const BackupHistoryList: React.FC<Props> = ({ records }) => {
  if (records.length === 0) {
    return <p className="settings-hint">Aún no se ha hecho ningún backup.</p>;
  }

  return (
    <div className="backup-history">
      <table className="backup-history-table">
        <thead>
          <tr>
            <th>Fecha</th>
            <th>Tipo</th>
            <th>Estado</th>
            <th>Tamaño</th>
            <th>Duración</th>
            <th>Archivo</th>
          </tr>
        </thead>
        <tbody>
          {records.map(record => (
            <tr key={record.id}>
              <td>{formatDate(record.started_at)}</td>
              <td>{BACKUP_KIND_LABELS[record.kind] ?? record.kind}</td>
              <td>
                {record.status === 'success' ? (
                  <span className="backup-badge backup-badge--ok">Correcto</span>
                ) : (
                  <span className="backup-badge backup-badge--fail" title={record.error_message ?? ''}>
                    Falló
                  </span>
                )}
              </td>
              <td>{formatSize(record.size_bytes)}</td>
              <td>{formatDuration(record.duration_ms)}</td>
              <td className="backup-history-file" title={record.file_path ?? record.error_message ?? ''}>
                {record.status === 'failed'
                  ? record.error_message
                  : record.file_deleted_at
                    ? `${record.file_name} (eliminado por rotación)`
                    : record.file_name}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
};
