export type {
  AssistantDeltaEvent,
  BootstrapEvent,
  CancelEvent,
  CommandsEvent,
  DoneEvent,
  EngineEvent,
  EngineInput,
  ErrorEvent,
  PermissionDecision,
  PermissionRequestEvent,
  PermissionStatusEvent,
  ProgressEvent,
  StatusEvent,
  ThinkingEvent,
  ToolEvent,
  UserSubmitEvent,
} from './events.js';

export type RuntimeStatus = {
  model?: string;
  profile?: string;
  sessionId?: string;
  permissionMode?: string;
  supportsTools?: boolean;
  supportsStreaming?: boolean;
  messageCount?: number;
  turns?: number;
  estimatedTokens?: number;
  sessionPath?: string;
  cwd?: string | null;
  projectRoot?: string | null;
  gitBranch?: string | null;
  gitState?: string | null;
  controls?: ControlStatus;
};

export type ControlStatus = {
  model?: {
    current?: string;
    profile?: string;
  };
  permission?: {
    mode?: string;
  };
  mcp?: {
    available?: boolean;
    command?: string | null;
  };
  memory?: {
    projectFiles?: number;
    estimatedTokens?: number;
    command?: string;
  };
  agent?: {
    available?: boolean;
    tool?: string;
  };
  goal?: {
    available?: boolean;
    status?: string;
  };
};

export type CommandDescriptor = {
  id: string;
  name: string;
  usage: string;
  kind: 'prompt' | 'local' | 'local-ui';
  source: 'builtin' | 'skills' | 'plugin' | 'workflow' | 'mcp';
  scope: 'process' | 'session';
  category: string;
  description: string;
  argumentHint?: string | null;
  aliases: string[];
  resumeSupported: boolean;
  remoteSafe: boolean;
  channelSafe: boolean;
  enabled: boolean;
  wiring: 'executable' | 'informational' | 'hidden';
  experimental: boolean;
  tags: string[];
};
