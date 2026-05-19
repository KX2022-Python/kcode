import { spawn, type ChildProcessWithoutNullStreams } from 'node:child_process';
import { createInterface } from 'node:readline';
import type { EngineEvent, EngineInput, PermissionDecision } from '../protocol/types.js';

export type RuntimeControllerOptions = {
  onEvent: (event: EngineEvent) => void;
  onError: (message: string) => void;
};

export class RuntimeController {
  #child: ChildProcessWithoutNullStreams;
  #onEvent: (event: EngineEvent) => void;
  #onError: (message: string) => void;

  constructor(options: RuntimeControllerOptions) {
    this.#onEvent = options.onEvent;
    this.#onError = options.onError;
    this.#child = this.#spawnEngine();
    this.#wireOutput();
  }

  submit(text: string): string {
    const id = requestId();
    this.#send({ type: 'user_submit', id, text });
    return id;
  }

  slash(command: string): string {
    const id = requestId();
    this.#send({ type: 'slash', id, command });
    return id;
  }

  refreshCommands(query?: string): string {
    const id = requestId();
    this.#send({ type: 'command_list', id, query });
    return id;
  }

  refreshStatus(): string {
    const id = requestId();
    this.#send({ type: 'status', id });
    return id;
  }

  cancel(id?: string): void {
    this.#send({ type: 'cancel', id });
  }

  respondPermission(id: string, decision: PermissionDecision): void {
    this.#send({ type: 'permission_response', id, decision });
  }

  dispose(): void {
    if (!this.#child.killed) {
      this.#send({ type: 'exit' });
      this.#child.stdin.end();
    }
  }

  #spawnEngine(): ChildProcessWithoutNullStreams {
    const bin = process.env.KCODE_ENGINE_BIN || 'kcode-engine';
    const baseArgs = splitArgs(process.env.KCODE_ENGINE_ARGS);
    const args = baseArgs.includes('tui-engine') ? baseArgs : [...baseArgs, 'tui-engine'];
    return spawn(bin, args, {
      env: process.env,
      stdio: ['pipe', 'pipe', 'pipe'],
    });
  }

  #wireOutput(): void {
    const stdout = createInterface({ input: this.#child.stdout });
    stdout.on('line', line => this.#handleLine(line));
    this.#child.stderr.setEncoding('utf8');
    this.#child.stderr.on('data', chunk => {
      const text = String(chunk).trim();
      if (text) {
        this.#onError(text);
      }
    });
    this.#child.on('error', error => this.#onError(error.message));
    this.#child.on('close', code => {
      if (code !== 0 && code !== null) {
        this.#onError(`Rust engine exited with code ${code}.`);
      }
    });
  }

  #handleLine(line: string): void {
    try {
      this.#onEvent(JSON.parse(line) as EngineEvent);
    } catch {
      this.#onError(line);
    }
  }

  #send(event: EngineInput): void {
    this.#child.stdin.write(`${JSON.stringify(event)}\n`);
  }
}

function splitArgs(value: string | undefined): string[] {
  if (!value) {
    return [];
  }
  return value.split(/\s+/).filter(Boolean);
}

function requestId(): string {
  return `${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}`;
}
