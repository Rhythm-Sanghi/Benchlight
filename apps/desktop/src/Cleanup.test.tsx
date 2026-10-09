import { cleanup, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, expect, it, vi } from 'vitest';
import { Cleanup } from './Cleanup';
import { api, type CleanupPlan } from './api';
vi.mock('./api', () => ({
  api: {
    cleanupPlans: vi.fn(),
    validateCleanupPlan: vi.fn(),
    applyCleanupPlan: vi.fn(),
  },
}));
afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});
it('requires revalidation and the exact plan confirmation before apply', async () => {
  const plan: CleanupPlan = {
    id: 7,
    created_at: 1,
    state: 'preview',
    items: [],
  };
  vi.mocked(api.cleanupPlans).mockResolvedValue([plan]);
  vi.mocked(api.validateCleanupPlan).mockResolvedValue(plan);
  vi.mocked(api.applyCleanupPlan).mockResolvedValue({
    ...plan,
    state: 'complete',
  });
  render(<Cleanup paths={[]} clear={() => {}} />);
  await userEvent.click(await screen.findByRole('button', { name: 'Plan 7' }));
  expect(
    screen.queryByRole('button', { name: 'Move to Recycle Bin' }),
  ).not.toBeInTheDocument();
  await userEvent.click(
    screen.getByRole('button', { name: 'Revalidate plan' }),
  );
  const input = await screen.findByLabelText('To confirm, type RECYCLE 7');
  await userEvent.type(input, 'RECYCLE 6');
  expect(
    screen.getByRole('button', { name: 'Move to Recycle Bin' }),
  ).toBeDisabled();
  expect(api.applyCleanupPlan).not.toHaveBeenCalled();
  await userEvent.clear(input);
  await userEvent.type(input, 'RECYCLE 7');
  await userEvent.click(
    screen.getByRole('button', { name: 'Move to Recycle Bin' }),
  );
  expect(api.applyCleanupPlan).toHaveBeenCalledWith(7, 'RECYCLE 7');
});
