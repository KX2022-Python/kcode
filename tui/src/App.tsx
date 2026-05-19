import React, { useState } from 'react';
import { Box, Text, useApp, useInput } from 'ink';
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

  useInput((chunk, key) => {
    if (busy) {
      return;
    }
    if (permissionOpen) {
      handlePermissionInput(chunk, setMessages, setPermissionOpen);
      return;
    }
    if (key.ctrl && chunk === 'c') {
      app.exit();
      return;
    }
    if (key.return) {
      void submit(input, goal, app.exit, setBusy, setInput, setGoal, setAgents, setMessages, setPermissionOpen);
      return;
    }
    if (key.backspace || key.delete) {
      setInput(value => value.slice(0, -1));
      return;
    }
    if (chunk && !key.ctrl && !key.meta) {
      setInput(value => value + chunk);
    }
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
        <Text color="cyan">{'> '}</Text>
        <Text>{input}</Text>
      </Box>
    </Box>
  );
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
): Promise<void> {
  const text = value.trim();
  if (!text) {
    return;
  }
  setInput('');
  setMessages(items => [...items, { role: 'user', text }]);
  setBusy(true);
  const result = text.startsWith('/') ? await handleSlashCommand(text, goal) : await handlePrompt(text);
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
