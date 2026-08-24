use clap::Parser;
use flareops::cli::{Cli, Commands};

#[test]
fn test_cli_parsing_sync() {
    let cli = Cli::try_parse_from([
        "flareops", "sync", "./my-app", "--mode", "worker", "--check",
    ])
    .unwrap();
    match cli.command {
        Commands::Sync(args) => {
            assert_eq!(args.path.to_str().unwrap(), "./my-app");
            assert_eq!(args.mode, "worker");
            assert!(args.check);
        }
        _ => panic!("Expected Sync command"),
    }
}

#[test]
fn test_cli_parsing_env_pull() {
    let cli = Cli::try_parse_from(["flareops", "env", "pull", "--example", "--force"]).unwrap();
    match cli.command {
        Commands::Env(cmd) => match cmd.subcommand {
            flareops::cli::EnvSubcommands::Pull(args) => {
                assert!(args.example);
                assert!(args.force);
            }
            _ => panic!("Expected Env Pull command"),
        },
        _ => panic!("Expected Env command"),
    }
}

#[test]
fn test_cli_parsing_routes_generate() {
    let cli = Cli::try_parse_from([
        "flareops",
        "routes",
        "generate",
        "dist",
        "--out",
        "dist/_routes.json",
    ])
    .unwrap();
    match cli.command {
        Commands::Routes(cmd) => match cmd.subcommand {
            flareops::cli::RoutesSubcommands::Generate(args) => {
                assert_eq!(args.path.to_str().unwrap(), "dist");
                assert_eq!(args.out.unwrap().to_str().unwrap(), "dist/_routes.json");
            }
            _ => panic!("Expected Routes Generate command"),
        },
        _ => panic!("Expected Routes command"),
    }
}

#[test]
fn test_cli_parsing_headers_commands() {
    let cli = Cli::try_parse_from([
        "flareops",
        "headers",
        "generate",
        "dist",
        "--out",
        "dist/_headers",
    ])
    .unwrap();
    match cli.command {
        Commands::Headers(cmd) => match cmd.subcommand {
            flareops::cli::HeadersSubcommands::Generate(args) => {
                assert_eq!(args.path.to_str().unwrap(), "dist");
                assert_eq!(args.out.unwrap().to_str().unwrap(), "dist/_headers");
            }
            _ => panic!("Expected Headers Generate command"),
        },
        _ => panic!("Expected Headers command"),
    }

    let cli_validate =
        Cli::try_parse_from(["flareops", "headers", "validate", "--strict"]).unwrap();
    match cli_validate.command {
        Commands::Headers(cmd) => match cmd.subcommand {
            flareops::cli::HeadersSubcommands::Validate(args) => {
                assert!(args.strict);
            }
            _ => panic!("Expected Headers Validate command"),
        },
        _ => panic!("Expected Headers command"),
    }
}

#[test]
fn test_cli_parsing_session_commands() {
    let cli = Cli::try_parse_from([
        "flareops", "session", "check", "--binding", "MY_SESSION", "--strict",
    ])
    .unwrap();
    match cli.command {
        Commands::Session(cmd) => match cmd.subcommand {
            flareops::cli::SessionSubcommands::Check(args) => {
                assert_eq!(args.binding.as_deref(), Some("MY_SESSION"));
                assert!(args.strict);
            }
            _ => panic!("Expected Session Check command"),
        },
        _ => panic!("Expected Session command"),
    }

    let cli_init =
        Cli::try_parse_from(["flareops", "session", "init", "--binding", "KV_SESSION"]).unwrap();
    match cli_init.command {
        Commands::Session(cmd) => match cmd.subcommand {
            flareops::cli::SessionSubcommands::Init(args) => {
                assert_eq!(args.binding, "KV_SESSION");
            }
            _ => panic!("Expected Session Init command"),
        },
        _ => panic!("Expected Session command"),
    }
}

#[test]
fn test_cli_parsing_completions() {
    let cli = Cli::try_parse_from(["flareops", "completions", "zsh"]).unwrap();
    match cli.command {
        Commands::Completions(args) => {
            assert!(matches!(args.shell, flareops::cli::ShellType::Zsh));
        }
        _ => panic!("Expected Completions command"),
    }
}
