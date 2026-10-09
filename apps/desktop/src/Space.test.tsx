import { cleanup, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, expect, it, vi } from 'vitest';
import { Space } from './Space';
import { api, type ScanRun } from './api';

vi.mock('./api', () => ({
  api: {
    candidates: vi.fn(),
    categoryTotals: vi.fn(),
    cleanupPlans: vi.fn().mockResolvedValue([]),
  },
}));
afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});
const scan: ScanRun = {
  id: 1,
  started_at: 0,
  finished_at: 1,
  state: 'complete',
  directories: 1,
  projects: 1,
  errors: 0,
  logical_bytes: 4096,
  skipped: 0,
};
it('shows classification evidence and incomplete measurements', async () => {
  vi.mocked(api.categoryTotals).mockResolvedValue([
    {
      category: 'Node dependencies',
      classification: 'Rebuildable',
      logical_bytes: 4096,
      items: 1,
    },
  ]);
  vi.mocked(api.candidates).mockResolvedValue([
    {
      path: 'C:\\fixture\\node_modules',
      project: 'C:\\fixture',
      category: 'Node dependencies',
      classification: 'Rebuildable',
      logical_bytes: 4096,
      files: 1,
      complete: false,
      modified: null,
      evidence: ['package.json'],
      reason: 'Project manifest found.',
      recreate: 'npm ci',
    },
  ]);
  render(<Space scan={scan} revision={0} />);
  await userEvent.click(
    await screen.findByRole('button', { name: 'fixture\\node_modules' }),
  );
  expect(
    screen.getByText('Rebuildable · incomplete measurement'),
  ).toBeInTheDocument();
  expect(screen.getByText('package.json')).toBeInTheDocument();
  expect(screen.getByText('npm ci')).toBeInTheDocument();
  expect(
    screen.queryByRole('button', { name: /Delete|Clean/ }),
  ).not.toBeInTheDocument();
});
