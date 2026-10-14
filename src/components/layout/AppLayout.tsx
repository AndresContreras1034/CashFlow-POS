import React from 'react';
import { Sidebar } from './Sidebar';
import { BackupAlertBanner } from '../backup/BackupAlertBanner';
import './AppLayout.css';

interface AppLayoutProps { children: React.ReactNode; }

export const AppLayout: React.FC<AppLayoutProps> = ({ children }) => (
  <div className="app-layout">
    <Sidebar />
    <main className="app-main">
      <BackupAlertBanner />
      {children}
    </main>
  </div>
);