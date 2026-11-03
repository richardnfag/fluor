use anyhow::{Context, Result, bail};
use clap::{ArgGroup, Args, Parser, Subcommand};
use fluor_cli::{
    Api, FunctionPatch, NewFunction, NewTrigger, Templates, canonical_language, created_output,
    templates_output,
};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser)]
#[command(
    name = "fluor",
    version,
    about = "Manage functions on a Fluor server",
    after_help = "\
Examples:
  fluor functions list
  fluor functions create --name example-rust --language rust --wasm ./example.wasm
  fluor triggers create --name example-rust-get --function example-rust --method GET --path /examples/rust
  fluor invoke /examples/rust --query name=pencil
  fluor templates list
  fluor templates new rust --name example-rust"
)]
struct Cli {
    /// Fluor API base URL.
    #[arg(
        long,
        global = true,
        env = "FLUOR_API_URL",
        default_value = "http://127.0.0.1:8080"
    )]
    server: String,

    /// Print API responses as JSON.
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// List, register, update, and remove functions.
    Functions(FunctionCommand),
    /// List, create, and remove HTTP triggers.
    Triggers(TriggerCommand),
    /// Call a trigger through the /function gateway.
    Invoke(InvokeArgs),
    /// List and generate projects from fluor-template-* repositories.
    Templates(TemplateCommand),
}

#[derive(Args)]
struct FunctionCommand {
    #[command(subcommand)]
    action: FunctionAction,
}

#[derive(Subcommand)]
enum FunctionAction {
    /// List functions.
    List,
    /// Show one function.
    Get { name: String },
    /// Register a function and upload its Wasm component.
    Create(CreateFunctionArgs),
    /// Change a function. A new Wasm file replaces the stored component.
    Update(UpdateFunctionArgs),
    /// Remove a function.
    Delete { name: String },
}

#[derive(Args)]
struct CreateFunctionArgs {
    #[arg(long)]
    name: String,
    #[arg(long, value_parser = canonical_language)]
    language: String,
    /// Path to the local Wasm component.
    #[arg(long)]
    wasm: PathBuf,
    #[arg(long, default_value = "100m")]
    cpu: String,
    #[arg(long, default_value = "128Mi")]
    memory: String,
}

#[derive(Args)]
#[command(group(
    ArgGroup::new("change")
        .required(true)
        .multiple(true)
        .args(["language", "wasm", "cpu", "memory"])
))]
struct UpdateFunctionArgs {
    name: String,
    #[arg(long, value_parser = canonical_language)]
    language: Option<String>,
    #[arg(long)]
    wasm: Option<PathBuf>,
    #[arg(long)]
    cpu: Option<String>,
    #[arg(long)]
    memory: Option<String>,
}

#[derive(Args)]
struct TriggerCommand {
    #[command(subcommand)]
    action: TriggerAction,
}

#[derive(Subcommand)]
enum TriggerAction {
    /// List triggers.
    List,
    /// Route an HTTP method and path to a function.
    Create(CreateTriggerArgs),
    /// Remove a trigger.
    Delete { name: String },
}

#[derive(Args)]
struct CreateTriggerArgs {
    #[arg(long)]
    name: String,
    #[arg(long)]
    function: String,
    #[arg(long)]
    method: String,
    #[arg(long)]
    path: String,
}

#[derive(Args)]
struct TemplateCommand {
    /// GitHub account that publishes the fluor-template-* repositories.
    #[arg(
        long,
        global = true,
        env = "FLUOR_GITHUB_OWNER",
        default_value = "richardnfag"
    )]
    owner: String,

    #[command(subcommand)]
    action: TemplateAction,
}

#[derive(Subcommand)]
enum TemplateAction {
    /// List the public function templates.
    List,
    /// Generate a project from a template. Placeholders use the project name.
    New(NewTemplateArgs),
}

#[derive(Args)]
struct NewTemplateArgs {
    /// Template language, for example rust, python, go, javascript, or csharp.
    #[arg(value_parser = canonical_language)]
    language: String,
    #[arg(long)]
    name: String,
    /// Directory to create. Defaults to the project name.
    #[arg(long)]
    output: Option<PathBuf>,
    /// Git ref to download. Defaults to the open pull request, then the default branch.
    #[arg(long = "ref")]
    git_ref: Option<String>,
}

#[derive(Args)]
struct InvokeArgs {
    /// Trigger path, with or without a leading slash.
    path: String,
    #[arg(long, default_value = "GET")]
    method: String,
    /// Raw query string, without the leading question mark.
    #[arg(long)]
    query: Option<String>,
    /// Request header in name:value form. Repeat for more than one header.
    #[arg(long = "header", value_name = "NAME:VALUE", value_parser = split_header)]
    headers: Vec<(String, String)>,
    /// Request body.
    #[arg(long, conflicts_with = "data_file")]
    data: Option<String>,
    /// Read the request body from a file.
    #[arg(long, conflicts_with = "data")]
    data_file: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(code) => code,
        Err(error) => {
            eprintln!("{error:#}");
            ExitCode::from(1)
        }
    }
}

async fn run() -> Result<ExitCode> {
    let cli = Cli::parse();
    let api = Api::new(&cli.server)?;
    match cli.command {
        Command::Functions(command) => functions(&api, cli.json, command).await,
        Command::Triggers(command) => triggers(&api, cli.json, command).await,
        Command::Invoke(args) => invoke(&api, cli.json, args).await,
        Command::Templates(command) => templates(cli.json, command).await,
    }
}

async fn functions(api: &Api, json: bool, command: FunctionCommand) -> Result<ExitCode> {
    match command.action {
        FunctionAction::List => {
            let functions = api.list_functions().await?;
            print_value(json, &functions, fluor_cli::functions_text(&functions))?;
        }
        FunctionAction::Get { name } => {
            let function = api.get_function(&name).await?;
            print_value(json, &function, fluor_cli::function_text(&function))?;
        }
        FunctionAction::Create(args) => {
            let created = api
                .create_function(&NewFunction {
                    name: args.name,
                    language: args.language,
                    wasm: args.wasm,
                    cpu: args.cpu,
                    memory: args.memory,
                })
                .await?;
            print_value(json, &created, fluor_cli::function_text(&created))?;
        }
        FunctionAction::Update(args) => {
            let updated = api
                .update_function(
                    &args.name,
                    &FunctionPatch {
                        language: args.language,
                        wasm: args.wasm,
                        cpu: args.cpu,
                        memory: args.memory,
                    },
                )
                .await?;
            print_value(json, &updated, fluor_cli::function_text(&updated))?;
        }
        FunctionAction::Delete { name } => {
            api.delete_function(&name).await?;
            print_done(json, &format!("deleted function {name}"))?;
        }
    }
    Ok(ExitCode::SUCCESS)
}

async fn triggers(api: &Api, json: bool, command: TriggerCommand) -> Result<ExitCode> {
    match command.action {
        TriggerAction::List => {
            let triggers = api.list_triggers().await?;
            print_value(json, &triggers, fluor_cli::triggers_text(&triggers))?;
        }
        TriggerAction::Create(args) => {
            let created = api
                .create_trigger(&NewTrigger {
                    name: args.name,
                    method: args.method.to_ascii_uppercase(),
                    path: args.path,
                    function: args.function,
                })
                .await?;
            print_value(json, &created, fluor_cli::trigger_text(&created))?;
        }
        TriggerAction::Delete { name } => {
            api.delete_trigger(&name).await?;
            print_done(json, &format!("deleted trigger {name}"))?;
        }
    }
    Ok(ExitCode::SUCCESS)
}

async fn templates(json: bool, command: TemplateCommand) -> Result<ExitCode> {
    let templates = Templates::new("https://api.github.com", &command.owner)?;
    match command.action {
        TemplateAction::List => {
            let listed = templates.list().await?;
            let text = templates_output(&listed, json)?;
            if json {
                println!("{text}");
            } else {
                print!("{text}");
            }
        }
        TemplateAction::New(args) => {
            let created = templates
                .create(
                    &args.language,
                    &args.name,
                    args.output.as_deref(),
                    args.git_ref.as_deref(),
                )
                .await?;
            print!("{}", created_output(&created, json)?);
        }
    }
    Ok(ExitCode::SUCCESS)
}

async fn invoke(api: &Api, json: bool, args: InvokeArgs) -> Result<ExitCode> {
    let body = match args.data_file {
        Some(path) => tokio::fs::read_to_string(&path)
            .await
            .with_context(|| format!("failed to read {}", path.display()))?,
        None => args.data.unwrap_or_default(),
    };
    let invocation = api
        .invoke(
            &args.path,
            &args.method.to_ascii_uppercase(),
            args.query.as_deref(),
            &args.headers,
            &body,
        )
        .await?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "status": invocation.status,
                "body": invocation.body,
            }))?
        );
    } else {
        println!("{}", invocation.body);
    }
    if invocation.status >= 400 {
        bail!("HTTP {}", invocation.status);
    }
    Ok(ExitCode::SUCCESS)
}

fn print_value<T: serde::Serialize>(json: bool, value: &T, text: String) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(value)?);
    } else {
        print!("{text}");
    }
    Ok(())
}

fn print_done(json: bool, message: &str) -> Result<()> {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({ "message": message }))?
        );
    } else {
        println!("{message}");
    }
    Ok(())
}

fn split_header(value: &str) -> Result<(String, String), String> {
    let (name, raw_value) = value
        .split_once(':')
        .ok_or_else(|| format!("expected name:value, got '{value}'"))?;
    let name = name.trim();
    if name.is_empty() {
        return Err("header name is empty".to_string());
    }
    Ok((name.to_string(), raw_value.trim().to_string()))
}
