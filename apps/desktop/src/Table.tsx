import {
  useEffect,
  useRef,
  useState,
  type KeyboardEvent,
  type MouseEvent,
} from 'react';
import { useStoredWidth } from './Layout';

export function SortHeader({
  tableId,
  label,
  field,
  sort,
  descending,
  onSort,
  initialWidth = 160,
}: {
  tableId: string;
  label: string;
  field: string;
  sort: string;
  descending: boolean;
  onSort: (field: string) => void;
  initialWidth?: number;
}) {
  const [width, resize] = useStoredWidth(
    `column:${tableId}:${field}`,
    initialWidth,
    90,
    800,
  );
  const drag = useRef<{ x: number; width: number } | null>(null);
  return (
    <th
      style={{ width }}
      aria-sort={
        sort === field ? (descending ? 'descending' : 'ascending') : 'none'
      }
    >
      <button className="sort-button" onClick={() => onSort(field)}>
        {label}
        {sort === field ? (descending ? ' ↓' : ' ↑') : ''}
      </button>
      <span
        className="column-resize"
        role="separator"
        aria-label={`Resize ${label} column`}
        aria-orientation="vertical"
        aria-valuemin={90}
        aria-valuemax={800}
        aria-valuenow={width}
        tabIndex={0}
        onKeyDown={(event) => {
          if (event.key === 'ArrowLeft' || event.key === 'ArrowRight') {
            event.preventDefault();
            resize(width + (event.key === 'ArrowLeft' ? -10 : 10));
          }
        }}
        onPointerDown={(event) => {
          drag.current = { x: event.clientX, width };
          event.currentTarget.setPointerCapture(event.pointerId);
        }}
        onPointerMove={(event) => {
          if (drag.current)
            resize(drag.current.width + event.clientX - drag.current.x);
        }}
        onPointerUp={() => {
          drag.current = null;
        }}
        onPointerCancel={() => {
          drag.current = null;
        }}
      />
    </th>
  );
}

export function navigateRows(event: KeyboardEvent<HTMLTableElement>) {
  if (
    !['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(event.key) ||
    !(event.target instanceof HTMLElement) ||
    !event.target.classList.contains('row-button')
  )
    return;
  const rows = [
    ...event.currentTarget.querySelectorAll<HTMLButtonElement>('.row-button'),
  ];
  const index = rows.indexOf(event.target as HTMLButtonElement);
  const next =
    event.key === 'Home'
      ? 0
      : event.key === 'End'
        ? rows.length - 1
        : index + (event.key === 'ArrowDown' ? 1 : -1);
  rows[Math.max(0, Math.min(rows.length - 1, next))]?.focus();
  event.preventDefault();
}

export function focusSelectedRow() {
  document.querySelector<HTMLButtonElement>('tr.selected .row-button')?.focus();
}

interface MenuAction {
  label: string;
  run: () => void;
  disabled?: boolean;
}
export function useContextMenu() {
  const [menu, setMenu] = useState<{
    x: number;
    y: number;
    actions: MenuAction[];
  }>();
  const element = useRef<HTMLDivElement>(null);
  const origin = useRef<HTMLElement | null>(null);
  function close() {
    setMenu(undefined);
    origin.current?.focus();
  }
  useEffect(() => {
    if (!menu) return;
    element.current
      ?.querySelector<HTMLButtonElement>('button:not(:disabled)')
      ?.focus();
    function outside(event: PointerEvent) {
      if (!element.current?.contains(event.target as Node)) setMenu(undefined);
    }
    window.addEventListener('pointerdown', outside);
    return () => window.removeEventListener('pointerdown', outside);
  }, [menu]);
  function open(
    event: MouseEvent<HTMLElement> | KeyboardEvent<HTMLElement>,
    actions: MenuAction[],
  ) {
    event.preventDefault();
    event.stopPropagation();
    origin.current =
      event.currentTarget.querySelector<HTMLButtonElement>('.row-button') ??
      event.currentTarget;
    const rectangle = event.currentTarget.getBoundingClientRect();
    const x =
      'clientX' in event && event.clientX ? event.clientX : rectangle.left;
    const y =
      'clientY' in event && event.clientY ? event.clientY : rectangle.bottom;
    setMenu({
      x: Math.max(0, Math.min(x, window.innerWidth - 245)),
      y: Math.max(
        0,
        Math.min(y, window.innerHeight - actions.length * 40 - 12),
      ),
      actions,
    });
  }
  return {
    open,
    view: menu && (
      <div
        ref={element}
        className="context-menu"
        role="menu"
        style={{ left: menu.x, top: menu.y }}
        onKeyDown={(event) => {
          if (event.key === 'Escape') {
            close();
            event.stopPropagation();
          }
          if (['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(event.key)) {
            const buttons = [
              ...event.currentTarget.querySelectorAll<HTMLButtonElement>(
                'button:not(:disabled)',
              ),
            ];
            const index = buttons.indexOf(
              document.activeElement as HTMLButtonElement,
            );
            const next =
              event.key === 'Home'
                ? 0
                : event.key === 'End'
                  ? buttons.length - 1
                  : (index +
                      (event.key === 'ArrowDown' ? 1 : -1) +
                      buttons.length) %
                    buttons.length;
            buttons[next]?.focus();
            event.preventDefault();
          }
        }}
      >
        {menu.actions.map((action) => (
          <button
            role="menuitem"
            key={action.label}
            disabled={action.disabled}
            onClick={() => {
              close();
              action.run();
            }}
          >
            {action.label}
          </button>
        ))}
      </div>
    ),
  };
}
