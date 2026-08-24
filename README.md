# flareops

[![CI](https://github.com/bhubbard/flareops/actions/workflows/ci.yml/badge.svg)](https://github.com/bhubbard/flareops/actions)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)
[![GitHub Pages](https://img.shields.io/badge/docs-GitHub%20Pages-black?logo=github)](https://bhubbard.github.io/flareops)

> ⚡ **Unified Developer Experience & Operations CLI for Cloudflare Workers, Pages, and Astro** — consolidating type synchronization, secret management, Pages `_routes.json` optimization, and pre-deploy verification.

---

## ⚡ Overview

`flareops` consolidates essential Cloudflare developer tooling into a single high-performance Rust CLI:
- **`flareops sync`** (formerly `astro-binding-sync`): Parses `wrangler.jsonc` or `wrangler.toml` and generates strictly typed `cloudflare-env.d.ts` declarations covering KV, D1, R2, Queues, Vectorize, Hyperdrive, AI, Services, Durable Objects, Workflows, and more.
- **`flareops env`**: Manages `.dev.vars` / local environment files safely, scaffolding templates, merging variables, preventing secret leaks, and validating security in `.gitignore`.
- **`flareops routes`** (formerly `cf-routes-json`): Scans Astro, Remix, and SvelteKit static asset output and auto-generates optimized, minified `_routes.json` staying under Cloudflare's strict 100-rule limit.
- **`flareops check`**: Unified pre-deploy audit verifying that TypeScript interfaces, local secrets, and route limits are in sync.
- **`flareops migrate`** (formerly `astro-env-migration`): Automated codemod migrating legacy `Astro.locals.runtime.env` patterns to modern Astro v5 `Astro.locals.*`.
- **`flareops completions`**: Generates shell autocompletions for `zsh`, `bash`, `fish`, `powershell`, and `elvish`.

---

## 🚀 Installation

### Using Pre-built Binaries / Cargo
```bash
cargo install --path .
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

### 4. `flareops check [PATH]`
Runs a comprehensive pre-deploy verification across wrangler config, TypeScript interfaces, local secrets, and `_routes.json`.

```bash
flareops check
```

### 5. `flareops migrate [PATH]`
Scans and codemods legacy Astro runtime references:

```bash
# Preview replacements
flareops migrate src/ --dry-run

# Apply migrations
flareops migrate src/
```

### 6. `flareops completions <SHELL>`
Generates autocompletion scripts for your shell:

```bash
flareops completions zsh > ~/.zsh/completion/_flareops
```

---

## 🛠️ Architecture & Features

| Capability | Cloudflare Bindings Supported |
| :--- | :--- |
| **Storage & State** | KV Namespaces (`KVNamespace`), D1 Databases (`D1Database`), R2 Buckets (`R2Bucket`), Vectorize Indexes (`VectorizeIndex`), Hyperdrive (`Hyperdrive`), Durable Objects (`DurableObjectNamespace`) |
| **Compute & Messaging** | Workers AI (`Ai`), Service Bindings (`Fetcher`), Queues (`Queue`), Workflows (`Workflow`), Browser Rendering (`Fetcher`) |
| **Security & Routing** | Send Email (`SendEmail`), Rate Limiting (`RateLimit`), MTLS (`MtlsCertificate`), Dispatch Namespaces (`DispatchNamespace`), Static Assets (`Fetcher`) |
| **Variables & Secrets** | Type-inferred configuration variables (`string`, `number`, `boolean`), Wrangler Secrets |

---

## 📄 License
Dual-licensed under either of:
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
