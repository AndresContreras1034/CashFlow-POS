import { invoke } from '@tauri-apps/api/core';
import type {
  CommercialEvent,
  CommercialEventInput,
  SalesPace,
  UpcomingCommercialEvent,
} from '../types/commercialCalendar';

export const listCommercialEvents = () =>
  invoke<CommercialEvent[]>('list_commercial_events');

export const createCommercialEvent = (dto: CommercialEventInput) =>
  invoke<CommercialEvent>('create_commercial_event', { dto });

export const updateCommercialEvent = (id: number, dto: CommercialEventInput) =>
  invoke<CommercialEvent>('update_commercial_event', { id, dto });

export const deleteCommercialEvent = (id: number, actor: string | null) =>
  invoke<void>('delete_commercial_event', { id, actor });

export const getUpcomingCommercialEvents = (horizonDays: number) =>
  invoke<UpcomingCommercialEvent[]>('get_upcoming_commercial_events', {
    horizonDays,
  });

export const getCurrentSalesPace = (days = 5) =>
  invoke<SalesPace>('get_current_sales_pace', { days });
