import { spawn } from 'node:child_process';

export type EngineResult = {
  ok: boolean;
  output: string;
  code: number | null;
  cancelled?: boolean;
};

export type EngineRunOptions = {
  signal?: AbortSignal;
};

export async function runEngine(args: string[], options: EngineRunOptions = {}): Promise<EngineResult> {
  if (options.signal?.aborted) {
    return { ok: false, output: 'Command cancelled.', code: null, cancelled: true };
  }
  const bin = process.env.KCODE_ENGINE_BIN || 'kcode-engine';
  const baseArgs = splitEngineArgs(process.env.KCODE_ENGINE_ARGS);
  const child = spawn(bin, [...baseArgs, ...args], {
    detached: true,
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
    let settled = false;
    let sigtermTimer: NodeJS.Timeout | undefined;
    let sigkillTimer: NodeJS.Timeout | undefined;
    const resolveOnce = (result: EngineResult) => {
      if (settled) {
        return;
      }
      settled = true;
      if (sigtermTimer) {
        clearTimeout(sigtermTimer);
      }
      if (sigkillTimer) {
        clearTimeout(sigkillTimer);
      }
      options.signal?.removeEventListener('abort', abortChild);
      resolve(result);
    };
    const abortChild = () => {
      if (child.exitCode !== null || child.signalCode !== null) {
        return;
      }
      killChildTree(child.pid, 'SIGINT');
      sigtermTimer = setTimeout(() => {
        if (child.exitCode === null && child.signalCode === null) {
          killChildTree(child.pid, 'SIGTERM');
        }
      }, 1500);
      sigkillTimer = setTimeout(() => {
        if (child.exitCode === null && child.signalCode === null) {
          killChildTree(child.pid, 'SIGKILL');
        }
      }, 4000);
    };

    options.signal?.addEventListener('abort', abortChild, { once: true });
    child.on('error', error => {
      resolveOnce({ ok: false, output: error.message, code: null });
    });
    child.on('close', code => {
      const output = [stdout.trimEnd(), stderr.trimEnd()].filter(Boolean).join('\n');
      if (options.signal?.aborted) {
        resolveOnce({ ok: false, output: output || 'Command cancelled.', code, cancelled: true });
        return;
      }
      resolveOnce({ ok: code === 0, output, code });
    });
    if (options.signal?.aborted) {
      abortChild();
    }
  });
}

function splitEngineArgs(value: string | undefined): string[] {
  if (!value) {
    return [];
  }
  return value.split(/\s+/).map(part => part.trim()).filter(Boolean);
}

function killChildTree(pid: number | undefined, signal: NodeJS.Signals): void {
  if (pid === undefined) {
    return;
  }
  try {
    process.kill(-pid, signal);
  } catch {
    try {
      process.kill(pid, signal);
    } catch {
      // Process already exited.
    }
  }
}
