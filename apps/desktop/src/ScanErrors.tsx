import { useEffect, useState } from 'react';
import { api, type ScanError } from './api';
import { displayPath } from './format';
import { SectionHeading } from './Interface';

export function ScanErrors({ scanId }: { scanId?: number }) {
  const [errors, setErrors] = useState<ScanError[]>([]);
  const [failure, setFailure] = useState('');
  const [loaded, setLoaded] = useState(false);
  useEffect(() => {
    let active = true;
    void api
      .scanErrors()
      .then((value) => {
        if (active) setErrors(value);
      })
      .catch((error) => {
        if (active) setFailure(String(error));
      })
      .finally(() => {
        if (active) setLoaded(true);
      });
    return () => {
      active = false;
    };
  }, [scanId]);
  return (
    <section aria-label="Scan errors">
      <SectionHeading>Paths that couldn't be inspected</SectionHeading>
      <p>
        The rest of the scan continued. Up to 500 error details are retained.
      </p>
      {failure && <p role="alert">{failure}</p>}
      {!loaded && <p role="status">Reading scan errors…</p>}
      {loaded && !failure && !errors.length && (
        <p className="empty">No errors were recorded for the last scan.</p>
      )}
      {!!errors.length && (
        <div className="table-scroll">
          <table>
            <thead>
              <tr>
                <th>Path</th>
                <th>Reason</th>
              </tr>
            </thead>
            <tbody>
              {errors.map((error, index) => (
                <tr key={index}>
                  <td>
                    <code>{displayPath(error.path)}</code>
                  </td>
                  <td>{error.message}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </section>
  );
}
