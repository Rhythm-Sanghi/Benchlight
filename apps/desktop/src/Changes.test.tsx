import { cleanup, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, expect, it, vi } from 'vitest';
import { Changes } from './Changes';
import { api, type Snapshot } from './api';
vi.mock('./api', () => ({
  api: {
    projects: vi.fn(),
    workingSnapshot: vi.fn(),
    markWorking: vi.fn(),
    compareWorking: vi.fn(),
  },
}));
afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});
it('requires a working baseline and shows changed observations without a causal claim', async () => {
  const snapshot: Snapshot = {
    project: 'C:\\source\\app',
    created_at: 1,
    observations: {},
    constraints: [],
    errors: [],
  };
  vi.mocked(api.projects).mockResolvedValue([
    {
      name: 'app',
      path: snapshot.project,
      evidence: ['package.json'],
      languages: ['JavaScript'],
      last_activity: null,
      logical_bytes: 0,
    },
  ]);
  vi.mocked(api.workingSnapshot).mockResolvedValue(null);
  vi.mocked(api.markWorking).mockResolvedValue(snapshot);
  vi.mocked(api.compareWorking).mockResolvedValue({
    previous: snapshot,
    current: snapshot,
    changes: [
      {
        field: 'Tool / Node.js / version',
        previous: { value: '20.17.0', error: null },
        current: { value: '22.5.1', error: null },
        state: 'Changed',
      },
    ],
  });
  render(<Changes scanId={1} />);
  expect(
    await screen.findByText('No working state recorded for this project.'),
  ).toBeInTheDocument();
  expect(
    screen.getByRole('button', { name: 'Compare with last working state' }),
  ).toBeDisabled();
  await userEvent.click(
    screen.getByRole('button', { name: 'Mark as working' }),
  );
  await userEvent.click(
    await screen.findByRole('button', {
      name: 'Compare with last working state',
    }),
  );
  expect(await screen.findByText('22.5.1')).toBeInTheDocument();
  expect(api.compareWorking).toHaveBeenCalledWith(snapshot.project);
  expect(screen.getByText(/not proof that it caused/)).toBeInTheDocument();
});
