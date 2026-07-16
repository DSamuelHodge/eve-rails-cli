use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::{Args, Parser, Subcommand, ValueEnum};
use serde::Deserialize;

#[derive(Debug, Parser)]
#[command(name = "eve-rails")]
#[command(about = "Rails-inspired convention layer for Eve agent fleets")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Show the files and components that would be generated from a manifest.
    Plan(ManifestCommand),
    /// Apply a manifest by rendering files. Stubbed until PR 2.
    Apply(ManifestCommand),
    /// Render generated files from YAML and templates.
    Render(RenderCommand),
    /// Validate project, manifest, templates, versions, and deployment readiness.
    Doctor(DoctorCommand),
    /// Generate an agent component.
    Generate(GenerateCommand),
    /// Show available component updates.
    Outdated(ManifestCommand),
    /// Plan or apply component updates.
    Update(UpdateCommand),
    /// Hot-load a compatible component update.
    Hotload(HotloadCommand),
    /// Run deploy preflight checks. Actual deployment delegates to Eve/Vercel later.
    Deploy(DeployCommand),
    /// Inspect one agent's composition.
    Inspect(AgentCommand),
    /// Print a fleet graph.
    Graph(GraphCommand),
}

#[derive(Debug, Args)]
struct ManifestCommand {
    /// Path to the fleet manifest.
    #[arg(default_value = "manifests/agents.yml")]
    manifest: PathBuf,

    /// Emit machine-readable JSON.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct RenderCommand {
    /// Render a single agent by name.
    #[arg(long)]
    agent: Option<String>,

    /// Render every agent in the manifest.
    #[arg(long)]
    all: bool,

    /// Check whether generated files are fresh without writing.
    #[arg(long)]
    check: bool,

    /// Path to the fleet manifest.
    #[arg(long, default_value = "manifests/agents.yml")]
    manifest: PathBuf,

    /// Path to the template directory.
    #[arg(long, default_value = "templates/agent")]
    templates: PathBuf,
}

#[derive(Debug, Args)]
struct DoctorCommand {
    /// Validate all agents.
    #[arg(long)]
    all: bool,

    /// Validate update and lockfile compatibility.
    #[arg(long)]
    updates: bool,

    /// Validate template rendering behavior.
    #[arg(long)]
    templates: bool,

    /// Emit machine-readable JSON.
    #[arg(long)]
    json: bool,

    /// Path to the fleet manifest.
    #[arg(long, default_value = "manifests/agents.yml")]
    manifest: PathBuf,
}

#[derive(Debug, Args)]
struct GenerateCommand {
    #[command(subcommand)]
    component: GenerateComponent,
}

#[derive(Debug, Subcommand)]
enum GenerateComponent {
    Agent(GenerateNamed),
    Tool(GenerateNamed),
    Skill(GenerateNamed),
    Subagent(GenerateNamed),
    Channel(GenerateNamed),
    Approval(GenerateNamed),
    Eval(GenerateNamed),
    Memory(GenerateNamed),
    Batch(ManifestCommand),
    Migration(GenerateNamed),
}

#[derive(Debug, Args)]
struct GenerateNamed {
    name: String,

    /// Show planned changes without writing files.
    #[arg(long)]
    dry_run: bool,

    /// Overwrite existing generated files where safe.
    #[arg(long)]
    force: bool,

    /// Emit machine-readable JSON.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct UpdateCommand {
    /// Agent to update.
    #[arg(long)]
    agent: Option<String>,

    /// Fleet manifest to update.
    #[arg(long)]
    fleet: Option<PathBuf>,

    /// Allow patch updates.
    #[arg(long)]
    patch: bool,

    /// Allow minor updates.
    #[arg(long)]
    minor: bool,

    /// Allow major updates.
    #[arg(long)]
    major: bool,

    /// Show plan without applying.
    #[arg(long)]
    plan: bool,

    /// Apply the update.
    #[arg(long)]
    apply: bool,
}

#[derive(Debug, Args)]
struct HotloadCommand {
    /// Agent receiving the hot-loaded update.
    #[arg(long)]
    agent: String,

    /// Component reference such as skill:handle_refund@2.0.1.
    component: String,
}

#[derive(Debug, Args)]
struct DeployCommand {
    /// Agent to deploy.
    #[arg(long)]
    agent: Option<String>,

    /// Fleet manifest to deploy.
    #[arg(long)]
    fleet: Option<PathBuf>,

    /// Deployment environment.
    #[arg(long, default_value = "staging")]
    env: String,

    /// Require eval pass before deployment.
    #[arg(long)]
    require_evals: bool,

    /// Require doctor pass before deployment.
    #[arg(long)]
    require_doctor: bool,

    /// Percentage of traffic for canary deployment.
    #[arg(long)]
    canary: Option<u8>,
}

#[derive(Debug, Args)]
struct AgentCommand {
    /// Agent to inspect.
    #[arg(long)]
    agent: String,

    /// Path to the fleet manifest.
    #[arg(long, default_value = "manifests/agents.yml")]
    manifest: PathBuf,

    /// Emit machine-readable JSON.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct GraphCommand {
    /// Include every agent in the manifest.
    #[arg(long)]
    all: bool,

    /// Graph one agent.
    #[arg(long)]
    agent: Option<String>,

    /// Output format.
    #[arg(long, default_value = "text")]
    format: GraphFormat,

    /// Path to the fleet manifest.
    #[arg(long, default_value = "manifests/agents.yml")]
    manifest: PathBuf,
}

#[derive(Clone, Debug, ValueEnum)]
enum GraphFormat {
    Text,
    Mermaid,
    Json,
}

#[derive(Debug, Deserialize)]
struct FleetManifest {
    #[serde(default)]
    defaults: ManifestDefaults,
    #[serde(default)]
    shared: SharedComponents,
    agents: Vec<AgentManifest>,
}

#[derive(Debug, Default, Deserialize)]
struct ManifestDefaults {
    model: Option<String>,
    owner: Option<String>,
    #[serde(default)]
    channels: Vec<String>,
    #[serde(default)]
    evals: Vec<String>,
    approvals: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct SharedComponents {
    #[serde(default)]
    tools: Vec<String>,
    #[serde(default)]
    skills: Vec<String>,
    #[serde(default)]
    memory: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct AgentManifest {
    name: String,
    version: Option<String>,
    owner: Option<String>,
    responsibility: String,
    model: Option<String>,
    #[serde(default)]
    tools: ComponentMap,
    #[serde(default)]
    skills: ComponentMap,
    #[serde(default)]
    subagents: Vec<String>,
    #[serde(default)]
    channels: Vec<String>,
    #[serde(default)]
    approvals: BTreeMap<String, String>,
    #[serde(default)]
    evals: Vec<String>,
    #[serde(default)]
    memory: ComponentMap,
}

type ComponentMap = BTreeMap<String, String>;

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Plan(command) => plan(command),
        Command::Apply(command) => stub_manifest_command("apply", command),
        Command::Render(command) => render(command),
        Command::Doctor(command) => doctor(command),
        Command::Generate(command) => generate(command),
        Command::Outdated(command) => stub_manifest_command("outdated", command),
        Command::Update(command) => update(command),
        Command::Hotload(command) => hotload(command),
        Command::Deploy(command) => deploy(command),
        Command::Inspect(command) => inspect(command),
        Command::Graph(command) => graph(command),
    }
}

fn plan(command: ManifestCommand) -> Result<()> {
    let manifest = load_manifest(&command.manifest)?;
    let report = validate_manifest(&manifest);

    if command.json {
        let output = serde_json::json!({
            "manifest": command.manifest,
            "agent_count": manifest.agents.len(),
            "errors": report.errors,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    println!("Plan for {}", command.manifest.display());
    println!("Agents: {}", manifest.agents.len());
    println!(
        "Shared: {} tools, {} skills, {} memory schemas",
        manifest.shared.tools.len(),
        manifest.shared.skills.len(),
        manifest.shared.memory.len()
    );

    for agent in &manifest.agents {
        println!();
        println!("- {}", agent.name);
        println!("  responsibility: {}", agent.responsibility);
        println!("  version: {}", agent.version.as_deref().unwrap_or("1.0.0"));
        println!(
            "  model: {}",
            agent
                .model
                .as_deref()
                .or(manifest.defaults.model.as_deref())
                .unwrap_or("<missing>")
        );
        println!(
            "  owner: {}",
            agent
                .owner
                .as_deref()
                .or(manifest.defaults.owner.as_deref())
                .unwrap_or("<missing>")
        );
        println!("  tools: {}", agent.tools.len());
        println!("  skills: {}", agent.skills.len());
        println!("  subagents: {}", agent.subagents.len());
        println!(
            "  channels: {}",
            agent.channels.len() + manifest.defaults.channels.len()
        );
        println!(
            "  approvals: {}",
            if agent.approvals.is_empty() {
                manifest.defaults.approvals.as_deref().unwrap_or("<none>")
            } else {
                "custom"
            }
        );
        println!(
            "  evals: {}",
            agent.evals.len() + manifest.defaults.evals.len()
        );
        println!("  memory: {}", agent.memory.len());
    }

    print_validation_report(&report)?;
    Ok(())
}

fn render(command: RenderCommand) -> Result<()> {
    let manifest = load_manifest(&command.manifest)?;
    let report = validate_manifest(&manifest);
    ensure_valid(&report)?;

    let scope = if command.all {
        "all agents".to_string()
    } else if let Some(agent) = command.agent {
        format!("agent {agent}")
    } else {
        bail!("pass --agent <name> or --all");
    };

    println!(
        "Render {scope} using {} ({})",
        command.templates.display(),
        if command.check {
            "check only"
        } else {
            "write mode"
        }
    );
    println!("Rendering is stubbed until PR 2.");
    Ok(())
}

fn doctor(command: DoctorCommand) -> Result<()> {
    let manifest = load_manifest(&command.manifest)?;
    let report = validate_manifest(&manifest);

    if command.json {
        let output = serde_json::json!({
            "manifest": command.manifest,
            "all": command.all,
            "updates": command.updates,
            "templates": command.templates,
            "passed": report.errors.is_empty(),
            "errors": report.errors,
            "warnings": report.warnings,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    println!("Doctor for {}", command.manifest.display());
    println!("Agents checked: {}", manifest.agents.len());
    if command.templates {
        println!("Template checks requested.");
    }
    if command.updates {
        println!("Update compatibility checks requested.");
    }
    print_validation_report(&report)?;
    ensure_valid(&report)
}

fn generate(command: GenerateCommand) -> Result<()> {
    match command.component {
        GenerateComponent::Batch(command) => plan(command),
        GenerateComponent::Agent(command) => stub_generate("agent", command),
        GenerateComponent::Tool(command) => stub_generate("tool", command),
        GenerateComponent::Skill(command) => stub_generate("skill", command),
        GenerateComponent::Subagent(command) => stub_generate("subagent", command),
        GenerateComponent::Channel(command) => stub_generate("channel", command),
        GenerateComponent::Approval(command) => stub_generate("approval", command),
        GenerateComponent::Eval(command) => stub_generate("eval", command),
        GenerateComponent::Memory(command) => stub_generate("memory", command),
        GenerateComponent::Migration(command) => stub_generate("migration", command),
    }
}

fn update(command: UpdateCommand) -> Result<()> {
    println!("Update command accepted: {command:?}");
    println!("Version resolution is stubbed until PR 6.");
    Ok(())
}

fn hotload(command: HotloadCommand) -> Result<()> {
    println!(
        "Hot-load requested for agent {} with component {}",
        command.agent, command.component
    );
    println!("Compatibility classification is stubbed until PR 7.");
    Ok(())
}

fn deploy(command: DeployCommand) -> Result<()> {
    println!("Deploy preflight for env {}", command.env);
    if command.require_doctor {
        println!("Doctor gate required.");
    }
    if command.require_evals {
        println!("Eval gate required.");
    }
    if let Some(canary) = command.canary {
        println!("Canary: {canary}%");
    }
    println!("Deployment delegation is stubbed until PR 8.");
    Ok(())
}

fn inspect(command: AgentCommand) -> Result<()> {
    let manifest = load_manifest(&command.manifest)?;
    let agent = manifest
        .agents
        .iter()
        .find(|agent| agent.name == command.agent)
        .with_context(|| format!("agent '{}' not found", command.agent))?;

    if command.json {
        let output = serde_json::json!({
            "name": agent.name,
            "version": agent.version.as_deref().unwrap_or("1.0.0"),
            "owner": agent.owner.as_deref().or(manifest.defaults.owner.as_deref()),
            "model": agent.model.as_deref().or(manifest.defaults.model.as_deref()),
            "responsibility": agent.responsibility,
            "tools": agent.tools,
            "skills": agent.skills,
            "subagents": agent.subagents,
            "channels": agent.channels,
            "approvals": agent.approvals,
            "evals": agent.evals,
            "memory": agent.memory,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    println!("{}", agent.name);
    println!("  version: {}", agent.version.as_deref().unwrap_or("1.0.0"));
    println!(
        "  owner: {}",
        agent
            .owner
            .as_deref()
            .or(manifest.defaults.owner.as_deref())
            .unwrap_or("<missing>")
    );
    println!(
        "  model: {}",
        agent
            .model
            .as_deref()
            .or(manifest.defaults.model.as_deref())
            .unwrap_or("<missing>")
    );
    println!("  responsibility: {}", agent.responsibility);
    println!("  tools: {}", agent.tools.len());
    println!("  skills: {}", agent.skills.len());
    println!("  subagents: {}", agent.subagents.len());
    println!("  approvals: {}", agent.approvals.len());
    println!(
        "  evals: {}",
        agent.evals.len() + manifest.defaults.evals.len()
    );
    println!("  memory: {}", agent.memory.len());
    Ok(())
}

fn graph(command: GraphCommand) -> Result<()> {
    let manifest = load_manifest(&command.manifest)?;
    let agents: Vec<&AgentManifest> = if command.all {
        manifest.agents.iter().collect()
    } else if let Some(agent_name) = &command.agent {
        vec![
            manifest
                .agents
                .iter()
                .find(|agent| agent.name == *agent_name)
                .with_context(|| format!("agent '{agent_name}' not found"))?,
        ]
    } else {
        bail!("pass --agent <name> or --all");
    };

    match command.format {
        GraphFormat::Text => {
            for agent in agents {
                println!("{}", agent.name);
                for subagent in &agent.subagents {
                    println!("  -> subagent/{subagent}");
                }
                for tool in agent.tools.keys() {
                    println!("  -> tools/{tool}");
                }
                for skill in agent.skills.keys() {
                    println!("  -> skills/{skill}");
                }
                for memory in agent.memory.keys() {
                    println!("  -> memory/{memory}");
                }
            }
        }
        GraphFormat::Mermaid => {
            println!("graph TD");
            for agent in agents {
                for subagent in &agent.subagents {
                    println!("  {} --> subagent_{}", node(&agent.name), node(subagent));
                }
                for tool in agent.tools.keys() {
                    println!("  {} --> tool_{}", node(&agent.name), node(tool));
                }
                for skill in agent.skills.keys() {
                    println!("  {} --> skill_{}", node(&agent.name), node(skill));
                }
                for memory in agent.memory.keys() {
                    println!("  {} --> memory_{}", node(&agent.name), node(memory));
                }
            }
        }
        GraphFormat::Json => {
            let output: Vec<_> = agents
                .iter()
                .map(|agent| {
                    serde_json::json!({
                        "name": agent.name,
                        "subagents": agent.subagents,
                        "tools": agent.tools.keys().collect::<Vec<_>>(),
                        "skills": agent.skills.keys().collect::<Vec<_>>(),
                        "memory": agent.memory.keys().collect::<Vec<_>>(),
                    })
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
    }

    Ok(())
}

fn stub_manifest_command(name: &str, command: ManifestCommand) -> Result<()> {
    let manifest = load_manifest(&command.manifest)?;
    let report = validate_manifest(&manifest);
    ensure_valid(&report)?;

    if command.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "command": name,
                "manifest": command.manifest,
                "agent_count": manifest.agents.len(),
                "implemented": false,
            }))?
        );
        return Ok(());
    }

    println!("{name} accepted for {}", command.manifest.display());
    println!("Implementation is stubbed for a later PR.");
    Ok(())
}

fn stub_generate(kind: &str, command: GenerateNamed) -> Result<()> {
    if command.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "kind": kind,
                "name": command.name,
                "dry_run": command.dry_run,
                "force": command.force,
                "implemented": false,
            }))?
        );
        return Ok(());
    }

    println!(
        "Generate {kind} '{}' ({})",
        command.name,
        if command.dry_run {
            "dry run"
        } else {
            "write mode"
        }
    );
    println!("Generator implementation is stubbed until PR 3.");
    Ok(())
}

fn load_manifest(path: &Path) -> Result<FleetManifest> {
    let source = fs::read_to_string(path)
        .with_context(|| format!("failed to read manifest '{}'", path.display()))?;
    let manifest: FleetManifest = serde_yaml::from_str(&source)
        .with_context(|| format!("failed to parse manifest '{}'", path.display()))?;
    Ok(manifest)
}

#[derive(Debug, Default)]
struct ValidationReport {
    errors: Vec<String>,
    warnings: Vec<String>,
}

fn validate_manifest(manifest: &FleetManifest) -> ValidationReport {
    let mut report = ValidationReport::default();

    if manifest.agents.is_empty() {
        report
            .errors
            .push("manifest must define at least one agent".to_string());
    }

    for agent in &manifest.agents {
        if agent.name.trim().is_empty() {
            report.errors.push("agent name cannot be empty".to_string());
        }
        if !is_slug(&agent.name) {
            report.errors.push(format!(
                "agent '{}' must use lowercase kebab-case or snake_case",
                agent.name
            ));
        }
        if agent.responsibility.trim().is_empty() {
            report
                .errors
                .push(format!("agent '{}' must define responsibility", agent.name));
        }
        if agent.model.is_none() && manifest.defaults.model.is_none() {
            report.errors.push(format!(
                "agent '{}' must define model or inherit defaults.model",
                agent.name
            ));
        }
        if agent.owner.is_none() && manifest.defaults.owner.is_none() {
            report.errors.push(format!(
                "agent '{}' must define owner or inherit defaults.owner",
                agent.name
            ));
        }
        if agent.evals.is_empty() && manifest.defaults.evals.is_empty() {
            report
                .warnings
                .push(format!("agent '{}' has no evals", agent.name));
        }

        for (tool, version) in &agent.tools {
            if version.trim().is_empty() {
                report.errors.push(format!(
                    "agent '{}' tool '{}' has empty version",
                    agent.name, tool
                ));
            }
            if is_risky_tool(tool) && !agent.approvals.contains_key(tool) {
                report.errors.push(format!(
                    "agent '{}' risky tool '{}' must have approval policy",
                    agent.name, tool
                ));
            }
        }

        for (memory, version) in &agent.memory {
            if version.trim().is_empty() {
                report.errors.push(format!(
                    "agent '{}' memory '{}' has empty version",
                    agent.name, memory
                ));
            }
        }
    }

    report
}

fn print_validation_report(report: &ValidationReport) -> Result<()> {
    if report.errors.is_empty() && report.warnings.is_empty() {
        println!();
        println!("Validation: passed");
        return Ok(());
    }

    if !report.warnings.is_empty() {
        println!();
        println!("Warnings:");
        for warning in &report.warnings {
            println!("- {warning}");
        }
    }

    if !report.errors.is_empty() {
        println!();
        println!("Errors:");
        for error in &report.errors {
            println!("- {error}");
        }
    }

    Ok(())
}

fn ensure_valid(report: &ValidationReport) -> Result<()> {
    if report.errors.is_empty() {
        Ok(())
    } else {
        bail!("validation failed with {} error(s)", report.errors.len())
    }
}

fn is_slug(value: &str) -> bool {
    value.chars().all(|char| {
        char.is_ascii_lowercase() || char.is_ascii_digit() || char == '-' || char == '_'
    })
}

fn is_risky_tool(tool: &str) -> bool {
    let tool = tool.to_ascii_lowercase();
    [
        "refund",
        "delete",
        "publish",
        "production",
        "payment",
        "charge",
        "email",
    ]
    .iter()
    .any(|risk| tool.contains(risk))
}

fn node(value: &str) -> String {
    value
        .chars()
        .map(|char| {
            if char.is_ascii_alphanumeric() {
                char
            } else {
                '_'
            }
        })
        .collect()
}
