import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, expect, it, vi } from 'vitest';
import { SplitPane } from './Interface';

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  localStorage.clear();
});
it('moves focus to a narrow inspector without changing wide-screen row focus', () => {
  vi.stubGlobal(
    'matchMedia',
    vi.fn(() => ({ matches: false })),
  );
  function Pane({ inspecting }: { inspecting: boolean }) {
    return (
      <SplitPane layoutId="test" inspecting={inspecting}>
        <button>Row</button>
        <section className="inspector" aria-label="Inspector">
          <button>Close</button>
        </section>
      </SplitPane>
    );
  }
  const view = render(<Pane inspecting={false} />);
  screen.getByRole('button', { name: 'Row' }).focus();
  view.rerender(<Pane inspecting={true} />);
  expect(screen.getByRole('button', { name: 'Row' })).toHaveFocus();
  view.rerender(<Pane inspecting={false} />);
  vi.stubGlobal(
    'matchMedia',
    vi.fn(() => ({ matches: true })),
  );
  view.rerender(<Pane inspecting={true} />);
  expect(screen.getByRole('button', { name: 'Close' })).toHaveFocus();
});
