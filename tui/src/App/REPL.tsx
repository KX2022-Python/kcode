import React, { useCallback, useEffect, useMemo, useReducer, useRef, useState } from 'react';
import { Box, useApp } from 'ink';
import { CommandPalette } from '../components/CommandPalette.js';
import { MessageViewport } from '../components/MessageViewport.js';
import { PromptInput } from '../components/PromptInput/PromptInput.js';
import { StatusBar } from '../components/StatusBar.js';
import { PermissionDialog } from '../dialogs/PermissionDialog.js';
import { ControlPanels } from '../panels/ControlPanels.js';
import type { CommandDescriptor, EngineEvent } from '../protocol/types.js';
import { RuntimeController } from '../runtime/RuntimeController.js';
import { reduceEngineEvent } from '../state/reducer.js';
import { initialState } from '../state/types.js';

export function REPL(): React.ReactNode {
  const app = useApp();
  const [state, dispatch] = useReducer(reduceEngineEvent, initialState);
  const [input, setInput] = useState('');
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [paletteQuery, setPaletteQuery] = useState('');
  const controllerRef = useRef<RuntimeController | null>(null);

  const onEvent = useCallback((event: EngineEvent) => {
    dispatch(event);
    if (event.type === 'done' && event.status === 'exit-requested') {
      app.exit();
    }
  }, [app]);

  const onError = useCallback((message: string) => {
    dispatch({ type: 'error', message, recoverable: true });
  }, []);

  useEffect(() => {
    controllerRef.current = new RuntimeController({ onEvent, onError });
    return () => controllerRef.current?.dispose();
  }, [onEvent, onError]);

  const busy = state.phase === 'running' || state.phase === 'booting';
  const promptValue = paletteOpen ? `/${paletteQuery}` : input;

  const submitPrompt = useCallback((value: string) => {
    setInput('');
    if (value.startsWith('/')) {
      controllerRef.current?.slash(value);
    } else {
      controllerRef.current?.submit(value);
    }
  }, []);

  const selectCommand = useCallback((command: CommandDescriptor) => {
    setPaletteOpen(false);
    setPaletteQuery('');
    controllerRef.current?.slash(`/${command.name}`);
  }, []);

  const commands = useMemo(() => state.commands, [state.commands]);

  return (
    <Box flexDirection="column" paddingX={1}>
      <StatusBar state={state} />
      <ControlPanels state={state} />
      <MessageViewport messages={state.messages} />
      {paletteOpen && (
        <CommandPalette
          commands={commands}
          query={paletteQuery}
          onQueryChange={setPaletteQuery}
          onSelect={selectCommand}
          onClose={() => {
            setPaletteOpen(false);
            setPaletteQuery('');
            setInput('');
          }}
        />
      )}
      {state.permission.pending && (
        <PermissionDialog
          request={state.permission.pending}
          onDecision={(id, decision) => controllerRef.current?.respondPermission(id, decision)}
        />
      )}
      <PromptInput
        value={promptValue}
        disabled={busy}
        onChange={value => {
          if (paletteOpen) {
            setPaletteQuery(value.replace(/^\//, ''));
          } else {
            setInput(value);
          }
        }}
        onSubmit={submitPrompt}
        onCancel={() => {
          if (paletteOpen) {
            setPaletteOpen(false);
            setPaletteQuery('');
            setInput('');
          } else if (state.currentRequestId) {
            controllerRef.current?.cancel(state.currentRequestId);
          }
        }}
        onOpenPalette={() => {
          setPaletteOpen(true);
          setPaletteQuery('');
          controllerRef.current?.refreshCommands();
        }}
      />
    </Box>
  );
}
