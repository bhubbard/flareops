# flareops

[![CI](https://github.com/bhubbard/flareops/actions/workflows/ci.yml/badge.svg)](https://github.com/bhubbard/flareops/actions)
[![crates.io](https://img.shields.io/crates/v/flareops.svg)](https://crates.io/crates/flareops)
[![npm](https://img.shields.io/npm/v/flareops.svg)](https://www.npmjs.com/package/flareops)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)
[![GitHub Pages](https://img.shields.io/badge/docs-GitHub%20Pages-black?logo=github)](https://bhubbard.github.io/flareops)

> ⚡ **Unified Developer Experience & Operations CLI for Cloudflare Workers, Pages, and Astro** — consolidating type synchronization, secret management, Pages `_routes.json` optimization, Pages `_headers` immutable caching & security rules, Astro KV session audits, and unified pre-deploy verification.

---

## ⚡ Overview

`flareops` consolidates essential Cloudflare developer tooling into a single high-performance Rust CLI:
- **`flareops sync`**: Parses `wrangler.jsonc` or `wrangler.toml` and generates strictly typed `cloudflare-env.d.ts` declarations covering KV, D1, R2, Queues, Vectorize, Hyperdrive, AI, Services, Durable Objects, Workflows, and more.
- **`flareops env`**: Manages `.dev.vars` / local environment files safely, scaffolding templates, merging variables, preventing secret leaks, and validating security in `.gitignore`.
- **`flareops routes`**: Scans Astro, Remix, and SvelteKit static asset output and auto-generates optimized, minified `_routes.json` staying under Cloudflare's strict 100-rule limit.
- **`flareops headers`**: Scans static build output and auto-generates, validates, or remediates Cloudflare Pages `_headers` files with immutable caching for hashed bundles (`/_astro/*`, fonts, images) and baseline security headers.
- **`flareops session`**: Validates Astro Cloudflare KV session configuration against `wrangler.jsonc` KV namespace bindings, and scaffolds missing session KV bindings into project configs.
- **`flareops check`**: Unified pre-deploy audit verifying that TypeScript interfaces, local secrets, route limits, Pages headers, and session bindings are in sync.
- **`flareops migrate`**: Automated codemod migrating legacy `Astro.locals.runtime.env` patterns to modern Astro v5 `Astro.locals.*`.
- **`flareops completions`**: Generates shell autocompletions for `zsh`, `bash`, `fish`, `powershell`, and `elvish`.

---

## 🚀 Installation

### Using npm / npx
```bash
# Run on-demand without installing
npx flareops --help

# Install as project devDependency
npm install -D flareops
# or
pnpm add -D flareops

# Install globally via npm
npm install -g flareops
```

### Using Cargo (crates.io)
```bash
cargo install flareops
```

### GitHub Actions Integration
```yaml
- name: Cloudflare Ops & Type Verification
  uses: bhubbard/flareops@main
  with:
    command: check
    path: .
    strict: true
```

---

## 📖 CLI Usage & Commands

### 1. `flareops sync [PATH]`
Parses `wrangler.jsonc` or `wrangler.toml` and generates strictly typed TypeScript declaration files.

```bash
# Sync types for Astro project (writes to src/env.d.ts)
flareops sync

# Sync types for Cloudflare Worker project
flareops sync --mode worker

# Dry-run type generation
flareops sync --dry-run

# Verify types are up to date in CI without writing
flareops sync --check
```

### 2. `flareops env`
Manages `.dev.vars` local development environments and secret safety.

```bash
# Pull/scaffold .dev.vars from wrangler vars and secrets without overwriting existing secrets
flareops env pull

# Generate .dev.vars.example template
flareops env pull --example

# Validate that .dev.vars contains all secrets and is gitignored
flareops env validate --strict
```

### 3. `flareops routes`
Generates and optimizes Cloudflare Pages `_routes.json`.

```bash
# Scan static build output (dist/) and generate optimized _routes.json
flareops routes generate dist

# Optimize and minify an existing _routes.json to stay under 100 rules
flareops routes optimize _routes.json

# Validate _routes.json rules and syntax
flareops routes validate _routes.json

# Simulate route resolution against rules
flareops routes simulate _routes.json /api/v1/users
```

### 4. `flareops headers`
Generates, validates, and fixes Cloudflare Pages `_headers` caching rules and security headers.

```bash
# Scan static build assets (dist/) and generate optimal _headers
flareops headers generate dist

# Validate existing _headers against edge caching and security best practices
flareops headers validate

# Auto-remediate missing immutable headers for hashed Vite/Astro assets
flareops headers fix
```

### 5. `flareops session`
Validates and scaffolds Astro Cloudflare KV session bindings.

```bash
# Audit astro.config.* session driver against wrangler.jsonc KV bindings
flareops session check

# Scaffold missing session KV binding into wrangler.jsonc and astro.config.*
flareops session init
```

### 6. `flareops check [PATH]`
Runs a comprehensive pre-deploy verification across wrangler config, TypeScript interfaces, local secrets, `_routes.json`, `_headers`, and session KV bindings.

```bash
flareops check
```

### 7. `flareops migrate [PATH]`
Scans and codemods legacy Astro runtime references:

```bash
# Preview replacements
flareops migrate src/ --dry-run

# Apply migrations
flareops migrate src/
```

### 8. `flareops completions <SHELL>`
Generates autocompletion scripts for your shell:

```bash
flareops completions zsh > ~/.zsh/completion/_flareops
```

---

## 🛠️ Architecture & Features

| Capability | Cloudflare Bindings & Standards Supported |
| :--- | :--- |
| **Storage & State** | KV Namespaces (`KVNamespace`), D1 Databases (`D1Database`), R2 Buckets (`R2Bucket`), Vectorize Indexes (`VectorizeIndex`), Hyperdrive (`Hyperdrive`), Durable Objects (`DurableObjectNamespace`) |
| **Compute & Messaging** | Workers AI (`Ai`), Service Bindings (`Fetcher`), Queues (`Queue`), Workflows (`Workflow`), Browser Rendering (`Fetcher`) |
| **Security & Routing** | Send Email (`SendEmail`), Rate Limiting (`RateLimit`), MTLS (`MtlsCertificate`), Dispatch Namespaces (`DispatchNamespace`), Static Assets (`Fetcher`) |
| **Edge Cache & Headers** | Cloudflare Pages `_headers`, `/_astro/*` immutable cache (1 year), font/image asset headers, X-Content-Type-Options, X-Frame-Options, Referrer-Policy |
| **Session Operations** | Astro Cloudflare KV session driver (`driver: 'cloudflare'`), automatic KV namespace binding verification & scaffolding |
| **Variables & Secrets** | Type-inferred configuration variables (`string`, `number`, `boolean`), Wrangler Secrets |

---

## 📄 License
Dual-licensed under either of:
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
