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
  vi.resetAllMocks();
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

function planWithItem(state: string, itemState: string): CleanupPlan {
  return {
    id: 8,
    created_at: 1,
    state,
    items: [
      {
        candidate: {
          path: 'C:\\fixture\\project\\node_modules',
          project: 'C:\\fixture\\project',
          category: 'Node dependencies',
          classification: 'Rebuildable',
          reason: 'Manifest declares dependencies.',
          evidence: ['package.json'],
          recreate: 'npm ci',
          logical_bytes: 17,
          files: 1,
          complete: true,
          modified: 1,
        },
        expected_type: 'directory',
        fingerprint: {
          logical_bytes: 17,
          files: 1,
          entries: 2,
          digest: 'fixture',
        },
        state: itemState,
        staged_path: 'C:\\fixture\\project\\.benchlight-recycle-fixture',
        message: null,
      },
    ],
  };
}

it('explains an interrupted staging state without offering another apply', async () => {
  vi.mocked(api.cleanupPlans).mockResolvedValue([
    planWithItem('applying', 'staged'),
  ]);
  render(<Cleanup paths={[]} clear={() => {}} />);
  await userEvent.click(await screen.findByRole('button', { name: 'Plan 8' }));
  expect(screen.getByText(/This cleanup was interrupted/)).toBeInTheDocument();
  expect(screen.getByText(/Recycling was not confirmed/)).toBeInTheDocument();
  expect(screen.getByText(/Staging \/ Recycle Bin name/)).toBeInTheDocument();
  expect(
    screen.getByText(/not guaranteed recovered space/),
  ).toBeInTheDocument();
  expect(
    screen.queryByRole('button', { name: 'Revalidate plan' }),
  ).not.toBeInTheDocument();
});

it('reloads a persisted failure after apply returns an error', async () => {
  const preview = planWithItem('preview', 'pending');
  const failed = planWithItem('partial', 'failed');
  vi.mocked(api.cleanupPlans)
    .mockResolvedValueOnce([preview])
    .mockResolvedValueOnce([preview])
    .mockResolvedValueOnce([failed]);
  vi.mocked(api.validateCleanupPlan).mockResolvedValue(preview);
  vi.mocked(api.applyCleanupPlan).mockRejectedValue(
    new Error('Shell operation failed'),
  );
  render(<Cleanup paths={[]} clear={() => {}} />);
  await userEvent.click(await screen.findByRole('button', { name: 'Plan 8' }));
  await userEvent.click(
    screen.getByRole('button', { name: 'Revalidate plan' }),
  );
  await userEvent.type(
    await screen.findByLabelText('To confirm, type RECYCLE 8'),
    'RECYCLE 8',
  );
  await userEvent.click(
    screen.getByRole('button', { name: 'Move to Recycle Bin' }),
  );
  expect(
    await screen.findByText(
      /Cleanup stopped without a confirmed recycle result/,
    ),
  ).toBeInTheDocument();
  expect(screen.getByText(/If it is absent, look/)).toBeInTheDocument();
  expect(screen.getByText(/benchlight-recycle-fixture/)).toBeInTheDocument();
  expect(screen.getByRole('alert')).toHaveTextContent('Shell operation failed');
  expect(
    screen.queryByRole('button', { name: 'Revalidate plan' }),
  ).not.toBeInTheDocument();
});
