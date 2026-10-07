import React from 'react';
import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom';

import { AppLayout } from '../components/layout/AppLayout';
import { LicenseGate } from '../components/licensing/LicenseGate';
import { Inventory } from '../pages/Inventory/Inventory';
import { ProductDetail } from '../pages/ProductDetail/ProductDetail';
import { Settings } from '../pages/Settings/Settings';
import Cash from '../pages/Cash/Cash';
import Sales from '../pages/Sales/Sales';
import SalesHistory from '../pages/SalesHistory/SalesHistory';
import { StocktakePage } from '../pages/Stocktake/Stocktake';

const Placeholder: React.FC<{ name: string }> = ({ name }) => (
  <div
    style={{
      flex: 1,
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      flexDirection: 'column',
      gap: 12,
      color: 'var(--text-muted)',
      fontFamily: 'var(--font-sans)',
    }}
  >
    <span style={{ fontSize: 36, opacity: 0.3 }}>🚧</span>
    <p style={{ fontSize: 16 }}>{name} — próximamente</p>
  </div>
);

export const AppRouter: React.FC = () => (
  <BrowserRouter>
    <LicenseGate>
      <AppLayout>
        <Routes>
        <Route path="/" element={<Navigate to="/inventory" replace />} />

        {/* Inventario */}
        <Route path="/inventory" element={<Inventory />} />
        <Route path="/inventory/:id" element={<ProductDetail />} />
        <Route path="/stocktake" element={<StocktakePage />} />

        {/* Ventas */}
        <Route path="/sales" element={<Sales />} />

        {/* Historial de ventas */}
        <Route path="/sales-history" element={<SalesHistory />} />

        {/* Caja */}
        <Route path="/cash" element={<Cash />} />

        {/* Clientes */}
        <Route
          path="/customers"
          element={<Placeholder name="Clientes" />}
        />

        {/* Deudas */}
        <Route
          path="/debts"
          element={<Placeholder name="Deudas" />}
        />

        {/* Proveedores */}
        <Route
          path="/suppliers"
          element={<Placeholder name="Proveedores" />}
        />

        {/* Reportes */}
        <Route
          path="/reports"
          element={<Placeholder name="Reportes" />}
        />

        {/* Configuración */}
        <Route path="/settings" element={<Settings />} />
        </Routes>
      </AppLayout>
    </LicenseGate>
  </BrowserRouter>
);