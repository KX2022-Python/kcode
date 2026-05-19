import type { EngineEvent } from '../protocol/types.js';
import type { AppState, UiMessage } from './types.js';

export function reduceEngineEvent(state: AppState, event: EngineEvent): AppState {
  switch (event.type) {
    case 'bootstrap':
      return {
        ...state,
        phase: 'idle',
        status: event.status,
        permission: { mode: event.status.permissionMode },
        messages: appendMessage(state.messages, {
          id: 'bootstrap',
          kind: 'system',
          text: event.message,
          tone: 'success',
        }),
      };
    case 'commands':
      return {
        ...state,
        commands: event.commands,
        commandSafetyProfile: event.safetyProfile,
      };
    case 'user_submit':
      return {
        ...state,
        phase: 'running',
        currentRequestId: event.id,
        messages: appendMessage(state.messages, {
          id: event.id ?? `user-${Date.now()}`,
          kind: 'user',
          text: event.text,
        }),
      };
    case 'assistant_delta':
      return {
        ...state,
        messages: appendMessage(state.messages, {
          id: event.id ?? `assistant-${Date.now()}`,
          kind: 'assistant',
          text: event.text,
        }),
      };
    case 'thinking':
      return {
        ...state,
        messages: appendMessage(state.messages, {
          id: event.id ?? `thinking-${Date.now()}`,
          kind: 'thinking',
          text: event.text,
          title: 'thinking',
        }),
      };
    case 'tool':
      return {
        ...state,
        messages: appendMessage(state.messages, toolMessage(event)),
      };
    case 'status':
      return {
        ...state,
        status: event.status ? { ...state.status, ...event.status } : state.status,
        permission: event.status?.permissionMode
          ? { mode: event.status.permissionMode }
          : state.permission,
        messages: event.message
          ? appendMessage(state.messages, {
              id: event.id ?? `status-${Date.now()}`,
              kind: 'system',
              text: event.message,
              tone: event.level ?? 'info',
            })
          : state.messages,
      };
    case 'progress':
      return {
        ...state,
        phase: 'running',
        messages: appendMessage(state.messages, {
          id: event.id ?? `progress-${Date.now()}`,
          kind: 'system',
          text: event.detail ? `${event.phase}: ${event.detail}` : event.phase,
          title: 'progress',
        }),
      };
    case 'permission_request':
      return {
        ...state,
        permission: {
          ...state.permission,
          pending: {
            id: event.id,
            toolName: event.toolName,
            input: event.input,
            currentMode: event.currentMode,
            requiredMode: event.requiredMode,
            reason: event.reason,
            status: event.status,
          },
        },
        messages: appendMessage(state.messages, {
          id: event.id ?? `permission-${Date.now()}`,
          kind: 'system',
          title: 'permission',
          text: `${event.toolName} ${event.currentMode} -> ${event.requiredMode}`,
          tone: 'warning',
        }),
      };
    case 'permission':
      return {
        ...state,
        permission:
          state.permission.pending?.id === event.id
            ? { ...state.permission, pending: undefined }
            : state.permission,
        messages: event.message
          ? appendMessage(state.messages, {
              id: event.id,
              kind: 'system',
              title: 'permission',
              text: event.message,
              tone: event.status === 'denied' ? 'warning' : 'info',
            })
          : state.messages,
      };
    case 'error':
      return {
        ...state,
        phase: event.recoverable === false ? 'error' : 'idle',
        error: event.message,
        messages: appendMessage(state.messages, {
          id: event.id ?? `error-${Date.now()}`,
          kind: 'error',
          text: event.message,
          tone: 'error',
        }),
      };
    case 'done':
      return {
        ...state,
        phase: event.status === 'exit-requested' ? 'closed' : 'idle',
        currentRequestId: undefined,
        permission: { ...state.permission, pending: undefined },
      };
    case 'cancel':
      return {
        ...state,
        phase: 'idle',
        currentRequestId: undefined,
        messages: appendMessage(state.messages, {
          id: event.id ?? `cancel-${Date.now()}`,
          kind: 'system',
          text: event.message ?? event.status,
          tone: 'warning',
        }),
      };
  }
}

function appendMessage(messages: UiMessage[], message: UiMessage): UiMessage[] {
  return [...messages, message].slice(-300);
}

function toolMessage(event: Extract<EngineEvent, { type: 'tool' }>): UiMessage {
  const text =
    event.event === 'call'
      ? event.input && event.input !== '{}'
        ? event.input
        : event.status ?? 'started'
      : event.output ?? '';
  return {
    id: event.id ?? `tool-${Date.now()}`,
    kind: 'tool',
    title: event.name,
    text,
    tone: event.isError ? 'error' : 'info',
  };
}
