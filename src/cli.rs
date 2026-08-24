use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "flareops",
    author = "Brandon Hubbard <bhubbard@users.noreply.github.com>",
    version,
    about = "⚡ Developer experience & workflow CLI for Cloudflare Workers, Pages, and Astro.",
    long_about = "⚡ flareops — Consolidated developer operations for the Cloudflare ecosystem.\n\
                  Generates strictly typed bindings, synchronizes secrets and .dev.vars, generates\n\
                  and optimizes Pages _routes.json, manages _headers caching rules, validates\n\
                  session bindings, and audits deployment integrity."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Parses wrangler config and generates strictly typed cloudflare-env.d.ts declarations.
    #[command(name = "sync")]
    Sync(SyncArgs),

    /// Manages and validates .dev.vars local development environments and secrets.
    #[command(name = "env")]
    Env(EnvCommand),

    /// Scans static assets and generates or optimizes Cloudflare Pages _routes.json.
    #[command(name = "routes")]
    Routes(RoutesCommand),

    /// Manages, validates, and generates Cloudflare Pages _headers caching and security rules.
    #[command(name = "headers")]
    Headers(HeadersCommand),

    /// Validates and initializes Astro Cloudflare KV session configuration.
    #[command(name = "session")]
    Session(SessionCommand),

    /// Verifies that generated TypeScript interfaces, .dev.vars, routes, headers, and sessions are in sync.
    #[command(name = "check")]
    Check(CheckArgs),

    /// Codemod legacy Astro runtime calls (Astro.locals.runtime.env -> Astro.locals).
    #[command(name = "migrate")]
    Migrate(MigrateArgs),

    /// Generate shell autocompletions for zsh, bash, fish, powershell, or elvish.
    #[command(name = "completions")]
    Completions(CompletionsArgs),
}

#[derive(Args, Debug)]
pub struct SyncArgs {
    /// Path to project root directory or wrangler config file.
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Output path for the generated TypeScript declaration file.
    #[arg(short, long)]
    pub out: Option<PathBuf>,

    /// Target generation mode: astro, worker, or standalone.
    #[arg(short, long, default_value = "astro")]
    pub mode: String,

    /// Target Wrangler environment (e.g. production, staging).
    #[arg(short, long)]
    pub env: Option<String>,

    /// Check if types are in sync without writing files (exits with 1 if out of sync).
    #[arg(long)]
    pub check: bool,

    /// Preview generated declarations without modifying filesystem.
    #[arg(short, long)]
    pub dry_run: bool,
}

#[derive(Args, Debug)]
pub struct EnvCommand {
    #[command(subcommand)]
    pub subcommand: EnvSubcommands,
}

#[derive(Subcommand, Debug)]
pub enum EnvSubcommands {
    /// Scaffolds or updates .dev.vars from wrangler configuration vars and secrets.
    #[command(name = "pull")]
    Pull(EnvPullArgs),

    /// Syncs and validates variables between wrangler config and .dev.vars.
    #[command(name = "sync")]
    Sync(EnvPullArgs),

    /// Validates .dev.vars against wrangler config and checks .gitignore security.
    #[command(name = "validate")]
    Validate(EnvValidateArgs),

    /// Codemod legacy Astro.locals.runtime.env calls across the project.
    #[command(name = "migrate")]
    Migrate(MigrateArgs),
}

#[derive(Args, Debug)]
pub struct EnvPullArgs {
    /// Path to project root directory.
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Target Wrangler environment.
    #[arg(short, long)]
    pub env: Option<String>,

    /// Generate .dev.vars.example template instead of .dev.vars.
    #[arg(long)]
    pub example: bool,

    /// Force overwrite of existing .dev.vars values.
    #[arg(short, long)]
    pub force: bool,
}

#[derive(Args, Debug)]
pub struct EnvValidateArgs {
    /// Path to project root directory.
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Target Wrangler environment.
    #[arg(short, long)]
    pub env: Option<String>,

    /// Fail with exit code 1 on warnings or errors.
    #[arg(long)]
    pub strict: bool,
}

#[derive(Args, Debug)]
pub struct RoutesCommand {
    #[command(subcommand)]
    pub subcommand: RoutesSubcommands,
}

#[derive(Subcommand, Debug)]
pub enum RoutesSubcommands {
    /// Scans static asset output and auto-generates optimized _routes.json.
    #[command(name = "generate")]
    Generate(RoutesGenerateArgs),

    /// Minifies and deduplicates an existing _routes.json under the 100-rule limit.
    #[command(name = "optimize")]
    Optimize(RoutesOptimizeArgs),

    /// Validates _routes.json rules against Cloudflare Pages limits and conflicts.
    #[command(name = "validate")]
    Validate(RoutesValidateArgs),

    /// Simulates route resolution for a URL path against _routes.json rules.
    #[command(name = "simulate")]
    Simulate(RoutesSimulateArgs),
}

#[derive(Args, Debug)]
pub struct RoutesGenerateArgs {
    /// Path to project directory or static asset output directory (e.g. dist).
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Output path for generated _routes.json file.
    #[arg(short, long)]
    pub out: Option<PathBuf>,

    /// Static asset directory override.
    #[arg(short, long)]
    pub dir: Option<PathBuf>,
}

#[derive(Args, Debug)]
pub struct RoutesOptimizeArgs {
    /// Path to _routes.json file or project directory containing it.
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Output path for optimized _routes.json.
    #[arg(short, long)]
    pub out: Option<PathBuf>,
}

#[derive(Args, Debug)]
pub struct RoutesValidateArgs {
    /// Path to _routes.json file or project directory.
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Fail if rule count exceeds 80 warning threshold.
    #[arg(long)]
    pub strict: bool,
}

#[derive(Args, Debug)]
pub struct RoutesSimulateArgs {
    /// URL path to test against rules (e.g. /api/users or /_astro/style.css).
    pub route_path: String,

    /// Path to _routes.json file or project directory.
    #[arg(short, long, default_value = ".")]
    pub path: PathBuf,
}

#[derive(Args, Debug)]
pub struct HeadersCommand {
    #[command(subcommand)]
    pub subcommand: HeadersSubcommands,
}

#[derive(Subcommand, Debug)]
pub enum HeadersSubcommands {
    /// Scans static assets and creates optimized _headers with immutable cache rules and security headers.
    #[command(name = "generate")]
    Generate(HeadersGenerateArgs),

    /// Checks _headers for syntax errors, conflicting max-age rules, and missing security headers.
    #[command(name = "validate")]
    Validate(HeadersValidateArgs),

    /// Auto-adds missing immutable headers for hashed Vite/Astro assets.
    #[command(name = "fix")]
    Fix(HeadersFixArgs),
}

#[derive(Args, Debug)]
pub struct HeadersGenerateArgs {
    /// Path to project directory or static asset output directory.
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Output path for generated _headers file.
    #[arg(short, long)]
    pub out: Option<PathBuf>,

    /// Static asset directory override (e.g. dist).
    #[arg(short, long)]
    pub dir: Option<PathBuf>,
}

#[derive(Args, Debug)]
pub struct HeadersValidateArgs {
    /// Path to project root or _headers file.
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Static asset directory override (e.g. dist).
    #[arg(short, long)]
    pub dir: Option<PathBuf>,

    /// Fail with exit code 1 on warnings or errors.
    #[arg(long)]
    pub strict: bool,
}

#[derive(Args, Debug)]
pub struct HeadersFixArgs {
    /// Path to project root or _headers file.
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Output destination for fixed _headers file.
    #[arg(short, long)]
    pub out: Option<PathBuf>,

    /// Static asset directory override (e.g. dist).
    #[arg(short, long)]
    pub dir: Option<PathBuf>,
}

#[derive(Args, Debug)]
pub struct SessionCommand {
    #[command(subcommand)]
    pub subcommand: SessionSubcommands,
}

#[derive(Subcommand, Debug)]
pub enum SessionSubcommands {
    /// Validates astro.config.* session driver against wrangler.jsonc KV bindings.
    #[command(name = "check")]
    Check(SessionCheckArgs),

    /// Scaffolds the missing session KV binding into wrangler.jsonc and astro.config.*.
    #[command(name = "init")]
    Init(SessionInitArgs),
}

#[derive(Args, Debug)]
pub struct SessionCheckArgs {
    /// Path to project root directory.
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Target KV binding name override (defaults to driver config or SESSION).
    #[arg(short, long)]
    pub binding: Option<String>,

    /// Target Wrangler environment.
    #[arg(short, long)]
    pub env: Option<String>,

    /// Fail with exit code 1 on warnings or errors.
    #[arg(long)]
    pub strict: bool,
}

#[derive(Args, Debug)]
pub struct SessionInitArgs {
    /// Path to project root directory.
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// KV binding name to scaffold (default: SESSION).
    #[arg(short, long, default_value = "SESSION")]
    pub binding: String,
}

#[derive(Args, Debug)]
pub struct CheckArgs {
    /// Path to project root directory.
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Target Wrangler environment.
    #[arg(short, long)]
    pub env: Option<String>,

    /// Fail with exit code 1 if any check fails.
    #[arg(long, default_value = "true")]
    pub strict: bool,
}

#[derive(Args, Debug)]
pub struct MigrateArgs {
    /// Path to project root or source directory.
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Preview code changes without modifying files.
    #[arg(short, long)]
    pub dry_run: bool,
}

#[derive(Args, Debug)]
pub struct CompletionsArgs {
    /// Target shell to generate completions for.
    #[arg(value_enum)]
    pub shell: ShellType,
}

#[derive(ValueEnum, Clone, Copy, Debug)]
pub enum ShellType {
    Bash,
    Zsh,
    Fish,
    PowerShell,
    Elvish,
}
