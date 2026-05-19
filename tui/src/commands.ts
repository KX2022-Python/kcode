import { runEngine } from './engine.js';
import type { AgentProgress, GoalState, UiMessage } from './types.js';

export type CommandResult = {
  messages: UiMessage[];
  goal?: GoalState;
  agents?: AgentProgress[];
  requestPermissionDemo?: boolean;
  exit?: boolean;
};

export async function handlePrompt(text: string): Promise<CommandResult> {
  const result = await runEngine(['prompt', text]);
  return {
    messages: [
      {
        role: result.ok ? 'assistant' : 'error',
        text: result.output || statusText(result.ok, result.code),
      },
    ],
  };
}

export async function handleSlashCommand(
  raw: string,
  goal: GoalState,
): Promise<CommandResult> {
  const [name, ...rest] = raw.trim().slice(1).split(/\s+/).filter(Boolean);
  const args = rest.join(' ');
  switch (name) {
    case 'exit':
    case 'quit':
      return { exit: true, messages: [] };
    case 'goal':
      return handleGoal(args, goal);
    case 'status':
      return fromEngine(['status']);
    case 'doctor':
      return fromEngine(['doctor']);
    case 'help':
      return fromEngine(['help']);
    case 'mcp':
      return fromEngine(['mcp', ...rest]);
    case 'memory':
      return fromEngine(['--resume', 'latest', '/memory']);
    case 'agents':
      return handleAgents(rest);
    case 'permission-demo':
      return {
        requestPermissionDemo: true,
        messages: [
          {
            role: 'system',
            text: 'Permission request demo: approve with a, deny with d.',
          },
        ],
      };
    default:
      return fromEngine(['--resume', 'latest', raw]);
  }
}

function handleGoal(args: string, current: GoalState): CommandResult {
  const trimmed = args.trim();
  if (trimmed === 'clear') {
    const next = { status: 'none' as const };
    return {
      goal: next,
      messages: [{ role: 'system', text: 'Goal cleared.' }],
    };
  }
  if (trimmed === 'done') {
    const next = { ...current, status: 'complete' as const };
    return {
      goal: next,
      messages: [{ role: 'system', text: 'Goal marked complete.' }],
    };
  }
  if (trimmed.length > 0) {
    const next = { status: 'active' as const, objective: trimmed };
    return {
      goal: next,
      messages: [{ role: 'system', text: `Goal set: ${trimmed}` }],
    };
  }
  return {
    messages: [
      {
        role: 'system',
        text:
          current.status === 'none'
            ? 'Goal: none. Use /goal <objective> to set one.'
            : `Goal: ${current.status} - ${current.objective ?? ''}`,
      },
    ],
  };
}

async function handleAgents(rest: string[]): Promise<CommandResult> {
  const result = await runEngine(['agents', ...rest]);
  return {
    agents: [
      {
        label: 'agents',
        status: result.ok ? 'complete' : 'error',
        detail: result.output || statusText(result.ok, result.code),
      },
    ],
    messages: [
      {
        role: result.ok ? 'system' : 'error',
        text: result.output || statusText(result.ok, result.code),
      },
    ],
  };
}

async function fromEngine(args: string[]): Promise<CommandResult> {
  const result = await runEngine(args);
  return {
    messages: [
      {
        role: result.ok ? 'system' : 'error',
        text: result.output || statusText(result.ok, result.code),
      },
    ],
  };
}

function statusText(ok: boolean, code: number | null): string {
  if (ok) {
    return 'Command completed without output.';
  }
  return code === null ? 'Command failed to start.' : `Command failed with code ${code}.`;
}
