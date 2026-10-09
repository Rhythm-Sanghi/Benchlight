import { useEffect, useState } from 'react';
import { api, type CleanupPlan } from './api';
import { displayPath, size } from './format';
import { SectionHeading, StateLabel } from './Interface';

export function Cleanup({
  paths,
  clear,
}: {
  paths: string[];
  clear: () => void;
}) {
  const [plans, setPlans] = useState<CleanupPlan[]>([]);
  const [selected, setSelected] = useState<CleanupPlan>();
  const [validated, setValidated] = useState(false);
  const [confirmation, setConfirmation] = useState('');
  const [busy, setBusy] = useState(false);
  const [activity, setActivity] = useState('');
  const [error, setError] = useState('');
  const [loaded, setLoaded] = useState(false);
  useEffect(() => {
    let active = true;
    void api
      .cleanupPlans()
      .then((value) => {
        if (active) setPlans(value);
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
      if (event.key === 'Escape' && !busy) {
        setSelected(undefined);
        setValidated(false);
        setConfirmation('');
      }
    }
    window.addEventListener('keydown', close);
    return () => window.removeEventListener('keydown', close);
  }, [busy]);
  async function perform(action: () => Promise<CleanupPlan>, activity: string) {
    setBusy(true);
    setActivity(activity);
    setError('');
    try {
      const value = await action();
      setSelected(value);
      setPlans(await api.cleanupPlans());
    } catch (error) {
      setError(String(error));
      setValidated(false);
    } finally {
      setBusy(false);
    }
  }
  return (
    <section aria-label="Cleanup plans" className="cleanup-section">
      <SectionHeading>Cleanup plans</SectionHeading>
      <p className="page-note">
        Rebuildable data can contain local changes. Caches may need downloading
        again. Windows may refuse recycling locked or unsupported items.
      </p>
      {!loaded && <p role="status">Reading cleanup plans…</p>}
      {loaded && !error && !paths.length && !plans.length && (
        <p className="empty">
          Select a complete Rebuildable or Cache directory above, then add it to
          cleanup.
        </p>
      )}
      {paths.length > 0 && (
        <>
          <p>{paths.length} selected directories</p>
          <ul>
            {paths.map((path) => (
              <li key={path}>
                <code>{displayPath(path)}</code>
              </li>
            ))}
          </ul>
          <button
            className="button-primary"
            disabled={busy}
            onClick={() =>
              void perform(async () => {
                const plan = await api.createCleanupPlan(paths);
                clear();
                setValidated(false);
                setConfirmation('');
                return plan;
              }, 'Creating cleanup plan…')
            }
          >
            Create plan
          </button>{' '}
          <button disabled={busy} onClick={clear}>
            Clear selection
          </button>
        </>
      )}
      {!!plans.length && (
        <div className="table-scroll">
          <table aria-label="Cleanup plan history">
            <thead>
              <tr>
                <th>Plan</th>
                <th>Created</th>
                <th>Items</th>
                <th>State</th>
              </tr>
            </thead>
            <tbody>
              {plans.map((plan) => (
                <tr key={plan.id}>
                  <td>
                    <button
                      disabled={busy}
                      className="row-button"
                      onClick={() => {
                        setSelected(plan);
                        setValidated(false);
                        setConfirmation('');
                      }}
                    >
                      Plan {plan.id}
                    </button>
                  </td>
                  <td className="technical">
                    {new Date(plan.created_at * 1000).toLocaleString()}
                  </td>
                  <td className="numeric">{plan.items.length}</td>
                  <td>
                    <StateLabel value={plan.state}>{plan.state}</StateLabel>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
      {selected && (
        <section aria-label="Cleanup review" className="cleanup-section">
          <div className="toolbar">
            <h3>
              Plan {selected.id} · {selected.state}
            </h3>
            <button
              disabled={busy}
              aria-label="Close cleanup review"
              onClick={() => {
                setSelected(undefined);
                setValidated(false);
              }}
            >
              ×
            </button>
          </div>
          <div className="table-scroll">
            <table className="cleanup-review">
              <thead>
                <tr>
                  <th>Directory</th>
                  <th>Classification</th>
                  <th className="numeric">Estimate</th>
                  <th>Result</th>
                </tr>
              </thead>
              <tbody>
                {selected.items.map((item) => (
                  <tr key={item.candidate.path}>
                    <td>
                      <code>{displayPath(item.candidate.path)}</code>
                      <p>{item.candidate.reason}</p>
                      {item.candidate.project && (
                        <small>
                          Project: {displayPath(item.candidate.project)}
                        </small>
                      )}
                      {item.message && <p>{item.message}</p>}
                      {item.staged_path && (
                        <p>
                          Recovery/Recycle Bin name:{' '}
                          <code>{displayPath(item.staged_path)}</code>
                        </p>
                      )}
                    </td>
                    <td>
                      <StateLabel value={item.candidate.classification}>
                        {item.candidate.classification}
                      </StateLabel>
                    </td>
                    <td className="numeric">
                      {size(item.fingerprint.logical_bytes)}
                    </td>
                    <td>
                      <StateLabel value={item.state}>{item.state}</StateLabel>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          {selected.state === 'preview' && (
            <>
              <p>
                Each directory is briefly renamed beside its original location,
                then sent to the Recycle Bin. If recycling fails, the result
                records its recovery path. Restoring from the Recycle Bin
                restores that staged name; rename it to the original name shown
                above. There is no permanent-delete fallback.
              </p>
              <button
                className="button-primary"
                disabled={busy}
                onClick={() =>
                  void perform(async () => {
                    const plan = await api.validateCleanupPlan(selected.id);
                    setValidated(true);
                    return plan;
                  }, 'Revalidating cleanup plan…')
                }
              >
                Revalidate plan
              </button>
              {validated && (
                <form
                  className="cleanup-confirmation"
                  onSubmit={(event) => {
                    event.preventDefault();
                    void perform(async () => {
                      setValidated(false);
                      const plan = await api.applyCleanupPlan(
                        selected.id,
                        confirmation,
                      );
                      setConfirmation('');
                      return plan;
                    }, 'Moving confirmed directories to the Recycle Bin…');
                  }}
                >
                  <label htmlFor="cleanup-confirmation">
                    To confirm, type RECYCLE {selected.id}
                  </label>
                  <div className="input-row">
                    <input
                      id="cleanup-confirmation"
                      autoComplete="off"
                      value={confirmation}
                      onChange={(event) => setConfirmation(event.target.value)}
                    />
                    <button
                      className="button-danger"
                      disabled={
                        busy || confirmation !== `RECYCLE ${selected.id}`
                      }
                    >
                      Move to Recycle Bin
                    </button>
                  </div>
                </form>
              )}
            </>
          )}
        </section>
      )}
      {busy && <p role="status">{activity}</p>}
      {error && <p role="alert">{error}</p>}
    </section>
  );
}
