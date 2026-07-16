use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::{Args, Parser, Subcommand, ValueEnum};
use minijinja::{Environment, context};
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

    /// Path to the reusable component catalog.
    #[arg(long, default_value = "manifests/catalog.yml")]
    catalog: PathBuf,

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

    /// Path to the reusable component catalog.
    #[arg(long, default_value = "manifests/catalog.yml")]
    catalog: PathBuf,

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

    /// Path to the reusable component catalog.
    #[arg(long, default_value = "manifests/catalog.yml")]
    catalog: PathBuf,
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

    /// Path to the fleet manifest.
    #[arg(long, default_value = "manifests/agents.yml")]
    manifest: PathBuf,

    /// Path to the reusable component catalog.
    #[arg(long, default_value = "manifests/catalog.yml")]
    catalog: PathBuf,

    /// Component version.
    #[arg(long, default_value = "1.0.0")]
    version: String,

    /// Agent owner.
    #[arg(long)]
    owner: Option<String>,

    /// Agent model.
    #[arg(long)]
    model: Option<String>,

    /// Agent or component responsibility/description.
    #[arg(long)]
    description: Option<String>,

    /// Tool side-effect class.
    #[arg(long, default_value = "read")]
    side_effects: SideEffects,

    /// Memory retention period, such as 180d.
    #[arg(long)]
    retention: Option<String>,

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

    /// Path to the reusable component catalog.
    #[arg(long, default_value = "manifests/catalog.yml")]
    catalog: PathBuf,

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

    /// Path to the reusable component catalog.
    #[arg(long, default_value = "manifests/catalog.yml")]
    catalog: PathBuf,
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

#[derive(Debug, Default, Deserialize)]
struct CatalogManifest {
    #[serde(default)]
    tools: BTreeMap<String, CatalogComponent>,
    #[serde(default)]
    skills: BTreeMap<String, CatalogComponent>,
    #[serde(default)]
    evals: BTreeMap<String, CatalogComponent>,
    #[serde(default)]
    approvals: BTreeMap<String, CatalogComponent>,
    #[serde(default)]
    memory: BTreeMap<String, CatalogComponent>,
    #[serde(default)]
    channels: BTreeMap<String, CatalogComponent>,
}

#[derive(Debug, Default, Deserialize)]
struct CatalogComponent {
    version: String,
    side_effects: Option<SideEffects>,
    retention: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, ValueEnum)]
#[serde(rename_all = "kebab-case")]
enum SideEffects {
    None,
    Read,
    Write,
    External,
    Money,
    Production,
}

impl std::fmt::Display for SideEffects {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            SideEffects::None => "none",
            SideEffects::Read => "read",
            SideEffects::Write => "write",
            SideEffects::External => "external",
            SideEffects::Money => "money",
            SideEffects::Production => "production",
        };
        formatter.write_str(value)
    }
}

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
    let catalog = load_catalog(&command.catalog)?;
    let report = validate_manifest(&manifest, &catalog);

    if command.json {
        let output = serde_json::json!({
            "manifest": command.manifest,
            "catalog": command.catalog,
            "agent_count": manifest.agents.len(),
            "errors": report.errors,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    println!("Plan for {}", command.manifest.display());
    println!("Catalog: {}", command.catalog.display());
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
    let catalog = load_catalog(&command.catalog)?;
    let report = validate_manifest(&manifest, &catalog);
    ensure_valid(&report)?;

    let agents: Vec<&AgentManifest> = if command.all {
        manifest.agents.iter().collect()
    } else if let Some(agent) = command.agent {
        vec![
            manifest
                .agents
                .iter()
                .find(|candidate| candidate.name == agent)
                .with_context(|| format!("agent '{agent}' not found"))?,
        ]
    } else {
        bail!("pass --agent <name> or --all");
    };

    let renderer = Renderer::load(&command.templates)?;
    let mut stale = Vec::new();

    for agent in agents {
        let files = renderer.render_agent(agent, &manifest, &catalog)?;
        for rendered in files {
            if command.check {
                if !file_matches(&rendered.path, &rendered.content)? {
                    stale.push(rendered.path);
                }
            } else {
                if let Some(parent) = rendered.path.parent() {
                    fs::create_dir_all(parent).with_context(|| {
                        format!("failed to create output directory '{}'", parent.display())
                    })?;
                }
                fs::write(&rendered.path, rendered.content)
                    .with_context(|| format!("failed to write '{}'", rendered.path.display()))?;
                println!("wrote {}", rendered.path.display());
            }
        }
    }

    if command.check {
        if stale.is_empty() {
            println!("Generated files are fresh.");
        } else {
            for path in &stale {
                println!("stale {}", path.display());
            }
            bail!("{} generated file(s) are stale", stale.len());
        }
    }

    Ok(())
}

fn doctor(command: DoctorCommand) -> Result<()> {
    let manifest = load_manifest(&command.manifest)?;
    let catalog = load_catalog(&command.catalog)?;
    let report = validate_manifest(&manifest, &catalog);

    if command.json {
        let output = serde_json::json!({
            "manifest": command.manifest,
            "catalog": command.catalog,
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
    println!("Catalog: {}", command.catalog.display());
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
        GenerateComponent::Agent(command) => generate_named(GeneratorKind::Agent, command),
        GenerateComponent::Tool(command) => generate_named(GeneratorKind::Tool, command),
        GenerateComponent::Skill(command) => generate_named(GeneratorKind::Skill, command),
        GenerateComponent::Subagent(command) => generate_named(GeneratorKind::Subagent, command),
        GenerateComponent::Channel(command) => generate_named(GeneratorKind::Channel, command),
        GenerateComponent::Approval(command) => generate_named(GeneratorKind::Approval, command),
        GenerateComponent::Eval(command) => generate_named(GeneratorKind::Eval, command),
        GenerateComponent::Memory(command) => generate_named(GeneratorKind::Memory, command),
        GenerateComponent::Migration(command) => generate_named(GeneratorKind::Migration, command),
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
    let catalog = load_catalog(&command.catalog)?;
    let report = validate_manifest(&manifest, &catalog);
    ensure_valid(&report)?;
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
    let catalog = load_catalog(&command.catalog)?;
    let report = validate_manifest(&manifest, &catalog);
    ensure_valid(&report)?;
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
    let catalog = load_catalog(&command.catalog)?;
    let report = validate_manifest(&manifest, &catalog);
    ensure_valid(&report)?;

    if command.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "command": name,
                "manifest": command.manifest,
                "catalog": command.catalog,
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

#[derive(Clone, Copy, Debug)]
enum GeneratorKind {
    Agent,
    Tool,
    Skill,
    Subagent,
    Channel,
    Approval,
    Eval,
    Memory,
    Migration,
}

impl GeneratorKind {
    fn as_str(self) -> &'static str {
        match self {
            GeneratorKind::Agent => "agent",
            GeneratorKind::Tool => "tool",
            GeneratorKind::Skill => "skill",
            GeneratorKind::Subagent => "subagent",
            GeneratorKind::Channel => "channel",
            GeneratorKind::Approval => "approval",
            GeneratorKind::Eval => "eval",
            GeneratorKind::Memory => "memory",
            GeneratorKind::Migration => "migration",
        }
    }
}

#[derive(Debug)]
struct PlannedChange {
    path: PathBuf,
    action: ChangeAction,
    content: String,
}

#[derive(Debug)]
enum ChangeAction {
    Create,
    Update,
}

fn generate_named(kind: GeneratorKind, command: GenerateNamed) -> Result<()> {
    ensure_slug(&command.name)?;
    let changes = plan_generate(kind, &command)?;

    if command.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "kind": kind.as_str(),
                "name": command.name,
                "dry_run": command.dry_run,
                "force": command.force,
                "changes": changes.iter().map(|change| {
                    serde_json::json!({
                        "path": change.path,
                        "action": match change.action {
                            ChangeAction::Create => "create",
                            ChangeAction::Update => "update",
                        },
                    })
                }).collect::<Vec<_>>(),
            }))?
        );
        if command.dry_run {
            return Ok(());
        }
    }

    if command.dry_run {
        print_generate_plan(kind, &command, &changes);
        return Ok(());
    }

    for change in changes {
        if let Some(parent) = change.path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create '{}'", parent.display()))?;
        }
        fs::write(&change.path, change.content)
            .with_context(|| format!("failed to write '{}'", change.path.display()))?;
        println!("{} {}", change_word(&change.action), change.path.display());
    }

    Ok(())
}

fn plan_generate(kind: GeneratorKind, command: &GenerateNamed) -> Result<Vec<PlannedChange>> {
    match kind {
        GeneratorKind::Agent => plan_generate_agent(command),
        GeneratorKind::Tool
        | GeneratorKind::Skill
        | GeneratorKind::Channel
        | GeneratorKind::Approval
        | GeneratorKind::Eval
        | GeneratorKind::Memory => plan_generate_catalog_component(kind, command),
        GeneratorKind::Subagent => plan_generate_subagent(command),
        GeneratorKind::Migration => plan_generate_migration(command),
    }
}

fn plan_generate_agent(command: &GenerateNamed) -> Result<Vec<PlannedChange>> {
    let source = read_or_default(&command.manifest, default_manifest_yaml())?;
    if source.contains(&format!("- name: {}", command.name)) && !command.force {
        bail!(
            "agent '{}' already exists in '{}'; pass --force to overwrite generated manifest text",
            command.name,
            command.manifest.display()
        );
    }

    let entry = format!(
        "\n  - name: {name}\n    version: {version}\n    owner: {owner}\n    responsibility: {responsibility:?}\n    model: {model}\n    tools: {{}}\n    skills: {{}}\n    subagents: []\n    channels: []\n    approvals: {{}}\n    evals: []\n    memory: {{}}\n",
        name = command.name,
        version = command.version,
        owner = command.owner.as_deref().unwrap_or("agent-platform"),
        responsibility = command
            .description
            .as_deref()
            .unwrap_or("Describe this agent's bounded responsibility."),
        model = command.model.as_deref().unwrap_or("openai/gpt-5.5"),
    );
    let content = append_yaml_list_entry(source, "agents:", &entry);

    Ok(vec![PlannedChange {
        path: command.manifest.clone(),
        action: change_action(&command.manifest),
        content,
    }])
}

fn plan_generate_catalog_component(
    kind: GeneratorKind,
    command: &GenerateNamed,
) -> Result<Vec<PlannedChange>> {
    let section = catalog_section(kind);
    let source = read_or_default(&command.catalog, default_catalog_yaml())?;
    if catalog_entry_exists(&source, section, &command.name) && !command.force {
        bail!(
            "{} '{}' already exists in '{}'; pass --force to overwrite generated catalog text",
            kind.as_str(),
            command.name,
            command.catalog.display()
        );
    }

    let entry = catalog_entry(kind, command);
    let catalog_content = upsert_catalog_entry(&source, section, &command.name, &entry);
    let mut changes = vec![PlannedChange {
        path: command.catalog.clone(),
        action: change_action(&command.catalog),
        content: catalog_content,
    }];

    if let Some(stub) = component_stub(kind, command)? {
        changes.push(stub);
    }

    Ok(changes)
}

fn plan_generate_subagent(command: &GenerateNamed) -> Result<Vec<PlannedChange>> {
    let path = PathBuf::from("catalog")
        .join("subagents")
        .join(&command.name)
        .join("instructions.md");
    ensure_writable(&path, command.force)?;
    Ok(vec![PlannedChange {
        path,
        action: ChangeAction::Create,
        content: format!(
            "# {}\n\n## Responsibility\n\n{}\n",
            command.name,
            command
                .description
                .as_deref()
                .unwrap_or("Describe this subagent's bounded context.")
        ),
    }])
}

fn plan_generate_migration(command: &GenerateNamed) -> Result<Vec<PlannedChange>> {
    let path = PathBuf::from("agents")
        .join("migrations")
        .join(format!("{}_{}.ts", "000000000000", command.name));
    ensure_writable(&path, command.force)?;
    Ok(vec![PlannedChange {
        path,
        action: ChangeAction::Create,
        content: "export async function up() {\n  // TODO: implement migration.\n}\n\nexport async function down() {\n  // TODO: implement rollback.\n}\n".to_string(),
    }])
}

fn catalog_section(kind: GeneratorKind) -> &'static str {
    match kind {
        GeneratorKind::Tool => "tools:",
        GeneratorKind::Skill => "skills:",
        GeneratorKind::Channel => "channels:",
        GeneratorKind::Approval => "approvals:",
        GeneratorKind::Eval => "evals:",
        GeneratorKind::Memory => "memory:",
        _ => unreachable!("kind does not live in catalog"),
    }
}

fn catalog_entry(kind: GeneratorKind, command: &GenerateNamed) -> String {
    let mut entry = format!("  {}:\n    version: {}\n", command.name, command.version);
    match kind {
        GeneratorKind::Tool => {
            entry.push_str(&format!("    side_effects: {}\n", command.side_effects));
        }
        GeneratorKind::Memory => {
            entry.push_str(&format!(
                "    retention: {}\n",
                command.retention.as_deref().unwrap_or("180d")
            ));
        }
        _ => {}
    }
    entry
}

fn component_stub(kind: GeneratorKind, command: &GenerateNamed) -> Result<Option<PlannedChange>> {
    let (dir, extension, content) = match kind {
        GeneratorKind::Tool => (
            "tools",
            "ts",
            format!(
                "export async function {}() {{\n  throw new Error(\"{} is not implemented yet\");\n}}\n",
                command.name, command.name
            ),
        ),
        GeneratorKind::Skill => (
            "skills",
            "md",
            format!(
                "# {}\n\n## Trigger\n\nUse this skill when ...\n\n## Procedure\n\n- Gather context.\n- Use approved tools.\n- Verify the result.\n",
                command.name
            ),
        ),
        GeneratorKind::Eval => (
            "evals",
            "ts",
            format!(
                "export async function {}Eval() {{\n  throw new Error(\"{} eval is not implemented yet\");\n}}\n",
                command.name, command.name
            ),
        ),
        GeneratorKind::Approval => (
            "approvals",
            "ts",
            format!(
                "export default {{\n  name: \"{}\",\n  policy: \"required\",\n}};\n",
                command.name
            ),
        ),
        GeneratorKind::Memory => (
            "memory",
            "ts",
            format!(
                "export default {{\n  name: \"{}\",\n  version: \"{}\",\n  retention: \"{}\",\n}};\n",
                command.name,
                command.version,
                command.retention.as_deref().unwrap_or("180d")
            ),
        ),
        GeneratorKind::Channel => (
            "channels",
            "ts",
            format!("export default {{\n  name: \"{}\",\n}};\n", command.name),
        ),
        _ => return Ok(None),
    };
    let path = PathBuf::from("catalog")
        .join(dir)
        .join(format!("{}.{}", command.name, extension));
    ensure_writable(&path, command.force)?;
    Ok(Some(PlannedChange {
        path,
        action: ChangeAction::Create,
        content,
    }))
}

fn print_generate_plan(kind: GeneratorKind, command: &GenerateNamed, changes: &[PlannedChange]) {
    println!("Generate {} '{}' (dry run)", kind.as_str(), command.name);
    for change in changes {
        println!("{} {}", change_word(&change.action), change.path.display());
    }
}

fn read_or_default(path: &Path, default: &str) -> Result<String> {
    match fs::read_to_string(path) {
        Ok(source) => Ok(source),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(default.to_string()),
        Err(error) => Err(error).with_context(|| format!("failed to read '{}'", path.display())),
    }
}

fn default_manifest_yaml() -> &'static str {
    "defaults:\n  model: openai/gpt-5.5\n  owner: agent-platform\n  channels: []\n  evals: []\n\nagents:\n"
}

fn default_catalog_yaml() -> &'static str {
    "tools:\nskills:\nevals:\napprovals:\nmemory:\nchannels:\n"
}

fn append_yaml_list_entry(source: String, section: &str, entry: &str) -> String {
    if source.contains(section) {
        format!("{}{}", source.trim_end(), entry)
    } else {
        format!("{}\n{}{}", source.trim_end(), section, entry)
    }
}

fn upsert_catalog_entry(source: &str, section: &str, name: &str, entry: &str) -> String {
    let without_existing = remove_catalog_entry(source, section, name);
    insert_catalog_entry(&without_existing, section, entry)
}

fn insert_catalog_entry(source: &str, section: &str, entry: &str) -> String {
    let mut output = Vec::new();
    let mut inserted = false;
    let mut in_section = false;

    for line in source.lines() {
        let is_top_level = !line.starts_with(' ') && line.ends_with(':');
        if is_top_level && in_section && !inserted {
            output.push(entry.trim_end().to_string());
            inserted = true;
        }
        output.push(line.to_string());
        if is_top_level {
            in_section = line == section;
        }
    }

    if in_section && !inserted {
        output.push(entry.trim_end().to_string());
        inserted = true;
    }

    if !inserted {
        output.push(section.to_string());
        output.push(entry.trim_end().to_string());
    }

    format!("{}\n", output.join("\n"))
}

fn remove_catalog_entry(source: &str, section: &str, name: &str) -> String {
    let mut output = Vec::new();
    let mut in_section = false;
    let mut skipping = false;
    let entry_prefix = format!("  {name}:");

    for line in source.lines() {
        let is_top_level = !line.starts_with(' ') && line.ends_with(':');
        if is_top_level {
            in_section = line == section;
            skipping = false;
        }
        if in_section && line == entry_prefix {
            skipping = true;
            continue;
        }
        if skipping {
            if line.starts_with("    ") || line.trim().is_empty() {
                continue;
            }
            skipping = false;
        }
        output.push(line);
    }

    format!("{}\n", output.join("\n"))
}

fn catalog_entry_exists(source: &str, section: &str, name: &str) -> bool {
    let mut in_section = false;
    let entry_prefix = format!("  {name}:");

    for line in source.lines() {
        let is_top_level = !line.starts_with(' ') && line.ends_with(':');
        if is_top_level {
            in_section = line == section;
        }
        if in_section && line == entry_prefix {
            return true;
        }
    }

    false
}

fn ensure_slug(value: &str) -> Result<()> {
    if is_slug(value) {
        Ok(())
    } else {
        bail!("'{}' must use lowercase kebab-case or snake_case", value)
    }
}

fn ensure_writable(path: &Path, force: bool) -> Result<()> {
    if path.exists() && !force {
        bail!(
            "'{}' already exists; pass --force to overwrite",
            path.display()
        )
    } else {
        Ok(())
    }
}

fn change_action(path: &Path) -> ChangeAction {
    if path.exists() {
        ChangeAction::Update
    } else {
        ChangeAction::Create
    }
}

fn change_word(action: &ChangeAction) -> &'static str {
    match action {
        ChangeAction::Create => "create",
        ChangeAction::Update => "update",
    }
}

fn load_manifest(path: &Path) -> Result<FleetManifest> {
    let source = fs::read_to_string(path)
        .with_context(|| format!("failed to read manifest '{}'", path.display()))?;
    let manifest: FleetManifest = serde_yaml::from_str(&source)
        .with_context(|| format!("failed to parse manifest '{}'", path.display()))?;
    Ok(manifest)
}

fn load_catalog(path: &Path) -> Result<CatalogManifest> {
    let source = fs::read_to_string(path)
        .with_context(|| format!("failed to read catalog '{}'", path.display()))?;
    let catalog: CatalogManifest = serde_yaml::from_str(&source)
        .with_context(|| format!("failed to parse catalog '{}'", path.display()))?;
    Ok(catalog)
}

struct Renderer<'source> {
    env: Environment<'source>,
}

struct RenderedFile {
    path: PathBuf,
    content: String,
}

impl<'source> Renderer<'source> {
    fn load(template_dir: &Path) -> Result<Self> {
        let mut env = Environment::new();
        env.set_undefined_behavior(minijinja::UndefinedBehavior::Strict);

        for template_name in ["instructions.md.j2", "agent.ts.j2"] {
            let path = template_dir.join(template_name);
            let source = fs::read_to_string(&path)
                .with_context(|| format!("failed to read template '{}'", path.display()))?;
            env.add_template_owned(template_name.to_string(), source)
                .with_context(|| format!("failed to load template '{}'", path.display()))?;
        }

        Ok(Self { env })
    }

    fn render_agent(
        &self,
        agent: &AgentManifest,
        manifest: &FleetManifest,
        catalog: &CatalogManifest,
    ) -> Result<Vec<RenderedFile>> {
        let output_root = PathBuf::from("agents").join(&agent.name).join("agent");
        let model = agent
            .model
            .as_deref()
            .or(manifest.defaults.model.as_deref())
            .unwrap_or_default();
        let owner = agent
            .owner
            .as_deref()
            .or(manifest.defaults.owner.as_deref())
            .unwrap_or_default();
        let version = agent.version.as_deref().unwrap_or("1.0.0");
        let agent_context = context! {
            name => agent.name.as_str(),
            version => version,
            owner => owner,
            responsibility => agent.responsibility.as_str(),
            model => model,
        };

        let instructions = self
            .env
            .get_template("instructions.md.j2")?
            .render(context! { agent => agent_context.clone() })?;
        let agent_ts = self
            .env
            .get_template("agent.ts.j2")?
            .render(context! { agent => agent_context })?;

        Ok(vec![
            RenderedFile {
                path: output_root.join("instructions.md"),
                content: with_generated_header(
                    CommentStyle::Hash,
                    &agent.name,
                    "instructions.md.j2",
                    &instructions,
                ),
            },
            RenderedFile {
                path: output_root.join("agent.ts"),
                content: with_generated_header(
                    CommentStyle::Slash,
                    &agent.name,
                    "agent.ts.j2",
                    &agent_ts,
                ),
            },
            RenderedFile {
                path: output_root.join("agent.manifest.yml"),
                content: render_agent_manifest(agent, manifest),
            },
            RenderedFile {
                path: output_root.join("versions.lock"),
                content: render_versions_lock(agent, manifest, catalog),
            },
        ])
    }
}

enum CommentStyle {
    Hash,
    Slash,
}

fn with_generated_header(style: CommentStyle, agent: &str, template: &str, body: &str) -> String {
    let prefix = match style {
        CommentStyle::Hash => "#",
        CommentStyle::Slash => "//",
    };
    format!(
        "{prefix} Generated by eve-rails. Do not edit generated regions.\n{prefix} agent: {agent}\n{prefix} template: {template}\n\n{}\n",
        body.trim_end()
    )
}

fn render_agent_manifest(agent: &AgentManifest, manifest: &FleetManifest) -> String {
    let mut output = String::new();
    output.push_str("# Generated by eve-rails. Do not edit generated regions.\n");
    output.push_str(&format!("name: {}\n", agent.name));
    output.push_str(&format!(
        "version: {}\n",
        agent.version.as_deref().unwrap_or("1.0.0")
    ));
    output.push_str(&format!(
        "owner: {}\n",
        agent
            .owner
            .as_deref()
            .or(manifest.defaults.owner.as_deref())
            .unwrap_or("")
    ));
    output.push_str(&format!(
        "model: {}\n",
        agent
            .model
            .as_deref()
            .or(manifest.defaults.model.as_deref())
            .unwrap_or("")
    ));
    output.push_str(&format!("responsibility: {:?}\n", agent.responsibility));
    output.push_str("uses:\n");
    append_component_map(&mut output, "tools", &agent.tools);
    append_component_map(&mut output, "skills", &agent.skills);
    append_component_map(&mut output, "memory", &agent.memory);
    let channels = manifest
        .defaults
        .channels
        .iter()
        .chain(agent.channels.iter())
        .cloned()
        .collect::<Vec<_>>();
    let evals = manifest
        .defaults
        .evals
        .iter()
        .chain(agent.evals.iter())
        .cloned()
        .collect::<Vec<_>>();
    append_string_list(&mut output, "channels", &channels);
    append_string_list(&mut output, "evals", &evals);
    output
}

fn render_versions_lock(
    agent: &AgentManifest,
    manifest: &FleetManifest,
    catalog: &CatalogManifest,
) -> String {
    let mut output = String::new();
    output.push_str("# Generated by eve-rails. Do not edit generated regions.\n");
    output.push_str("resolved:\n");
    append_lock_entries(&mut output, "tools", &agent.tools);
    append_lock_entries(&mut output, "skills", &agent.skills);
    append_lock_entries(&mut output, "memory", &agent.memory);
    for channel in manifest
        .defaults
        .channels
        .iter()
        .chain(agent.channels.iter())
    {
        let version = catalog_version(&catalog.channels, channel);
        output.push_str(&format!(
            "  catalog/channels/{channel}@{version}:\n    source: manifests/catalog.yml\n"
        ));
    }
    for eval in manifest.defaults.evals.iter().chain(agent.evals.iter()) {
        let version = catalog_version(&catalog.evals, eval);
        output.push_str(&format!(
            "  catalog/evals/{eval}@{version}:\n    source: manifests/catalog.yml\n"
        ));
    }
    output
}

fn catalog_version<'a>(components: &'a BTreeMap<String, CatalogComponent>, name: &str) -> &'a str {
    components
        .get(name)
        .map(|component| component.version.as_str())
        .unwrap_or("unknown")
}

fn append_component_map(output: &mut String, label: &str, components: &ComponentMap) {
    output.push_str(&format!("  {label}:\n"));
    if components.is_empty() {
        output.push_str("    {}\n");
        return;
    }
    for (name, version) in components {
        output.push_str(&format!("    {name}: {version}\n"));
    }
}

fn append_string_list(output: &mut String, label: &str, values: &[String]) {
    output.push_str(&format!("  {label}:"));
    if values.is_empty() {
        output.push_str(" []\n");
        return;
    }
    output.push('\n');
    for value in values {
        output.push_str(&format!("    - {value}\n"));
    }
}

fn append_lock_entries(output: &mut String, kind: &str, components: &ComponentMap) {
    for (name, version) in components {
        output.push_str(&format!(
            "  catalog/{kind}/{name}@{version}:\n    source: manifests/catalog.yml\n"
        ));
    }
}

fn file_matches(path: &Path, expected: &str) -> Result<bool> {
    match fs::read_to_string(path) {
        Ok(actual) => Ok(actual == expected),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error).with_context(|| format!("failed to read '{}'", path.display())),
    }
}

#[derive(Debug, Default)]
struct ValidationReport {
    errors: Vec<String>,
    warnings: Vec<String>,
}

fn validate_manifest(manifest: &FleetManifest, catalog: &CatalogManifest) -> ValidationReport {
    let mut report = ValidationReport::default();

    if manifest.agents.is_empty() {
        report
            .errors
            .push("manifest must define at least one agent".to_string());
    }

    validate_named_list(
        &mut report,
        "shared",
        "tool",
        &manifest.shared.tools,
        &catalog.tools,
    );
    validate_named_list(
        &mut report,
        "shared",
        "skill",
        &manifest.shared.skills,
        &catalog.skills,
    );
    validate_named_list(
        &mut report,
        "shared",
        "memory",
        &manifest.shared.memory,
        &catalog.memory,
    );
    if let Some(policy) = &manifest.defaults.approvals {
        validate_policy(&mut report, "defaults", policy, catalog);
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
        if agent.version.as_deref().is_none_or(str::is_empty) {
            report
                .errors
                .push(format!("agent '{}' must define version", agent.name));
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
        validate_named_list(
            &mut report,
            &format!("agent '{}'", agent.name),
            "channel",
            &manifest.defaults.channels,
            &catalog.channels,
        );
        validate_named_list(
            &mut report,
            &format!("agent '{}'", agent.name),
            "channel",
            &agent.channels,
            &catalog.channels,
        );
        validate_named_list(
            &mut report,
            &format!("agent '{}'", agent.name),
            "eval",
            &manifest.defaults.evals,
            &catalog.evals,
        );
        validate_named_list(
            &mut report,
            &format!("agent '{}'", agent.name),
            "eval",
            &agent.evals,
            &catalog.evals,
        );

        for (tool, version) in &agent.tools {
            if version.trim().is_empty() {
                report.errors.push(format!(
                    "agent '{}' tool '{}' has empty version",
                    agent.name, tool
                ));
            }
            validate_component_version(
                &mut report,
                &format!("agent '{}'", agent.name),
                "tool",
                tool,
                version,
                &catalog.tools,
            );
            if is_risky_tool(tool, catalog) && !agent.approvals.contains_key(tool) {
                report.errors.push(format!(
                    "agent '{}' risky tool '{}' must have approval policy",
                    agent.name, tool
                ));
            }
        }

        for (skill, version) in &agent.skills {
            validate_component_version(
                &mut report,
                &format!("agent '{}'", agent.name),
                "skill",
                skill,
                version,
                &catalog.skills,
            );
        }

        for (memory, version) in &agent.memory {
            if version.trim().is_empty() {
                report.errors.push(format!(
                    "agent '{}' memory '{}' has empty version",
                    agent.name, memory
                ));
            }
            validate_component_version(
                &mut report,
                &format!("agent '{}'", agent.name),
                "memory",
                memory,
                version,
                &catalog.memory,
            );
            if catalog
                .memory
                .get(memory)
                .is_some_and(|component| component.retention.as_deref().is_none_or(str::is_empty))
            {
                report.errors.push(format!(
                    "agent '{}' memory '{}' must declare retention in catalog",
                    agent.name, memory
                ));
            }
        }

        for (tool, policy) in &agent.approvals {
            if !agent.tools.contains_key(tool) {
                report.errors.push(format!(
                    "agent '{}' approval policy for '{}' does not match an agent tool",
                    agent.name, tool
                ));
            }
            if policy.trim().is_empty() {
                report.errors.push(format!(
                    "agent '{}' approval policy for '{}' cannot be empty",
                    agent.name, tool
                ));
            }
            validate_policy(
                &mut report,
                &format!("agent '{}'", agent.name),
                policy,
                catalog,
            );
        }
    }

    report
}

fn validate_policy(
    report: &mut ValidationReport,
    scope: &str,
    policy: &str,
    catalog: &CatalogManifest,
) {
    if !catalog.approvals.contains_key(policy) {
        report.errors.push(format!(
            "{scope} references missing approval policy '{policy}'"
        ));
    }
}

fn validate_named_list(
    report: &mut ValidationReport,
    scope: &str,
    kind: &str,
    names: &[String],
    catalog: &BTreeMap<String, CatalogComponent>,
) {
    for name in names {
        if !catalog.contains_key(name) {
            report
                .errors
                .push(format!("{scope} references missing {kind} '{name}'"));
        }
    }
}

fn validate_component_version(
    report: &mut ValidationReport,
    scope: &str,
    kind: &str,
    name: &str,
    requested_version: &str,
    catalog: &BTreeMap<String, CatalogComponent>,
) {
    let Some(component) = catalog.get(name) else {
        report
            .errors
            .push(format!("{scope} references missing {kind} '{name}'"));
        return;
    };

    if requested_version != component.version {
        report.errors.push(format!(
            "{scope} {kind} '{name}' requests version {requested_version}, but catalog has {}",
            component.version
        ));
    }
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

fn is_risky_tool(tool: &str, catalog: &CatalogManifest) -> bool {
    let Some(component) = catalog.tools.get(tool) else {
        return false;
    };

    matches!(
        component.side_effects,
        Some(SideEffects::Write)
            | Some(SideEffects::External)
            | Some(SideEffects::Money)
            | Some(SideEffects::Production)
    )
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn manifest(source: &str) -> FleetManifest {
        serde_yaml::from_str(source).expect("manifest should parse")
    }

    fn catalog(source: &str) -> CatalogManifest {
        serde_yaml::from_str(source).expect("catalog should parse")
    }

    fn valid_catalog() -> CatalogManifest {
        catalog(
            r#"
tools:
  search_customers:
    version: 1.0.0
    side_effects: read
  prepare_refund:
    version: 1.0.0
    side_effects: money
skills:
  handle_refund:
    version: 1.0.0
evals:
  standard:
    version: 1.0.0
approvals:
  required:
    version: 1.0.0
memory:
  customer_profile:
    version: 1.0.0
    retention: 180d
channels:
  web:
    version: 1.0.0
"#,
        )
    }

    fn valid_manifest() -> FleetManifest {
        manifest(
            r#"
defaults:
  model: openai/gpt-5.5
  owner: agent-platform
  channels: [web]
  evals: [standard]
agents:
  - name: billing
    version: 1.0.0
    responsibility: Answer billing questions.
    tools:
      search_customers: 1.0.0
      prepare_refund: 1.0.0
    skills:
      handle_refund: 1.0.0
    approvals:
      prepare_refund: required
    memory:
      customer_profile: 1.0.0
"#,
        )
    }

    #[test]
    fn valid_manifest_resolves_against_catalog() {
        let report = validate_manifest(&valid_manifest(), &valid_catalog());

        assert!(report.errors.is_empty(), "{:?}", report.errors);
    }

    #[test]
    fn missing_catalog_component_fails() {
        let mut manifest = valid_manifest();
        manifest.agents[0]
            .tools
            .insert("missing_tool".to_string(), "1.0.0".to_string());

        let report = validate_manifest(&manifest, &valid_catalog());

        assert!(
            report
                .errors
                .iter()
                .any(|error| error.contains("missing tool 'missing_tool'")),
            "{:?}",
            report.errors
        );
    }

    #[test]
    fn version_mismatch_fails() {
        let mut manifest = valid_manifest();
        manifest.agents[0]
            .skills
            .insert("handle_refund".to_string(), "2.0.0".to_string());

        let report = validate_manifest(&manifest, &valid_catalog());

        assert!(
            report.errors.iter().any(|error| error
                .contains("skill 'handle_refund' requests version 2.0.0, but catalog has 1.0.0")),
            "{:?}",
            report.errors
        );
    }

    #[test]
    fn risky_tool_without_approval_fails() {
        let mut manifest = valid_manifest();
        manifest.agents[0].approvals.clear();

        let report = validate_manifest(&manifest, &valid_catalog());

        assert!(
            report
                .errors
                .iter()
                .any(|error| error.contains("risky tool 'prepare_refund'")),
            "{:?}",
            report.errors
        );
    }

    #[test]
    fn memory_without_retention_fails() {
        let catalog = catalog(
            r#"
tools:
  search_customers:
    version: 1.0.0
    side_effects: read
  prepare_refund:
    version: 1.0.0
    side_effects: money
skills:
  handle_refund:
    version: 1.0.0
evals:
  standard:
    version: 1.0.0
approvals:
  required:
    version: 1.0.0
memory:
  customer_profile:
    version: 1.0.0
channels:
  web:
    version: 1.0.0
"#,
        );

        let report = validate_manifest(&valid_manifest(), &catalog);

        assert!(
            report
                .errors
                .iter()
                .any(|error| error.contains("memory 'customer_profile' must declare retention")),
            "{:?}",
            report.errors
        );
    }

    #[test]
    fn renderer_outputs_core_agent_files() {
        let renderer = Renderer::load(Path::new("templates/agent")).expect("renderer loads");
        let manifest = valid_manifest();
        let catalog = valid_catalog();
        let files = renderer
            .render_agent(&manifest.agents[0], &manifest, &catalog)
            .expect("agent renders");

        assert_eq!(files.len(), 4);
        assert!(
            files
                .iter()
                .any(|file| file.path.ends_with("instructions.md")
                    && file.content.contains("You are the billing agent."))
        );
        assert!(files.iter().any(|file| file.path.ends_with("agent.ts")
            && file.content.contains("model: \"openai/gpt-5.5\"")));
        assert!(
            files
                .iter()
                .any(|file| file.path.ends_with("agent.manifest.yml")
                    && file.content.contains("    - web"))
        );
        assert!(files.iter().any(|file| file.path.ends_with("versions.lock")
            && file.content.contains("catalog/evals/standard@1.0.0")));
    }

    #[test]
    fn renderer_fails_on_undefined_template_variables() {
        let dir = temp_dir("undefined-template");
        fs::create_dir_all(&dir).expect("temp dir");
        fs::write(
            dir.join("instructions.md.j2"),
            "hello {{ agent.missing_field }}",
        )
        .expect("template");
        fs::write(dir.join("agent.ts.j2"), "export default {};").expect("template");

        let renderer = Renderer::load(&dir).expect("renderer loads");
        let manifest = valid_manifest();
        let catalog = valid_catalog();
        let result = renderer.render_agent(&manifest.agents[0], &manifest, &catalog);

        assert!(result.is_err());
    }

    #[test]
    fn file_matches_detects_stale_output() {
        let dir = temp_dir("stale-output");
        fs::create_dir_all(&dir).expect("temp dir");
        let file = dir.join("generated.txt");
        fs::write(&file, "old").expect("write");

        assert!(!file_matches(&file, "new").expect("compare"));
        assert!(file_matches(&file, "old").expect("compare"));
    }

    #[test]
    fn generate_agent_plans_manifest_update() {
        let dir = temp_dir("generate-agent");
        fs::create_dir_all(&dir).expect("temp dir");
        let manifest_path = dir.join("agents.yml");
        fs::write(&manifest_path, default_manifest_yaml()).expect("manifest");

        let command = generate_command("support", manifest_path.clone(), dir.join("catalog.yml"));
        let changes = plan_generate(GeneratorKind::Agent, &command).expect("plan");

        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].path, manifest_path);
        assert!(changes[0].content.contains("- name: support"));
        assert!(changes[0].content.contains("tools: {}"));
    }

    #[test]
    fn generate_tool_plans_catalog_and_stub() {
        let dir = temp_dir("generate-tool");
        fs::create_dir_all(&dir).expect("temp dir");
        let catalog_path = dir.join("catalog.yml");
        fs::write(&catalog_path, default_catalog_yaml()).expect("catalog");

        let mut command = generate_command("refund_customer", dir.join("agents.yml"), catalog_path);
        command.side_effects = SideEffects::Money;
        let changes = plan_generate(GeneratorKind::Tool, &command).expect("plan");

        assert_eq!(changes.len(), 2);
        assert!(changes[0].content.contains("refund_customer:"));
        assert!(changes[0].content.contains("side_effects: money"));
        assert_eq!(
            changes[1].path,
            PathBuf::from("catalog/tools/refund_customer.ts")
        );
    }

    #[test]
    fn catalog_upsert_keeps_entry_in_requested_section() {
        let source = "memory:\n  customer_profile:\n    version: 1.0.0\napprovals:\n  required:\n    version: 1.0.0\n";
        let entry = "  account_context:\n    version: 1.0.0\n    retention: 90d\n";

        let updated = upsert_catalog_entry(source, "memory:", "account_context", entry);

        let memory_index = updated.find("  account_context:").expect("memory entry");
        let approvals_index = updated.find("approvals:").expect("approvals section");
        assert!(memory_index < approvals_index, "{updated}");
    }

    #[test]
    fn generate_duplicate_requires_force() {
        let dir = temp_dir("generate-duplicate");
        fs::create_dir_all(&dir).expect("temp dir");
        let catalog_path = dir.join("catalog.yml");
        fs::write(
            &catalog_path,
            "tools:\n  search_customers:\n    version: 1.0.0\n",
        )
        .expect("catalog");

        let command = generate_command("search_customers", dir.join("agents.yml"), catalog_path);
        let result = plan_generate(GeneratorKind::Tool, &command);

        assert!(result.is_err());
    }

    fn generate_command(name: &str, manifest: PathBuf, catalog: PathBuf) -> GenerateNamed {
        GenerateNamed {
            name: name.to_string(),
            manifest,
            catalog,
            version: "1.0.0".to_string(),
            owner: None,
            model: None,
            description: None,
            side_effects: SideEffects::Read,
            retention: None,
            dry_run: true,
            force: false,
            json: false,
        }
    }

    fn temp_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        std::env::temp_dir().join(format!("eve-rails-{name}-{nanos}"))
    }
}
