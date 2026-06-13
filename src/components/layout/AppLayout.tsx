import React from 'react';
import { Sidebar } from './Sidebar';
import './AppLayout.css';

interface AppLayoutProps { children: React.ReactNode; }

export const AppLayout: React.FC<AppLayoutProps> = ({ children }) => (
  <div className="app-layout">
    <Sidebar />
    <main className="app-main">{children}</main>
  </div>
);