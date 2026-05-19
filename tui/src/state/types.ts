import type { CommandDescriptor, RuntimeStatus } from '../protocol/types.js';

export type MessageKind = 'user' | 'assistant' | 'thinking' | 'tool' | 'system' | 'error';

export type UiMessage = {
  id: string;
  kind: MessageKind;
  text: string;
  title?: string;
  tone?: 'info' | 'warning' | 'error' | 'success';
};

export type RuntimePhase = 'booting' | 'idle' | 'running' | 'error' | 'closed';

export type PermissionState = {
  mode?: string;
  pending?: {
    id?: string;
    toolName: string;
    input: string;
    currentMode: string;
    requiredMode: string;
    reason?: string | null;
    status?: string;
  };
};

export type AgentState = {
  label: string;
  status: string;
  detail?: string;
};

export type GoalState = {
  status: 'none' | 'active' | 'complete' | 'unknown';
  objective?: string;
};

export type AppState = {
  phase: RuntimePhase;
  messages: UiMessage[];
  commands: CommandDescriptor[];
  commandSafetyProfile?: string;
  status: RuntimeStatus;
  permission: PermissionState;
  agents: AgentState[];
  goal: GoalState;
  currentRequestId?: string;
  error?: string;
};

export const initialState: AppState = {
  phase: 'booting',
  messages: [],
  commands: [],
  status: {},
  permission: {},
  agents: [],
  goal: { status: 'unknown' },
};
