import { useEffect, useState } from 'react';

// Local presentation preferences contain stable UI keys and numbers, never paths.
export function useStoredWidth(
  key: string,
  initial: number,
  min: number,
  max: number,
) {
  const storageKey = `benchlight.layout.v1:${key}`;
  const clamp = (value: number) => Math.max(min, Math.min(max, value));
  const [width, setWidth] = useState(() => {
    try {
      const saved = localStorage.getItem(storageKey);
      const value = saved === null || saved.trim() === '' ? NaN : Number(saved);
      return Number.isFinite(value) ? clamp(value) : initial;
    } catch {
      return initial;
    }
  });
  useEffect(() => {
    try {
      localStorage.setItem(storageKey, String(width));
    } catch {
      // Storage may be unavailable; resizing still works for this session.
    }
  }, [storageKey, width]);
  return [width, (value: number) => setWidth(clamp(value))] as const;
}
