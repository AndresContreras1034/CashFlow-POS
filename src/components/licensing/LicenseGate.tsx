import React, { useCallback, useEffect, useState } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import type { LicenseStatusDto } from '../../types';
import { activateLicense, getLicenseStatus } from '../../services/licensing.service';
import './LicenseGate.css';

interface LicenseGateProps {
  children: React.ReactNode;
}

const errorText = (error: unknown): string =>
  typeof error === 'string'
    ? error
    : error instanceof Error
      ? error.message
      : 'Ocurrió un error inesperado.';

export const LicenseGate: React.FC<LicenseGateProps> = ({ children }) => {
  const [status, setStatus] = useState<LicenseStatusDto | null>(null);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const [notice, setNotice] = useState('');

  const refreshStatus = useCallback(async () => {
    try {
      const nextStatus = await getLicenseStatus();
      setStatus(nextStatus);
      setError('');
      return nextStatus;
    } catch (cause) {
      setError(`No se pudo comprobar la licencia: ${errorText(cause)}`);
      return null;
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refreshStatus();
    const timer = window.setInterval(() => void refreshStatus(), 30_000);
    return () => window.clearInterval(timer);
  }, [refreshStatus]);

  const handleCopyId = async () => {
    if (!status) return;
    try {
      await navigator.clipboard.writeText(status.installation_id);
      setNotice('Identificador copiado.');
    } catch (cause) {
      setError(`No se pudo copiar el identificador: ${errorText(cause)}`);
    }
  };

  const handleActivate = async () => {
    setBusy(true);
    setError('');
    setNotice('');
    try {
      const selected = await open({
        multiple: false,
        filters: [{ name: 'Licencia CashFlow-POS', extensions: ['json'] }],
      });
      if (!selected || Array.isArray(selected)) return;

      await activateLicense(selected);
      const nextStatus = await refreshStatus();
      if (nextStatus?.status === 'active') {
        setNotice('Licencia activada. Ya puedes usar el sistema.');
      } else {
        setError(nextStatus?.message ?? 'No se pudo confirmar la licencia activada.');
      }
    } catch (cause) {
      setError(`No se pudo activar la licencia: ${errorText(cause)}`);
    } finally {
      setBusy(false);
    }
  };

  if (loading) {
    return (
      <main className="license-screen">
        <section className="license-card">
          <div className="license-brand">CASHFLOW POS</div>
          <p>Comprobando licencia…</p>
        </section>
      </main>
    );
  }

  if (status?.status === 'active') return <>{children}</>;

  const statusTitle = status?.status === 'expired'
    ? 'El alquiler venció'
    : status?.status === 'invalid'
      ? 'Licencia no válida'
      : 'Activa CashFlow POS';

  return (
    <main className="license-screen">
      <section className="license-card">
        <div className="license-brand">CASHFLOW POS</div>
        <h1>{statusTitle}</h1>
        <p className="license-description">
          La licencia puede ser de compra perpetua o de alquiler. Para continuar,
          importa el archivo de licencia emitido para esta instalación.
        </p>

        {status && (
          <div className="license-installation">
            <span className="license-field-label">Identificador de instalación</span>
            <div className="license-installation-row">
              <code>{status.installation_id}</code>
              <button type="button" className="license-copy" onClick={handleCopyId}>
                Copiar
              </button>
            </div>
            <span className="license-help">
              Comparte este identificador con quien emite tu licencia.
            </span>
          </div>
        )}

        {status?.message && <p className="license-status-message">{status.message}</p>}
        {error && <p className="license-error" role="alert">{error}</p>}
        {notice && <p className="license-notice" role="status">{notice}</p>}

        <button
          type="button"
          className="license-activate"
          onClick={handleActivate}
          disabled={busy || !status}
        >
          {busy ? 'Validando…' : 'Importar archivo de licencia'}
        </button>

        <p className="license-source-note">
          El acceso al código fuente se entrega conforme al acuerdo de compra o alquiler;
          la activación de esta instalación no sustituye esa entrega.
        </p>
      </section>
    </main>
  );
};
