import { useCallback, useEffect, useState } from 'react';
import { api, type AppStatus } from './api';
import { Projects } from './Projects';
import { ScanErrors } from './ScanErrors';
import { Space } from './Space';
import { Settings } from './Settings';
import { Tools } from './Tools';
import { Changes } from './Changes';
import { displayPath, size } from './format';
import { SectionHeading } from './Interface';

const pages = [
  'Overview',
  'Space',
  'Projects',
  'Tools',
  'Changes',
  'Settings',
] as const;
const descriptions: Record<string, string> = {
  Overview: 'Project folders and the last scan',
  Space: 'Measured storage and cleanup review',
  Projects: 'Manifests, sizes and filesystem activity',
  Tools: 'PATH executables and installation evidence',
  Changes: 'Compare with a working environment',
  Settings: 'Scan scope, appearance and local data',
  'Scan errors': 'Paths the scanner could not inspect',
};

function Lamp() {
  return (
    <svg aria-hidden="true" viewBox="0 0 32 32" width="26" height="26">
      <path
        d="M5 27h23M12 26V15l8-9M16 4l11 8H14z"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
      />
      <path d="M15 16h11l2 6H13z" fill="currentColor" opacity=".2" />
    </svg>
  );
}

export function App() {
  const [status, setStatus] = useState<AppStatus>();
  const [page, setPage] = useState<
    | 'Overview'
    | 'Space'
    | 'Projects'
    | 'Tools'
    | 'Changes'
    | 'Settings'
    | 'Scan errors'
  >('Overview');
  const [path, setPath] = useState('');
  const [error, setError] = useState('');
  const [busy, setBusy] = useState(false);
  const [scanning, setScanning] = useState(false);
  const [revision, setRevision] = useState(0);
  const refresh = useCallback(async () => {
    try {
      setStatus(await api.status());
      setError('');
    } catch (error) {
      setError(String(error));
    }
  }, []);
  useEffect(() => {
    let active = true;
    void api
      .status()
      .then((value) => {
        if (active) setStatus(value);
      })
      .catch((error) => {
        if (active) setError(String(error));
      });
    return () => {
      active = false;
    };
  }, []);
  useEffect(() => {
    document.documentElement.dataset.theme = status?.theme ?? 'system';
  }, [status?.theme]);
  useEffect(() => {
    if (!scanning) return;
    let active = true;
    let pending = false;
    const timer = window.setInterval(() => {
      if (pending) return;
      pending = true;
      void api
        .scanStatus()
        .then((result) => {
          if (active) {
            setStatus(result.status);
            setScanning(result.running);
            setRevision((value) => value + 1);
            if (result.error) setError(result.error);
          }
        })
        .catch((error) => {
          if (active) {
            setError(String(error));
            setScanning(false);
          }
        })
        .finally(() => {
          pending = false;
        });
    }, 500);
    return () => {
      active = false;
      window.clearInterval(timer);
    };
  }, [scanning]);

  async function perform(action: () => Promise<void>) {
    setBusy(true);
    try {
      await action();
      await refresh();
    } catch (error) {
      setError(String(error));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="application">
      <a className="skip-link" href="#main-workspace">
        Skip to workspace
      </a>
      <aside className="app-sidebar">
        <div className="brand">
          <Lamp />
          <strong>Benchlight</strong>
        </div>
        <nav aria-label="Main navigation">
          {pages.map((name, index) => (
            <button
              key={name}
              aria-current={page === name ? 'page' : undefined}
              onClick={() => setPage(name)}
            >
              <span className="nav-index" aria-hidden="true">
                {String(index + 1).padStart(2, '0')}
              </span>
              {name}
            </button>
          ))}
        </nav>
        <div className="local-note">
          <span className="local-marker" aria-hidden="true" />
          Local workspace
          <p>
            Scans run when you ask.
            <br />
            Nothing is uploaded.
          </p>
        </div>
      </aside>
      <main aria-labelledby="page-title">
        <header className="page-header">
          <div className="page-heading">
            <span className="page-index" aria-hidden="true">
              {String(
                pages.indexOf(page as (typeof pages)[number]) + 1 || 7,
              ).padStart(2, '0')}{' '}
              /
            </span>
            <div>
              <h1 id="page-title">{page}</h1>
              <p>{descriptions[page]}</p>
            </div>
          </div>
          <div className="toolbar">
            {status &&
              (scanning ? (
                <button
                  onClick={() =>
                    void api
                      .cancelScan()
                      .catch((error) => setError(String(error)))
                  }
                >
                  Cancel scan
                </button>
              ) : (
                <button
                  className="button-primary"
                  disabled={!status.roots.length || busy}
                  onClick={() =>
                    void perform(async () => {
                      await api.startScan();
                      setScanning(true);
                    })
                  }
                >
                  Scan
                </button>
              ))}
          </div>
        </header>
        <div className="workspace" id="main-workspace" tabIndex={-1}>
          {error && (
            <div role="alert" className="error">
              {error}
              <br />
              <button onClick={() => void refresh()}>Try again</button>
            </div>
          )}
          {!status && !error && <p role="status">Opening local database…</p>}
          {status && (
            <>
              {page === 'Projects' && (
                <Projects scanId={status.scan?.id} revision={revision} />
              )}
              {page === 'Tools' && <Tools />}
              {page === 'Changes' && <Changes scanId={status.scan?.id} />}
              {page === 'Space' && (
                <Space scan={status.scan} revision={revision} />
              )}
              {page === 'Scan errors' && (
                <ScanErrors scanId={status.scan?.id} />
              )}
              {page === 'Overview' && (
                <section>
                  <h2>
                    {status.roots.length
                      ? 'Project folders'
                      : 'Choose your project folders'}
                  </h2>
                  <p>
                    {status.roots.length
                      ? `${status.roots.length} project folder${status.roots.length === 1 ? '' : 's'} selected. Everything stays on this computer.`
                      : 'Choose where your projects usually live. Everything stays on this computer.'}
                  </p>
                  {status.scan && (
                    <dl className="scan-summary" aria-label="Last scan">
                      <div>
                        <dt>Last scan · {status.scan.state}</dt>
                        <dd className="technical">
                          {new Date(
                            (status.scan.finished_at ??
                              status.scan.started_at) * 1000,
                          ).toLocaleString()}
                        </dd>
                      </div>
                      <div>
                        <dt>Projects</dt>
                        <dd>{status.scan.projects}</dd>
                      </div>
                      <div>
                        <dt>Measured files</dt>
                        <dd>{size(status.scan.logical_bytes)}</dd>
                      </div>
                      <div>
                        <dt>Errors</dt>
                        <dd>{status.scan.errors}</dd>
                      </div>
                    </dl>
                  )}
                  <p className="muted">
                    Folders are only inspected when you start a scan.
                  </p>
                  <Roots
                    status={status}
                    busy={busy}
                    remove={(path) => void perform(() => api.removeRoot(path))}
                  />
                  <div className="suggested-roots">
                    {status.suggested_roots
                      .filter((root) => !status.roots.includes(root))
                      .map((root) => (
                        <button
                          key={root}
                          onClick={() => void perform(() => api.addRoot(root))}
                        >
                          Add {displayPath(root)}
                        </button>
                      ))}
                  </div>
                </section>
              )}
              {page === 'Settings' && (
                <section>
                  <SectionHeading index="01">Project folders</SectionHeading>
                  <Roots
                    status={status}
                    busy={busy}
                    remove={(path) => void perform(() => api.removeRoot(path))}
                  />
                </section>
              )}
              {(page === 'Overview' || page === 'Settings') && (
                <form
                  className="root-form"
                  onSubmit={(event) => {
                    event.preventDefault();
                    void perform(async () => {
                      await api.addRoot(path);
                      setPath('');
                    });
                  }}
                >
                  <label htmlFor="root-path">Project folder path</label>
                  <div className="input-row">
                    <input
                      id="root-path"
                      value={path}
                      onChange={(event) => setPath(event.target.value)}
                      placeholder="C:\Users\you\source"
                      spellCheck={false}
                    />
                    <button disabled={busy || !path.trim()}>Add folder</button>
                  </div>
                </form>
              )}
              {page === 'Settings' && (
                <Settings
                  status={status}
                  busy={busy}
                  scanning={scanning}
                  perform={perform}
                />
              )}
            </>
          )}
        </div>
      </main>
      <footer>
        <span className="status-context">Local · Offline</span>
        <span role="status">
          {status?.scan
            ? `Scan ${status.scan.state} · ${status.scan.projects} projects · ${size(status.scan.logical_bytes)} measured · ${status.scan.errors} errors`
            : 'No scan has been started.'}
        </span>
        {!!status?.scan?.errors && (
          <button onClick={() => setPage('Scan errors')}>Show errors</button>
        )}
        <span className="status-version">
          {import.meta.env.DEV && 'Development · '}v{status?.version ?? '0.1.0'}
        </span>
      </footer>
    </div>
  );
}

function Roots({
  status,
  busy,
  remove,
}: {
  status: AppStatus;
  busy: boolean;
  remove: (path: string) => void;
}) {
  return status.roots.length ? (
    <table className="roots-table" aria-label="Project folders">
      <thead>
        <tr>
          <th>Project folder</th>
          <th />
        </tr>
      </thead>
      <tbody>
        {status.roots.map((root) => (
          <tr key={root}>
            <td>
              <code>{displayPath(root)}</code>
            </td>
            <td>
              <button
                disabled={busy}
                className="button-quiet"
                aria-label={`Remove ${displayPath(root)}`}
                onClick={() => remove(root)}
              >
                Remove
              </button>
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  ) : (
    <p className="empty">No project folders yet.</p>
  );
}
