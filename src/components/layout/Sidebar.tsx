import React from 'react';
import { NavLink } from 'react-router-dom';
import './Sidebar.css';

const NAV_ITEMS = [
  { to: '/inventory', icon: '📦', label: 'Inventario'  },
  { to: '/sales',     icon: '🧾', label: 'Ventas'      },
  { to: '/cash',      icon: '💵', label: 'Caja'        },
  { to: '/customers', icon: '👤', label: 'Clientes'    },
  { to: '/debts',     icon: '📋', label: 'Deudas'      },
  { to: '/suppliers', icon: '🚚', label: 'Proveedores' },
  { to: '/reports',   icon: '📊', label: 'Reportes'    },
  { to: '/settings',  icon: '⚙️',  label: 'Ajustes'    },
];

export const Sidebar: React.FC = () => (
  <aside className="sidebar">
    <div className="sidebar-logo">
      <span className="sidebar-logo-icon">◈</span>
      <span className="sidebar-logo-text">POS</span>
    </div>
    <nav className="sidebar-nav">
      {NAV_ITEMS.map(item => (
        <NavLink
          key={item.to}
          to={item.to}
          className={({ isActive }) => `sidebar-link ${isActive ? 'sidebar-link--active' : ''}`}
        >
          <span className="sidebar-link-icon">{item.icon}</span>
          <span className="sidebar-link-label">{item.label}</span>
        </NavLink>
      ))}
    </nav>
  </aside>
);