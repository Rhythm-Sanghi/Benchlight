import { useEffect, useState } from 'react';
import { api, type Project } from './api';
import {
  SortHeader,
  navigateRows,
  useContextMenu,
  focusSelectedRow,
} from './Table';
import { displayPath, size } from './format';
import { SplitPane } from './Interface';

export function Projects({
  scanId,
  revision,
}: {
  scanId?: number;
  revision: number;
}) {
  const [projects, setProjects] = useState<Project[]>([]);
  const [selected, setSelected] = useState<Project>();
  const menu = useContextMenu();
  const [sort, setSort] = useState('name');
  const [descending, setDescending] = useState(false);
  function changeSort(field: string) {
    setSort(field);
    setDescending(field === sort ? !descending : false);
    setOffset(0);
  }
  const [offset, setOffset] = useState(0);
  const [error, setError] = useState('');
  const [loaded, setLoaded] = useState(false);
  useEffect(() => {
    let active = true;
    void api
      .projects(offset, 100, sort, descending)
      .then((value) => {
        if (active) {
          setProjects(value);
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
  }, [offset, scanId, revision, sort, descending]);
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
      <p className="page-note">
        Detected from project manifests and Git metadata. Activity reflects
        immediate folder entries.
      </p>
      {error && <p role="alert">Couldn't read projects: {error}</p>}
      <SplitPane layoutId="projects" inspecting={!!selected}>
        <div>
          {!loaded ? (
            <p role="status">Reading projects…</p>
          ) : !projects.length && !error ? (
            <p className="empty">
              No projects in this page. Add project folders in Settings, then
              scan.
            </p>
          ) : projects.length > 0 ? (
            <table
              className="data-table"
              aria-label="Projects"
              onKeyDown={navigateRows}
            >
              <thead>
                <tr>
                  <SortHeader
                    tableId="projects"
                    label="Name"
                    field="name"
                    initialWidth={180}
                    sort={sort}
                    descending={descending}
                    onSort={changeSort}
                  />
                  <SortHeader
                    tableId="projects"
                    label="Stack"
                    field="stack"
                    initialWidth={160}
                    sort={sort}
                    descending={descending}
                    onSort={changeSort}
                  />
                  <SortHeader
                    tableId="projects"
                    label="Size"
                    field="size"
                    initialWidth={100}
                    sort={sort}
                    descending={descending}
                    onSort={changeSort}
                  />
                  <SortHeader
                    tableId="projects"
                    label="Last filesystem activity"
                    field="activity"
                    initialWidth={160}
                    sort={sort}
                    descending={descending}
                    onSort={changeSort}
                  />
                </tr>
              </thead>
              <tbody>
                {projects.map((project) => (
                  <tr
                    key={project.path}
                    onContextMenu={(event) =>
                      menu.open(event, [
                        {
                          label: 'Copy path',
                          run: () =>
                            void navigator.clipboard
                              .writeText(project.path)
                              .catch((error) => setError(String(error))),
                        },
                        {
                          label: 'Open folder',
                          run: () =>
                            void api
                              .openProjectFolder(project.path)
                              .catch((error) => setError(String(error))),
                        },
                      ])
                    }
                    onKeyDown={(event) => {
                      if (event.key === 'F10' && event.shiftKey)
                        menu.open(event, [
                          {
                            label: 'Copy path',
                            run: () =>
                              void navigator.clipboard
                                .writeText(project.path)
                                .catch((error) => setError(String(error))),
                          },
                          {
                            label: 'Open folder',
                            run: () =>
                              void api
                                .openProjectFolder(project.path)
                                .catch((error) => setError(String(error))),
                          },
                        ]);
                    }}
                    className={
                      selected?.path === project.path ? 'selected' : ''
                    }
                  >
                    <td>
                      <button
                        className="row-button"
                        aria-expanded={selected?.path === project.path}
                        aria-controls="projects-inspector"
                        onClick={() => setSelected(project)}
                      >
                        {project.name}
                      </button>
                    </td>
                    <td>{project.languages.join(' · ') || 'Git repository'}</td>
                    <td className="numeric">{size(project.logical_bytes)}</td>
                    <td className="technical">
                      {project.last_activity
                        ? new Date(
                            project.last_activity * 1000,
                          ).toLocaleDateString()
                        : 'Unknown'}
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
              {projects.length
                ? `${offset + 1}–${offset + projects.length}`
                : '0 projects'}
            </span>
            <button
              disabled={projects.length < 100}
              onClick={() => setOffset((value) => value + 100)}
            >
              Next
            </button>
          </div>
        </div>
        {selected && (
          <section
            id="projects-inspector"
            className="inspector"
            aria-label="Project inspector"
          >
            <div className="toolbar">
              <h2>{selected.name}</h2>
              <button
                onClick={() => {
                  focusSelectedRow();
                  setSelected(undefined);
                }}
                aria-label="Close inspector"
              >
                ×
              </button>
            </div>
            <code>{displayPath(selected.path)}</code>
            <h3>Detected from</h3>
            <ul>
              {selected.evidence.map((item) => (
                <li key={item}>
                  <code>{item}</code>
                </li>
              ))}
            </ul>
            <h3>Stack evidence</h3>
            <p>
              {selected.languages.join(' · ') || 'No language manifest found.'}
            </p>
            <div className="toolbar">
              <button
                onClick={() =>
                  void navigator.clipboard
                    .writeText(selected.path)
                    .catch((error) => setError(String(error)))
                }
              >
                Copy path
              </button>
              <button
                onClick={() =>
                  void api
                    .openProjectFolder(selected.path)
                    .catch((error) => setError(String(error)))
                }
              >
                Open folder
              </button>
            </div>
          </section>
        )}
      </SplitPane>
    </>
  );
}
