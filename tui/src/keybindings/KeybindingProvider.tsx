import React, { createContext, useContext, useMemo } from 'react';

export type KeybindingMap = {
  submit: string;
  newline: string;
  palette: string;
  cancel: string;
  close: string;
  status: string;
};

const defaultBindings: KeybindingMap = {
  submit: 'Enter',
  newline: 'Shift+Enter',
  palette: '/',
  cancel: 'Esc',
  close: 'Ctrl+C',
  status: 'Ctrl+S',
};

const KeybindingContext = createContext<KeybindingMap>(defaultBindings);

export function KeybindingProvider({ children }: { children: React.ReactNode }): React.ReactNode {
  const bindings = useMemo(() => defaultBindings, []);
  return <KeybindingContext.Provider value={bindings}>{children}</KeybindingContext.Provider>;
}

export function useKeybindings(): KeybindingMap {
  return useContext(KeybindingContext);
}
