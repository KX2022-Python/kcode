import React from 'react';
import { render } from 'ink';
import { handleSlashCommand } from './commands.js';
import { App } from './App.js';

async function main(): Promise<void> {
  if (process.env.KCODE_TS_TUI_SMOKE === '1') {
    const command = process.env.KCODE_TS_TUI_SMOKE_COMMAND;
    console.log('Kcode TS TUI ready');
    console.log('Default frontend: TypeScript/React/Ink');
    console.log(`Engine: ${process.env.KCODE_ENGINE_BIN ?? 'kcode-engine'}`);
    if (command) {
      const result = await handleSlashCommand(command, { status: 'none' });
      for (const message of result.messages) {
        console.log(`${message.role}: ${message.text}`);
      }
    }
    return;
  }

  render(<App />);
}

main().catch(error => {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
});
