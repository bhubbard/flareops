#!/usr/bin/env node

const fs = require('fs');
const path = require('path');
const https = require('https');
const os = require('os');

// Repo guard: Skip downloading if working inside the flareops source repository
const repoRoot = path.resolve(__dirname, '..');
if (fs.existsSync(path.join(repoRoot, 'Cargo.toml')) && fs.existsSync(path.join(repoRoot, 'src'))) {
  // Source repo detected — we use target/release/flareops or cargo
  process.exit(0);
}

const platform = os.platform();
const arch = os.arch();
const pkg = require('./package.json');
const VERSION = pkg.version;
const REPO = 'bhubbard/flareops';

let assetName = null;

if (platform === 'win32' && arch === 'x64') {
  assetName = 'flareops-win32-x64.exe';
} else if (platform === 'darwin' && arch === 'arm64') {
  assetName = 'flareops-darwin-arm64';
} else if (platform === 'darwin' && arch === 'x64') {
  assetName = 'flareops-darwin-x64';
} else if (platform === 'linux' && arch === 'x64') {
  assetName = 'flareops-linux-x64';
} else if (platform === 'linux' && arch === 'arm64') {
  assetName = 'flareops-linux-arm64';
}

if (!assetName) {
  console.warn(`[flareops] Pre-built binary not available for platform ${platform}-${arch}.`);
  console.warn('[flareops] You can build and install from source: cargo install flareops');
  process.exit(0);
}

const binDir = path.join(__dirname, 'bin');
const targetBinary = path.join(binDir, platform === 'win32' ? 'flareops.exe' : 'flareops');

if (fs.existsSync(targetBinary)) {
  process.exit(0);
}

fs.mkdirSync(binDir, { recursive: true });

const releaseUrl = `https://github.com/${REPO}/releases/download/v${VERSION}/${assetName}`;

function download(url, dest, redirects = 0) {
  if (redirects > 5) {
    return Promise.reject(new Error('Too many redirects while downloading binary'));
  }

  return new Promise((resolve, reject) => {
    const file = fs.createWriteStream(dest);
    https.get(url, (response) => {
      if (response.statusCode === 301 || response.statusCode === 302 || response.statusCode === 307 || response.statusCode === 308) {
        file.close();
        try { fs.unlinkSync(dest); } catch (_) {}
        const redirectUrl = response.headers.location;
        if (!redirectUrl) {
          return reject(new Error('Redirect header missing location'));
        }
        return resolve(download(redirectUrl, dest, redirects + 1));
      }

      if (response.statusCode !== 200) {
        file.close();
        try { fs.unlinkSync(dest); } catch (_) {}
        return reject(new Error(`HTTP ${response.statusCode}: ${response.statusMessage}`));
      }

      response.pipe(file);
      file.on('finish', () => {
        file.close(resolve);
      });
      file.on('error', (err) => {
        try { fs.unlinkSync(dest); } catch (_) {}
        reject(err);
      });
    }).on('error', (err) => {
      try { fs.unlinkSync(dest); } catch (_) {}
      reject(err);
    });
  });
}

console.log(`[flareops] Downloading pre-built binary v${VERSION} (${assetName})...`);
download(releaseUrl, targetBinary)
  .then(() => {
    if (platform !== 'win32') {
      fs.chmodSync(targetBinary, 0o755);
    }
    console.log('[flareops] Native binary successfully installed.');
  })
  .catch((err) => {
    console.warn(`[flareops] Notice: Could not download pre-built binary (${err.message}).`);
    console.warn('[flareops] You can install from Cargo via: cargo install flareops');
    // Exit 0 so npm install does not abort; fallback to cargo or PATH binary will be used.
    process.exit(0);
  });
