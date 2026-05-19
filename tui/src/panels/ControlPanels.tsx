import React from 'react';
import { Box, Text } from 'ink';
import type { AppState } from '../state/types.js';

export function ControlPanels({ state }: { state: AppState }): React.ReactNode {
  const controls = state.status.controls;
  const memoryFiles = controls?.memory?.projectFiles ?? 0;
  const memoryTokens = controls?.memory?.estimatedTokens ?? state.status.estimatedTokens ?? 0;
  return (
    <Box flexDirection="column" marginTop={1}>
      <Box>
        <PanelLabel title="Model" value={controls?.model?.current ?? state.status.model ?? 'unknown'} />
        <PanelLabel title="Permission" value={controls?.permission?.mode ?? state.status.permissionMode ?? 'unknown'} />
        <PanelLabel title="MCP" value={controls?.mcp?.available ? 'available' : 'unavailable'} />
      </Box>
      <Box>
        <PanelLabel title="Memory" value={`${memoryFiles} files / ${memoryTokens} tokens`} />
        <PanelLabel title="Agent" value={controls?.agent?.available ? 'tool available' : 'unavailable'} />
        <PanelLabel title="Goal" value={controls?.goal?.status ?? 'not-reported'} />
      </Box>
    </Box>
  );
}

function PanelLabel({ title, value }: { title: string; value: string }): React.ReactNode {
  return (
    <Box marginRight={2}>
      <Text dimColor>{title} </Text>
      <Text>{value}</Text>
    </Box>
  );
}
