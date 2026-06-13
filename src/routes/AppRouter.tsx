import React from 'react';
import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom';

import { AppLayout } from '../components/layout/AppLayout';
import { Inventory } from '../pages/Inventory/Inventory';
import { ProductDetail } from '../pages/ProductDetail/ProductDetail';

const Placeholder: React.FC<{ name: string }> = ({ name }) => (
  <div style={{
    flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center',
    flexDirection: 'column', gap: 12, color: 'var(--text-muted)', fontFamily: 'var(--font-sans)',
  }}>
    <span style={{ fontSize: 36, opacity: 0.3 }}>🚧</span>
    <p style={{ fontSize: 16 }}>{name} — próximamente</p>
  </div>
);

export const AppRouter: React.FC = () => (
  <BrowserRouter>
    <AppLayout>
      <Routes>
        <Route path="/" element={<Navigate to="/inventory" replace />} />
        <Route path="/inventory" element={<Inventory />} />
        <Route path="/inventory/:id" element={<ProductDetail />} />
        <Route path="/sales"     element={<Placeholder name="Ventas" />} />
        <Route path="/cash"      element={<Placeholder name="Caja" />} />
        <Route path="/customers" element={<Placeholder name="Clientes" />} />
        <Route path="/debts"     element={<Placeholder name="Deudas" />} />
        <Route path="/suppliers" element={<Placeholder name="Proveedores" />} />
        <Route path="/reports"   element={<Placeholder name="Reportes" />} />
        <Route path="/settings"  element={<Placeholder name="Ajustes" />} />
      </Routes>
    </AppLayout>
  </BrowserRouter>
);