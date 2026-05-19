export type UiMessage = {
  role: 'user' | 'assistant' | 'system' | 'error';
  text: string;
};

export type GoalState = {
  status: 'none' | 'active' | 'complete';
  objective?: string;
};

export type AgentProgress = {
  label: string;
  status: 'queued' | 'running' | 'complete' | 'error' | 'cancelled';
  detail?: string;
};
