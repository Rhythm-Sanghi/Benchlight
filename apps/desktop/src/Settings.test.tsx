import { cleanup, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, expect, it, vi } from 'vitest';
import { Settings } from './Settings';
import { api, type AppStatus } from './api';

vi.mock('./api', () => ({
  api: {
    setTheme: vi.fn(),
    setExclusions: vi.fn(),
    setIncludeCaches: vi.fn(),
    exportLogs: vi.fn(),
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
  theme: 'light',
  scan: null,
  include_caches: false,
  exclusions: ['C:\\source\\private'],
};
it('keeps theme, exclusions, cache scope and explicit log export connected to the core', async () => {
  const perform = async (action: () => Promise<void>) => {
    await action();
  };
  render(
    <Settings
      status={status}
      busy={false}
      scanning={false}
      perform={perform}
    />,
  );
  await userEvent.selectOptions(screen.getByLabelText('Theme'), 'dark');
  expect(api.setTheme).toHaveBeenCalledWith('dark');
  await userEvent.click(screen.getByRole('checkbox'));
  expect(api.setIncludeCaches).toHaveBeenCalledWith(true);
  await userEvent.type(
    screen.getByLabelText('Exclude an absolute folder path'),
    'C:\\source\\temp',
  );
  await userEvent.click(screen.getByRole('button', { name: 'Add exclusion' }));
  expect(api.setExclusions).toHaveBeenCalledWith([
    'C:\\source\\private',
    'C:\\source\\temp',
  ]);
  await userEvent.type(
    screen.getByLabelText('New JSON file in an existing folder'),
    'C:\\Local\\log.json',
  );
  await userEvent.click(screen.getByRole('button', { name: 'Export logs' }));
  await waitFor(() =>
    expect(api.exportLogs).toHaveBeenCalledWith('C:\\Local\\log.json'),
  );
  expect(await screen.findByRole('status')).toHaveTextContent('Exported to');
});
it('locks scope changes while a scan is running', () => {
  render(
    <Settings
      status={status}
      busy={false}
      scanning={true}
      perform={async (action) => {
        await action();
      }}
    />,
  );
  expect(screen.getByRole('checkbox')).toBeDisabled();
  expect(
    screen.getByRole('button', { name: 'Remove exclusion' }),
  ).toBeDisabled();
});
