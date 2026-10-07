import './styles/theme.css';
import { AppProvider } from './context/AppContext';
import { AppRouter } from './routes/AppRouter';

function App() {
  return (
    <AppProvider>
      <AppRouter />
    </AppProvider>
  );
}

export default App;