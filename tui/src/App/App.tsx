import React from 'react';
import { KeybindingProvider } from '../keybindings/KeybindingProvider.js';
import { REPL } from './REPL.js';

export function App(): React.ReactNode {
  return (
    <KeybindingProvider>
      <REPL />
    </KeybindingProvider>
  );
}
