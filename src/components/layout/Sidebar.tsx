import React from 'react';
import { NavLink } from 'react-router-dom';
import posIcon from '../../assets/pos.png';
import inventoryIcon from '../../assets/inventory.png';
import sellingIcon from '../../assets/selling.png';
import historyIcon from '../../assets/history.png';
import cashIcon from '../../assets/cash-machine.png';
import clientIcon from '../../assets/client.png';
import debtIcon from '../../assets/debt.png';
import supplierIcon from '../../assets/supplier.png';
import reportIcon from '../../assets/report.png';
import settingsIcon from '../../assets/settings.png';
import './Sidebar.css';

const NAV_ITEMS = [
  { to: '/inventory',     icon: inventoryIcon, label: 'Inventario' },
  { to: '/stocktake',     icon: inventoryIcon, label: 'Toma de inventario' },
  { to: '/sales',         icon: sellingIcon,   label: 'Ventas' },
  { to: '/sales-history', icon: historyIcon,   label: 'Historial ventas' },
  { to: '/cash',          icon: cashIcon,      label: 'Caja' },
  { to: '/customers',     icon: clientIcon,    label: 'Clientes' },
  { to: '/debts',         icon: debtIcon,      label: 'Deudas' },
  { to: '/suppliers',     icon: supplierIcon,  label: 'Proveedores' },
  { to: '/reports',       icon: reportIcon,    label: 'Reportes' },
  { to: '/settings',      icon: settingsIcon,  label: 'Ajustes' },
];

export const Sidebar: React.FC = () => (
  <aside className="sidebar">
    <div className="sidebar-logo">
      <img className="sidebar-logo-icon" src={posIcon} alt="" />
      <span className="sidebar-logo-text">POS</span>
    </div>

    <nav className="sidebar-nav">
      {NAV_ITEMS.map(item => (
        <NavLink
          key={item.to}
          to={item.to}
          className={({ isActive }) =>
            `sidebar-link ${isActive ? 'sidebar-link--active' : ''}`
          }
        >
          <img className="sidebar-link-icon" src={item.icon} alt="" />
          <span className="sidebar-link-label">{item.label}</span>
        </NavLink>
      ))}
    </nav>
  </aside>
);