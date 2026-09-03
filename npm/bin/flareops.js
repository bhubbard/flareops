#!/usr/bin/env node

const { spawnSync } = require('child_process');
const path = require('path');
const os = require('os');
const fs = require('fs');

const platform = os.platform();
const binName = platform === 'win32' ? 'flareops.exe' : 'flareops';

// 1. Packaged/downloaded binary in this package's bin/
const localBin = path.join(__dirname, binName);

// 2. Development/source repository binary (target/release or target/debug)
const repoRoot = path.resolve(__dirname, '..', '..');
const devReleaseBin = path.join(repoRoot, 'target', 'release', binName);
const devDebugBin = path.join(repoRoot, 'target', 'debug', binName);

let targetBin = null;

if (fs.existsSync(localBin)) {
  targetBin = localBin;
} else if (fs.existsSync(devReleaseBin)) {
  targetBin = devReleaseBin;
} else if (fs.existsSync(devDebugBin)) {
  targetBin = devDebugBin;
} else {
  // 3. Check if installed in system PATH
  const checkGlobal = spawnSync(binName, ['--version'], { encoding: 'utf-8' });
  if (checkGlobal.status === 0) {
    targetBin = binName;
  }
}

if (!targetBin) {
  console.error('[flareops] Error: Could not locate the flareops native binary.');
  console.error('[flareops] Tried:');
  console.error(`  - ${localBin}`);
  console.error(`  - ${devReleaseBin}`);
  console.error('[flareops] Please run: npm rebuild flareops');
  console.error('[flareops] Or install via Cargo: cargo install flareops');
  process.exit(1);
}

// Ensure execution permissions on Unix
if (targetBin !== binName && platform !== 'win32') {
  try {
    const stat = fs.statSync(targetBin);
    if ((stat.mode & 0o111) === 0) {
      fs.chmodSync(targetBin, 0o755);
    }
  } catch (_) {}
}

const args = process.argv.slice(2);
const result = spawnSync(targetBin, args, { stdio: 'inherit' });

if (result.error) {
  console.error('[flareops] Execution failed:', result.error.message);
  process.exit(1);
}

process.exit(result.status ?? 0);
