export type KcodeEvent =
  | SessionEvent
  | AssistantTextEvent
  | ThinkingEvent
  | ToolCallEvent
  | ToolResultEvent
  | PermissionRequestEvent
  | GoalStateEvent
  | AgentProgressEvent
  | ErrorEvent;

export type SessionEvent = {
  type: 'session';
  event: 'started' | 'resumed' | 'completed';
  sessionId: string;
};

export type AssistantTextEvent = {
  type: 'assistant_text';
  text: string;
  delta?: boolean;
};

export type ThinkingEvent = {
  type: 'thinking';
  text: string;
};

export type ToolCallEvent = {
  type: 'tool_call';
  id: string;
  name: string;
  input: unknown;
};

export type ToolResultEvent = {
  type: 'tool_result';
  id: string;
  name: string;
  output: string;
  isError?: boolean;
};

export type PermissionRequestEvent = {
  type: 'permission_request';
  id: string;
  toolName: string;
  inputSummary: string;
  requiredMode: string;
};

export type GoalStateEvent = {
  type: 'goal_state';
  status: 'none' | 'active' | 'complete';
  objective?: string;
};

export type AgentProgressEvent = {
  type: 'agent_progress';
  agentId: string;
  label: string;
  status: 'queued' | 'running' | 'complete' | 'error';
  detail?: string;
};

export type ErrorEvent = {
  type: 'error';
  message: string;
  recoverable?: boolean;
};
