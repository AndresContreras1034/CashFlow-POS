import { describe, expect, it } from 'vitest';
import { formatAgo, formatDuration, formatSize, hasDay, toggleDay } from './backup';

describe('backup utils', () => {
  it('formats byte sizes correctly', () => {
    expect(formatSize(null)).toBe('—');
    expect(formatSize(512)).toBe('512 B');
    expect(formatSize(1024 * 500)).toBe('500.0 KB');
    expect(formatSize(1024 * 1024 * 1.5)).toBe('1.5 MB');
    expect(formatSize(1024 * 1024 * 1024 * 2.5)).toBe('2.50 GB');
  });

  it('formats durations correctly', () => {
    expect(formatDuration(500)).toBe('500 ms');
    expect(formatDuration(1500)).toBe('1.5 s');
    expect(formatDuration(65000)).toBe('1 min 5 s');
  });

  it('formats ago strings correctly', () => {
    expect(formatAgo(null)).toBe('nunca');
    expect(formatAgo(0)).toBe('hace menos de 1 h');
    expect(formatAgo(5)).toBe('hace 5 h');
    expect(formatAgo(24)).toBe('hace 1 día');
    expect(formatAgo(48)).toBe('hace 2 días');
  });

  it('checks and toggles weekday bitmask correctly', () => {
    const mondayOnly = 1; // 1 << 0
    expect(hasDay(mondayOnly, 0)).toBe(true);
    expect(hasDay(mondayOnly, 1)).toBe(false);

    const toggledTuesday = toggleDay(mondayOnly, 1);
    expect(hasDay(toggledTuesday, 0)).toBe(true);
    expect(hasDay(toggledTuesday, 1)).toBe(true);

    const toggledMondayOff = toggleDay(toggledTuesday, 0);
    expect(hasDay(toggledMondayOff, 0)).toBe(false);
    expect(hasDay(toggledMondayOff, 1)).toBe(true);
  });
});
