import React, { useEffect, useRef, useState } from 'react';
import { Box, Text, useApp, useCursor, useStdin } from 'ink';
import stringWidth from 'string-width';
import { handlePrompt, handleSlashCommand } from './commands.js';
import type { AgentProgress, GoalState, UiMessage } from './types.js';

const initialMessages: UiMessage[] = [
  {
    role: 'system',
    text: 'Kcode TS TUI. Type /help, /status, /goal, /mcp, /memory, /permission-demo, or /exit.',
  },
];

export function App(): React.ReactNode {
  const app = useApp();
  const [messages, setMessages] = useState<UiMessage[]>(initialMessages);
  const [input, setInput] = useState('');
  const [busy, setBusy] = useState(false);
  const [goal, setGoal] = useState<GoalState>({ status: 'none' });
  const [agents, setAgents] = useState<AgentProgress[]>([]);
  const [permissionOpen, setPermissionOpen] = useState(false);
  const activeRun = useRef<AbortController | null>(null);
  const { setCursorPosition } = useCursor();
  const visibleMessageCount = Math.min(messages.length, 12);
  const agentLineCount = agents.length > 0 ? agents.length + 2 : 0;
  const permissionLineCount = permissionOpen ? 7 : 0;
  const prompt = `${busy ? 'running' : 'ready'} > `;
  setCursorPosition({
    x: stringWidth(prompt + input),
    y: 4 + visibleMessageCount + agentLineCount + permissionLineCount,
  });
  useLineInput({
    busy,
    permissionOpen,
    input,
    setInput,
    onSubmit: value =>
      submit(value, goal, app.exit, setBusy, setInput, setGoal, setAgents, setMessages, setPermissionOpen, activeRun),
    onPermissionInput: value => handlePermissionInput(value, setMessages, setPermissionOpen),
    onExit: app.exit,
  });
  useBusyCancelInput({
    busy,
    onCancel: () => {
      if (!activeRun.current || activeRun.current.signal.aborted) {
        return;
      }
      activeRun.current.abort();
      setMessages(items => [...items, { role: 'system', text: 'Cancellation requested.' }]);
    },
  });

  return (
    <Box flexDirection="column" paddingX={1}>
      <Header goal={goal} busy={busy} />
      <Box flexDirection="column" marginTop={1}>
        {messages.slice(-12).map((message, index) => (
          <MessageLine key={`${index}-${message.role}`} message={message} />
        ))}
      </Box>
      {agents.length > 0 && <AgentPanel agents={agents} />}
      {permissionOpen && <PermissionBox />}
      <Box marginTop={1}>
        <Text color={busy ? 'yellow' : 'green'}>{busy ? 'running' : 'ready'} </Text>
        <Text color="cyan">{'>'} </Text>
        <Text>{input}</Text>
      </Box>
    </Box>
  );
}

type LineInputOptions = {
  busy: boolean;
  permissionOpen: boolean;
  input: string;
  setInput: React.Dispatch<React.SetStateAction<string>>;
  onSubmit: (value: string) => void;
  onPermissionInput: (value: string) => void;
  onExit: () => void;
};

function useLineInput({
  busy,
  permissionOpen,
  input,
  setInput,
  onSubmit,
  onPermissionInput,
  onExit,
}: LineInputOptions): void {
  const { stdin, setRawMode, isRawModeSupported } = useStdin();

  useEffect(() => {
    if (isRawModeSupported) {
      setRawMode(false);
    }
  }, [isRawModeSupported, setRawMode]);

  useEffect(() => {
    if (busy) {
      return;
    }
    // Keep idle input in canonical terminal mode so IMEs and scrollback keep working.
    stdin.setEncoding('utf8');
    const handleData = (chunk: string) => {
      if (isTerminalControlSequence(chunk)) {
        return;
      }
      if (chunk === '\u0003') {
        onExit();
        return;
      }
      if (permissionOpen) {
        onPermissionInput(chunk.trim());
        return;
      }
      if (chunk === '\u007f' || chunk === '\b') {
        setInput(value => Array.from(value).slice(0, -1).join(''));
        return;
      }
      if (chunk.includes('\r') || chunk.includes('\n')) {
        const submitted = input + chunk.replace(/[\r\n]+$/g, '');
        setInput('');
        onSubmit(submitted);
        return;
      }
      setInput(value => value + chunk);
    };

    stdin.on('data', handleData);
    stdin.resume();
    return () => {
      stdin.off('data', handleData);
    };
  }, [busy, input, onExit, onPermissionInput, onSubmit, permissionOpen, setInput, stdin]);
}

function isTerminalControlSequence(value: string): boolean {
  return value.startsWith('\u001B[') || value.startsWith('\u001B]');
}

function useBusyCancelInput({ busy, onCancel }: { busy: boolean; onCancel: () => void }): void {
  const { stdin, setRawMode, isRawModeSupported } = useStdin();

  useEffect(() => {
    if (!busy) {
      return;
    }
    stdin.setEncoding('utf8');
    if (isRawModeSupported) {
      setRawMode(true);
    }
    const handleData = (chunk: string) => {
      if (chunk === '\u001B' || chunk === '\u0003') {
        onCancel();
      }
    };
    stdin.on('data', handleData);
    stdin.resume();
    return () => {
      stdin.off('data', handleData);
      if (isRawModeSupported) {
        setRawMode(false);
      }
    };
  }, [busy, isRawModeSupported, onCancel, setRawMode, stdin]);
}

function Header({ goal, busy }: { goal: GoalState; busy: boolean }): React.ReactNode {
  const goalText = goal.status === 'none' ? 'goal: none' : `goal: ${goal.status} - ${goal.objective ?? ''}`;
  return (
    <Box flexDirection="column">
      <Text bold>Kcode</Text>
      <Text dimColor>
        TS/React/Ink frontend - {busy ? 'engine running' : 'engine idle'} - {goalText}
      </Text>
    </Box>
  );
}

function MessageLine({ message }: { message: UiMessage }): React.ReactNode {
  const color = message.role === 'error' ? 'red' : message.role === 'assistant' ? 'green' : 'gray';
  const label = message.role.padEnd(9, ' ');
  return (
    <Box flexDirection="column">
      <Text color={color}>{label} {message.text}</Text>
    </Box>
  );
}

function AgentPanel({ agents }: { agents: AgentProgress[] }): React.ReactNode {
  return (
    <Box flexDirection="column" marginTop={1}>
      <Text bold>Agent Progress</Text>
      {agents.map((agent, index) => (
        <Text key={`${agent.label}-${index}`} color={agent.status === 'error' ? 'red' : 'cyan'}>
          {'|- '} {agent.label} [{agent.status}] {agent.detail ?? ''}
        </Text>
      ))}
    </Box>
  );
}

function PermissionBox(): React.ReactNode {
  return (
    <Box borderStyle="round" borderColor="yellow" flexDirection="column" marginTop={1} paddingX={1}>
      <Text bold>Permission request</Text>
      <Text>Tool: demo_tool</Text>
      <Text>Required mode: workspace-write</Text>
      <Text>Press a to allow, d to deny.</Text>
    </Box>
  );
}

async function submit(
  value: string,
  goal: GoalState,
  exit: () => void,
  setBusy: React.Dispatch<React.SetStateAction<boolean>>,
  setInput: React.Dispatch<React.SetStateAction<string>>,
  setGoal: React.Dispatch<React.SetStateAction<GoalState>>,
  setAgents: React.Dispatch<React.SetStateAction<AgentProgress[]>>,
  setMessages: React.Dispatch<React.SetStateAction<UiMessage[]>>,
  setPermissionOpen: React.Dispatch<React.SetStateAction<boolean>>,
  activeRun: React.MutableRefObject<AbortController | null>,
): Promise<void> {
  const text = value.trim();
  if (!text) {
    return;
  }
  setInput('');
  setMessages(items => [...items, { role: 'user', text }]);
  const controller = new AbortController();
  activeRun.current = controller;
  setBusy(true);
  const result = text.startsWith('/')
    ? await handleSlashCommand(text, goal, controller.signal)
    : await handlePrompt(text, controller.signal);
  activeRun.current = null;
  setBusy(false);
  if (result.exit) {
    exit();
    return;
  }
  if (result.goal) {
    setGoal(result.goal);
  }
  if (result.agents) {
    setAgents(result.agents);
  }
  if (result.requestPermissionDemo) {
    setPermissionOpen(true);
  }
  setMessages(items => [...items, ...result.messages]);
}

function handlePermissionInput(
  chunk: string,
  setMessages: React.Dispatch<React.SetStateAction<UiMessage[]>>,
  setPermissionOpen: React.Dispatch<React.SetStateAction<boolean>>,
): void {
  if (chunk === 'a') {
    setPermissionOpen(false);
    setMessages(items => [...items, { role: 'system', text: 'Permission approved for this turn.' }]);
  }
  if (chunk === 'd') {
    setPermissionOpen(false);
    setMessages(items => [...items, { role: 'system', text: 'Permission denied.' }]);
  }
}
