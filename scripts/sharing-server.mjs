// Real server process for browser integration tests; isolated credentials and DB.
import { spawn } from 'node:child_process';
import { mkdtempSync, writeFileSync, rmSync, existsSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { createHash } from 'node:crypto';

const binary = resolve(process.env.PACE_SERVER_BIN || '../pace_server/target/debug/pace_server');
if (!existsSync(binary)) throw new Error('Build pace_server and set PACE_SERVER_BIN before running browser tests');
const directory = mkdtempSync(join(tmpdir(), 'pace-sharing-e2e-'));
const tokens = ['alice', 'bob', 'eve'].map(principal => ({
    principal, sha256: createHash('sha256').update(`${principal}-browser-test-token-00000000000000000000`).digest('hex'),
}));
writeFileSync(join(directory, 'auth.json'), JSON.stringify({ tokens }), { mode: 0o600 });
const child = spawn(binary, [], { stdio: 'inherit', env: { ...process.env,
    PACE_BIND: '127.0.0.1:3100', PACE_DATABASE: join(directory, 'projects.sqlite3'),
    PACE_AUTH_FILE: join(directory, 'auth.json'), PACE_ALLOWED_ORIGINS: 'http://127.0.0.1:8080',
} });
for (const signal of ['SIGTERM', 'SIGINT']) process.on(signal, () => child.kill(signal));
child.on('exit', code => { rmSync(directory, { recursive: true, force: true }); process.exit(code ?? 0); });
child.on('error', error => { console.error(error.message); rmSync(directory, { recursive: true, force: true }); process.exit(1); });
