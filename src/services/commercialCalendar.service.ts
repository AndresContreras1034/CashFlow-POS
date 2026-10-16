import { invoke } from '@tauri-apps/api/core';
import type {
  CommercialEvent,
  CommercialEventInput,
  SalesPace,
  UpcomingCommercialEvent,
} from '../types/commercialCalendar';
import type { AiSettings, CommercialContext, StoredAdvice } from '../types/insights';

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

export const getCommercialContext = () =>
  invoke<CommercialContext>('get_commercial_context');

export const getAiAdvice = (scope: string) =>
  invoke<StoredAdvice | null>('get_ai_advice', { scope });

export const generateAiAdvice = (scope: string) =>
  invoke<StoredAdvice>('generate_ai_advice', { scope });

export const getAiSettings = () =>
  invoke<AiSettings>('get_ai_settings');

export const updateAiSettings = (settings: {
  enabled: boolean;
  dailyTokenCap: number;
  apiKey: string | null;
}) => invoke<AiSettings>('update_ai_settings', settings);

export const setAiFeedback = (id: number, feedback: 'useful' | 'not_applicable') =>
  invoke<void>('set_ai_feedback', { id, feedback });
