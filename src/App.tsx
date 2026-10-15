import './styles/theme.css';
import { AppProvider } from './context/AppContext';
import { AppRouter } from './routes/AppRouter';
import { useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { syncDeveloperMode } from './utils/developerMode';

function App() {
  useEffect(() => {
    void invoke('show_main_window').catch((error: unknown) => {
      console.error('No se pudo mostrar la ventana principal:', error);
    });
    void syncDeveloperMode();
  }, []);

  return (
    <AppProvider>
      <AppRouter />
    </AppProvider>
  );
}

export default App;

