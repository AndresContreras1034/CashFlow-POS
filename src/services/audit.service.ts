import { invoke } from '@tauri-apps/api/core';
import type { AuditEvent, AuditFilterDto, PaginatedResponse } from '../types';

// Las claves de primer nivel van en camelCase; los campos dentro del filtro
// conservan snake_case. Los errores llegan como string.

export const listAuditEvents = (
  filter: AuditFilterDto = {}
): Promise<PaginatedResponse<AuditEvent>> =>
  invoke('list_audit_events', { filter });

export const getAuditEvent = (id: number): Promise<AuditEvent> =>
  invoke('get_audit_event', { id });

/** Máx. 1000 eventos, del más antiguo al más reciente. */
export const listAuditEventsByCorrelation = (
  correlationId: string
): Promise<AuditEvent[]> =>
  invoke('list_audit_events_by_correlation', { correlationId });