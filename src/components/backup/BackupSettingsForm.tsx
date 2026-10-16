import React, { useCallback, useEffect, useState } from 'react';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import {
  checkBackupEnvironment,
  getBackupSecretsStatus,
  getBackupSettings,
  isBackupRunning,
  listBackupHistory,
  runBackupNow,
  setBackupEncryptionPassword,
  setBackupSmtpPassword,
  updateBackupSettings,
} from '../../services/backup.service';
import type {
  BackupEnvironment,
  BackupRecord,
  BackupSecretsStatus,
  BackupSettings,
} from '../../types/backup';
import {
  DAY_LABELS,
  DAY_NAMES,
  errorText,
  formatSize,
  hasDay,
  isOneDrivePath,
  toggleDay,
} from '../../utils/backup';
import { notifyBackupChanged } from '../../hooks/useBackupStatus';
import { getOperatorOrNull } from '../../utils/preferences';
import { BackupHistoryList } from './BackupHistoryList';
import { BackupRecipients } from './BackupRecipients';
import { RestoreModal } from './RestoreModal';
import { SecretField } from './SecretField';

interface Props {
  onToast: (message: string, type: 'success' | 'error' | 'warning' | 'info') => void;
}

interface FormState {
  local_dir: string;
  pg_bin_dir: string;
  keep_daily: string;
  keep_weekly: string;
  keep_monthly: string;
  auto_enabled: boolean;
  schedule_time: string;
  schedule_days: number;
  warn_after_days: string;
  critical_after_days: string;
  encrypt_enabled: boolean;
  smtp_enabled: boolean;
  smtp_host: string;
  smtp_port: string;
  smtp_security: 'tls' | 'starttls';
  smtp_username: string;
  smtp_from_name: string;
  email_on_failure: boolean;
}

const toForm = (s: BackupSettings): FormState => ({
  local_dir: s.local_dir ?? '',
  pg_bin_dir: s.pg_bin_dir ?? '',
  keep_daily: String(s.keep_daily),
  keep_weekly: String(s.keep_weekly),
  keep_monthly: String(s.keep_monthly),
  auto_enabled: s.auto_enabled,
  schedule_time: s.schedule_time.slice(0, 5),
  schedule_days: s.schedule_days,
  warn_after_days: String(s.warn_after_days),
  critical_after_days: String(s.critical_after_days),
  encrypt_enabled: s.encrypt_enabled,
  smtp_enabled: s.smtp_enabled,
  smtp_host: s.smtp_host,
  smtp_port: String(s.smtp_port),
  smtp_security: s.smtp_security,
  smtp_username: s.smtp_username ?? '',
  smtp_from_name: s.smtp_from_name ?? '',
  email_on_failure: s.email_on_failure,
});

const inRange = (value: string, min: number, max: number) => {
  const n = Number(value);
  return value.trim() !== '' && Number.isInteger(n) && n >= min && n <= max;
};

export const BackupSettingsForm: React.FC<Props> = ({ onToast }) => {
  const [saved, setSaved] = useState<BackupSettings | null>(null);
  const [form, setForm] = useState<FormState | null>(null);
  const [env, setEnv] = useState<BackupEnvironment | null>(null);
  const [history, setHistory] = useState<BackupRecord[]>([]);
  const [secrets, setSecrets] = useState<BackupSecretsStatus>({
    encryption_password_set: false,
    smtp_password_set: false,
  });
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [running, setRunning] = useState(false);
  const [restoreOpen, setRestoreOpen] = useState(false);

  const refreshEnv = useCallback(async () => {
    try {
      setEnv(await checkBackupEnvironment());
    } catch (error) {
      onToast(errorText(error, 'No se pudo revisar el entorno de backup'), 'error');
    }
  }, [onToast]);

  const refreshHistory = useCallback(async () => {
    try {
      setHistory(await listBackupHistory(20));
    } catch (error) {
      onToast(errorText(error, 'No se pudo cargar el historial'), 'error');
    }
  }, [onToast]);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const [settings, secretsStatus] = await Promise.all([
          getBackupSettings(),
          getBackupSecretsStatus(),
        ]);
        if (cancelled) return;
        setSaved(settings);
        setForm(toForm(settings));
        setSecrets(secretsStatus);
        setRunning(await isBackupRunning());
      } catch (error) {
        onToast(errorText(error, 'No se pudieron cargar los ajustes de backup'), 'error');
      } finally {
        if (!cancelled) setLoading(false);
      }
      await Promise.all([refreshEnv(), refreshHistory()]);
    })();
    return () => {
      cancelled = true;
    };
  }, [onToast, refreshEnv, refreshHistory]);

  if (loading || !form || !saved) {
    return <p className="settings-hint">Cargando ajustes de backup…</p>;
  }

  const dirty = JSON.stringify(form) !== JSON.stringify(toForm(saved));
  const valid =
    inRange(form.keep_daily, 1, 90) &&
    inRange(form.keep_weekly, 0, 52) &&
    inRange(form.keep_monthly, 0, 60) &&
    /^\d{2}:\d{2}$/.test(form.schedule_time) &&
    form.schedule_days > 0 &&
    inRange(form.warn_after_days, 1, 30) &&
    inRange(form.critical_after_days, 1, 60) &&
    Number(form.critical_after_days) > Number(form.warn_after_days) &&
    form.smtp_host.trim() !== '' &&
    inRange(form.smtp_port, 1, 65535);

  const setField =
    (field: keyof FormState) => (event: React.ChangeEvent<HTMLInputElement>) =>
      setForm(current => (current ? { ...current, [field]: event.target.value } : current));

  const saveSecret = (
    action: (value: string | null) => Promise<BackupSecretsStatus>,
    okText: string,
    removedText: string,
  ) => async (value: string | null): Promise<boolean> => {
    try {
      setSecrets(await action(value));
      onToast(value === null ? removedText : okText, 'success');
      return true;
    } catch (error) {
      onToast(errorText(error, 'No se pudo guardar la contraseña'), 'error');
      return false;
    }
  };

  const handleSelectDir = async () => {
    try {
      const selected = await openDialog({
        directory: true,
        multiple: false,
        defaultPath: form.local_dir.trim() || env?.default_dir,
      });
      if (typeof selected === 'string') {
        setForm(current => (current ? { ...current, local_dir: selected } : current));
      }
    } catch (error) {
      onToast(errorText(error, 'No se pudo abrir el selector de carpetas'), 'error');
    }
  };

  const handleSelectPgBin = async () => {
    try {
      const selected = await openDialog({
        directory: true,
        multiple: false,
        defaultPath: form.pg_bin_dir.trim() || undefined,
      });
      if (typeof selected === 'string') {
        setForm(current => (current ? { ...current, pg_bin_dir: selected } : current));
      }
    } catch (error) {
      onToast(errorText(error, 'No se pudo abrir el selector de carpetas'), 'error');
    }
  };

  const handleSave = async () => {
    if (!valid) {
      onToast(
        'Revisa los números y la programación (hay que elegir al menos un día y el nivel crítico debe ser mayor que el aviso)',
        'warning',
      );
      return;
    }
    if (
      saved.encrypt_enabled &&
      !form.encrypt_enabled &&
      !window.confirm(
        '¿Desactivar el cifrado? Los próximos backups se guardarán sin cifrar y podrían quedar expuestos si se copia o sincroniza la carpeta.',
      )
    ) {
      return;
    }
    setSaving(true);
    try {
      const updated = await updateBackupSettings({
        local_dir: form.local_dir.trim() || null,
        pg_bin_dir: form.pg_bin_dir.trim() || null,
        keep_daily: Number(form.keep_daily),
        keep_weekly: Number(form.keep_weekly),
        keep_monthly: Number(form.keep_monthly),
        auto_enabled: form.auto_enabled,
        schedule_time: form.schedule_time,
        schedule_days: form.schedule_days,
        warn_after_days: Number(form.warn_after_days),
        critical_after_days: Number(form.critical_after_days),
        encrypt_enabled: form.encrypt_enabled,
        smtp_enabled: form.smtp_enabled,
        smtp_host: form.smtp_host.trim(),
        smtp_port: Number(form.smtp_port),
        smtp_security: form.smtp_security,
        smtp_username: form.smtp_username.trim() || null,
        smtp_from_name: form.smtp_from_name.trim() || null,
        email_on_failure: form.email_on_failure,
      });
      setSaved(updated);
      setForm(toForm(updated));
      onToast('Ajustes de backup guardados', 'success');
      notifyBackupChanged();
      await refreshEnv();
    } catch (error) {
      onToast(errorText(error, 'No se pudieron guardar los ajustes de backup'), 'error');
    } finally {
      setSaving(false);
    }
  };

  const handleRunNow = async () => {
    setRunning(true);
    try {
      const record = await runBackupNow(getOperatorOrNull());
      onToast(`Backup creado (${formatSize(record.size_bytes)})`, 'success');
    } catch (error) {
      onToast(errorText(error, 'El backup falló'), 'error');
    } finally {
      setRunning(false);
      notifyBackupChanged();
      await Promise.all([refreshHistory(), refreshEnv()]);
    }
  };

  const toolsOk = env !== null && env.tools_error === null && env.dir_writable;

  return (
    <>
      <section className="settings-section">
        <h2 className="settings-section-title">Estado</h2>
        {env ? (
          <>
            <ul className="settings-hint" style={{ listStyle: 'none', padding: 0, margin: 0 }}>
              <li>Base de datos: {env.database} (PostgreSQL {env.server_version}), {formatSize(env.database_size_bytes)}</li>
              <li>
                pg_dump: {env.pg_dump_path
                  ? `${env.pg_dump_version} — ${env.pg_dump_path}`
                  : 'no encontrado'}
              </li>
              <li>
                Carpeta de backups: {env.effective_dir}{' '}
                {env.dir_writable ? '(se puede escribir)' : '(NO se puede escribir)'}
              </li>
            </ul>
            {env.tools_error && <div className="settings-notice">{env.tools_error}</div>}
          </>
        ) : (
          <p className="settings-hint">Revisando…</p>
        )}
        <div>
          <button
            type="button"
            className="btn btn-primary"
            onClick={handleRunNow}
            disabled={running || !toolsOk}
          >
            {running ? 'Backup en curso…' : 'Hacer backup ahora'}
          </button>
        </div>
      </section>

      <section className="settings-section">
        <h2 className="settings-section-title">Programación</h2>

        <div className="settings-switch-list">
          <label className="settings-switch">
            <span>Backup automático</span>
            <input
              type="checkbox"
              checked={form.auto_enabled}
              onChange={e => setForm(c => (c ? { ...c, auto_enabled: e.target.checked } : c))}
            />
            <span className="settings-switch-track" />
          </label>
        </div>

        <div className="form-field">
          <label className="form-label" htmlFor="backup-time">Hora diaria</label>
          <input
            id="backup-time"
            className="form-input"
            type="time"
            value={form.schedule_time}
            onChange={e => setForm(c => (c ? { ...c, schedule_time: e.target.value } : c))}
          />
        </div>

        <div className="form-field">
          <span className="form-label">Días</span>
          <div style={{ display: 'flex', gap: 6 }}>
            {DAY_LABELS.map((label, i) => (
              <button
                key={label + i}
                type="button"
                title={DAY_NAMES[i]}
                className={`btn ${hasDay(form.schedule_days, i) ? 'btn-primary' : 'btn-ghost'}`}
                onClick={() => setForm(c => (c ? { ...c, schedule_days: toggleDay(c.schedule_days, i) } : c))}
              >
                {label}
              </button>
            ))}
          </div>
        </div>

        <div className="form-row">
          <div className="form-field">
            <label className="form-label" htmlFor="backup-warn">Advertir tras (días sin backup)</label>
            <input id="backup-warn" className="form-input" type="number" min={1} max={30}
              value={form.warn_after_days} onChange={setField('warn_after_days')} />
          </div>
          <div className="form-field">
            <label className="form-label" htmlFor="backup-critical">Nivel crítico tras (días)</label>
            <input id="backup-critical" className="form-input" type="number" min={2} max={60}
              value={form.critical_after_days} onChange={setField('critical_after_days')} />
          </div>
        </div>

        <span className="settings-hint">
          Usa la hora del equipo. Si el PC está apagado a esa hora, el backup se hace al encenderlo
          (unos 2 minutos después de abrir la app). Con el nivel crítico se hace de inmediato, aunque no
          toque ese día.
        </span>
      </section>

      <section className="settings-section">
        <h2 className="settings-section-title">Destino y retención</h2>
        <div className="form-field">
          <label className="form-label" htmlFor="backup-dir">Carpeta de backups</label>
          <div style={{ display: 'flex', gap: '8px' }}>
            <input
              id="backup-dir"
              className="form-input"
              style={{ flex: 1 }}
              type="text"
              value={form.local_dir}
              onChange={setField('local_dir')}
              placeholder={env?.default_dir ?? 'Carpeta por defecto (Documentos\\CashFlow-Backups)'}
            />
            <button
              type="button"
              className="btn btn-ghost"
              onClick={handleSelectDir}
            >
              Examinar…
            </button>
          </div>
          <span className="settings-hint">
            Déjala vacía para usar la carpeta por defecto. Para protegerte de un fallo del disco,
            usa una unidad distinta (otro disco, USB o una carpeta de red).
          </span>
          {isOneDrivePath(form.local_dir.trim() || env?.effective_dir || '') && (
            <div className="settings-notice" role="alert">
              Esta carpeta parece estar sincronizada con OneDrive. El cifrado protege el contenido
              del backup; evita desactivarlo mientras uses este destino.
            </div>
          )}
        </div>

        <div className="form-field">
          <label className="form-label" htmlFor="backup-pgbin">Carpeta bin de PostgreSQL (opcional)</label>
          <div style={{ display: 'flex', gap: '8px' }}>
            <input
              id="backup-pgbin"
              className="form-input"
              style={{ flex: 1 }}
              type="text"
              value={form.pg_bin_dir}
              onChange={setField('pg_bin_dir')}
              placeholder="Automática (ej: C:\Program Files\PostgreSQL\18\bin)"
            />
            <button
              type="button"
              className="btn btn-ghost"
              onClick={handleSelectPgBin}
            >
              Examinar…
            </button>
          </div>
          <span className="settings-hint">
            Solo si la detección automática no encuentra pg_dump.
          </span>
        </div>

        <div className="form-row">
          <div className="form-field">
            <label className="form-label" htmlFor="backup-daily">Diarios a conservar</label>
            <input
              id="backup-daily"
              className="form-input"
              type="number"
              min={1}
              max={90}
              value={form.keep_daily}
              onChange={setField('keep_daily')}
            />
          </div>
          <div className="form-field">
            <label className="form-label" htmlFor="backup-weekly">Semanales</label>
            <input
              id="backup-weekly"
              className="form-input"
              type="number"
              min={0}
              max={52}
              value={form.keep_weekly}
              onChange={setField('keep_weekly')}
            />
          </div>
          <div className="form-field">
            <label className="form-label" htmlFor="backup-monthly">Mensuales</label>
            <input
              id="backup-monthly"
              className="form-input"
              type="number"
              min={0}
              max={60}
              value={form.keep_monthly}
              onChange={setField('keep_monthly')}
            />
          </div>
        </div>

        <div className="form-actions">
          <button
            type="button"
            className="btn btn-ghost"
            onClick={() => setForm(toForm(saved))}
            disabled={saving || !dirty}
          >
            Descartar cambios
          </button>
          <button
            type="button"
            className="btn btn-primary"
            onClick={handleSave}
            disabled={saving || !dirty}
          >
            {saving ? 'Guardando…' : 'Guardar ajustes de backup'}
          </button>
        </div>
      </section>

      <section className="settings-section">
        <h2 className="settings-section-title">Cifrado</h2>
        {form.auto_enabled && form.encrypt_enabled && !secrets.encryption_password_set && (
          <div className="settings-notice" role="alert">
            Configuración inicial: guarda una contraseña de cifrado para habilitar los backups
            automáticos. Sin ella, los backups no se crearán.
          </div>
        )}
        <div className="settings-switch-list">
          <label className="settings-switch">
            <span>Cifrar los backups (AES-256)</span>
            <input
              type="checkbox"
              checked={form.encrypt_enabled}
              onChange={e => setForm(c => (c ? { ...c, encrypt_enabled: e.target.checked } : c))}
            />
            <span className="settings-switch-track" />
          </label>
        </div>

        <SecretField
          id="backup-enc-pass"
          label="Contraseña de cifrado"
          isSet={secrets.encryption_password_set}
          onSave={saveSecret(
            setBackupEncryptionPassword,
            'Contraseña de cifrado guardada',
            'Contraseña de cifrado eliminada',
          )}
          hint="Mínimo 8 caracteres."
        />

        <div className="settings-notice">
          Anota esta contraseña en un lugar seguro fuera de este equipo. Si el PC falla y la pierdes, los
          backups cifrados NO se pueden recuperar. Si la cambias, los backups anteriores siguen
          necesitando la contraseña con la que se hicieron. Con el cifrado activo, el archivo local es
          solo la versión cifrada y es la única que se adjunta a los correos.
        </div>
      </section>

      <section className="settings-section">
        <h2 className="settings-section-title">Envío por correo (Gmail)</h2>
        <div className="settings-switch-list">
          <label className="settings-switch">
            <span>Enviar un correo al terminar cada backup</span>
            <input
              type="checkbox"
              checked={form.smtp_enabled}
              onChange={e => setForm(c => (c ? { ...c, smtp_enabled: e.target.checked } : c))}
            />
            <span className="settings-switch-track" />
          </label>
          <label className="settings-switch">
            <span>Avisar también cuando un backup falle</span>
            <input
              type="checkbox"
              checked={form.email_on_failure}
              onChange={e => setForm(c => (c ? { ...c, email_on_failure: e.target.checked } : c))}
            />
            <span className="settings-switch-track" />
          </label>
        </div>

        <div className="form-row">
          <div className="form-field">
            <label className="form-label" htmlFor="smtp-user">Cuenta de Gmail que envía</label>
            <input id="smtp-user" className="form-input" type="text" value={form.smtp_username}
              onChange={setField('smtp_username')} placeholder="micuenta@gmail.com" />
          </div>
          <div className="form-field">
            <label className="form-label" htmlFor="smtp-from">Nombre del remitente (opcional)</label>
            <input id="smtp-from" className="form-input" type="text" maxLength={100}
              value={form.smtp_from_name} onChange={setField('smtp_from_name')} placeholder="CashFlow POS" />
          </div>
        </div>

        <SecretField
          id="smtp-pass"
          label="Contraseña de aplicación"
          isSet={secrets.smtp_password_set}
          onSave={saveSecret(
            setBackupSmtpPassword,
            'Contraseña de correo guardada',
            'Contraseña de correo eliminada',
          )}
          hint="No es la contraseña normal de Gmail. En tu cuenta de Google: Seguridad → Verificación en dos pasos (actívala) → Contraseñas de aplicaciones → crea una. Pégala tal cual, con o sin espacios."
        />

        <div className="form-row">
          <div className="form-field">
            <label className="form-label" htmlFor="smtp-host">Servidor SMTP</label>
            <input id="smtp-host" className="form-input" type="text" value={form.smtp_host}
              onChange={setField('smtp_host')} />
          </div>
          <div className="form-field">
            <label className="form-label" htmlFor="smtp-port">Puerto</label>
            <input id="smtp-port" className="form-input" type="number" min={1} max={65535}
              value={form.smtp_port} onChange={setField('smtp_port')} />
          </div>
          <div className="form-field">
            <label className="form-label" htmlFor="smtp-sec">Seguridad</label>
            <select id="smtp-sec" className="form-input" value={form.smtp_security}
              onChange={e => setForm(c => (c ? { ...c, smtp_security: e.target.value as 'tls' | 'starttls' } : c))}>
              <option value="tls">TLS directo (465)</option>
              <option value="starttls">STARTTLS (587)</option>
            </select>
          </div>
        </div>
        <span className="settings-hint">
          Para Gmail déjalo en smtp.gmail.com, 465 y TLS directo. Guarda los ajustes antes de probar el envío.
        </span>
      </section>

      <section className="settings-section">
        <h2 className="settings-section-title">Destinatarios de los correos</h2>
        <BackupRecipients onToast={onToast} />
      </section>

      <section className="settings-section">
        <h2 className="settings-section-title">Restaurar</h2>
        <p className="settings-hint">
          Vuelve a un punto anterior usando un archivo de backup (de la carpeta, de un USB o el
          adjunto de un correo). Si está cifrado necesitarás la contraseña con la que se hizo.
        </p>
        <div>
          <button
            type="button"
            className="btn btn-ghost"
            onClick={() => setRestoreOpen(true)}
            disabled={running}
          >
            Restaurar desde un backup…
          </button>
        </div>
      </section>

      <section className="settings-section">
        <h2 className="settings-section-title">Historial</h2>
        <BackupHistoryList records={history} />
      </section>

      {restoreOpen && <RestoreModal onClose={() => setRestoreOpen(false)} />}
    </>
  );
};
