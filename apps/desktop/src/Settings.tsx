import { useState } from 'react';
import { api, type AppStatus } from './api';
import { displayPath } from './format';
import { SectionHeading } from './Interface';

export function Settings({
  status,
  busy,
  scanning,
  perform,
}: {
  status: AppStatus;
  busy: boolean;
  scanning: boolean;
  perform: (action: () => Promise<void>) => Promise<void>;
}) {
  const [excludedPath, setExcludedPath] = useState('');
  const [exportPath, setExportPath] = useState('');
  const [exported, setExported] = useState('');
  return (
    <>
      <section className="settings-section">
        <SectionHeading index="02">Export local logs</SectionHeading>
        <div>
          <p>
            Exports recent scan errors and cleanup operation records as JSON. It
            includes local paths. Review it before sharing; nothing is uploaded.
          </p>
          <form
            onSubmit={(event) => {
              event.preventDefault();
              void perform(async () => {
                await api.exportLogs(exportPath);
                setExported(exportPath);
              });
            }}
          >
            <label htmlFor="log-export-path">
              New JSON file in an existing folder
            </label>
            <div className="input-row">
              <input
                id="log-export-path"
                value={exportPath}
                onChange={(event) => setExportPath(event.target.value)}
                placeholder="C:\Users\you\Documents\benchlight-logs.json"
              />
              <button disabled={busy || !exportPath.trim()}>Export logs</button>
            </div>
          </form>
          {exported && (
            <p role="status">
              Exported to <code>{displayPath(exported)}</code>.
            </p>
          )}
        </div>
      </section>
      <section className="settings-section">
        <SectionHeading index="03">Scan scope</SectionHeading>
        <div>
          <label>
            <input
              type="checkbox"
              checked={!!status.include_caches}
              disabled={busy || scanning}
              onChange={(event) =>
                void perform(() => api.setIncludeCaches(event.target.checked))
              }
            />{' '}
            Include known per-user caches, Docker, WSL and Android storage
          </label>
          <p className="muted">
            Checks standard npm, pnpm, Yarn, Bun, pip, uv, Poetry, Cargo,
            Gradle, Maven and NuGet locations. Virtual disks remain protected.
            Custom cache locations are not guessed. This runs only when you
            scan.
          </p>
          <h3>Excluded folders</h3>
          {(status.exclusions ?? []).map((path) => (
            <p key={path} className="exclusion-row">
              <code>{displayPath(path)}</code>{' '}
              <button
                disabled={busy || scanning}
                onClick={() =>
                  void perform(() =>
                    api.setExclusions(
                      (status.exclusions ?? []).filter(
                        (value) => value !== path,
                      ),
                    ),
                  )
                }
              >
                Remove exclusion
              </button>
            </p>
          ))}
          <form
            onSubmit={(event) => {
              event.preventDefault();
              void perform(async () => {
                await api.setExclusions([
                  ...(status.exclusions ?? []),
                  excludedPath,
                ]);
                setExcludedPath('');
              });
            }}
          >
            <label htmlFor="exclude-path">
              Exclude an absolute folder path
            </label>
            <div className="input-row">
              <input
                id="exclude-path"
                value={excludedPath}
                onChange={(event) => setExcludedPath(event.target.value)}
              />
              <button disabled={busy || scanning || !excludedPath.trim()}>
                Add exclusion
              </button>
            </div>
          </form>
        </div>
      </section>
      <section className="settings-section">
        <SectionHeading index="04">Appearance</SectionHeading>
        <div>
          <label htmlFor="theme">Theme </label>
          <select
            id="theme"
            value={status.theme}
            disabled={busy}
            onChange={(event) =>
              void perform(() => api.setTheme(event.target.value))
            }
          >
            <option value="system">System</option>
            <option value="light">Light</option>
            <option value="dark">Dark</option>
          </select>
        </div>
      </section>
      <section className="settings-section">
        <SectionHeading index="05">Privacy</SectionHeading>
        <div>
          <p>
            Benchlight works locally. No account exists. No telemetry is sent.
            <br />
            No source code is uploaded. No project information leaves this
            computer.
          </p>
          <h3>Local database</h3>
          <code>{displayPath(status.data_directory)}</code>
        </div>
      </section>
    </>
  );
}
