import { useEffect, useState } from 'react';
import { api, type Candidate, type CategoryTotal, type ScanRun } from './api';
import {
  SortHeader,
  navigateRows,
  useContextMenu,
  focusSelectedRow,
} from './Table';
import { displayPath, size } from './format';
import { Cleanup } from './Cleanup';
import { SectionHeading, SplitPane, StateLabel } from './Interface';

export function Space({
  scan,
  revision,
}: {
  scan?: ScanRun | null;
  revision: number;
}) {
  const [totals, setTotals] = useState<CategoryTotal[]>([]);
  const [candidates, setCandidates] = useState<Candidate[]>([]);
  const [selected, setSelected] = useState<Candidate>();
  const menu = useContextMenu();
  const [sort, setSort] = useState('size');
  const [descending, setDescending] = useState(true);
  function changeSort(field: string) {
    setSort(field);
    setDescending(field === sort ? !descending : false);
    setOffset(0);
  }
  const [offset, setOffset] = useState(0);
  const [error, setError] = useState('');
  const [loaded, setLoaded] = useState(false);
  const [cleanupPaths, setCleanupPaths] = useState<string[]>([]);
  useEffect(() => {
    let active = true;
    void Promise.all([
      api.categoryTotals(),
      api.candidates(offset, 100, sort, descending),
    ])
      .then(([totals, candidates]) => {
        if (active) {
          setTotals(totals);
          setCandidates(candidates);
          setError('');
        }
      })
      .catch((error) => {
        if (active) setError(String(error));
      })
      .finally(() => {
        if (active) setLoaded(true);
      });
    return () => {
      active = false;
    };
  }, [scan?.id, revision, offset, sort, descending]);
  useEffect(() => {
    function close(event: KeyboardEvent) {
      if (event.key === 'Escape') {
        focusSelectedRow();
        setSelected(undefined);
      }
    }
    window.addEventListener('keydown', close);
    return () => window.removeEventListener('keydown', close);
  }, []);
  return (
    <>
      {menu.view}
      {!scan ? (
        <p className="empty">
          No storage scan yet. Choose project folders in Settings, then scan.
        </p>
      ) : (
        <>
          <dl className="measurement">
            <div>
              <dt>Measured files</dt>
              <dd>{size(scan.logical_bytes)}</dd>
            </div>
            <div>
              <dt>Rebuildable</dt>
              <dd>
                {loaded && !error
                  ? size(
                      totals
                        .filter((item) => item.classification === 'Rebuildable')
                        .reduce((sum, item) => sum + item.logical_bytes, 0),
                    )
                  : '—'}
              </dd>
            </div>
            <div>
              <dt>Needs review</dt>
              <dd>
                {loaded && !error
                  ? size(
                      totals
                        .filter((item) => item.classification === 'Review')
                        .reduce((sum, item) => sum + item.logical_bytes, 0),
                    )
                  : '—'}
              </dd>
            </div>
          </dl>
          <p className="scan-note">
            Logical size · scan {scan.state} · {scan.skipped} links,
            placeholders or exclusions skipped. Partial sizes may be lower than
            actual usage.
          </p>
          <table className="category-table" aria-label="Storage categories">
            <thead>
              <tr>
                <th>Type</th>
                <th>Classification</th>
                <th className="numeric">Size</th>
                <th className="numeric">Items</th>
              </tr>
            </thead>
            <tbody>
              {totals.map((item) => (
                <tr key={`${item.category}-${item.classification}`}>
                  <td>{item.category}</td>
                  <td>
                    <StateLabel value={item.classification}>
                      {item.classification}
                    </StateLabel>
                  </td>
                  <td className="numeric">{size(item.logical_bytes)}</td>
                  <td className="numeric">{item.items}</td>
                </tr>
              ))}
            </tbody>
          </table>
          <SectionHeading
            detail={`${loaded ? candidates.length : '…'} on this page${cleanupPaths.length ? ` · ${cleanupPaths.length} selected for cleanup` : ''}`}
          >
            Directories
          </SectionHeading>
          <SplitPane layoutId="space" inspecting={!!selected}>
            <div>
              {!loaded ? (
                <p role="status">Reading directories…</p>
              ) : !candidates.length && !error ? (
                <p className="empty">
                  No recognized developer artifacts on this page.
                </p>
              ) : candidates.length > 0 ? (
                <table
                  className="data-table"
                  aria-label="Storage directories"
                  onKeyDown={navigateRows}
                >
                  <thead>
                    <tr>
                      <SortHeader
                        tableId="space"
                        label="Folder"
                        field="path"
                        initialWidth={240}
                        sort={sort}
                        descending={descending}
                        onSort={changeSort}
                      />
                      <SortHeader
                        tableId="space"
                        label="Size"
                        field="size"
                        initialWidth={100}
                        sort={sort}
                        descending={descending}
                        onSort={changeSort}
                      />
                      <SortHeader
                        tableId="space"
                        label="Classification"
                        field="classification"
                        initialWidth={140}
                        sort={sort}
                        descending={descending}
                        onSort={changeSort}
                      />
                    </tr>
                  </thead>
                  <tbody>
                    {candidates.map((item) => (
                      <tr
                        key={item.path}
                        onContextMenu={(event) =>
                          menu.open(event, [
                            {
                              label: 'Copy path',
                              run: () =>
                                void navigator.clipboard
                                  .writeText(item.path)
                                  .catch((error) => setError(String(error))),
                            },
                            {
                              label: 'Open folder',
                              run: () =>
                                void api
                                  .openStorageFolder(item.path)
                                  .catch((error) => setError(String(error))),
                            },
                            {
                              label: 'Add to cleanup',
                              disabled:
                                !item.complete ||
                                !['Rebuildable', 'Cache'].includes(
                                  item.classification,
                                ) ||
                                cleanupPaths.includes(item.path),
                              run: () =>
                                setCleanupPaths((paths) => [
                                  ...paths,
                                  item.path,
                                ]),
                            },
                          ])
                        }
                        className={
                          selected?.path === item.path ? 'selected' : ''
                        }
                      >
                        <td>
                          <button
                            className="row-button"
                            aria-expanded={selected?.path === item.path}
                            aria-controls="space-inspector"
                            onClick={() => setSelected(item)}
                            title={displayPath(item.path)}
                          >
                            {displayPath(item.path)
                              .split('\\')
                              .slice(-2)
                              .join('\\')}
                          </button>
                        </td>
                        <td className="numeric">{size(item.logical_bytes)}</td>
                        <td>
                          <StateLabel
                            value={
                              item.complete ? item.classification : 'partial'
                            }
                          >
                            {item.classification}
                            {!item.complete && ' · partial'}
                          </StateLabel>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              ) : null}
              <div className="toolbar pagination">
                <button
                  disabled={!offset}
                  onClick={() => setOffset((value) => Math.max(0, value - 100))}
                >
                  Previous
                </button>
                <span>
                  {candidates.length
                    ? `${offset + 1}–${offset + candidates.length}`
                    : '0 directories'}
                </span>
                <button
                  disabled={candidates.length < 100}
                  onClick={() => setOffset((value) => value + 100)}
                >
                  Next
                </button>
              </div>
            </div>
            {selected && (
              <section
                id="space-inspector"
                className="inspector"
                aria-label="Storage inspector"
              >
                <div className="toolbar">
                  <h2>{selected.category}</h2>
                  <button
                    aria-label="Close inspector"
                    onClick={() => {
                      focusSelectedRow();
                      setSelected(undefined);
                    }}
                  >
                    ×
                  </button>
                </div>
                <p className="inspector-size">{size(selected.logical_bytes)}</p>
                <code>{displayPath(selected.path)}</code>
                <h3>
                  {selected.classification}
                  {!selected.complete && ' · incomplete measurement'}
                </h3>
                <p>{selected.reason}</p>
                <h3>Evidence</h3>
                <ul>
                  {selected.evidence.map((item) => (
                    <li key={item}>
                      <code>{item}</code>
                    </li>
                  ))}
                </ul>
                {selected.recreate && (
                  <>
                    <h3>Recreate with</h3>
                    <code>{selected.recreate}</code>
                    <p>
                      <button
                        onClick={() =>
                          void navigator.clipboard
                            .writeText(selected.recreate!)
                            .catch((error) => setError(String(error)))
                        }
                      >
                        Copy command
                      </button>
                    </p>
                  </>
                )}
                <button
                  onClick={() =>
                    void navigator.clipboard
                      .writeText(selected.path)
                      .catch((error) => setError(String(error)))
                  }
                >
                  Copy path
                </button>{' '}
                <button
                  onClick={() =>
                    void api
                      .openStorageFolder(selected.path)
                      .catch((error) => setError(String(error)))
                  }
                >
                  Open folder
                </button>
                {selected.complete &&
                  ['Rebuildable', 'Cache'].includes(
                    selected.classification,
                  ) && (
                    <p>
                      <button
                        disabled={cleanupPaths.includes(selected.path)}
                        onClick={() =>
                          setCleanupPaths((paths) => [...paths, selected.path])
                        }
                      >
                        Add to cleanup
                      </button>
                    </p>
                  )}
              </section>
            )}
          </SplitPane>
          <Cleanup paths={cleanupPaths} clear={() => setCleanupPaths([])} />
        </>
      )}
      {error && <p role="alert">Couldn't read storage results: {error}</p>}
    </>
  );
}
