import { useEffect, useState } from 'react';
import {
  api,
  type Comparison,
  type Observation,
  type Project,
  type Snapshot,
} from './api';
import { displayPath } from './format';
import { SectionHeading, StateLabel } from './Interface';

function observation(value: Observation) {
  return value.error ? `Unknown: ${value.error}` : (value.value ?? 'Absent');
}

export function Changes({ scanId }: { scanId?: number }) {
  const [projects, setProjects] = useState<Project[]>([]);
  const [projectsLoaded, setProjectsLoaded] = useState(false);
  const [offset, setOffset] = useState(0);
  const [path, setPath] = useState('');
  const [savedBaseline, setBaseline] = useState<Snapshot | null>(null);
  const [savedComparison, setComparison] = useState<Comparison>();
  const baseline = savedBaseline?.project === path ? savedBaseline : null;
  const comparison =
    savedComparison?.current.project === path ? savedComparison : undefined;
  const [error, setError] = useState('');
  const [busy, setBusy] = useState(false);
  const [loadedPath, setLoadedPath] = useState('');
  const loading = !!path && loadedPath !== path;
  const [showUnchanged, setShowUnchanged] = useState(false);
  useEffect(() => {
    let active = true;
    void api
      .projects(offset, 100)
      .then((value) => {
        if (active) {
          setProjects(value);
          setPath(value[0]?.path ?? '');
        }
      })
      .catch((error) => {
        if (active) setError(String(error));
      })
      .finally(() => {
        if (active) setProjectsLoaded(true);
      });
    return () => {
      active = false;
    };
  }, [offset, scanId]);
  useEffect(() => {
    let active = true;
    if (!path) return;
    void api
      .workingSnapshot(path)
      .then((value) => {
        if (active) {
          setBaseline(value);
          setError('');
        }
      })
      .catch((error) => {
        if (active) setError(String(error));
      })
      .finally(() => {
        if (active) setLoadedPath(path);
      });
    return () => {
      active = false;
    };
  }, [path]);
  async function capture(mark: boolean) {
    setBusy(true);
    setError('');
    try {
      if (mark) {
        setBaseline(await api.markWorking(path));
        setComparison(undefined);
      } else setComparison(await api.compareWorking(path));
    } catch (error) {
      setError(String(error));
    } finally {
      setBusy(false);
    }
  }
  return (
    <section>
      <p className="page-note">
        Record a project after you have checked that it works. Compare later to
        see what changed.
      </p>
      <p className="muted">
        Captures current process PATH tools, lockfile hashes and Git metadata.
        Environment checks record presence in Benchlight's process; values and
        .env files are never read. No watcher runs.
      </p>
      {error && (
        <p role="alert" className="error">
          {error}
        </p>
      )}
      {!projectsLoaded && <p role="status">Reading projects…</p>}
      {projectsLoaded && !error && !projects.length && (
        <p className="empty">
          No projects on this page. Add project folders and scan first.
        </p>
      )}
      <div className="changes-project">
        <label htmlFor="snapshot-project">Project</label>
        <select
          id="snapshot-project"
          value={path}
          disabled={busy || !projects.length}
          onChange={(event) => {
            setPath(event.target.value);
            setBaseline(null);
            setComparison(undefined);
          }}
        >
          {projects.map((project) => (
            <option key={project.path} value={project.path}>
              {project.name} — {displayPath(project.path)}
            </option>
          ))}
        </select>
      </div>
      <div className="toolbar work-toolbar">
        <div className="action-group">
          <button
            disabled={busy || loading || !path}
            onClick={() => void capture(true)}
          >
            Mark as working
          </button>{' '}
          <button
            className="button-primary"
            disabled={busy || loading || !path || !baseline}
            onClick={() => void capture(false)}
          >
            Compare with last working state
          </button>
        </div>
        <div className="action-group">
          <button
            disabled={busy || offset === 0}
            onClick={() => setOffset((value) => Math.max(0, value - 100))}
          >
            Previous projects
          </button>{' '}
          <button
            disabled={busy || projects.length < 100}
            onClick={() => setOffset((value) => value + 100)}
          >
            Next projects
          </button>
        </div>
      </div>
      {loading && <p role="status">Reading last working state…</p>}
      {busy && (
        <p role="status">
          Capturing local metadata. Each version command is limited to five
          seconds.
        </p>
      )}
      {path && !loading && !baseline && (
        <p className="empty">No working state recorded for this project.</p>
      )}
      {baseline && !loading && (
        <p className="baseline-note">
          Last marked as working:{' '}
          <code>{new Date(baseline.created_at * 1000).toLocaleString()}</code>.
          This records your assertion; Benchlight does not run the project's
          tests.
        </p>
      )}
      {comparison && (
        <>
          <SectionHeading>
            Compared{' '}
            {new Date(comparison.current.created_at * 1000).toLocaleString()}
          </SectionHeading>
          <p>
            {
              comparison.changes.filter((change) => change.state === 'Changed')
                .length
            }{' '}
            changed fields ·{' '}
            {
              comparison.changes.filter((change) => change.state === 'Unknown')
                .length
            }{' '}
            unknown fields
          </p>
          <label>
            <input
              type="checkbox"
              checked={showUnchanged}
              onChange={(event) => setShowUnchanged(event.target.checked)}
            />{' '}
            Show unchanged fields
          </label>
          <div className="table-scroll">
            <table
              className="changes-table"
              aria-label="Working state comparison"
            >
              <thead>
                <tr>
                  <th>Field</th>
                  <th>Last working state</th>
                  <th>Current</th>
                  <th>Result</th>
                </tr>
              </thead>
              <tbody>
                {comparison.changes
                  .filter(
                    (change) => showUnchanged || change.state !== 'Unchanged',
                  )
                  .map((change) => (
                    <tr key={change.field}>
                      <td>{change.field}</td>
                      <td>
                        <code>{observation(change.previous)}</code>
                      </td>
                      <td>
                        <code>{observation(change.current)}</code>
                      </td>
                      <td>
                        <StateLabel value={change.state}>
                          {change.state}
                        </StateLabel>
                      </td>
                    </tr>
                  ))}
              </tbody>
            </table>
          </div>
          {!showUnchanged &&
            comparison.changes.every(
              (change) => change.state === 'Unchanged',
            ) && (
              <p className="empty">
                No changed or unknown fields. Show unchanged fields to review
                the recorded observations.
              </p>
            )}
          <p className="muted">
            A changed field is evidence, not proof that it caused a failure.
            Missing or unreadable observations remain unknown.
          </p>
        </>
      )}
      {!!(comparison?.current ?? baseline)?.constraints.length && (
        <section>
          <SectionHeading>Runtime constraints</SectionHeading>
          <div className="table-scroll">
            <table
              className="constraints-table"
              aria-label="Runtime constraints"
            >
              <thead>
                <tr>
                  <th>Tool / source</th>
                  <th>Declared</th>
                  <th>Process PATH version</th>
                  <th>Result</th>
                </tr>
              </thead>
              <tbody>
                {(comparison?.current ?? baseline)?.constraints.map(
                  (constraint) => (
                    <tr key={`${constraint.tool}:${constraint.source}`}>
                      <td>
                        {constraint.tool}
                        <br />
                        <code>{constraint.source}</code>
                      </td>
                      <td>
                        <code>{constraint.declared}</code>
                      </td>
                      <td>
                        <code>{constraint.current ?? 'Unknown'}</code>
                      </td>
                      <td>
                        <StateLabel value={constraint.result}>
                          {constraint.result}
                        </StateLabel>
                      </td>
                    </tr>
                  ),
                )}
              </tbody>
            </table>
          </div>
        </section>
      )}
      {!!(comparison?.current ?? baseline)?.errors.length && (
        <section>
          <h3>Inspection limits and errors</h3>
          <ul>
            {(comparison?.current ?? baseline)?.errors.map((error, index) => (
              <li key={index}>{error}</li>
            ))}
          </ul>
        </section>
      )}
    </section>
  );
}
