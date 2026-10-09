import './styles/theme.css';
import { AppProvider } from './context/AppContext';
import { AppRouter } from './routes/AppRouter';
import { useEffect } from 'react';
import { syncDeveloperMode } from './utils/developerMode';

function App() {
  useEffect(() => {
    void syncDeveloperMode();
  }, []);

  return (
    <AppProvider>
      <AppRouter />
    </AppProvider>
  );
}

export default App;


