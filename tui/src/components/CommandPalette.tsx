import React, { useMemo, useState } from 'react';
import { Box, Text, useInput } from 'ink';
import type { CommandDescriptor } from '../protocol/types.js';

type Props = {
  commands: CommandDescriptor[];
  query: string;
  onQueryChange: (value: string) => void;
  onSelect: (command: CommandDescriptor) => void;
  onClose: () => void;
};

export function CommandPalette({
  commands,
  query,
  onQueryChange,
  onSelect,
  onClose,
}: Props): React.ReactNode {
  const [selectedIndex, setSelectedIndex] = useState(0);
  const filtered = useMemo(() => filterCommands(commands, query).slice(0, 10), [commands, query]);
  const selected = filtered[Math.min(selectedIndex, Math.max(filtered.length - 1, 0))];

  useInput((input, key) => {
    if (key.escape) {
      onClose();
      return;
    }
    if (key.return && selected) {
      onSelect(selected);
      return;
    }
    if (key.upArrow) {
      setSelectedIndex(index => Math.max(0, index - 1));
      return;
    }
    if (key.downArrow) {
      setSelectedIndex(index => Math.min(Math.max(filtered.length - 1, 0), index + 1));
      return;
    }
    if (key.backspace || key.delete) {
      onQueryChange(Array.from(query).slice(0, -1).join(''));
      setSelectedIndex(0);
      return;
    }
    if (input && !key.ctrl && !key.meta) {
      onQueryChange(query + input);
      setSelectedIndex(0);
    }
  });

  return (
    <Box borderStyle="single" borderColor="cyan" flexDirection="column" marginTop={1} paddingX={1}>
      <Text bold>Commands</Text>
      <Text dimColor>/{query}</Text>
      {filtered.length === 0 && <Text color="yellow">No command matches active backend surface.</Text>}
      {filtered.map((command, index) => (
        <CommandRow key={command.id} command={command} selected={index === selectedIndex} />
      ))}
      {selected && <CommandDetail command={selected} />}
    </Box>
  );
}

function CommandRow({
  command,
  selected,
}: {
  command: CommandDescriptor;
  selected: boolean;
}): React.ReactNode {
  return (
    <Text color={selected ? 'black' : command.experimental ? 'yellow' : 'white'} backgroundColor={selected ? 'cyan' : undefined}>
      {selected ? '>' : ' '} {command.usage.padEnd(24, ' ')} {command.description}
    </Text>
  );
}

function CommandDetail({ command }: { command: CommandDescriptor }): React.ReactNode {
  return (
    <Box flexDirection="column" marginTop={1}>
      <Text dimColor>
        {command.category} | {command.kind} | {command.source} | {command.resumeSupported ? 'resume' : 'session'}
      </Text>
      <Text dimColor>
        remote {String(command.remoteSafe)} | channel {String(command.channelSafe)}
        {command.wiring === 'informational' ? ' | info' : ''}
        {command.experimental ? ' | experimental' : ''}
      </Text>
    </Box>
  );
}

function filterCommands(commands: CommandDescriptor[], query: string): CommandDescriptor[] {
  const needle = query.trim().replace(/^\//, '').toLowerCase();
  if (!needle) {
    return commands;
  }
  return commands.filter(command => {
    return (
      command.name.toLowerCase().includes(needle) ||
      command.description.toLowerCase().includes(needle) ||
      command.category.toLowerCase().includes(needle) ||
      command.aliases.some(alias => alias.toLowerCase().includes(needle))
    );
  });
}
