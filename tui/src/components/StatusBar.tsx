import React from 'react';
import { Box, Text } from 'ink';
import type { AppState } from '../state/types.js';

export function StatusBar({ state }: { state: AppState }): React.ReactNode {
  const status = state.status;
  const phaseColor = state.phase === 'running' ? 'yellow' : state.phase === 'error' ? 'red' : 'green';
  return (
    <Box flexDirection="column">
      <Box>
        <Text bold>Kcode </Text>
        <Text color={phaseColor}>{state.phase}</Text>
        <Text dimColor> | </Text>
        <Text>{status.model ?? 'model?'}</Text>
        <Text dimColor> | </Text>
        <Text>{status.profile ?? 'profile?'}</Text>
        <Text dimColor> | </Text>
        <Text>{status.permissionMode ?? state.permission.mode ?? 'permission?'}</Text>
      </Box>
      <Box>
        <Text dimColor>
          session {short(status.sessionId)} | turns {status.turns ?? 0} | messages{' '}
          {status.messageCount ?? 0} | tokens {status.estimatedTokens ?? 0}
        </Text>
      </Box>
      <Box>
        <Text dimColor>
          {status.gitBranch ?? 'no-git'} | {status.gitState ?? 'workspace unknown'}
        </Text>
      </Box>
    </Box>
  );
}

function short(value: string | undefined): string {
  if (!value) {
    return 'pending';
  }
  return value.length > 12 ? value.slice(0, 12) : value;
}
