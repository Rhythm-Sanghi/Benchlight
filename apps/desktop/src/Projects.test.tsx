import { cleanup, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, expect, it, vi } from 'vitest';
import { Projects } from './Projects';
import { api } from './api';

vi.mock('./api', () => ({ api: { projects: vi.fn() } }));
afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});
it('shows project evidence and closes the inspector with Escape', async () => {
  vi.mocked(api.projects).mockResolvedValue([
    {
      name: 'fixture',
      path: 'C:\\fixture',
      languages: ['Rust'],
      evidence: ['Cargo.toml'],
      last_activity: null,
      logical_bytes: 0,
    },
  ]);
  render(<Projects revision={0} />);
  await userEvent.click(await screen.findByRole('button', { name: 'fixture' }));
  expect(screen.getByText('Cargo.toml')).toBeInTheDocument();
  const resize = screen.getByRole('separator', { name: 'Resize inspector' });
  resize.focus();
  await userEvent.keyboard('{ArrowLeft}');
  expect(resize).toHaveAttribute('aria-valuenow', '380');
  expect(screen.getByLabelText('Project inspector')).toBeInTheDocument();
  await userEvent.keyboard('{Tab}');
  expect(screen.getByLabelText('Project inspector')).toBeInTheDocument();
  await userEvent.keyboard('{Escape}');
  expect(screen.queryByLabelText('Project inspector')).not.toBeInTheDocument();
});
