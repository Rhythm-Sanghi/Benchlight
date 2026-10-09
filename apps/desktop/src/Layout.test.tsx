import { cleanup, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, expect, it, vi } from 'vitest';
import { useStoredWidth } from './Layout';

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  localStorage.clear();
});
function Width({ id = 'projects' }: { id?: string }) {
  const [width, resize] = useStoredWidth(id, 360, 280, 560);
  return <button onClick={() => resize(width + 20)}>{width}</button>;
}
it('restores widths after remount and keeps screens independent', async () => {
  const first = render(<Width />);
  await userEvent.click(screen.getByRole('button'));
  first.unmount();
  const other = render(<Width id="tools" />);
  expect(screen.getByRole('button')).toHaveTextContent('360');
  other.unmount();
  render(<Width />);
  expect(screen.getByRole('button')).toHaveTextContent('380');
});
it.each([
  ['invalid', 360],
  ['Infinity', 360],
  ['', 360],
  ['9999', 560],
  ['-1', 280],
])('handles stored width %s', (value, expected) => {
  localStorage.setItem('benchlight.layout.v1:projects', value);
  render(<Width />);
  expect(screen.getByRole('button')).toHaveTextContent(String(expected));
});
it('keeps resizing usable when browser storage is unavailable', async () => {
  vi.spyOn(Storage.prototype, 'getItem').mockImplementation(() => {
    throw new Error('unavailable');
  });
  vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
    throw new Error('unavailable');
  });
  render(<Width />);
  await userEvent.click(screen.getByRole('button'));
  expect(screen.getByRole('button')).toHaveTextContent('380');
});
