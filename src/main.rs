use anyhow::{Context, Result, bail};
use clap::{CommandFactory, Parser};
use clap_complete::{Shell, generate};
use colored::Colorize;
use flareops::cli::{
    CheckArgs, Cli, Commands, CompletionsArgs, EnvCommand, EnvPullArgs, EnvSubcommands,
    EnvValidateArgs, HeadersCommand, HeadersFixArgs, HeadersGenerateArgs, HeadersSubcommands,
    HeadersValidateArgs, MigrateArgs, RoutesCommand, RoutesGenerateArgs, RoutesOptimizeArgs,
    RoutesSimulateArgs, RoutesSubcommands, RoutesValidateArgs, SessionCheckArgs, SessionCommand,
    SessionInitArgs, SessionSubcommands, ShellType, SyncArgs,
};
use flareops::env::{EnvDiagnosticSeverity, pull_dev_vars, scan_and_migrate, validate_env};
use flareops::headers::{
    HeaderSeverity, HeadersFile, find_headers_file, generate_optimal_headers,
    resolve_headers_target, validate_headers, write_headers_file,
};
use flareops::routes::{
    RouteMatchResult, RoutesConfig, Severity, find_static_dir, generate_routes_from_dir,
    optimize_routes, simulate_route, validate_routes_file, write_routes_json,
};
use flareops::session::{
    AstroConfigInfo, SessionSeverity, find_astro_config, init_session, parse_astro_config,
    scan_directory_for_session, validate_session,
};
use flareops::sync::{GeneratorOptions, SyncMode, sync_env_file};
use flareops::wrangler::{find_wrangler_config, parse_wrangler_file};
use std::fs;
use std::io;
use std::path::Path;
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
        Commands::Headers(cmd) => handle_headers(cmd),
        Commands::Session(cmd) => handle_session(cmd),
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

    let project_dir = wrangler_path.parent().unwrap_or_else(|| Path::new("."));

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
    let project_dir = wrangler_path.parent().unwrap_or_else(|| Path::new("."));

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
    let project_dir = wrangler_path.parent().unwrap_or_else(|| Path::new("."));

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

fn handle_headers(cmd: HeadersCommand) -> Result<()> {
    match cmd.subcommand {
        HeadersSubcommands::Generate(args) => handle_headers_generate(args),
        HeadersSubcommands::Validate(args) => handle_headers_validate(args),
        HeadersSubcommands::Fix(args) => handle_headers_fix(args),
    }
}

fn handle_headers_generate(args: HeadersGenerateArgs) -> Result<()> {
    let project_dir = if args.path.is_file() {
        args.path.parent().unwrap_or_else(|| Path::new("."))
    } else {
        &args.path
    };

    let default_dist = project_dir.join("dist");
    let dist_dir = args.dir.as_deref().or_else(|| {
        if default_dist.is_dir() {
            Some(default_dist.as_path())
        } else {
            None
        }
    });

    let existing_content = find_headers_file(project_dir)
        .and_then(|p| fs::read_to_string(p).ok())
        .unwrap_or_default();

    let parsed = HeadersFile::parse(&existing_content);
    let optimal = generate_optimal_headers(parsed, dist_dir);

    let out_path = args
        .out
        .unwrap_or_else(|| resolve_headers_target(project_dir, dist_dir));
    write_headers_file(&optimal, &out_path)?;

    println!(
        "{}",
        format!(
            "✔ Generated {} with {} header rules (including immutable cache & security headers).",
            out_path.display(),
            optimal.rules.len()
        )
        .green()
        .bold()
    );

    Ok(())
}

fn handle_headers_validate(args: HeadersValidateArgs) -> Result<()> {
    let headers_path = if args.path.is_file() {
        args.path.clone()
    } else {
        find_headers_file(&args.path)
            .with_context(|| format!("No _headers file found in {}", args.path.display()))?
    };

    let default_dist = headers_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("dist");
    let dist_dir = args.dir.as_deref().or_else(|| {
        if default_dist.is_dir() {
            Some(default_dist.as_path())
        } else {
            None
        }
    });

    let content = fs::read_to_string(&headers_path)
        .with_context(|| format!("Failed to read {}", headers_path.display()))?;
    let parsed = HeadersFile::parse(&content);
    let report = validate_headers(&parsed, dist_dir);

    println!();
    println!("{}", "⚡ FLAREOPS / PAGES _HEADERS VALIDATION".bold());
    println!("{}", "═".repeat(60).dimmed());
    println!("File: {}", headers_path.display().to_string().bold());
    println!("Total rules: {}", report.rules_count);

    if report.astro_assets_found > 0 {
        println!("Astro hashed assets found: {}", report.astro_assets_found);
    }

    for diag in &report.diagnostics {
        let prefix = match diag.severity {
            HeaderSeverity::Error => "✖ [ERROR]".red().bold(),
            HeaderSeverity::Warning => "▲ [WARN]".yellow().bold(),
            HeaderSeverity::Info => "● [INFO]".cyan(),
        };
        println!("{prefix} [{}] {}", diag.rule, diag.message);
        if let Some(ref sug) = diag.suggestion {
            println!("   ↳ {}", sug.dimmed());
        }
    }

    println!();
    if report.is_clean() {
        println!(
            "{}",
            "✔ _headers caching rules and security headers are valid."
                .green()
                .bold()
        );
        Ok(())
    } else {
        if args.strict {
            bail!("_headers validation failed with errors or warnings.");
        }
        Ok(())
    }
}

fn handle_headers_fix(args: HeadersFixArgs) -> Result<()> {
    let project_dir = if args.path.is_file() {
        args.path.parent().unwrap_or_else(|| Path::new("."))
    } else {
        &args.path
    };

    let headers_path = if args.path.is_file() {
        args.path.clone()
    } else {
        find_headers_file(project_dir).unwrap_or_else(|| project_dir.join("_headers"))
    };

    let default_dist = project_dir.join("dist");
    let dist_dir = args.dir.as_deref().or_else(|| {
        if default_dist.is_dir() {
            Some(default_dist.as_path())
        } else {
            None
        }
    });

    let existing_content = if headers_path.exists() {
        fs::read_to_string(&headers_path).unwrap_or_default()
    } else {
        String::new()
    };

    let parsed = HeadersFile::parse(&existing_content);
    let optimal = generate_optimal_headers(parsed, dist_dir);

    let out_path = args.out.unwrap_or(headers_path);
    write_headers_file(&optimal, &out_path)?;

    println!(
        "{}",
        format!(
            "✔ Remediated {} with optimized immutable and security header rules.",
            out_path.display()
        )
        .green()
        .bold()
    );

    Ok(())
}

fn handle_session(cmd: SessionCommand) -> Result<()> {
    match cmd.subcommand {
        SessionSubcommands::Check(args) => handle_session_check(args),
        SessionSubcommands::Init(args) => handle_session_init(args),
    }
}

fn handle_session_check(args: SessionCheckArgs) -> Result<()> {
    let project_dir = if args.path.is_file() {
        args.path.parent().unwrap_or_else(|| Path::new("."))
    } else {
        &args.path
    };

    let astro_config_path = find_astro_config(project_dir);
    let astro_config = if let Some(ref apath) = astro_config_path {
        parse_astro_config(apath).unwrap_or_default()
    } else {
        AstroConfigInfo::default()
    };

    let wrangler_path = find_wrangler_config(project_dir);
    let bindings = if let Some(ref wpath) = wrangler_path {
        parse_wrangler_file(wpath, args.env.as_deref()).ok()
    } else {
        None
    };

    let src_dir = project_dir.join("src");
    let session_usages = scan_directory_for_session(&src_dir).unwrap_or_default();

    let report = validate_session(
        project_dir,
        &astro_config,
        bindings.as_ref(),
        &session_usages,
        args.binding.as_deref(),
        args.strict,
    );

    println!();
    println!("{}", "⚡ FLAREOPS / ASTRO SESSION KV BINDING AUDIT".bold());
    println!("{}", "═".repeat(60).dimmed());

    if let Some(ref path) = astro_config.file_path {
        println!("Astro config: {}", path.display().to_string().cyan());
    }
    if let Some(ref path) = wrangler_path {
        println!("Wrangler config: {}", path.display().to_string().cyan());
    }
    println!("Detected session usages: {}", session_usages.len());
    println!();

    for diag in &report.diagnostics {
        let prefix = match diag.severity {
            SessionSeverity::Error => "✖ [ERROR]".red().bold(),
            SessionSeverity::Warning => "▲ [WARN]".yellow().bold(),
            SessionSeverity::Info => "● [INFO]".cyan(),
            SessionSeverity::Success => "✔ [PASS]".green().bold(),
        };
        println!("{prefix} [{}] {}", diag.code.as_str(), diag.message);
        if let Some(ref sug) = diag.suggestion {
            println!("   ↳ {}", sug.dimmed());
        }
    }

    println!();
    if report.passed {
        println!(
            "{}",
            "✔ Astro Cloudflare KV Session configuration is valid."
                .green()
                .bold()
        );
        Ok(())
    } else {
        bail!(
            "Astro session validation failed with {} error(s) and {} warning(s).",
            report.error_count,
            report.warning_count
        );
    }
}

fn handle_session_init(args: SessionInitArgs) -> Result<()> {
    let project_dir = if args.path.is_file() {
        args.path.parent().unwrap_or_else(|| Path::new("."))
    } else {
        &args.path
    };

    let result = init_session(project_dir, &args.binding)?;

    println!();
    println!("{}", "⚡ FLAREOPS / ASTRO SESSION INITIALIZATION".bold());
    println!("{}", "═".repeat(60).dimmed());

    for msg in &result.messages {
        println!("✔ {msg}");
    }

    println!();
    println!(
        "{}",
        format!(
            "✔ Session KV binding '{}' successfully scaffolded.",
            args.binding
        )
        .green()
        .bold()
    );

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
