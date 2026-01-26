// Task status type
export type TaskStatus = 'queued' | 'running' | 'blocked' | 'needs-review' | 'merged' | 'dropped';

// Session status type
export type SessionStatus = 'spawned' | 'ready' | 'running' | 'handoff' | 'exit';

// Convoy status type
export type ConvoyStatus = 'active' | 'paused' | 'complete' | 'failed';

// Task counts by status
export interface TaskCounts {
  queued: number;
  running: number;
  blocked: number;
  needs_review: number;
  merged: number;
  dropped: number;
}

// Convoy interface
export interface Convoy {
  convoy_id: string;
  grit_issue_id: string;
  title: string;
  body: string;
  status: string;
}

// Convoy with task counts
export interface ConvoyWithCounts extends Convoy {
  task_counts: TaskCounts;
}

// Task interface
export interface Task {
  task_id: string;
  grit_issue_id: string;
  convoy_id: string;
  title: string;
  body: string;
  status: string;
}

// Session interface
export interface Session {
  session_id: string;
  task_id: string;
  grit_issue_id: string;
  engine: string;
  status: string;
  pid: number | null;
  worktree: string | null;
  started_ts: number;
  exit_code: number | null;
  exit_reason: string | null;
}

// Repository status output
export interface StatusOutput {
  schema_version: number;
  generated_ts: number;
  repo_root: string;
  convoys: ConvoyWithCounts[];
  tasks: { total: number; by_status: TaskCounts };
  sessions: Session[];
}

// Repository summary
export interface Repo {
  id: string;
  path: string;
  name: string;
}

// Mayor status
export interface MayorStatus {
  active: boolean;
  session_id?: string;
}

// Mayor message for chat display
export interface MayorMessage {
  type: 'user' | 'mayor';
  content: string;
  timestamp?: number;
}

// Mayor ask response
export interface MayorAskResponse {
  response: string[];
}

// Mayor history response
export interface MayorHistoryResponse {
  lines: string[];
}

// Session logs response
export interface SessionLogsResponse {
  lines: string[];
  has_more: boolean;
}

// API error response
export interface ApiError {
  error: string;
}

// Create convoy request
export interface CreateConvoyRequest {
  title: string;
  body?: string;
}

// Create task request
export interface CreateTaskRequest {
  convoy_id: string;
  title: string;
  body?: string;
}

// Update task request
export interface UpdateTaskRequest {
  status: string;
}

// WebSocket event types (from backend BratEvent enum)
export type BratEventType =
  | 'TaskUpdated'
  | 'SessionStarted'
  | 'SessionExited'
  | 'MergeCompleted'
  | 'MergeFailed'
  | 'MergeRolledBack'
  | 'MergeRetryScheduled';

// WebSocket event data payloads
export interface TaskUpdatedData {
  task_id: string;
  status: string;
  convoy_id: string | null;
}

export interface SessionStartedData {
  session_id: string;
  task_id: string;
  engine: string;
}

export interface SessionExitedData {
  session_id: string;
  task_id: string;
  exit_code: number;
}

export interface MergeCompletedData {
  task_id: string;
  commit_sha: string;
  branch: string;
}

export interface MergeFailedData {
  task_id: string;
  error: string;
  attempt: number;
}

export interface MergeRolledBackData {
  task_id: string;
  reset_sha: string;
  reason: string;
}

export interface MergeRetryScheduledData {
  task_id: string;
  retry_at: string;
  attempt: number;
}

// Union type for all event data
export type BratEventData =
  | { type: 'TaskUpdated'; data: TaskUpdatedData }
  | { type: 'SessionStarted'; data: SessionStartedData }
  | { type: 'SessionExited'; data: SessionExitedData }
  | { type: 'MergeCompleted'; data: MergeCompletedData }
  | { type: 'MergeFailed'; data: MergeFailedData }
  | { type: 'MergeRolledBack'; data: MergeRolledBackData }
  | { type: 'MergeRetryScheduled'; data: MergeRetryScheduledData };
