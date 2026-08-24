use anyhow::{Context, Result, bail};
use clap::{CommandFactory, Parser};
use clap_complete::{Shell, generate};
use colored::Colorize;
use flareops::cli::{
    CheckArgs, Cli, Commands, CompletionsArgs, EnvCommand, EnvPullArgs, EnvSubcommands,
    EnvValidateArgs, MigrateArgs, RoutesCommand, RoutesGenerateArgs, RoutesOptimizeArgs,
    RoutesSimulateArgs, RoutesSubcommands, RoutesValidateArgs, ShellType, SyncArgs,
};
use flareops::env::{EnvDiagnosticSeverity, pull_dev_vars, scan_and_migrate, validate_env};
use flareops::routes::{
    RouteMatchResult, RoutesConfig, Severity, find_static_dir, generate_routes_from_dir,
    optimize_routes, simulate_route, validate_routes_file, write_routes_json,
};
use flareops::sync::{GeneratorOptions, SyncMode, sync_env_file};
use flareops::wrangler::{find_wrangler_config, parse_wrangler_file};
use std::fs;
use std::io;
use std::process::exit;

fn main() {
    if let Err(err) = run() {
        eprintln!("\n{} {err:#}", "✖ Error:".red().bold());
        exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Sync(args) => handle_sync(args),
        Commands::Env(cmd) => handle_env(cmd),
        Commands::Routes(cmd) => handle_routes(cmd),
        Commands::Check(args) => handle_check(args),
        Commands::Migrate(args) => handle_migrate(args),
        Commands::Completions(args) => handle_completions(args),
    }
}

fn handle_sync(args: SyncArgs) -> Result<()> {
    let wrangler_path = if args.path.is_file() {
        args.path.clone()
    } else {
        find_wrangler_config(&args.path)
            .with_context(|| format!("No wrangler config found in {}", args.path.display()))?
    };

    let project_dir = wrangler_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));

    let bindings = parse_wrangler_file(&wrangler_path, args.env.as_deref())?;
    let mode: SyncMode = args.mode.parse().unwrap_or(SyncMode::Astro);

    let out_path = if let Some(out) = args.out {
        out
    } else {
        match mode {
            SyncMode::Astro => project_dir.join("src/env.d.ts"),
            _ => project_dir.join("cloudflare-env.d.ts"),
        }
    };

    let options = GeneratorOptions {
        mode,
        include_comments: true,
    };

    if args.dry_run {
        let result = sync_env_file(&bindings, &out_path, &options, true)?;
        println!(
            "{}",
            "⚡ DRY RUN / Generated Type Declarations:".cyan().bold()
        );
        println!("{}", result.content);
        return Ok(());
    }

    if args.check {
        let result = sync_env_file(&bindings, &out_path, &options, true)?;
        if result.changed {
            println!(
                "{}",
                format!(
                    "✖ Types in {} are out of sync with wrangler config.",
                    out_path.display()
                )
                .red()
                .bold()
            );
            exit(1);
        } else {
            println!(
                "{}",
                format!("✔ Types in {} are up to date.", out_path.display())
                    .green()
                    .bold()
            );
            return Ok(());
        }
    }

    let result = sync_env_file(&bindings, &out_path, &options, false)?;
    if result.created {
        println!(
            "{}",
            format!(
                "✔ Created {} with {} bindings.",
                result.file_path.display(),
                result.bindings_count
            )
            .green()
            .bold()
        );
    } else if result.changed {
        println!(
            "{}",
            format!(
                "✔ Updated {} with {} bindings.",
                result.file_path.display(),
                result.bindings_count
            )
            .green()
            .bold()
        );
    } else {
        println!(
            "{}",
            format!(
                "✔ {} is already up to date ({} bindings).",
                result.file_path.display(),
                result.bindings_count
            )
            .cyan()
        );
    }

    Ok(())
}

fn handle_env(cmd: EnvCommand) -> Result<()> {
    match cmd.subcommand {
        EnvSubcommands::Pull(args) | EnvSubcommands::Sync(args) => handle_env_pull(args),
        EnvSubcommands::Validate(args) => handle_env_validate(args),
        EnvSubcommands::Migrate(args) => handle_migrate(args),
    }
}

fn handle_env_pull(args: EnvPullArgs) -> Result<()> {
    let wrangler_path = find_wrangler_config(&args.path)
        .with_context(|| format!("No wrangler config found in {}", args.path.display()))?;
    let project_dir = wrangler_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));

    let bindings = parse_wrangler_file(&wrangler_path, args.env.as_deref())?;
    let res = pull_dev_vars(&bindings, project_dir, args.example, args.force)?;

    if res.created {
        println!(
            "{}",
            format!(
                "✔ Created {} ({} added, {} preserved)",
                res.target_file.display(),
                res.added_keys.len(),
                res.preserved_keys.len()
            )
            .green()
            .bold()
        );
    } else {
        println!(
            "{}",
            format!(
                "✔ Synced {} ({} added, {} preserved)",
                res.target_file.display(),
                res.added_keys.len(),
                res.preserved_keys.len()
            )
            .green()
            .bold()
        );
    }

    Ok(())
}

fn handle_env_validate(args: EnvValidateArgs) -> Result<()> {
    let wrangler_path = find_wrangler_config(&args.path)
        .with_context(|| format!("No wrangler config found in {}", args.path.display()))?;
    let project_dir = wrangler_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));

    let bindings = parse_wrangler_file(&wrangler_path, args.env.as_deref())?;
    let report = validate_env(&bindings, project_dir);

    println!();
    println!(
        "{}",
        "⚡ FLAREOPS / ENVIRONMENT SECURITY & BINDINGS CHECK".bold()
    );
    println!("{}", "═".repeat(60).dimmed());

    for diag in &report.diagnostics {
        let prefix = match diag.severity {
            EnvDiagnosticSeverity::Error => "✖ [ERROR]".red().bold(),
            EnvDiagnosticSeverity::Warning => "▲ [WARN]".yellow().bold(),
            EnvDiagnosticSeverity::Info => "● [INFO]".cyan(),
        };
        println!("{prefix} {}: {}", diag.key.bold(), diag.message);
        if let Some(ref sug) = diag.suggestion {
            println!("   ↳ {}", sug.dimmed());
        }
    }

    println!();
    if report.is_clean() {
        println!(
            "{}",
            "✔ Environment (.dev.vars) is clean and secure."
                .green()
                .bold()
        );
        Ok(())
    } else {
        if args.strict {
            bail!("Environment validation failed with errors or warnings.");
        }
        Ok(())
    }
}

fn handle_routes(cmd: RoutesCommand) -> Result<()> {
    match cmd.subcommand {
        RoutesSubcommands::Generate(args) => handle_routes_generate(args),
        RoutesSubcommands::Optimize(args) => handle_routes_optimize(args),
        RoutesSubcommands::Validate(args) => handle_routes_validate(args),
        RoutesSubcommands::Simulate(args) => handle_routes_simulate(args),
    }
}

fn handle_routes_generate(args: RoutesGenerateArgs) -> Result<()> {
    let static_dir = if let Some(dir) = args.dir {
        dir
    } else if args.path.is_dir() {
        find_static_dir(&args.path).unwrap_or(args.path.clone())
    } else {
        args.path.clone()
    };

    if !static_dir.exists() {
        bail!("Static directory not found: {}", static_dir.display());
    }

    let config = generate_routes_from_dir(&static_dir)?;
    let out_path = if let Some(out) = args.out {
        out
    } else {
        static_dir.join("_routes.json")
    };

    write_routes_json(&config, &out_path)?;
    println!(
        "{}",
        format!(
            "✔ Generated {} ({} include rules, {} exclude rules, {} total).",
            out_path.display(),
            config.include.len(),
            config.exclude.len(),
            config.total_rules()
        )
        .green()
        .bold()
    );

    Ok(())
}

fn handle_routes_optimize(args: RoutesOptimizeArgs) -> Result<()> {
    let routes_path = if args.path.is_file() {
        args.path.clone()
    } else {
        args.path.join("_routes.json")
    };

    let content = fs::read_to_string(&routes_path)
        .with_context(|| format!("Failed to read {}", routes_path.display()))?;
    let config: RoutesConfig = serde_json::from_str(&content)
        .with_context(|| format!("Invalid JSON format in {}", routes_path.display()))?;

    let optimized = optimize_routes(&config);
    let out_path = args.out.unwrap_or(routes_path);

    write_routes_json(&optimized, &out_path)?;
    println!(
        "{}",
        format!(
            "✔ Optimized {} (reduced from {} to {} total rules).",
            out_path.display(),
            config.total_rules(),
            optimized.total_rules()
        )
        .green()
        .bold()
    );

    Ok(())
}

fn handle_routes_validate(args: RoutesValidateArgs) -> Result<()> {
    let routes_path = if args.path.is_file() {
        args.path.clone()
    } else {
        args.path.join("_routes.json")
    };

    let report = validate_routes_file(&routes_path)?;

    println!();
    println!("{}", "⚡ FLAREOPS / PAGES _ROUTES.JSON VALIDATION".bold());
    println!("{}", "═".repeat(60).dimmed());
    println!("Total rules: {} / 100 maximum", report.total_rules);

    for diag in &report.diagnostics {
        let prefix = match diag.severity {
            Severity::Error => "✖ [ERROR]".red().bold(),
            Severity::Warning => "▲ [WARN]".yellow().bold(),
            Severity::Info => "● [INFO]".cyan(),
        };
        println!("{prefix} [{}] {}", diag.code, diag.message);
        if let Some(ref sug) = diag.suggestion {
            println!("   ↳ {}", sug.dimmed());
        }
    }

    println!();
    if report.is_clean() {
        println!(
            "{}",
            "✔ _routes.json is valid and within Cloudflare limits."
                .green()
                .bold()
        );
        Ok(())
    } else {
        if args.strict {
            bail!("_routes.json validation failed with errors.");
        }
        Ok(())
    }
}

fn handle_routes_simulate(args: RoutesSimulateArgs) -> Result<()> {
    let routes_path = if args.path.is_file() {
        args.path.clone()
    } else {
        args.path.join("_routes.json")
    };

    let content = fs::read_to_string(&routes_path)
        .with_context(|| format!("Failed to read {}", routes_path.display()))?;
    let config: RoutesConfig = serde_json::from_str(&content)?;

    let result = simulate_route(&config, &args.route_path);

    println!();
    println!("{}", "⚡ FLAREOPS / ROUTE RESOLUTION SIMULATION".bold());
    println!("{}", "═".repeat(60).dimmed());
    println!("Testing route: {}", args.route_path.bold());

    match result {
        RouteMatchResult::InvokesFunction { matched_include } => {
            println!(
                "Result: {} (matched include rule `{}`)",
                "INVOKES PAGES FUNCTION (Server-Side Execution)"
                    .green()
                    .bold(),
                matched_include.yellow()
            );
        }
        RouteMatchResult::BypassesFunction { matched_exclude } => {
            println!(
                "Result: {} (matched exclude rule `{}`)",
                "SERVES STATIC ASSET (Bypasses Worker Function)"
                    .cyan()
                    .bold(),
                matched_exclude.yellow()
            );
        }
        RouteMatchResult::NotHandled => {
            println!(
                "Result: {}",
                "NOT HANDLED (Falls back to static / 404)".yellow().bold()
            );
        }
    }
    println!();

    Ok(())
}

fn handle_check(args: CheckArgs) -> Result<()> {
    let report = flareops::check::run_full_check(&args.path, args.env.as_deref())?;
    report.print_summary();

    if !report.is_clean() && args.strict {
        exit(1);
    }

    Ok(())
}

fn handle_migrate(args: MigrateArgs) -> Result<()> {
    let summary = scan_and_migrate(&args.path, args.dry_run, None);

    println!();
    println!("{}", "⚡ FLAREOPS / ASTRO V5 RUNTIME ENV CODEMOD".bold());
    println!("{}", "═".repeat(60).dimmed());
    println!("Scanned files: {}", summary.files_scanned);
    println!("Modified files: {}", summary.files_migrated);
    println!("Total replacements: {}", summary.total_replacements);

    if args.dry_run {
        println!("{}", "\nDry run complete. No files were modified.".cyan());
    } else if summary.total_replacements > 0 {
        println!(
            "{}",
            "\n✔ Migration complete! All files updated to modern Astro.locals."
                .green()
                .bold()
        );
    } else {
        println!(
            "{}",
            "\n✔ No legacy locals.runtime.env calls found.".green()
        );
    }

    Ok(())
}

fn handle_completions(args: CompletionsArgs) -> Result<()> {
    let mut cmd = Cli::command();
    let shell = match args.shell {
        ShellType::Bash => Shell::Bash,
        ShellType::Zsh => Shell::Zsh,
        ShellType::Fish => Shell::Fish,
        ShellType::PowerShell => Shell::PowerShell,
        ShellType::Elvish => Shell::Elvish,
    };
    generate(shell, &mut cmd, "flareops", &mut io::stdout());
    Ok(())
}
