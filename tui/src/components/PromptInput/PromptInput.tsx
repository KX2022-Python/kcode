import React, { useCallback, useEffect } from 'react';
import { Box, Text, useInput, useStdin } from 'ink';

type Props = {
  value: string;
  disabled: boolean;
  onChange: (value: string) => void;
  onSubmit: (value: string) => void;
  onCancel: () => void;
  onOpenPalette: () => void;
};

export function PromptInput({
  value,
  disabled,
  onChange,
  onSubmit,
  onCancel,
  onOpenPalette,
}: Props): React.ReactNode {
  const { stdin, setRawMode, isRawModeSupported } = useStdin();
  const handleSubmit = useCallback(() => {
    const submitted = value.trim();
    if (submitted) {
      onSubmit(submitted);
    }
  }, [onSubmit, value]);

  useInput((input, key) => {
    if (key.escape) {
      onCancel();
      return;
    }
    if (disabled) {
      return;
    }
    if (key.return) {
      handleSubmit();
      return;
    }
    if (key.backspace || key.delete) {
      onChange(Array.from(value).slice(0, -1).join(''));
      return;
    }
    if (input === '/' && value.length === 0) {
      onChange('/');
      onOpenPalette();
      return;
    }
    if (input && !key.ctrl && !key.meta) {
      onChange(value + input);
    }
  }, { isActive: disabled });

  useEffect(() => {
    if (disabled && value.length > 0) {
      onChange('');
    }
  }, [disabled, onChange, value.length]);

  useEffect(() => {
    if (disabled) {
      return;
    }
    if (isRawModeSupported) {
      setRawMode(false);
    }
    stdin.setEncoding('utf8');
    const handleData = (chunk: string) => {
      if (chunk === '\u0003') {
        onCancel();
        return;
      }
      if (chunk === '\u007f' || chunk === '\b') {
        onChange(Array.from(value).slice(0, -1).join(''));
        return;
      }
      if (chunk.includes('\r') || chunk.includes('\n')) {
        handleSubmit();
        return;
      }
      if (isTerminalControlSequence(chunk)) {
        return;
      }
      if (chunk === '/' && value.length === 0) {
        onChange('/');
        onOpenPalette();
        return;
      }
      onChange(value + chunk);
    };
    stdin.on('data', handleData);
    stdin.resume();
    return () => {
      stdin.off('data', handleData);
    };
  }, [
    disabled,
    handleSubmit,
    isRawModeSupported,
    onCancel,
    onChange,
    onOpenPalette,
    setRawMode,
    stdin,
    value,
  ]);

  return (
    <Box marginTop={1}>
      <Text color={disabled ? 'yellow' : 'green'}>{disabled ? 'running' : 'ready'} </Text>
      <Text color="cyan">{'>'} </Text>
      <Text>{value}</Text>
      {!value && <Text dimColor> type / for commands</Text>}
    </Box>
  );
}

function isTerminalControlSequence(value: string): boolean {
  return value.startsWith('\u001B[') || value.startsWith('\u001B]');
}
