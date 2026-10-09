import { cleanup, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, expect, it, vi } from 'vitest';
import { Tools } from './Tools';
import { api } from './api';
vi.mock('./api', () => ({
  api: { tools: vi.fn(), refreshTools: vi.fn(), toolProjects: vi.fn() },
}));
afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});
it('shows uncertain provenance and manifest-based project relationships', async () => {
  vi.mocked(api.tools).mockResolvedValue({
    tools: [
      {
        id: 'node',
        name: 'Node.js',
        executable: 'C:\\tools\\node.exe',
        active: true,
        path_entry: 'C:\\tools',
        path_index: 0,
        version: '22.5.1',
        version_source: 'Version command',
        error: null,
        observed_at: 1,
        related_projects: 1,
        provenance: {
          likely_source: null,
          confidence: 'Unknown',
          evidence: ['Installation channel could not be established.'],
        },
      },
    ],
    errors: [],
  });
  vi.mocked(api.toolProjects).mockResolvedValue([
    {
      path: 'C:\\source\\project',
      name: 'project',
      evidence: ['package.json'],
      languages: ['JavaScript'],
      last_activity: null,
      logical_bytes: 0,
    },
  ]);
  render(<Tools />);
  await userEvent.click(await screen.findByRole('button', { name: 'Node.js' }));
  expect(screen.getByText('Unknown · Confidence: Unknown')).toBeInTheDocument();
  await userEvent.click(
    screen.getByRole('button', { name: 'Show projects using this' }),
  );
  expect(await screen.findByText('project')).toBeInTheDocument();
  await userEvent.keyboard('{Escape}');
  expect(screen.queryByLabelText('Tool inspector')).not.toBeInTheDocument();
});
