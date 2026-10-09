import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, expect, it, vi } from 'vitest';
import { ScanErrors } from './ScanErrors';
import { api } from './api';
vi.mock('./api', () => ({ api: { scanErrors: vi.fn() } }));
afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});
it('distinguishes loading, an empty scan and a failed request', async () => {
  vi.mocked(api.scanErrors).mockResolvedValue([]);
  const view = render(<ScanErrors />);
  expect(screen.getByRole('status')).toHaveTextContent('Reading scan errors');
  expect(
    await screen.findByText('No errors were recorded for the last scan.'),
  ).toBeInTheDocument();
  view.unmount();
  vi.mocked(api.scanErrors).mockRejectedValue('Database unavailable');
  render(<ScanErrors />);
  expect(await screen.findByRole('alert')).toHaveTextContent(
    'Database unavailable',
  );
  expect(
    screen.queryByText('No errors were recorded for the last scan.'),
  ).not.toBeInTheDocument();
});
