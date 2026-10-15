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
    snapshotHistorySize: vi.fn(),
    removeOlderSnapshots: vi.fn(),
  },
}));
afterEach(() => {
  cleanup();
  vi.resetAllMocks();
});
it('requires exact confirmation to remove older history and retains the latest baseline', async () => {
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
      evidence: [],
      languages: [],
      last_activity: null,
      logical_bytes: 0,
    },
  ]);
  vi.mocked(api.workingSnapshot).mockResolvedValue(snapshot);
  vi.mocked(api.snapshotHistorySize).mockResolvedValue({
    records: 3,
    stored_bytes: 3072,
  });
  vi.mocked(api.removeOlderSnapshots).mockResolvedValue({
    records: 1,
    stored_bytes: 1024,
  });
  render(<Changes scanId={1} />);
  const remove = await screen.findByRole('button', {
    name: 'Remove older history',
  });
  expect(remove).toBeDisabled();
  expect(screen.getByText(/3 records/)).toBeInTheDocument();
  await userEvent.type(
    screen.getByLabelText(/To remove 2 older records/),
    'REMOVE 2 OLDER SNAPSHOTS',
  );
  expect(remove).toBeEnabled();
  await userEvent.click(remove);
  expect(api.removeOlderSnapshots).toHaveBeenCalledWith(
    snapshot.project,
    3,
    'REMOVE 2 OLDER SNAPSHOTS',
  );
  expect(await screen.findByText(/1 record ·/)).toBeInTheDocument();
  expect(screen.getByText(/Last marked as working/)).toBeInTheDocument();
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
  vi.mocked(api.snapshotHistorySize).mockResolvedValue({
    records: 0,
    stored_bytes: 0,
  });
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
