export function size(bytes: number) {
  if (!Number.isFinite(bytes)) return 'Unknown';
  const units = ['B', 'KiB', 'MiB', 'GiB', 'TiB'];
  let unit = 0;
  while (bytes >= 1024 && unit < units.length - 1) {
    bytes /= 1024;
    unit++;
  }
  return `${bytes.toFixed(unit ? 1 : 0)} ${units[unit]}`;
}

export function displayPath(path: string) {
  return path.startsWith('\\\\?\\UNC\\')
    ? `\\\\${path.slice(8)}`
    : path.startsWith('\\\\?\\')
      ? path.slice(4)
      : path;
}
