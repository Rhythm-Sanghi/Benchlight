import { cleanup, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, expect, it, vi } from 'vitest';
import { SortHeader, navigateRows, useContextMenu } from './Table';
afterEach(() => {
  cleanup();
  localStorage.clear();
});
it('sorts and resizes a column using the keyboard', async () => {
  const sort = vi.fn();
  render(
    <table>
      <thead>
        <tr>
          <SortHeader
            tableId="test"
            label="Size"
            field="size"
            sort="size"
            descending={true}
            onSort={sort}
            initialWidth={120}
          />
        </tr>
      </thead>
    </table>,
  );
  await userEvent.click(screen.getByRole('button', { name: 'Size ↓' }));
  expect(sort).toHaveBeenCalledWith('size');
  const resize = screen.getByRole('separator', { name: 'Resize Size column' });
  resize.focus();
  await userEvent.keyboard('{ArrowRight}');
  expect(resize).toHaveAttribute('aria-valuenow', '130');
});
it('moves between rows and opens a keyboard context menu with focus restoration', async () => {
  const action = vi.fn();
  function Rows() {
    const menu = useContextMenu();
    return (
      <>
        {menu.view}
        <table onKeyDown={navigateRows}>
          <tbody>
            {['First', 'Second'].map((name) => (
              <tr
                key={name}
                onKeyDown={(event) => {
                  if (event.key === 'F10' && event.shiftKey)
                    menu.open(event, [{ label: 'Copy path', run: action }]);
                }}
              >
                <td>
                  <button className="row-button">{name}</button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </>
    );
  }
  render(<Rows />);
  screen.getByRole('button', { name: 'First' }).focus();
  await userEvent.keyboard('{ArrowDown}');
  expect(screen.getByRole('button', { name: 'Second' })).toHaveFocus();
  await userEvent.keyboard('{Shift>}{F10}{/Shift}');
  expect(screen.getByRole('menuitem', { name: 'Copy path' })).toHaveFocus();
  await userEvent.keyboard('{Enter}');
  expect(action).toHaveBeenCalledOnce();
  expect(screen.getByRole('button', { name: 'Second' })).toHaveFocus();
});
