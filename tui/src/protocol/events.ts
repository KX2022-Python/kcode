import type { CommandDescriptor, RuntimeStatus } from './types.js';

export type EngineInput =
  | { type: 'user_submit'; id: string; text: string }
  | { type: 'slash'; id: string; command: string }
  | { type: 'command_list'; id: string; query?: string }
  | { type: 'permission_response'; id: string; decision: PermissionDecision }
  | { type: 'status'; id: string }
  | { type: 'cancel'; id?: string }
  | { type: 'exit' };

export type EngineEvent =
  | BootstrapEvent
  | CommandsEvent
  | UserSubmitEvent
  | AssistantDeltaEvent
  | ThinkingEvent
  | ToolEvent
  | StatusEvent
  | ErrorEvent
  | DoneEvent
  | CancelEvent
  | ProgressEvent
  | PermissionRequestEvent
  | PermissionStatusEvent;

export type PermissionDecision = 'allow_once' | 'allow_turn' | 'deny_once' | 'deny_turn';

export type BootstrapEvent = {
  type: 'bootstrap';
  message: string;
  status: RuntimeStatus;
};

export type CommandsEvent = {
  type: 'commands';
  id?: string;
  safetyProfile: string;
  commands: CommandDescriptor[];
};

export type UserSubmitEvent = {
  type: 'user_submit';
  id?: string;
  text: string;
  slash?: boolean;
};

export type AssistantDeltaEvent = {
  type: 'assistant_delta';
  id?: string;
  text: string;
  delta?: boolean;
};

export type ThinkingEvent = {
  type: 'thinking';
  id?: string;
  text: string;
};

export type ToolEvent = {
  type: 'tool';
  id?: string;
  event: 'call' | 'result';
  name: string;
  input?: string;
  output?: string;
  status?: string;
  isError?: boolean;
};

export type StatusEvent = {
  type: 'status';
  id?: string;
  message?: string;
  level?: 'info' | 'warning' | 'error' | 'success';
  status?: RuntimeStatus;
  usage?: {
    inputTokens?: number;
    outputTokens?: number;
    cost?: string;
  };
};

export type ErrorEvent = {
  type: 'error';
  id?: string;
  message: string;
  recoverable?: boolean;
};

export type DoneEvent = {
  type: 'done';
  id?: string;
  status: string;
  sessionId?: string;
};

export type CancelEvent = {
  type: 'cancel';
  id?: string;
  status: string;
  message?: string;
};

export type ProgressEvent = {
  type: 'progress';
  id?: string;
  phase: string;
  detail?: string;
};

export type PermissionRequestEvent = {
  type: 'permission_request';
  id: string;
  toolName: string;
  input: string;
  currentMode: string;
  requiredMode: string;
  reason?: string | null;
  status?: 'active';
};

export type PermissionStatusEvent = {
  type: 'permission';
  id: string;
  status: 'allowed' | 'denied' | 'ignored';
  decision?: PermissionDecision;
  message?: string;
};
