import { spawn } from 'node:child_process';

export type EngineResult = {
  ok: boolean;
  output: string;
  code: number | null;
};

export async function runEngine(args: string[]): Promise<EngineResult> {
  const bin = process.env.KCODE_ENGINE_BIN || 'kcode-engine';
  const baseArgs = splitEngineArgs(process.env.KCODE_ENGINE_ARGS);
  const child = spawn(bin, [...baseArgs, ...args], {
    env: process.env,
    stdio: ['ignore', 'pipe', 'pipe'],
  });

  let stdout = '';
  let stderr = '';
  child.stdout.setEncoding('utf8');
  child.stderr.setEncoding('utf8');
  child.stdout.on('data', chunk => {
    stdout += chunk;
  });
  child.stderr.on('data', chunk => {
    stderr += chunk;
  });

  return new Promise(resolve => {
    child.on('error', error => {
      resolve({ ok: false, output: error.message, code: null });
    });
    child.on('close', code => {
      const output = [stdout.trimEnd(), stderr.trimEnd()].filter(Boolean).join('\n');
      resolve({ ok: code === 0, output, code });
    });
  });
}

function splitEngineArgs(value: string | undefined): string[] {
  if (!value) {
    return [];
  }
  return value.split(/\s+/).map(part => part.trim()).filter(Boolean);
}
