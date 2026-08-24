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
fn test_cli_parsing_completions() {
    let cli = Cli::try_parse_from(["flareops", "completions", "zsh"]).unwrap();
    match cli.command {
        Commands::Completions(args) => {
            assert!(matches!(args.shell, flareops::cli::ShellType::Zsh));
        }
        _ => panic!("Expected Completions command"),
    }
}
