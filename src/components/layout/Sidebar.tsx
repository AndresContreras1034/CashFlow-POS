import React, { useCallback, useEffect, useState } from 'react';
import { NavLink, useNavigate } from 'react-router-dom';
import { Modal } from '../ui/Modal';
import { useSettings } from '../../context/AppContext';
import {
  NAVIGATION_SHORTCUTS,
  onNavigationShortcutsChanged,
  readNavigationShortcuts,
} from '../../utils/shortcuts';
import './Sidebar.css';

export const Sidebar: React.FC = () => {
  const [termsOpen, setTermsOpen] = useState(false);
  const { settings } = useSettings();
  const navigate = useNavigate();
  const [now, setNow] = useState(() => new Date());
  const [shortcuts, setShortcuts] = useState(() => {
    try {
      return readNavigationShortcuts();
    } catch (error) {
      console.error('No se pudieron leer los atajos:', error);
      return {};
    }
  });

  useEffect(() => {
    let interval: number | undefined;
    let timeout: number | undefined;
    const updateClock = () => setNow(new Date());
    const startClock = () => {
      updateClock();
      if (timeout !== undefined) window.clearTimeout(timeout);
      if (interval !== undefined) window.clearInterval(interval);
      const delayToNextSecond = 1_000 - (Date.now() % 1_000);
      timeout = window.setTimeout(() => {
        timeout = undefined;
        updateClock();
        interval = window.setInterval(updateClock, 1_000);
      }, delayToNextSecond);
    };

    startClock();
    document.addEventListener('visibilitychange', startClock);
    return () => {
      if (timeout !== undefined) window.clearTimeout(timeout);
      if (interval !== undefined) window.clearInterval(interval);
      document.removeEventListener('visibilitychange', startClock);
    };
  }, []);

  const refreshShortcuts = useCallback(() => {
    try {
      setShortcuts(readNavigationShortcuts());
    } catch (error) {
      console.error('No se pudieron leer los atajos:', error);
    }
  }, []);

  useEffect(() => onNavigationShortcutsChanged(refreshShortcuts), [refreshShortcuts]);

  useEffect(() => {
    const handleShortcut = (event: KeyboardEvent) => {
      if (event.altKey || event.ctrlKey || event.metaKey || event.shiftKey) return;
      const target = event.target;
      if (
        target instanceof HTMLElement &&
        (target.isContentEditable || /^(INPUT|TEXTAREA|SELECT)$/.test(target.tagName))
      ) {
        return;
      }

      const key = event.key.toUpperCase();
      const destination = NAVIGATION_SHORTCUTS.find(item => shortcuts[item.path] === key);
      if (!destination) return;
      event.preventDefault();
      navigate(destination.path);
    };

    window.addEventListener('keydown', handleShortcut);
    return () => window.removeEventListener('keydown', handleShortcut);
  }, [navigate, shortcuts]);

  const dateLabel = new Intl.DateTimeFormat('es-CO', {
    weekday: 'long',
    day: 'numeric',
    month: 'long',
    year: 'numeric',
    timeZone: settings.timezone,
  }).format(now);
  const timeLabel = new Intl.DateTimeFormat('es-CO', {
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    hour12: false,
    timeZone: settings.timezone,
  }).format(now);

  return (
    <aside className="sidebar">
      <div className="sidebar-logo">
        <span className="sidebar-logo-text">POS</span>
      </div>
      <div className="sidebar-date" aria-label={`Fecha: ${dateLabel}`}>
        {dateLabel}
      </div>
      <time className="sidebar-clock" aria-label={`Hora: ${timeLabel}`}>
        {timeLabel}
      </time>

      <nav className="sidebar-nav">
        {NAVIGATION_SHORTCUTS.map(item => (
          <NavLink
            key={item.path}
            to={item.path}
            className={({ isActive }) =>
              `sidebar-link ${isActive ? 'sidebar-link--active' : ''}`
            }
          >
            <span className="sidebar-link-label">{item.label}</span>
          </NavLink>
        ))}
      </nav>

      <footer className="sidebar-footer">
        <span className="sidebar-footer-credit">Desarrollado por <strong>AFCM</strong></span>
        <a className="sidebar-footer-contact" href="tel:+573012249460">
          Contacto: 301 224 9460
        </a>
        <span className="sidebar-footer-version">Versión 1.12</span>
        <span className="sidebar-footer-license">
          Licencia de uso: compra perpetua o alquiler. Incluye acceso al código fuente.
        </span>
        <button
          type="button"
          className="sidebar-terms-button"
          onClick={() => setTermsOpen(true)}
        >
          Términos y condiciones
        </button>
      </footer>

      {termsOpen && (
        <Modal title="Términos y condiciones de licencia" onClose={() => setTermsOpen(false)} width={720}>
          <div className="sidebar-terms-content">
            <p className="sidebar-terms-intro">
              Estos términos resumen las condiciones de uso de CashFlow POS. La modalidad,
              el precio, el periodo de alquiler y los datos de las partes deben quedar
              confirmados en el acuerdo de compra o alquiler.
            </p>

            <section>
              <h3>1. Modalidades y activación</h3>
              <p>
                La licencia se concede en modalidad de compra perpetua o alquiler por el
                periodo acordado. Cada licencia activa una instalación identificada por
                un código único. La activación se realiza sin conexión mediante un archivo
                firmado emitido para ese código.
              </p>
              <p>
                La compra perpetua no tiene vencimiento. El alquiler termina en la fecha
                indicada en la licencia; desde ese momento, el acceso al sistema queda
                bloqueado hasta importar una renovación válida.
              </p>
            </section>

            <section>
              <h3>2. Acceso al código fuente</h3>
              <p>
                Tanto la compra perpetua como el alquiler incluyen acceso al código fuente.
                No se ofrece una modalidad de alquiler que excluya dicho acceso. El código
                se entrega por separado, mediante el canal y en el momento acordados con
                AFCM; el archivo de activación por sí solo no contiene ni entrega los fuentes.
              </p>
              <p>
                El acceso al código fuente no autoriza a revender, publicar ni redistribuir
                el sistema o sus fuentes a terceros. Cualquier permiso adicional debe
                constar expresamente en el acuerdo de licencia.
              </p>
            </section>

            <section>
              <h3>3. Uso permitido y restricciones</h3>
              <p>
                El titular puede usar CashFlow POS en la instalación licenciada para la
                operación de su negocio. No debe copiar la activación para habilitar
                instalaciones adicionales, compartir la licencia con terceros ni alterar
                el mecanismo de firma o vencimiento para evadir estas condiciones.
                Para trasladar la licencia a otro equipo, debe coordinar la reactivación
                con AFCM.
              </p>
            </section>

            <section>
              <h3>4. Información y copias de seguridad</h3>
              <p>
                El titular es responsable de la exactitud de la información registrada,
                de proteger sus credenciales y de mantener copias de seguridad periódicas
                de sus datos. Antes de actualizar el sistema o cambiar de equipo, debe
                verificar que cuenta con una copia recuperable.
              </p>
            </section>

            <section>
              <h3>5. Soporte, cambios y disponibilidad</h3>
              <p>
                El soporte, las actualizaciones, las integraciones y los tiempos de atención
                se rigen por lo que se acuerde por escrito. La licencia no implica por sí
                sola servicios de instalación, capacitación, personalización o soporte
                permanente.
              </p>
            </section>

            <section>
              <h3>6. Aceptación y acuerdo comercial</h3>
              <p>
                La compra o el alquiler deben documentar el titular, la modalidad, el
                importe, las fechas aplicables y la forma de entrega del código fuente.
                Si existe una diferencia entre este resumen y el acuerdo firmado, prevalece
                lo pactado expresamente por las partes, dentro de la legislación aplicable.
              </p>
            </section>

            <p className="sidebar-terms-note">
              Este texto es informativo y debe revisarse y formalizarse antes de utilizarse
              como contrato comercial.
            </p>
          </div>
        </Modal>
      )}
    </aside>
  );
};