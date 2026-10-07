import React, {
  createContext, useCallback, useContext, useEffect, useState,
} from 'react';
import type { AppSettings } from '../types';
import { getSettings } from '../services/settings.service';
import { setMoneyConfig } from '../utils/format';

interface AppContextValue {
  settings: AppSettings;
  refreshSettings: () => Promise<void>;
  applySettings: (settings: AppSettings) => void;
}

const AppContext = createContext<AppContextValue | null>(null);

export const AppProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [error, setError] = useState<string | null>(null);

  const applySettings = useCallback((nextSettings: AppSettings) => {
    setMoneyConfig({
      currency: nextSettings.currency,
      decimals: nextSettings.currency_decimals,
    });
    setSettings(nextSettings);
  }, []);

  const refreshSettings = useCallback(async () => {
    try {
      setError(null);
      applySettings(await getSettings());
    } catch (cause) {
      setError(
        typeof cause === 'string'
          ? cause
          : cause instanceof Error
            ? cause.message
            : 'No se pudieron cargar los ajustes',
      );
    }
  }, [applySettings]);

  useEffect(() => {
    void refreshSettings();
  }, [refreshSettings]);

  if (error) {
    return (
      <div style={{ padding: 32, color: 'var(--text-primary)', fontFamily: 'var(--font-sans)' }}>
        <p>{error}</p>
        <button className="btn btn-primary" style={{ marginTop: 12 }} onClick={refreshSettings}>
          Reintentar
        </button>
      </div>
    );
  }

  if (!settings) return null;

  return (
    <AppContext.Provider value={{ settings, refreshSettings, applySettings }}>
      <React.Fragment key={`${settings.currency}-${settings.currency_decimals}`}>
        {children}
      </React.Fragment>
    </AppContext.Provider>
  );
};

export const useSettings = (): AppContextValue => {
  const context = useContext(AppContext);
  if (!context) throw new Error('useSettings debe usarse dentro de <AppProvider>');
  return context;
};