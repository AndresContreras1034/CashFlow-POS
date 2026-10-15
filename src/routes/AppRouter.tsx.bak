import React from 'react';
import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom';

import { AppLayout } from '../components/layout/AppLayout';
import { LicenseGate } from '../components/licensing/LicenseGate';
import { Audit } from '../pages/Audit/Audit';
import Customers from '../pages/Customers/Customers';
import { Debts } from '../pages/Debts/Debts';
import { Developer } from '../pages/Developer/Developer';
import { Inventory } from '../pages/Inventory/Inventory';
import { ProductDetail } from '../pages/ProductDetail/ProductDetail';
import Suppliers from '../pages/Suppliers/Suppliers';
import { Settings } from '../pages/Settings/Settings';
import Cash from '../pages/Cash/Cash';
import Sales from '../pages/Sales/Sales';
import SalesHistory from '../pages/SalesHistory/SalesHistory';
import { StocktakePage } from '../pages/Stocktake/Stocktake';
import Reports from '../pages/Reports/Reports';

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
        <Route path="/customers" element={<Customers />} />

        {/* Deudas */}
        <Route path="/debts" element={<Debts />} />

        {/* Proveedores */}
        <Route path="/suppliers" element={<Suppliers />} />

        {/* Reportes */}
        <Route path="/reports" element={<Reports />} />

        {/* Auditoría */}
        <Route path="/audit" element={<Audit />} />

        {/* Developer Mode */}
        <Route path="/developer" element={<Developer />} />

        {/* Configuración */}
        <Route path="/settings" element={<Settings />} />
        </Routes>
      </AppLayout>
    </LicenseGate>
  </BrowserRouter>
);