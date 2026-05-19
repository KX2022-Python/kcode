import React, { useState } from 'react';
import { Box, Text, useInput } from 'ink';
import type { PermissionDecision } from '../protocol/types.js';
import type { PermissionState } from '../state/types.js';

type Props = {
  request: NonNullable<PermissionState['pending']>;
  onDecision: (id: string, decision: PermissionDecision) => void;
};

const choices: Array<{ label: string; decision: PermissionDecision }> = [
  { label: 'Allow once', decision: 'allow_once' },
  { label: 'Allow turn', decision: 'allow_turn' },
  { label: 'Deny once', decision: 'deny_once' },
  { label: 'Deny turn', decision: 'deny_turn' },
];

export function PermissionDialog({ request, onDecision }: Props): React.ReactNode {
  const [selected, setSelected] = useState(0);
  const requestId = request.id ?? `permission-${request.toolName}`;

  useInput((_input, key) => {
    if (key.leftArrow) {
      setSelected(index => (index + choices.length - 1) % choices.length);
      return;
    }
    if (key.rightArrow || key.tab) {
      setSelected(index => (index + 1) % choices.length);
      return;
    }
    if (key.return) {
      onDecision(requestId, choices[selected]?.decision ?? 'deny_once');
      return;
    }
    if (key.escape) {
      onDecision(requestId, 'deny_once');
    }
  });

  return (
    <Box borderStyle="single" borderColor="yellow" flexDirection="column" marginTop={1} paddingX={1}>
      <Text color="yellow" bold>
        Permission required
      </Text>
      <Text>
        {request.toolName} {request.currentMode} -&gt; {request.requiredMode}
      </Text>
      {request.reason && <Text dimColor>{request.reason}</Text>}
      <Text dimColor>{previewInput(request.input)}</Text>
      <Box marginTop={1}>
        {choices.map((choice, index) => (
          <Box key={choice.decision} marginRight={1}>
            <Text color={index === selected ? 'black' : 'white'} backgroundColor={index === selected ? 'yellow' : undefined}>
              {choice.label}
            </Text>
          </Box>
        ))}
      </Box>
    </Box>
  );
}

function previewInput(input: string): string {
  const normalized = input.replace(/\s+/g, ' ').trim();
  return normalized.length > 120 ? `${normalized.slice(0, 117)}...` : normalized;
}
