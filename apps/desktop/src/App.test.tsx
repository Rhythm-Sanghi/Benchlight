import { cleanup, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { App } from './App';
import { api, type AppStatus } from './api';

vi.mock('./api', () => ({
  api: {
    status: vi.fn(),
    addRoot: vi.fn(),
    removeRoot: vi.fn(),
    setTheme: vi.fn(),
  },
}));
afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});
const status: AppStatus = {
  version: '0.1.0',
  data_directory: 'C:\\Local\\Benchlight',
  roots: [],
  suggested_roots: [],
  theme: 'system',
  scan: null,
  include_caches: false,
  exclusions: [],
};

describe('first run', () => {
  it('starts empty and sends a chosen root to Rust', async () => {
    vi.mocked(api.status).mockResolvedValue(status);
    vi.mocked(api.addRoot).mockResolvedValue(undefined);
    render(<App />);
    expect(
      await screen.findByText('No project folders yet.'),
    ).toBeInTheDocument();
    await userEvent.type(
      screen.getByLabelText('Project folder path'),
      'C:\\source',
    );
    await userEvent.click(screen.getByRole('button', { name: 'Add folder' }));
    await waitFor(() => expect(api.addRoot).toHaveBeenCalledWith('C:\\source'));
  });
  it('shows backend errors rather than inventing machine data', async () => {
    vi.mocked(api.status).mockRejectedValue(
      'Windows denied access to local database.',
    );
    render(<App />);
    expect(await screen.findByRole('alert')).toHaveTextContent(
      'Windows denied access',
    );
  });
});
