import React from 'react';
import { render } from 'ink';
import { RuntimeController } from './runtime/RuntimeController.js';
import { App } from './App/App.js';

async function main(): Promise<void> {
  if (process.env.KCODE_TS_TUI_SMOKE === '1') {
    await runSmoke();
    return;
  }
  render(<App />);
}

async function runSmoke(): Promise<void> {
  console.log('Kcode TS TUI ready');
  console.log('Default frontend: TypeScript/React/Ink');
  await new Promise<void>(resolve => {
    const controller = new RuntimeController({
      onEvent: event => {
        console.log(JSON.stringify(event));
        if (event.type === 'bootstrap') {
          const command = process.env.KCODE_TS_TUI_SMOKE_COMMAND;
          if (command) {
            controller.slash(command);
          } else {
            controller.dispose();
            resolve();
          }
        }
        if (event.type === 'done') {
          controller.dispose();
          resolve();
        }
      },
      onError: message => console.error(message),
    });
  });
}

main().catch(error => {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
});
