import {
  Children,
  useRef,
  useEffect,
  type CSSProperties,
  type ReactNode,
} from 'react';
import { useStoredWidth } from './Layout';

export function SectionHeading({
  index,
  children,
  detail,
}: {
  index?: string;
  children: ReactNode;
  detail?: ReactNode;
}) {
  return (
    <div className="section-heading">
      <h2>
        {index && (
          <span className="section-index" aria-hidden="true">
            {index}
          </span>
        )}
        {children}
      </h2>
      {detail && <span className="section-detail">{detail}</span>}
    </div>
  );
}

export function StateLabel({
  children,
  value,
}: {
  children: ReactNode;
  value: string;
}) {
  const tone = ['Protected', 'Unknown', 'partial', 'cancelled'].includes(value)
    ? 'muted'
    : ['Review', 'Changed'].includes(value)
      ? 'warning'
      : ['failed', 'error'].includes(value)
        ? 'danger'
        : 'neutral';
  return (
    <span className={`state-label state-${tone}`}>
      <span aria-hidden="true" className="state-mark" />
      {children}
    </span>
  );
}

// A single list/detail pattern keeps the table usable while the inspector grows.
// Width is presentation state only; selecting or resizing never calls the core.
export function SplitPane({
  inspecting,
  layoutId,
  children,
}: {
  inspecting: boolean;
  layoutId: string;
  children: ReactNode;
}) {
  const [width, resize] = useStoredWidth(
    `inspector:${layoutId}`,
    360,
    280,
    560,
  );
  const pane = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (inspecting && window.matchMedia?.('(max-width: 1050px)').matches)
      pane.current
        ?.querySelector<HTMLButtonElement>('.inspector button')
        ?.focus();
  }, [inspecting]);
  const drag = useRef<{ x: number; width: number } | null>(null);
  const parts = Children.toArray(children);
  return (
    <div
      ref={pane}
      className={`split ${inspecting ? 'inspecting' : ''}`}
      style={{ '--inspector-width': `${width}px` } as CSSProperties}
    >
      {parts[0]}
      {inspecting && (
        <>
          <div
            className="pane-resize"
            role="separator"
            aria-label="Resize inspector"
            aria-orientation="vertical"
            aria-valuemin={280}
            aria-valuemax={560}
            aria-valuenow={width}
            tabIndex={0}
            onKeyDown={(event) => {
              if (['ArrowLeft', 'ArrowRight'].includes(event.key)) {
                event.preventDefault();
                resize(width + (event.key === 'ArrowLeft' ? 20 : -20));
              }
            }}
            onPointerDown={(event) => {
              drag.current = { x: event.clientX, width };
              event.currentTarget.setPointerCapture(event.pointerId);
            }}
            onPointerMove={(event) => {
              if (drag.current)
                resize(drag.current.width - event.clientX + drag.current.x);
            }}
            onPointerUp={() => {
              drag.current = null;
            }}
            onPointerCancel={() => {
              drag.current = null;
            }}
          />
          {parts[1]}
        </>
      )}
    </div>
  );
}
