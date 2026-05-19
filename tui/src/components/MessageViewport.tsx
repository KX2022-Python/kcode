import React from 'react';
import { Box, Text } from 'ink';
import type { UiMessage } from '../state/types.js';

export function MessageViewport({ messages }: { messages: UiMessage[] }): React.ReactNode {
  const visible = messages.slice(-18);
  return (
    <Box flexDirection="column" marginTop={1}>
      {visible.map((message, index) => (
        <MessageLine key={`${message.id}-${index}`} message={message} />
      ))}
    </Box>
  );
}

function MessageLine({ message }: { message: UiMessage }): React.ReactNode {
  const color = colorFor(message);
  const label = message.title ?? message.kind;
  return (
    <Box flexDirection="column" marginBottom={message.kind === 'user' ? 0 : 1}>
      <Text color={color}>
        {label.padEnd(10, ' ')}
        {message.text}
      </Text>
    </Box>
  );
}

function colorFor(message: UiMessage): string {
  if (message.kind === 'error' || message.tone === 'error') {
    return 'red';
  }
  if (message.tone === 'warning') {
    return 'yellow';
  }
  if (message.kind === 'assistant') {
    return 'green';
  }
  if (message.kind === 'tool') {
    return 'cyan';
  }
  if (message.kind === 'user') {
    return 'white';
  }
  return 'gray';
}
