import React, { useEffect } from 'react';
import './Toast.css';

export type ToastType = 'success' | 'error' | 'warning' | 'info';

export interface ToastData {
  id: string;
  message: string;
  type: ToastType;
}

const ICONS: Record<ToastType, string> = {
  success: '✓', error: '✕', warning: '⚠', info: 'ℹ',
};

export const Toast: React.FC<{ toast: ToastData; onDismiss: (id: string) => void }> = ({ toast, onDismiss }) => {
  useEffect(() => {
    const t = setTimeout(() => onDismiss(toast.id), 3500);
    return () => clearTimeout(t);
  }, [toast.id, onDismiss]);

  return (
    <div className={`toast toast--${toast.type}`}>
      <span className="toast-icon">{ICONS[toast.type]}</span>
      <span className="toast-msg">{toast.message}</span>
      <button className="toast-dismiss" onClick={() => onDismiss(toast.id)}>×</button>
    </div>
  );
};

export const ToastContainer: React.FC<{ toasts: ToastData[]; onDismiss: (id: string) => void }> = ({ toasts, onDismiss }) => (
  <div className="toast-container">
    {toasts.map(t => <Toast key={t.id} toast={t} onDismiss={onDismiss} />)}
  </div>
);