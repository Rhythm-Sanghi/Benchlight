import { useEffect, useRef, useState } from 'react';
import { api, type Project, type Tool, type ToolReport } from './api';
import {
  SortHeader,
  navigateRows,
  useContextMenu,
  focusSelectedRow,
} from './Table';
import { displayPath } from './format';
import { SplitPane } from './Interface';

export function Tools() {
  const selectedPath = useRef<string | undefined>(undefined);
  const [report, setReport] = useState<ToolReport>({ tools: [], errors: [] });
  const [selected, setSelected] = useState<Tool>();
  const [related, setRelated] = useState<Project[]>([]);
  const menu = useContextMenu();
  const [sort, setSort] = useState('name');
  const [descending, setDescending] = useState(false);
  function changeSort(field: string) {
    setSort(field);
    setDescending(field === sort ? !descending : false);
    setOffset(0);
  }
  const [offset, setOffset] = useState(0);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const [loaded, setLoaded] = useState(false);
  useEffect(() => {
    let active = true;
    void api
      .tools()
      .then((value) => {
        if (active) setReport(value);
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
  }, []);
  useEffect(() => {
    function close(event: KeyboardEvent) {
      if (event.key === 'Escape') {
        selectedPath.current = undefined;
        focusSelectedRow();
        setSelected(undefined);
        setRelated([]);
      }
    }
    window.addEventListener('keydown', close);
    return () => window.removeEventListener('keydown', close);
  }, []);
  async function refresh() {
    selectedPath.current = undefined;
    setBusy(true);
    setError('');
    try {
      setReport(await api.refreshTools());
      focusSelectedRow();
      setSelected(undefined);
      setRelated([]);
    } catch (error) {
      setError(String(error));
    } finally {
      setBusy(false);
    }
  }
  async function showProjects(offset: number) {
    if (!selected) return;
    try {
      const path = selected.executable;
      const projects = await api.toolProjects(selected.id, offset);
      if (selectedPath.current === path) {
        setRelated(projects);
        setOffset(offset);
      }
    } catch (error) {
      setError(String(error));
    }
  }
  return (
    <>
      {menu.view}
      <div className="toolbar work-toolbar">
        <p>
          Known tools on PATH. Refresh runs bounded version commands for native
          executables; script wrappers are inspected as metadata.
        </p>
        <button
          className="button-primary"
          disabled={busy}
          onClick={() => void refresh()}
        >
          Refresh tools
        </button>
      </div>
      {busy && (
        <p role="status">
          Inspecting PATH and local installation metadata. Each version command
          is limited to five seconds.
        </p>
      )}
      {!loaded && <p role="status">Reading tool observations…</p>}
      {loaded && !busy && !error && report.tools.length === 0 && (
        <p className="empty">
          No tool observations yet. Refresh tools to inspect this machine.
        </p>
      )}
      <SplitPane layoutId="tools" inspecting={!!selected}>
        <div>
          {!!report.tools.length && (
            <table
              className="data-table tools-table"
              aria-label="Installed tools"
              onKeyDown={navigateRows}
            >
              <thead>
                <tr>
                  <SortHeader
                    tableId="tools"
                    label="Tool"
                    field="name"
                    initialWidth={140}
                    sort={sort}
                    descending={descending}
                    onSort={changeSort}
                  />
                  <SortHeader
                    tableId="tools"
                    label="Version"
                    field="version"
                    initialWidth={150}
                    sort={sort}
                    descending={descending}
                    onSort={changeSort}
                  />
                  <SortHeader
                    tableId="tools"
                    label="Executable"
                    field="executable"
                    initialWidth={360}
                    sort={sort}
                    descending={descending}
                    onSort={changeSort}
                  />
                </tr>
              </thead>
              <tbody>
                {[...report.tools]
                  .sort(
                    (a, b) =>
                      String(
                        a[sort as 'name' | 'version' | 'executable'] ?? '',
                      ).localeCompare(
                        String(
                          b[sort as 'name' | 'version' | 'executable'] ?? '',
                        ),
                        undefined,
                        { numeric: true },
                      ) * (descending ? -1 : 1),
                  )
                  .map((tool) => (
                    <tr
                      key={tool.executable}
                      onContextMenu={(event) =>
                        menu.open(event, [
                          {
                            label: 'Copy path',
                            run: () =>
                              void navigator.clipboard
                                .writeText(tool.executable)
                                .catch((error) => setError(String(error))),
                          },
                          {
                            label: 'Open location',
                            run: () =>
                              void api
                                .openToolLocation(tool.executable)
                                .catch((error) => setError(String(error))),
                          },
                        ])
                      }
                      className={
                        selected?.executable === tool.executable
                          ? 'selected'
                          : ''
                      }
                    >
                      <td>
                        <button
                          className="row-button"
                          aria-expanded={
                            selected?.executable === tool.executable
                          }
                          aria-controls="tools-inspector"
                          onClick={() => {
                            selectedPath.current = tool.executable;
                            setSelected(tool);
                            setRelated([]);
                            setOffset(0);
                          }}
                        >
                          {tool.name}
                          {!tool.active && ' · shadowed'}
                        </button>
                      </td>
                      <td>
                        <code>{tool.version ?? 'Unknown'}</code>
                      </td>
                      <td>
                        <code title={displayPath(tool.executable)}>
                          {displayPath(tool.executable)}
                        </code>
                      </td>
                    </tr>
                  ))}
              </tbody>
            </table>
          )}
        </div>
        {selected && (
          <section
            id="tools-inspector"
            className="inspector"
            aria-label="Tool inspector"
          >
            <div className="toolbar">
              <h2>{selected.name}</h2>
              <button
                aria-label="Close inspector"
                onClick={() => {
                  selectedPath.current = undefined;
                  focusSelectedRow();
                  setSelected(undefined);
                }}
              >
                ×
              </button>
            </div>
            <p>
              <code>{selected.version ?? 'Version unknown'}</code>
            </p>
            <code>{displayPath(selected.executable)}</code>
            <p>
              {selected.active
                ? 'First match on process PATH'
                : 'Shadowed PATH match'}
            </p>
            <p>
              PATH entry {selected.path_index ?? 'unknown'}:{' '}
              <code>
                {selected.path_entry
                  ? displayPath(selected.path_entry)
                  : 'Not established'}
              </code>
            </p>
            {selected.version_source && (
              <p>Version evidence: {selected.version_source}</p>
            )}
            {selected.error && <p>{selected.error}</p>}
            <h3>Likely installed through</h3>
            <p>
              {selected.provenance.likely_source ?? 'Unknown'} · Confidence:{' '}
              {selected.provenance.confidence}
            </p>
            <h3>Installation evidence</h3>
            <ul>
              {selected.provenance.evidence.map((item) => (
                <li key={item}>{item}</li>
              ))}
            </ul>
            <p className="muted">
              Observed {new Date(selected.observed_at * 1000).toLocaleString()}.
              Project links come from manifests, not process tracking.
            </p>
            <button
              onClick={() =>
                void navigator.clipboard
                  .writeText(selected.executable)
                  .catch((error) => setError(String(error)))
              }
            >
              Copy path
            </button>{' '}
            <button
              onClick={() =>
                void api
                  .openToolLocation(selected.executable)
                  .catch((error) => setError(String(error)))
              }
            >
              Open location
            </button>
            <h3>
              Projects apparently using this tool: {selected.related_projects}
            </h3>
            <button onClick={() => void showProjects(0)}>
              Show projects using this
            </button>
            {related.map((project) => (
              <p className="related-project" key={project.path}>
                <strong>{project.name}</strong>
                <br />
                <code>{displayPath(project.path)}</code>
                <br />
                <button
                  onClick={() =>
                    void api
                      .openProjectFolder(project.path)
                      .catch((error) => setError(String(error)))
                  }
                >
                  Open project folder
                </button>
              </p>
            ))}
            {!!related.length && (
              <div className="toolbar pagination">
                <button
                  disabled={!offset}
                  onClick={() => void showProjects(Math.max(0, offset - 100))}
                >
                  Previous
                </button>
                <button
                  disabled={related.length < 100}
                  onClick={() => void showProjects(offset + 100)}
                >
                  Next
                </button>
              </div>
            )}
          </section>
        )}
      </SplitPane>
      {report.errors.map((error, index) => (
        <p key={index} role="alert">
          {error}
        </p>
      ))}
      {error && <p role="alert">{error}</p>}
    </>
  );
}
