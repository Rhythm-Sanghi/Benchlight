import { invoke } from '@tauri-apps/api/core';

export interface AppStatus {
  version: string;
  data_directory: string;
  roots: string[];
  suggested_roots: string[];
  theme: 'system' | 'light' | 'dark';
  scan: ScanRun | null;
  include_caches: boolean;
  exclusions: string[];
}

export interface ScanRun {
  id: number;
  started_at: number;
  finished_at: number | null;
  state: string;
  directories: number;
  projects: number;
  errors: number;
  logical_bytes: number;
  skipped: number;
}
export interface Project {
  name: string;
  path: string;
  evidence: string[];
  languages: string[];
  last_activity: number | null;
  logical_bytes: number;
}
export interface ScanError {
  path: string;
  message: string;
}
export interface Candidate {
  path: string;
  project: string | null;
  category: string;
  classification: 'Protected' | 'Review' | 'Rebuildable' | 'Cache';
  reason: string;
  evidence: string[];
  recreate: string | null;
  logical_bytes: number;
  files: number;
  complete: boolean;
  modified: number | null;
}
export interface CategoryTotal {
  category: string;
  classification: string;
  logical_bytes: number;
  items: number;
}
export interface CleanupItem {
  candidate: Candidate;
  expected_type: string;
  fingerprint: {
    logical_bytes: number;
    files: number;
    entries: number;
    digest: string;
  };
  state: string;
  staged_path: string | null;
  message: string | null;
}
export interface CleanupPlan {
  id: number;
  created_at: number;
  state: string;
  items: CleanupItem[];
}
export interface Tool {
  id: string;
  name: string;
  executable: string;
  active: boolean;
  path_entry: string | null;
  path_index: number | null;
  version: string | null;
  version_source: string | null;
  error: string | null;
  observed_at: number;
  related_projects: number;
  provenance: {
    likely_source: string | null;
    confidence: string;
    evidence: string[];
  };
}
export interface ToolReport {
  tools: Tool[];
  errors: string[];
}
export interface Observation {
  value: string | null;
  error: string | null;
}
export interface Snapshot {
  project: string;
  created_at: number;
  observations: Record<string, Observation>;
  constraints: {
    tool: string;
    source: string;
    declared: string;
    current: string | null;
    result: string;
  }[];
  errors: string[];
}
export interface Comparison {
  previous: Snapshot;
  current: Snapshot;
  changes: {
    field: string;
    previous: Observation;
    current: Observation;
    state: string;
  }[];
}

export const api = {
  exportLogs: (path: string) => invoke<void>('export_logs', { path }),
  workingSnapshot: (path: string) =>
    invoke<Snapshot | null>('working_snapshot', { path }),
  markWorking: (path: string) => invoke<Snapshot>('mark_working', { path }),
  compareWorking: (path: string) =>
    invoke<Comparison>('compare_working', { path }),
  tools: () => invoke<ToolReport>('tools'),
  refreshTools: () => invoke<ToolReport>('refresh_tools'),
  toolProjects: (id: string, offset = 0) =>
    invoke<Project[]>('tool_projects', { id, offset }),
  openToolLocation: (path: string) =>
    invoke<void>('open_tool_location', { path }),
  createCleanupPlan: (paths: string[]) =>
    invoke<CleanupPlan>('create_cleanup_plan', { paths }),
  cleanupPlans: () => invoke<CleanupPlan[]>('cleanup_plans'),
  validateCleanupPlan: (id: number) =>
    invoke<CleanupPlan>('validate_cleanup_plan', { id }),
  applyCleanupPlan: (id: number, confirmation: string) =>
    invoke<CleanupPlan>('apply_cleanup_plan', { id, confirmation }),
  status: () => invoke<AppStatus>('status'),
  addRoot: (path: string) => invoke<void>('add_root', { path }),
  removeRoot: (path: string) => invoke<void>('remove_root', { path }),
  setTheme: (theme: string) => invoke<void>('set_theme', { theme }),
  startScan: () => invoke<void>('start_scan'),
  cancelScan: () => invoke<void>('cancel_scan'),
  scanStatus: () =>
    invoke<{ running: boolean; error: string | null; status: AppStatus }>(
      'scan_status',
    ),
  projects: (offset = 0, limit = 100, sort = 'name', descending = false) =>
    invoke<Project[]>('projects', { offset, limit, sort, descending }),
  scanErrors: () => invoke<ScanError[]>('scan_errors'),
  openProjectFolder: (path: string) =>
    invoke<void>('open_project_folder', { path }),
  candidates: (offset = 0, limit = 100, sort = 'size', descending = true) =>
    invoke<Candidate[]>('candidates', { offset, limit, sort, descending }),
  categoryTotals: () => invoke<CategoryTotal[]>('category_totals'),
  setIncludeCaches: (enabled: boolean) =>
    invoke<void>('set_include_caches', { enabled }),
  setExclusions: (paths: string[]) => invoke<void>('set_exclusions', { paths }),
  openStorageFolder: (path: string) =>
    invoke<void>('open_storage_folder', { path }),
};
