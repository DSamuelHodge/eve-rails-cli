use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use clap::{Args, Parser, Subcommand, ValueEnum};
use minijinja::{Environment, context};
use serde::{Deserialize, Serialize};

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
    /// Create a Rails-style Eve Rails project skeleton.
    Init(InitCommand),
    /// Show the files and components that would be generated from a manifest.
    Plan(ManifestCommand),
    /// Apply a manifest by rendering generated files.
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
    /// Run deploy gates and optionally delegate deployment to Eve/Vercel.
    Deploy(DeployCommand),
    /// Delegate eval execution to Eve for a generated agent.
    Eval(RuntimeCommand),
    /// Run CLI checks plus generated-agent runtime checks.
    Test(RuntimeCommand),
    /// Preview a generated agent with Eve dev.
    Preview(RuntimeCommand),
    /// Plan or apply behavior and memory migrations.
    Migrate(MigrateCommand),
    /// Plan rollback by agent or component version.
    Rollback(RollbackCommand),
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

    /// Path to the template directory.
    #[arg(long, default_value = "templates/agent")]
    templates: PathBuf,

    /// Emit machine-readable JSON.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct InitCommand {
    /// Project directory to create.
    name: String,

    /// Starter template.
    #[arg(long, default_value = "basic")]
    template: String,

    /// Default model for generated manifests.
    #[arg(long, default_value = "openai/gpt-5.5")]
    model: String,

    /// Default owner for generated manifests.
    #[arg(long, default_value = "agent-platform")]
    owner: String,

    /// Accept defaults without prompting.
    #[arg(long)]
    yes: bool,

    /// Show planned changes without writing files.
    #[arg(long)]
    dry_run: bool,

    /// Overwrite existing files.
    #[arg(long)]
    force: bool,

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

    /// Plan safe mechanical repairs.
    #[arg(long)]
    fix: bool,

    /// Environment name from environments.yml.
    #[arg(long)]
    env: Option<String>,

    /// Validate required connections and secrets for the selected environment.
    #[arg(long)]
    connections: bool,

    /// Validate cost, token, and timeout budgets.
    #[arg(long)]
    budgets: bool,

    /// Emit machine-readable JSON.
    #[arg(long)]
    json: bool,

    /// Path to the fleet manifest.
    #[arg(long, default_value = "manifests/agents.yml")]
    manifest: PathBuf,

    /// Path to the reusable component catalog.
    #[arg(long, default_value = "manifests/catalog.yml")]
    catalog: PathBuf,

    /// Path to the template directory.
    #[arg(long, default_value = "templates/agent")]
    template_dir: PathBuf,

    /// Path to environment policy config.
    #[arg(long, default_value = "manifests/environments.yml")]
    environments: PathBuf,
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
    Schedule(GenerateNamed),
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

    /// Tools to attach when generating an agent.
    #[arg(long, value_delimiter = ',')]
    with_tools: Vec<String>,

    /// Skills to attach when generating an agent.
    #[arg(long, value_delimiter = ',')]
    with_skills: Vec<String>,

    /// Subagents to attach when generating an agent.
    #[arg(long, value_delimiter = ',')]
    with_subagents: Vec<String>,

    /// Channels to attach when generating an agent.
    #[arg(long, value_delimiter = ',')]
    with_channels: Vec<String>,

    /// Evals to attach when generating an agent.
    #[arg(long, value_delimiter = ',')]
    with_evals: Vec<String>,

    /// Memory schemas to attach when generating an agent.
    #[arg(long, value_delimiter = ',')]
    with_memory: Vec<String>,

    /// Schedules to attach when generating an agent.
    #[arg(long, value_delimiter = ',')]
    with_schedules: Vec<String>,

    /// Approval policy to apply to generated agent tools.
    #[arg(long)]
    approval: Option<String>,

    /// Cron expression for a generated schedule component.
    #[arg(long)]
    schedule: Option<String>,

    /// Risk classification.
    #[arg(long)]
    risk: Option<String>,

    /// Authentication mode.
    #[arg(long)]
    auth: Option<String>,

    /// Visibility policy.
    #[arg(long)]
    visibility: Option<String>,

    /// Cost budget for generated agent metadata.
    #[arg(long)]
    cost_budget: Option<f64>,

    /// Token budget for generated agent metadata.
    #[arg(long)]
    token_budget: Option<u64>,

    /// Timeout for generated agent metadata.
    #[arg(long)]
    timeout: Option<String>,

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

    /// Path to the fleet manifest.
    #[arg(long, default_value = "manifests/agents.yml")]
    manifest: PathBuf,

    /// Path to the reusable component catalog.
    #[arg(long, default_value = "manifests/catalog.yml")]
    catalog: PathBuf,

    /// Emit machine-readable JSON.
    #[arg(long)]
    json: bool,

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

    /// Current running component version.
    #[arg(long, default_value = "1.0.0")]
    current: String,

    /// Emit machine-readable JSON.
    #[arg(long)]
    json: bool,

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

    /// Require approval coverage before deployment.
    #[arg(long)]
    require_approvals: bool,

    /// Percentage of traffic for canary deployment.
    #[arg(long)]
    canary: Option<u8>,

    /// Promote a previous or canary deployment.
    #[arg(long)]
    promote: bool,

    /// Roll back to deployment id.
    #[arg(long)]
    rollback_to: Option<String>,

    /// Show deploy gates and delegated command without invoking Eve.
    #[arg(long)]
    dry_run: bool,

    /// Path to the fleet manifest.
    #[arg(long, default_value = "manifests/agents.yml")]
    manifest: PathBuf,

    /// Path to the reusable component catalog.
    #[arg(long, default_value = "manifests/catalog.yml")]
    catalog: PathBuf,

    /// Path to the template directory.
    #[arg(long, default_value = "templates/agent")]
    template_dir: PathBuf,

    /// Emit machine-readable JSON.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct RuntimeCommand {
    /// Agent to run.
    #[arg(long)]
    agent: String,

    /// Path to the fleet manifest.
    #[arg(long, default_value = "manifests/agents.yml")]
    manifest: PathBuf,

    /// Path to the reusable component catalog.
    #[arg(long, default_value = "manifests/catalog.yml")]
    catalog: PathBuf,

    /// Environment name.
    #[arg(long, default_value = "development")]
    env: String,

    /// Print command and checks without invoking long-running commands.
    #[arg(long)]
    dry_run: bool,

    /// Emit machine-readable JSON.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct MigrateCommand {
    /// Agent to migrate.
    #[arg(long)]
    agent: Option<String>,

    /// Fleet manifest to migrate.
    #[arg(long)]
    fleet: Option<PathBuf>,

    /// Environment name.
    #[arg(long, default_value = "staging")]
    env: String,

    /// Path to the fleet manifest.
    #[arg(long, default_value = "manifests/agents.yml")]
    manifest: PathBuf,

    /// Apply migration status changes. Omit for dry-run plan.
    #[arg(long)]
    apply: bool,

    /// Show migration plan without writing files.
    #[arg(long)]
    dry_run: bool,

    /// Emit machine-readable JSON.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct RollbackCommand {
    /// Agent to roll back.
    #[arg(long)]
    agent: String,

    /// Target agent version.
    #[arg(long)]
    to: Option<String>,

    /// Component rollback reference such as skill:handle_refund@1.0.0.
    #[arg(long)]
    component: Option<String>,

    /// Path to the fleet manifest.
    #[arg(long, default_value = "manifests/agents.yml")]
    manifest: PathBuf,

    /// Path to the reusable component catalog.
    #[arg(long, default_value = "manifests/catalog.yml")]
    catalog: PathBuf,

    /// Path to the template directory.
    #[arg(long, default_value = "templates/agent")]
    template_dir: PathBuf,

    /// Emit machine-readable JSON.
    #[arg(long)]
    json: bool,
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

    /// Path to the template directory.
    #[arg(long, default_value = "templates/agent")]
    template_dir: PathBuf,

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

    /// Path to the template directory.
    #[arg(long, default_value = "templates/agent")]
    template_dir: PathBuf,
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
    version_policy: VersionPolicy,
    #[serde(default)]
    shared: SharedComponents,
    agents: Vec<AgentManifest>,
}

#[derive(Debug, Default, Deserialize)]
struct ManifestDefaults {
    model: Option<String>,
    owner: Option<String>,
    auth: Option<String>,
    visibility: Option<String>,
    cost_budget: Option<f64>,
    token_budget: Option<u64>,
    timeout: Option<String>,
    #[serde(default)]
    channels: Vec<String>,
    #[serde(default)]
    schedules: Vec<String>,
    #[serde(default)]
    evals: Vec<String>,
    approvals: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct VersionPolicy {
    default: Option<PolicyUpdateKind>,
    approvals: Option<PolicyUpdateKind>,
    memory: Option<PolicyUpdateKind>,
    #[serde(default)]
    tools: BTreeMap<String, PolicyUpdateKind>,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum PolicyUpdateKind {
    Pin,
    Patch,
    PatchAuto,
    Minor,
    Major,
}

#[derive(Debug, Default, Deserialize)]
struct SharedComponents {
    #[serde(default)]
    tools: Vec<String>,
    #[serde(default)]
    skills: Vec<String>,
    #[serde(default)]
    memory: Vec<String>,
    #[serde(default)]
    schedules: Vec<String>,
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
    schedules: Vec<String>,
    #[serde(default)]
    approvals: BTreeMap<String, String>,
    #[serde(default)]
    evals: Vec<String>,
    #[serde(default)]
    memory: ComponentMap,
    risk: Option<String>,
    auth: Option<String>,
    visibility: Option<String>,
    cost_budget: Option<f64>,
    token_budget: Option<u64>,
    timeout: Option<String>,
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
    #[serde(default)]
    schedules: BTreeMap<String, CatalogComponent>,
}

#[derive(Debug, Default, Deserialize)]
struct CatalogComponent {
    version: String,
    side_effects: Option<SideEffects>,
    retention: Option<String>,
    schedule: Option<String>,
    owner: Option<String>,
    auth: Option<String>,
    visibility: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct EnvironmentsManifest {
    #[serde(default)]
    environments: BTreeMap<String, EnvironmentPolicy>,
}

#[derive(Debug, Default, Deserialize)]
struct EnvironmentPolicy {
    #[serde(default)]
    required_env: Vec<String>,
    #[serde(default)]
    required_secrets: Vec<String>,
    #[serde(default)]
    required_connections: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_observability")]
    observability: Option<bool>,
}

fn deserialize_observability<'de, D>(deserializer: D) -> Result<Option<bool>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<serde_yaml::Value>::deserialize(deserializer)?;
    let Some(value) = value else {
        return Ok(None);
    };
    match value {
        serde_yaml::Value::Bool(value) => Ok(Some(value)),
        serde_yaml::Value::String(value) => Ok(Some(matches!(
            value.as_str(),
            "required" | "enabled" | "verbose" | "true" | "yes"
        ))),
        _ => Ok(Some(false)),
    }
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
        Command::Init(command) => init(command),
        Command::Plan(command) => plan(command),
        Command::Apply(command) => apply(command),
        Command::Render(command) => render(command),
        Command::Doctor(command) => doctor(command),
        Command::Generate(command) => generate(command),
        Command::Outdated(command) => outdated(command),
        Command::Update(command) => update(command),
        Command::Hotload(command) => hotload(command),
        Command::Deploy(command) => deploy(command),
        Command::Eval(command) => runtime_delegate(RuntimeKind::Eval, command),
        Command::Test(command) => runtime_delegate(RuntimeKind::Test, command),
        Command::Preview(command) => runtime_delegate(RuntimeKind::Preview, command),
        Command::Migrate(command) => migrate(command),
        Command::Rollback(command) => rollback(command),
        Command::Inspect(command) => inspect(command),
        Command::Graph(command) => graph(command),
    }
}

fn plan(command: ManifestCommand) -> Result<()> {
    let manifest = load_manifest(&command.manifest)?;
    let catalog = load_catalog(&command.catalog)?;
    let report = validate_manifest(&manifest, &catalog);
    let batch = batch_plan(&manifest, &catalog, &command.templates)?;

    if command.json {
        let output = serde_json::json!({
            "manifest": command.manifest,
            "catalog": command.catalog,
            "templates": command.templates,
            "agent_count": manifest.agents.len(),
            "summary": batch.summary(),
            "operations": batch.report(),
            "errors": report.errors,
            "warnings": report.warnings,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    println!("Plan for {}", command.manifest.display());
    println!("Catalog: {}", command.catalog.display());
    println!("Templates: {}", command.templates.display());
    println!("Agents: {}", manifest.agents.len());
    print_batch_summary(&batch);
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
            "  schedules: {}",
            effective_schedules(agent, &manifest).len()
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

fn apply(command: ManifestCommand) -> Result<()> {
    let manifest = load_manifest(&command.manifest)?;
    let catalog = load_catalog(&command.catalog)?;
    let report = validate_manifest(&manifest, &catalog);
    ensure_valid(&report)?;
    let batch = batch_plan(&manifest, &catalog, &command.templates)?;

    for operation in &batch.operations {
        match operation.action {
            BatchAction::Skip => {}
            BatchAction::Create | BatchAction::Update => {
                if let Some(parent) = operation.path.parent() {
                    fs::create_dir_all(parent).with_context(|| {
                        format!("failed to create output directory '{}'", parent.display())
                    })?;
                }
                fs::write(&operation.path, &operation.content)
                    .with_context(|| format!("failed to write '{}'", operation.path.display()))?;
            }
        }
    }

    if command.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "manifest": command.manifest,
                "catalog": command.catalog,
                "templates": command.templates,
                "agent_count": manifest.agents.len(),
                "summary": batch.summary(),
                "operations": batch.report(),
            }))?
        );
        return Ok(());
    }

    println!("Applied {}", command.manifest.display());
    print_batch_summary(&batch);
    for operation in &batch.operations {
        if operation.action != BatchAction::Skip {
            println!("{} {}", operation.action, operation.path.display());
        }
    }

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

fn init(command: InitCommand) -> Result<()> {
    if command.template != "basic" && command.template != "customer-support" {
        bail!(
            "unknown template '{}'; expected basic or customer-support",
            command.template
        );
    }
    let root = PathBuf::from(&command.name);
    let changes = init_changes(&command, &root);
    for change in &changes {
        ensure_writable(&change.path, command.force || command.dry_run)?;
    }
    if command.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "project": command.name,
                "template": command.template,
                "dry_run": command.dry_run,
                "changes": changes.iter().map(|change| serde_json::json!({
                    "path": change.path,
                    "action": change.action,
                })).collect::<Vec<_>>(),
            }))?
        );
        return Ok(());
    }
    if command.dry_run {
        println!("Init '{}' ({}) dry run", command.name, command.template);
        for change in changes {
            println!("{} {}", change_word(&change.action), change.path.display());
        }
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

fn init_changes(command: &InitCommand, root: &Path) -> Vec<PlannedChange> {
    vec![
        PlannedChange {
            path: root.join("README.md"),
            action: change_action(&root.join("README.md")),
            content: format!(
                "# {}\n\nRails-style Eve Rails project generated by eve-rails-cli.\n",
                command.name
            ),
        },
        PlannedChange {
            path: root.join(".gitignore"),
            action: change_action(&root.join(".gitignore")),
            content: "agents/*/node_modules/\nagents/*/.eve/\nagents/*/.output/\nagents/*/package-lock.json\n.env.*\n!.env.example\n".to_string(),
        },
        PlannedChange {
            path: root.join("manifests").join("agents.yml"),
            action: change_action(&root.join("manifests").join("agents.yml")),
            content: format!(
                "defaults:\n  model: {}\n  owner: {}\n  channels: []\n  schedules: []\n  evals: []\n\nagents:\n",
                command.model, command.owner
            ),
        },
        PlannedChange {
            path: root.join("manifests").join("catalog.yml"),
            action: change_action(&root.join("manifests").join("catalog.yml")),
            content: default_catalog_yaml().to_string(),
        },
        PlannedChange {
            path: root.join("manifests").join("environments.yml"),
            action: change_action(&root.join("manifests").join("environments.yml")),
            content: "environments:\n  development:\n    observability: false\n  production:\n    observability: true\n    required_env: []\n    required_secrets: []\n    required_connections: []\n".to_string(),
        },
        PlannedChange {
            path: root.join("templates").join("agent").join("instructions.md.j2"),
            action: change_action(&root.join("templates").join("agent").join("instructions.md.j2")),
            content: include_str!("../templates/agent/instructions.md.j2").to_string(),
        },
        PlannedChange {
            path: root.join("templates").join("agent").join("agent.ts.j2"),
            action: change_action(&root.join("templates").join("agent").join("agent.ts.j2")),
            content: include_str!("../templates/agent/agent.ts.j2").to_string(),
        },
        PlannedChange {
            path: root.join("templates").join("agent").join("schedule.ts.j2"),
            action: change_action(&root.join("templates").join("agent").join("schedule.ts.j2")),
            content: include_str!("../templates/agent/schedule.ts.j2").to_string(),
        },
    ]
}

fn doctor(command: DoctorCommand) -> Result<()> {
    let manifest = load_manifest(&command.manifest)?;
    let catalog = load_catalog(&command.catalog)?;
    let report = run_doctor(&manifest, &catalog, &command)?;
    let passed = !report.has_failures();

    if command.json {
        let output = serde_json::json!({
            "manifest": command.manifest,
            "catalog": command.catalog,
            "templates_path": command.template_dir,
            "all": command.all,
            "updates": command.updates,
            "templates": command.templates,
            "passed": passed,
            "checks": report.checks,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return if passed {
            Ok(())
        } else {
            bail!("doctor failed")
        };
    }

    println!("Doctor for {}", command.manifest.display());
    println!("Catalog: {}", command.catalog.display());
    println!("Templates: {}", command.template_dir.display());
    println!("Agents checked: {}", manifest.agents.len());
    print_doctor_report(&report);

    if passed {
        Ok(())
    } else {
        bail!("doctor failed")
    }
}

fn generate(command: GenerateCommand) -> Result<()> {
    match command.component {
        GenerateComponent::Batch(command) => plan(command),
        GenerateComponent::Agent(command) => generate_named(GeneratorKind::Agent, command),
        GenerateComponent::Tool(command) => generate_named(GeneratorKind::Tool, command),
        GenerateComponent::Skill(command) => generate_named(GeneratorKind::Skill, command),
        GenerateComponent::Subagent(command) => generate_named(GeneratorKind::Subagent, command),
        GenerateComponent::Channel(command) => generate_named(GeneratorKind::Channel, command),
        GenerateComponent::Schedule(command) => generate_named(GeneratorKind::Schedule, command),
        GenerateComponent::Approval(command) => generate_named(GeneratorKind::Approval, command),
        GenerateComponent::Eval(command) => generate_named(GeneratorKind::Eval, command),
        GenerateComponent::Memory(command) => generate_named(GeneratorKind::Memory, command),
        GenerateComponent::Migration(command) => generate_named(GeneratorKind::Migration, command),
    }
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
enum RuntimeKind {
    Eval,
    Test,
    Preview,
}

fn runtime_delegate(kind: RuntimeKind, command: RuntimeCommand) -> Result<()> {
    let manifest = load_manifest(&command.manifest)?;
    let agent = manifest
        .agents
        .iter()
        .find(|agent| agent.name == command.agent)
        .with_context(|| format!("agent '{}' not found", command.agent))?;
    let agent_dir = PathBuf::from("agents").join(&agent.name);
    let commands = runtime_commands(kind, &agent_dir);
    if command.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "agent": command.agent,
                "env": command.env,
                "kind": kind,
                "agent_dir": agent_dir,
                "dry_run": command.dry_run,
                "commands": commands,
            }))?
        );
        return Ok(());
    }
    println!("{kind:?} for agent {}", agent.name);
    for command_line in &commands {
        println!("$ {}", command_line.join(" "));
    }
    if command.dry_run || matches!(kind, RuntimeKind::Preview) {
        if matches!(kind, RuntimeKind::Preview) {
            println!("Preview is long-running; run the command above to start Eve dev.");
        }
        return Ok(());
    }
    for command_line in commands {
        run_process(&agent_dir, &command_line)?;
    }
    Ok(())
}

fn runtime_commands(kind: RuntimeKind, agent_dir: &Path) -> Vec<Vec<String>> {
    let dir = agent_dir.display().to_string();
    match kind {
        RuntimeKind::Eval => vec![vec![
            "npm".to_string(),
            "exec".to_string(),
            "--".to_string(),
            "eve".to_string(),
            "eval".to_string(),
        ]],
        RuntimeKind::Test => vec![
            vec![
                "npm".to_string(),
                "run".to_string(),
                "typecheck".to_string(),
            ],
            vec![
                "npm".to_string(),
                "exec".to_string(),
                "--".to_string(),
                "eve".to_string(),
                "info".to_string(),
                "--json".to_string(),
            ],
        ],
        RuntimeKind::Preview => vec![vec![
            "cd".to_string(),
            dir,
            "&&".to_string(),
            "npm".to_string(),
            "exec".to_string(),
            "--".to_string(),
            "eve".to_string(),
            "dev".to_string(),
            "--no-ui".to_string(),
        ]],
    }
}

fn run_process(cwd: &Path, command_line: &[String]) -> Result<()> {
    let Some((program, args)) = command_line.split_first() else {
        return Ok(());
    };
    let status = ProcessCommand::new(program)
        .args(args)
        .current_dir(cwd)
        .status()
        .with_context(|| format!("failed to run '{}'", command_line.join(" ")))?;
    if status.success() {
        Ok(())
    } else {
        bail!("command failed: {}", command_line.join(" "))
    }
}

#[derive(Debug, Serialize)]
struct MigrationPlan {
    env: String,
    agent: Option<String>,
    pending: Vec<PathBuf>,
    apply: bool,
}

fn migrate(command: MigrateCommand) -> Result<()> {
    let manifest_path = command.fleet.as_deref().unwrap_or(&command.manifest);
    let manifest = load_manifest(manifest_path)?;
    let agents = selected_agents(&manifest, command.agent.as_deref())?;
    let mut pending = Vec::new();
    for agent in agents {
        let path = PathBuf::from("agents")
            .join(&agent.name)
            .join("agent")
            .join("migrations");
        if path.exists() {
            for entry in fs::read_dir(&path)
                .with_context(|| format!("failed to read '{}'", path.display()))?
            {
                let entry = entry?;
                if entry.path().extension().is_some_and(|ext| ext == "ts") {
                    pending.push(entry.path());
                }
            }
        }
    }
    let plan = MigrationPlan {
        env: command.env,
        agent: command.agent,
        pending,
        apply: command.apply && !command.dry_run,
    };
    if command.json {
        println!("{}", serde_json::to_string_pretty(&plan)?);
    } else {
        println!("Migration plan for {}", manifest_path.display());
        println!("Environment: {}", plan.env);
        println!("Apply: {}", plan.apply);
        if plan.pending.is_empty() {
            println!("No pending migration files found.");
        } else {
            for path in &plan.pending {
                println!("pending {}", path.display());
            }
        }
    }
    Ok(())
}

fn update(command: UpdateCommand) -> Result<()> {
    let manifest_path = command.fleet.as_deref().unwrap_or(&command.manifest);
    let manifest = load_manifest(manifest_path)?;
    let catalog = load_catalog(&command.catalog)?;
    let updates = collect_version_reports(&manifest, &catalog, command.agent.as_deref());

    if command.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "manifest": manifest_path,
                "catalog": command.catalog,
                "mode": "update-plan",
                "updates": updates,
            }))?
        );
        return Ok(());
    }

    println!("Update plan for {}", manifest_path.display());
    print_version_reports(&updates);
    if command.apply {
        println!("Apply is reserved for PR 6 follow-up; manifest rewriting is not performed yet.");
    }
    Ok(())
}

fn outdated(command: ManifestCommand) -> Result<()> {
    let manifest = load_manifest(&command.manifest)?;
    let catalog = load_catalog(&command.catalog)?;
    let updates = collect_version_reports(&manifest, &catalog, None);

    if command.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "manifest": command.manifest,
                "catalog": command.catalog,
                "updates": updates,
            }))?
        );
        return Ok(());
    }

    println!("Outdated report for {}", command.manifest.display());
    print_version_reports(&updates);
    Ok(())
}

fn hotload(command: HotloadCommand) -> Result<()> {
    let component = ComponentRef::parse(&command.component)?;
    let classification = classify_hotload(&component, &command.current)?;

    if command.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "agent": command.agent,
                "component": component,
                "current": command.current,
                "classification": classification,
            }))?
        );
        return if classification.action == HotloadAction::Hotload {
            Ok(())
        } else {
            bail!("component is not hot-loadable")
        };
    }

    println!("Hot-load check for agent {}", command.agent);
    println!(
        "{}:{} {} -> {}",
        component.kind, component.name, command.current, component.version
    );
    println!("{} - {}", classification.action, classification.reason);

    if classification.action == HotloadAction::Hotload {
        Ok(())
    } else {
        bail!("component is not hot-loadable")
    }
}

#[derive(Clone, Debug, Serialize)]
struct ComponentRef {
    kind: ComponentKind,
    name: String,
    version: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
enum ComponentKind {
    Tool,
    Skill,
    Subagent,
    Channel,
    Schedule,
    Approval,
    Eval,
    Memory,
    Runtime,
}

#[derive(Debug, Serialize)]
struct HotloadClassification {
    action: HotloadAction,
    reason: String,
    update: UpdateKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
enum HotloadAction {
    Hotload,
    Restart,
    Redeploy,
    Migration,
    Deny,
}

impl std::fmt::Display for ComponentKind {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            ComponentKind::Tool => "tool",
            ComponentKind::Skill => "skill",
            ComponentKind::Subagent => "subagent",
            ComponentKind::Channel => "channel",
            ComponentKind::Schedule => "schedule",
            ComponentKind::Approval => "approval",
            ComponentKind::Eval => "eval",
            ComponentKind::Memory => "memory",
            ComponentKind::Runtime => "runtime",
        };
        formatter.write_str(value)
    }
}

impl std::fmt::Display for HotloadAction {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            HotloadAction::Hotload => "hotload",
            HotloadAction::Restart => "restart",
            HotloadAction::Redeploy => "redeploy",
            HotloadAction::Migration => "migration",
            HotloadAction::Deny => "deny",
        };
        formatter.write_str(value)
    }
}

impl ComponentRef {
    fn parse(value: &str) -> Result<Self> {
        let (kind, rest) = value
            .split_once(':')
            .with_context(|| format!("component ref '{value}' must look like kind:name@version"))?;
        let (name, version) = rest
            .rsplit_once('@')
            .with_context(|| format!("component ref '{value}' must include @version"))?;
        let kind = parse_component_kind(kind)?;
        ensure_slug(name)?;
        Ok(Self {
            kind,
            name: name.to_string(),
            version: version.to_string(),
        })
    }
}

fn parse_component_kind(value: &str) -> Result<ComponentKind> {
    match value {
        "tool" => Ok(ComponentKind::Tool),
        "skill" => Ok(ComponentKind::Skill),
        "subagent" => Ok(ComponentKind::Subagent),
        "channel" => Ok(ComponentKind::Channel),
        "schedule" => Ok(ComponentKind::Schedule),
        "approval" => Ok(ComponentKind::Approval),
        "eval" => Ok(ComponentKind::Eval),
        "memory" => Ok(ComponentKind::Memory),
        "runtime" => Ok(ComponentKind::Runtime),
        _ => bail!("unknown component kind '{value}'"),
    }
}

fn classify_hotload(component: &ComponentRef, current: &str) -> Result<HotloadClassification> {
    let update = classify_version_change(current, &component.version);
    let (action, reason) = match component.kind {
        ComponentKind::Skill if update == UpdateKind::Patch => (
            HotloadAction::Hotload,
            "skill patch preserves trigger/tool/output contract by policy",
        ),
        ComponentKind::Eval if matches!(update, UpdateKind::Patch | UpdateKind::Minor) => (
            HotloadAction::Hotload,
            "eval additions are safe to hot-load",
        ),
        ComponentKind::Approval if update == UpdateKind::Patch => (
            HotloadAction::Hotload,
            "approval patch is treated as stricter policy metadata",
        ),
        ComponentKind::Channel if update == UpdateKind::Patch => (
            HotloadAction::Restart,
            "channel adapter patch requires process restart",
        ),
        ComponentKind::Schedule => (
            HotloadAction::Redeploy,
            "schedule changes can increase autonomous activity and require redeploy",
        ),
        ComponentKind::Tool if update == UpdateKind::Patch => (
            HotloadAction::Redeploy,
            "tool patches may alter side effects or schema and require redeploy",
        ),
        ComponentKind::Memory => (
            HotloadAction::Migration,
            "memory schema changes require migration",
        ),
        ComponentKind::Runtime => (HotloadAction::Redeploy, "runtime changes require redeploy"),
        ComponentKind::Subagent => (
            HotloadAction::Redeploy,
            "subagent contract changes require redeploy",
        ),
        _ if update == UpdateKind::Major => (
            HotloadAction::Redeploy,
            "major updates require explicit redeploy",
        ),
        _ if update == UpdateKind::Invalid => (
            HotloadAction::Deny,
            "invalid version change cannot be classified",
        ),
        _ => (
            HotloadAction::Redeploy,
            "change is not eligible for conservative hot-load",
        ),
    };

    Ok(HotloadClassification {
        action,
        reason: reason.to_string(),
        update,
    })
}

fn classify_version_change(current: &str, next: &str) -> UpdateKind {
    let Some(current) = current.parse::<Semver>().ok() else {
        return UpdateKind::Invalid;
    };
    let Some(next) = next.parse::<Semver>().ok() else {
        return UpdateKind::Invalid;
    };
    if current == next {
        UpdateKind::Current
    } else if current.major != next.major {
        UpdateKind::Major
    } else if current.minor != next.minor {
        UpdateKind::Minor
    } else {
        UpdateKind::Patch
    }
}

#[derive(Debug, Serialize)]
struct DeployReport {
    env: String,
    agent: Option<String>,
    dry_run: bool,
    promote: bool,
    rollback_to: Option<String>,
    delegated_command: Vec<String>,
    gates: Vec<DeployGate>,
}

#[derive(Debug, Serialize)]
struct DeployGate {
    name: String,
    passed: bool,
    message: String,
}

#[derive(Debug, Serialize)]
struct RollbackReport {
    agent: String,
    target: String,
    operations: Vec<RollbackOperation>,
    blocked: bool,
    reason: Option<String>,
}

#[derive(Debug, Serialize)]
struct RollbackOperation {
    path: PathBuf,
    action: String,
}

fn deploy_preflight(
    command: &DeployCommand,
    manifest: &FleetManifest,
    catalog: &CatalogManifest,
) -> Result<DeployReport> {
    let mut gates = Vec::new();
    let doctor_command = DoctorCommand {
        all: command.agent.is_none(),
        updates: true,
        templates: true,
        fix: false,
        env: Some(command.env.clone()),
        connections: false,
        budgets: true,
        json: false,
        manifest: command
            .fleet
            .clone()
            .unwrap_or_else(|| command.manifest.clone()),
        catalog: command.catalog.clone(),
        template_dir: command.template_dir.clone(),
        environments: PathBuf::from("manifests/environments.yml"),
    };
    let doctor = run_doctor(manifest, catalog, &doctor_command)?;
    let doctor_passed = !doctor.has_failures();
    gates.push(DeployGate {
        name: "doctor".to_string(),
        passed: !command.require_doctor || doctor_passed,
        message: if doctor_passed {
            "doctor checks pass".to_string()
        } else {
            "doctor checks failed".to_string()
        },
    });

    let evals_passed = eval_gate_passes(manifest, command.agent.as_deref());
    gates.push(DeployGate {
        name: "evals".to_string(),
        passed: !command.require_evals || evals_passed,
        message: if evals_passed {
            "required eval references are present".to_string()
        } else {
            "one or more deploy targets have no eval references".to_string()
        },
    });

    let approvals_passed = validate_manifest(manifest, catalog)
        .errors
        .iter()
        .all(|error| !error.contains("risky tool"));
    gates.push(DeployGate {
        name: "approval-coverage".to_string(),
        passed: !command.require_approvals || approvals_passed,
        message: "risky tool approval coverage checked".to_string(),
    });

    gates.push(DeployGate {
        name: "rollback-target".to_string(),
        passed: true,
        message: "rollback metadata will use generated manifest and versions.lock".to_string(),
    });

    Ok(DeployReport {
        env: command.env.clone(),
        agent: command.agent.clone(),
        dry_run: command.dry_run,
        promote: command.promote,
        rollback_to: command.rollback_to.clone(),
        delegated_command: vec![
            "npm".to_string(),
            "exec".to_string(),
            "--".to_string(),
            "eve".to_string(),
            "deploy".to_string(),
        ],
        gates,
    })
}

fn eval_gate_passes(manifest: &FleetManifest, agent_filter: Option<&str>) -> bool {
    manifest
        .agents
        .iter()
        .filter(|agent| agent_filter.is_none_or(|filter| filter == agent.name))
        .all(|agent| !effective_evals(agent, manifest).is_empty())
}

fn rollback_plan(
    command: &RollbackCommand,
    manifest: &FleetManifest,
    catalog: &CatalogManifest,
) -> Result<RollbackReport> {
    let agent = manifest
        .agents
        .iter()
        .find(|agent| agent.name == command.agent)
        .with_context(|| format!("agent '{}' not found", command.agent))?;
    let target = command
        .to
        .clone()
        .or_else(|| command.component.clone())
        .context("pass --to <version> or --component kind:name@version")?;
    let component = command
        .component
        .as_deref()
        .map(ComponentRef::parse)
        .transpose()?;
    let blocked = component
        .as_ref()
        .is_some_and(|component| component.kind == ComponentKind::Memory);
    let reason = if blocked {
        Some("memory rollback requires an explicit migration plan".to_string())
    } else {
        None
    };
    let renderer = Renderer::load(&command.template_dir)?;
    let rendered = renderer.render_agent(agent, manifest, catalog)?;
    let operations = rendered
        .into_iter()
        .filter(|file| {
            file.path.ends_with("agent.manifest.yml") || file.path.ends_with("versions.lock")
        })
        .map(|file| RollbackOperation {
            path: file.path,
            action: "restore".to_string(),
        })
        .collect();

    Ok(RollbackReport {
        agent: command.agent.clone(),
        target,
        operations,
        blocked,
        reason,
    })
}

#[derive(Debug, Serialize)]
struct AgentSummary {
    name: String,
    version: String,
    owner: Option<String>,
    model: Option<String>,
    responsibility: String,
    tools: ComponentSummary,
    skills: ComponentSummary,
    subagents: Vec<String>,
    channels: Vec<String>,
    schedules: Vec<String>,
    approvals: BTreeMap<String, String>,
    evals: Vec<String>,
    memory: ComponentSummary,
    generated: GeneratedSummary,
    deployment: DeploymentSummary,
}

type ComponentSummary = BTreeMap<String, String>;

#[derive(Debug, Serialize)]
struct GeneratedSummary {
    manifest_path: PathBuf,
    lockfile_path: PathBuf,
    fresh: bool,
}

#[derive(Debug, Serialize)]
struct DeploymentSummary {
    doctor_passed: bool,
    deployable: bool,
}

fn summarize_agent(
    agent: &AgentManifest,
    manifest: &FleetManifest,
    catalog: &CatalogManifest,
    template_dir: &Path,
) -> Result<AgentSummary> {
    let doctor_command = DoctorCommand {
        all: false,
        updates: true,
        templates: true,
        fix: false,
        env: None,
        connections: false,
        budgets: true,
        json: false,
        manifest: PathBuf::from("manifests/agents.yml"),
        catalog: PathBuf::from("manifests/catalog.yml"),
        template_dir: template_dir.to_path_buf(),
        environments: PathBuf::from("manifests/environments.yml"),
    };
    let doctor = run_doctor(manifest, catalog, &doctor_command)?;
    let generated = generated_summary(agent, manifest, catalog, template_dir)?;
    let doctor_passed = !doctor.has_failures();

    Ok(AgentSummary {
        name: agent.name.clone(),
        version: agent.version.clone().unwrap_or_else(|| "1.0.0".to_string()),
        owner: agent
            .owner
            .clone()
            .or_else(|| manifest.defaults.owner.clone()),
        model: agent
            .model
            .clone()
            .or_else(|| manifest.defaults.model.clone()),
        responsibility: agent.responsibility.clone(),
        tools: resolved_component_summary(&agent.tools, &catalog.tools),
        skills: resolved_component_summary(&agent.skills, &catalog.skills),
        subagents: agent.subagents.clone(),
        channels: manifest
            .defaults
            .channels
            .iter()
            .chain(agent.channels.iter())
            .cloned()
            .collect(),
        schedules: effective_schedules(agent, manifest),
        approvals: agent.approvals.clone(),
        evals: effective_evals(agent, manifest),
        memory: resolved_component_summary(&agent.memory, &catalog.memory),
        generated,
        deployment: DeploymentSummary {
            doctor_passed,
            deployable: doctor_passed && !effective_evals(agent, manifest).is_empty(),
        },
    })
}

fn generated_summary(
    agent: &AgentManifest,
    manifest: &FleetManifest,
    catalog: &CatalogManifest,
    template_dir: &Path,
) -> Result<GeneratedSummary> {
    let renderer = Renderer::load(template_dir)?;
    let files = renderer.render_agent(agent, manifest, catalog)?;
    let fresh = files.iter().try_fold(true, |fresh, file| {
        Ok::<bool, anyhow::Error>(fresh && file_matches(&file.path, &file.content)?)
    })?;
    let output_root = PathBuf::from("agents").join(&agent.name).join("agent");
    Ok(GeneratedSummary {
        manifest_path: output_root.join("agent.manifest.yml"),
        lockfile_path: output_root.join("versions.lock"),
        fresh,
    })
}

fn resolved_component_summary(
    components: &ComponentMap,
    catalog: &BTreeMap<String, CatalogComponent>,
) -> ComponentSummary {
    components
        .iter()
        .map(|(name, requested)| {
            (
                name.clone(),
                resolve_version(requested, catalog.get(name)).unwrap_or_else(|| requested.clone()),
            )
        })
        .collect()
}

fn format_components(components: &ComponentSummary) -> String {
    if components.is_empty() {
        return "<none>".to_string();
    }
    components
        .iter()
        .map(|(name, version)| format!("{name}@{version}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn deploy(command: DeployCommand) -> Result<()> {
    let manifest_path = command.fleet.as_deref().unwrap_or(&command.manifest);
    let manifest = load_manifest(manifest_path)?;
    let catalog = load_catalog(&command.catalog)?;
    let report = deploy_preflight(&command, &manifest, &catalog)?;
    let passed = !report.gates.iter().any(|gate| !gate.passed);

    if command.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
        return if passed {
            Ok(())
        } else {
            bail!("deploy preflight failed")
        };
    }

    println!("Deploy preflight for {}", command.env);
    if let Some(agent) = &command.agent {
        println!("Agent: {agent}");
    }
    if let Some(canary) = command.canary {
        println!("Canary: {canary}%");
    }
    for gate in &report.gates {
        println!(
            "[{}] {} - {}",
            if gate.passed { "pass" } else { "fail" },
            gate.name,
            gate.message
        );
    }
    println!("Delegated command: {}", report.delegated_command.join(" "));
    if command.dry_run {
        println!("Dry run only; Eve deploy was not invoked.");
    }

    if !passed {
        bail!("deploy preflight failed")
    } else if command.dry_run {
        Ok(())
    } else {
        let agent_name = command
            .agent
            .clone()
            .context("non-dry deploy requires --agent <name>")?;
        let agent_dir = PathBuf::from("agents").join(agent_name);
        run_process(&agent_dir, &report.delegated_command)
    }
}

fn rollback(command: RollbackCommand) -> Result<()> {
    let manifest = load_manifest(&command.manifest)?;
    let catalog = load_catalog(&command.catalog)?;
    let report = rollback_plan(&command, &manifest, &catalog)?;
    let blocked = report.blocked;

    if command.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
        return if blocked {
            bail!("rollback requires migration")
        } else {
            Ok(())
        };
    }

    println!("Rollback plan for {}", command.agent);
    for operation in &report.operations {
        println!("{} {}", operation.action, operation.path.display());
    }
    if let Some(reason) = &report.reason {
        println!("{reason}");
    }

    if blocked {
        bail!("rollback requires migration")
    } else {
        Ok(())
    }
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
    let summary = summarize_agent(agent, &manifest, &catalog, &command.template_dir)?;

    if command.json {
        println!("{}", serde_json::to_string_pretty(&summary)?);
        return Ok(());
    }

    println!("{}", summary.name);
    println!("  version: {}", summary.version);
    println!(
        "  owner: {}",
        summary.owner.as_deref().unwrap_or("<missing>")
    );
    println!(
        "  model: {}",
        summary.model.as_deref().unwrap_or("<missing>")
    );
    println!("  responsibility: {}", summary.responsibility);
    println!("  tools: {}", format_components(&summary.tools));
    println!("  skills: {}", format_components(&summary.skills));
    println!("  subagents: {}", summary.subagents.join(", "));
    println!("  channels: {}", summary.channels.join(", "));
    println!("  schedules: {}", summary.schedules.join(", "));
    println!("  approvals: {}", summary.approvals.len());
    println!("  evals: {}", summary.evals.join(", "));
    println!("  memory: {}", format_components(&summary.memory));
    println!("  generated fresh: {}", summary.generated.fresh);
    println!("  deployable: {}", summary.deployment.deployable);
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

    let summaries = agents
        .iter()
        .map(|agent| summarize_agent(agent, &manifest, &catalog, &command.template_dir))
        .collect::<Result<Vec<_>>>()?;

    match command.format {
        GraphFormat::Text => {
            for summary in &summaries {
                println!("{}", summary.name);
                for subagent in &summary.subagents {
                    println!("  -> subagent/{subagent}");
                }
                for (tool, version) in &summary.tools {
                    println!("  -> tools/{tool}@{version}");
                }
                for (skill, version) in &summary.skills {
                    println!("  -> skills/{skill}@{version}");
                }
                for channel in &summary.channels {
                    println!("  -> channels/{channel}");
                }
                for schedule in &summary.schedules {
                    println!("  -> schedules/{schedule}");
                }
                for eval in &summary.evals {
                    println!("  -> evals/{eval}");
                }
                for (memory, version) in &summary.memory {
                    println!("  -> memory/{memory}@{version}");
                }
            }
        }
        GraphFormat::Mermaid => {
            println!("graph TD");
            for summary in &summaries {
                let agent_node = format!("agent_{}", node(&summary.name));
                println!("  {agent_node}[\"agent:{}\"]", summary.name);
                for subagent in &summary.subagents {
                    println!("  {agent_node} --> subagent_{}", node(subagent));
                }
                for (tool, version) in &summary.tools {
                    println!(
                        "  {agent_node} --> tool_{}[\"tool:{}@{}\"]",
                        node(tool),
                        tool,
                        version
                    );
                }
                for (skill, version) in &summary.skills {
                    println!(
                        "  {agent_node} --> skill_{}[\"skill:{}@{}\"]",
                        node(skill),
                        skill,
                        version
                    );
                }
                for channel in &summary.channels {
                    println!(
                        "  {agent_node} --> channel_{}[\"channel:{}\"]",
                        node(channel),
                        channel
                    );
                }
                for schedule in &summary.schedules {
                    println!(
                        "  {agent_node} --> schedule_{}[\"schedule:{}\"]",
                        node(schedule),
                        schedule
                    );
                }
                for eval in &summary.evals {
                    println!("  {agent_node} --> eval_{}[\"eval:{}\"]", node(eval), eval);
                }
                for (memory, version) in &summary.memory {
                    println!(
                        "  {agent_node} --> memory_{}[\"memory:{}@{}\"]",
                        node(memory),
                        memory,
                        version
                    );
                }
            }
        }
        GraphFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&summaries)?);
        }
    }

    Ok(())
}

#[derive(Clone, Copy, Debug)]
enum GeneratorKind {
    Agent,
    Tool,
    Skill,
    Subagent,
    Channel,
    Schedule,
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
            GeneratorKind::Schedule => "schedule",
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

#[derive(Debug, Serialize)]
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
        | GeneratorKind::Schedule
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

    let mut entry = format!(
        "\n  - name: {name}\n    version: {version}\n    owner: {owner}\n    responsibility: {responsibility:?}\n    model: {model}\n",
        name = command.name,
        version = command.version,
        owner = command.owner.as_deref().unwrap_or("agent-platform"),
        responsibility = command
            .description
            .as_deref()
            .unwrap_or("Describe this agent's bounded responsibility."),
        model = command.model.as_deref().unwrap_or("openai/gpt-5.5"),
    );
    append_generated_component_map(&mut entry, "tools", &command.with_tools);
    append_generated_component_map(&mut entry, "skills", &command.with_skills);
    append_generated_string_list(&mut entry, "subagents", &command.with_subagents);
    append_generated_string_list(&mut entry, "channels", &command.with_channels);
    append_generated_string_list(&mut entry, "schedules", &command.with_schedules);
    append_generated_approvals(&mut entry, command);
    append_generated_string_list(&mut entry, "evals", &command.with_evals);
    append_generated_component_map(&mut entry, "memory", &command.with_memory);
    append_optional_yaml_string(&mut entry, "risk", command.risk.as_deref());
    append_optional_yaml_string(&mut entry, "auth", command.auth.as_deref());
    append_optional_yaml_string(&mut entry, "visibility", command.visibility.as_deref());
    if let Some(cost_budget) = command.cost_budget {
        entry.push_str(&format!("    cost_budget: {cost_budget}\n"));
    }
    if let Some(token_budget) = command.token_budget {
        entry.push_str(&format!("    token_budget: {token_budget}\n"));
    }
    append_optional_yaml_string(&mut entry, "timeout", command.timeout.as_deref());
    let content = append_yaml_list_entry(source, "agents:", &entry);

    Ok(vec![PlannedChange {
        path: command.manifest.clone(),
        action: change_action(&command.manifest),
        content,
    }])
}

fn append_generated_component_map(output: &mut String, label: &str, values: &[String]) {
    output.push_str(&format!("    {label}:"));
    if values.is_empty() {
        output.push_str(" {}\n");
        return;
    }
    output.push('\n');
    for value in values {
        output.push_str(&format!("      {value}: 1.0.0\n"));
    }
}

fn append_generated_string_list(output: &mut String, label: &str, values: &[String]) {
    output.push_str(&format!("    {label}:"));
    if values.is_empty() {
        output.push_str(" []\n");
        return;
    }
    output.push_str(&format!(" [{}]\n", values.join(", ")));
}

fn append_generated_approvals(output: &mut String, command: &GenerateNamed) {
    output.push_str("    approvals:");
    let Some(policy) = command.approval.as_deref() else {
        output.push_str(" {}\n");
        return;
    };
    if command.with_tools.is_empty() {
        output.push_str(&format!(" {policy:?}\n"));
        return;
    }
    output.push('\n');
    for tool in &command.with_tools {
        output.push_str(&format!("      {tool}: {policy}\n"));
    }
}

fn append_optional_yaml_string(output: &mut String, label: &str, value: Option<&str>) {
    if let Some(value) = value.filter(|value| !value.trim().is_empty()) {
        output.push_str(&format!("    {label}: {value:?}\n"));
    }
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
    let timestamp = migration_timestamp();
    let path = PathBuf::from("agents")
        .join("migrations")
        .join(format!("{}_{}.ts", timestamp, command.name));
    ensure_writable(&path, command.force)?;
    Ok(vec![PlannedChange {
        path,
        action: ChangeAction::Create,
        content: format!(
            "export const name = \"{}\";\n\nexport async function up() {{\n  // TODO: implement migration.\n}}\n\nexport async function down() {{\n  // TODO: implement rollback.\n}}\n",
            command.name
        ),
    }])
}

fn migration_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn catalog_section(kind: GeneratorKind) -> &'static str {
    match kind {
        GeneratorKind::Tool => "tools:",
        GeneratorKind::Skill => "skills:",
        GeneratorKind::Channel => "channels:",
        GeneratorKind::Schedule => "schedules:",
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
        GeneratorKind::Schedule => {
            entry.push_str(&format!(
                "    schedule: {:?}\n",
                command.schedule.as_deref().unwrap_or("0 9 * * 1-5")
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
        GeneratorKind::Schedule => (
            "schedules",
            "ts",
            format!(
                "export default {{\n  name: \"{}\",\n  schedule: {:?},\n}};\n",
                command.name,
                command.schedule.as_deref().unwrap_or("0 9 * * 1-5")
            ),
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
    "defaults:\n  model: openai/gpt-5.5\n  owner: agent-platform\n  channels: []\n  schedules: []\n  evals: []\n\nagents:\n"
}

fn default_catalog_yaml() -> &'static str {
    "tools:\nskills:\nevals:\napprovals:\nmemory:\nchannels:\nschedules:\n"
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

fn load_environments(path: &Path) -> Result<EnvironmentsManifest> {
    match fs::read_to_string(path) {
        Ok(source) => serde_yaml::from_str(&source)
            .with_context(|| format!("failed to parse environments '{}'", path.display())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Ok(EnvironmentsManifest::default())
        }
        Err(error) => Err(error).with_context(|| format!("failed to read '{}'", path.display())),
    }
}

fn selected_agents<'a>(
    manifest: &'a FleetManifest,
    agent_filter: Option<&str>,
) -> Result<Vec<&'a AgentManifest>> {
    if let Some(agent_name) = agent_filter {
        Ok(vec![
            manifest
                .agents
                .iter()
                .find(|agent| agent.name == agent_name)
                .with_context(|| format!("agent '{agent_name}' not found"))?,
        ])
    } else {
        Ok(manifest.agents.iter().collect())
    }
}

struct Renderer<'source> {
    env: Environment<'source>,
}

struct RenderedFile {
    path: PathBuf,
    content: String,
}

#[derive(Debug)]
struct BatchPlan {
    operations: Vec<BatchOperation>,
}

#[derive(Debug)]
struct BatchOperation {
    path: PathBuf,
    action: BatchAction,
    content: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
enum BatchAction {
    Create,
    Update,
    Skip,
}

#[derive(Debug, Serialize)]
struct BatchOperationReport {
    path: PathBuf,
    action: BatchAction,
}

#[derive(Debug, Serialize)]
struct BatchSummary {
    create: usize,
    update: usize,
    skip: usize,
    total: usize,
}

impl BatchPlan {
    fn summary(&self) -> BatchSummary {
        let mut summary = BatchSummary {
            create: 0,
            update: 0,
            skip: 0,
            total: self.operations.len(),
        };

        for operation in &self.operations {
            match operation.action {
                BatchAction::Create => summary.create += 1,
                BatchAction::Update => summary.update += 1,
                BatchAction::Skip => summary.skip += 1,
            }
        }

        summary
    }

    fn report(&self) -> Vec<BatchOperationReport> {
        self.operations
            .iter()
            .map(|operation| BatchOperationReport {
                path: operation.path.clone(),
                action: operation.action,
            })
            .collect()
    }
}

impl std::fmt::Display for BatchAction {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BatchAction::Create => formatter.write_str("create"),
            BatchAction::Update => formatter.write_str("update"),
            BatchAction::Skip => formatter.write_str("skip"),
        }
    }
}

fn batch_plan(
    manifest: &FleetManifest,
    catalog: &CatalogManifest,
    template_dir: &Path,
) -> Result<BatchPlan> {
    let renderer = Renderer::load(template_dir)?;
    let mut operations = Vec::new();

    for agent in &manifest.agents {
        for rendered in renderer.render_agent(agent, manifest, catalog)? {
            let action = batch_action(&rendered.path, &rendered.content)?;
            operations.push(BatchOperation {
                path: rendered.path,
                action,
                content: rendered.content,
            });
        }
    }

    Ok(BatchPlan { operations })
}

fn batch_action(path: &Path, expected: &str) -> Result<BatchAction> {
    match fs::read_to_string(path) {
        Ok(actual) if actual == expected => Ok(BatchAction::Skip),
        Ok(_) => Ok(BatchAction::Update),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(BatchAction::Create),
        Err(error) => Err(error).with_context(|| format!("failed to read '{}'", path.display())),
    }
}

fn print_batch_summary(batch: &BatchPlan) {
    let summary = batch.summary();
    println!(
        "Batch: {} create, {} update, {} skip, {} total files",
        summary.create, summary.update, summary.skip, summary.total
    );
}

impl<'source> Renderer<'source> {
    fn load(template_dir: &Path) -> Result<Self> {
        let mut env = Environment::new();
        env.set_undefined_behavior(minijinja::UndefinedBehavior::Strict);

        for template_name in [
            "instructions.md.j2",
            "agent.ts.j2",
            "tool.ts.j2",
            "skill.md.j2",
            "schedule.ts.j2",
            "approval.ts.j2",
            "eval.ts.j2",
            "memory.ts.j2",
            "fixture.json.j2",
            "agent.README.md.j2",
        ] {
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
            .render(context! { agent => agent_context.clone() })?;
        let agent_readme = self
            .env
            .get_template("agent.README.md.j2")?
            .render(context! { agent => agent_context.clone() })?;

        let mut files = vec![
            RenderedFile {
                path: PathBuf::from("agents")
                    .join(&agent.name)
                    .join("package.json"),
                content: render_package_json(&agent.name),
            },
            RenderedFile {
                path: PathBuf::from("agents")
                    .join(&agent.name)
                    .join("tsconfig.json"),
                content: render_tsconfig_json(),
            },
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
                path: output_root.join("README.md"),
                content: with_generated_header(
                    CommentStyle::Hash,
                    &agent.name,
                    "agent.README.md.j2",
                    &agent_readme,
                ),
            },
            RenderedFile {
                path: output_root.join("channels").join("eve.ts"),
                content: render_eve_channel(),
            },
            RenderedFile {
                path: output_root.join("agent.manifest.yml"),
                content: render_agent_manifest(agent, manifest),
            },
            RenderedFile {
                path: output_root.join("versions.lock"),
                content: render_versions_lock(agent, manifest, catalog),
            },
        ];

        for (tool, version) in effective_components(&manifest.shared.tools, &agent.tools) {
            let tool_context = context! {
                name => tool.as_str(),
                version => version.as_str(),
                description => catalog
                    .tools
                    .get(&tool)
                    .map(tool_description)
                    .unwrap_or_else(|| format!("{tool} generated tool contract.")),
            };
            let tool_ts = self
                .env
                .get_template("tool.ts.j2")?
                .render(context! { tool => tool_context })?;
            files.push(RenderedFile {
                path: output_root.join("tools").join(format!("{tool}.ts")),
                content: with_generated_header(
                    CommentStyle::Slash,
                    &agent.name,
                    "tool.ts.j2",
                    &tool_ts,
                ),
            });
        }

        for (skill, version) in effective_components(&manifest.shared.skills, &agent.skills) {
            let skill_context = context! {
                name => skill.as_str(),
                version => version.as_str(),
                trigger => format!("the {skill} capability is relevant to the user's request"),
            };
            let skill_md = self
                .env
                .get_template("skill.md.j2")?
                .render(context! { skill => skill_context })?;
            files.push(RenderedFile {
                path: output_root.join("skills").join(format!("{skill}.md")),
                content: with_generated_header(
                    CommentStyle::Hash,
                    &agent.name,
                    "skill.md.j2",
                    &skill_md,
                ),
            });
        }

        for subagent in &agent.subagents {
            files.push(RenderedFile {
                path: output_root
                    .join("subagents")
                    .join(subagent)
                    .join("instructions.md"),
                content: render_subagent_placeholder(&agent.name, subagent),
            });
            files.push(RenderedFile {
                path: output_root
                    .join("subagents")
                    .join(subagent)
                    .join("agent.ts"),
                content: render_subagent_agent_ts(&agent.name, subagent, model),
            });
        }

        for (approval, policy) in &agent.approvals {
            let approval_context = context! {
                component => approval.as_str(),
                policy => policy.as_str(),
            };
            let approval_ts = self
                .env
                .get_template("approval.ts.j2")?
                .render(context! { approval => approval_context })?;
            files.push(RenderedFile {
                path: output_root.join("approvals").join(format!("{approval}.ts")),
                content: with_generated_header(
                    CommentStyle::Slash,
                    &agent.name,
                    "approval.ts.j2",
                    &approval_ts,
                ),
            });
        }

        for eval in effective_evals(agent, manifest) {
            let eval_context = context! {
                name => eval.as_str(),
            };
            let eval_ts = self
                .env
                .get_template("eval.ts.j2")?
                .render(context! { eval => eval_context })?;
            files.push(RenderedFile {
                path: PathBuf::from("agents")
                    .join(&agent.name)
                    .join("evals")
                    .join(format!("{eval}.eval.ts")),
                content: with_generated_header(
                    CommentStyle::Slash,
                    &agent.name,
                    "eval.ts.j2",
                    &eval_ts,
                ),
            });
            files.push(RenderedFile {
                path: output_root
                    .join("evals")
                    .join(format!("{eval}.contract.json")),
                content: render_contract_json("eval", &eval, "Eve eval contract placeholder."),
            });
        }

        for (memory, version) in effective_components(&manifest.shared.memory, &agent.memory) {
            let memory_context = context! {
                name => memory.as_str(),
                version => version.as_str(),
                retention => catalog
                    .memory
                    .get(&memory)
                    .and_then(|component| component.retention.as_deref())
                    .unwrap_or("session"),
            };
            let memory_ts = self
                .env
                .get_template("memory.ts.j2")?
                .render(context! { memory => memory_context })?;
            files.push(RenderedFile {
                path: output_root.join("memory").join(format!("{memory}.ts")),
                content: with_generated_header(
                    CommentStyle::Slash,
                    &agent.name,
                    "memory.ts.j2",
                    &memory_ts,
                ),
            });
        }

        let fixture_context = context! {
            name => format!("{}_smoke", agent.name),
            kind => "smoke",
            description => format!("Generated smoke fixture for {}.", agent.name),
        };
        let fixture_json = self
            .env
            .get_template("fixture.json.j2")?
            .render(context! { fixture => fixture_context })?;
        files.push(RenderedFile {
            path: output_root
                .join("fixtures")
                .join(format!("{}_smoke.json", agent.name)),
            content: format!("{}\n", fixture_json.trim_end()),
        });

        for schedule in effective_schedules(agent, manifest) {
            let schedule_context = context! {
                name => schedule.as_str(),
                schedule => catalog
                    .schedules
                    .get(&schedule)
                    .and_then(|component| component.schedule.as_deref())
                    .unwrap_or(""),
            };
            let schedule_ts = self
                .env
                .get_template("schedule.ts.j2")?
                .render(context! { schedule => schedule_context })?;
            files.push(RenderedFile {
                path: output_root.join("schedules").join(format!("{schedule}.ts")),
                content: with_generated_header(
                    CommentStyle::Slash,
                    &agent.name,
                    "schedule.ts.j2",
                    &schedule_ts,
                ),
            });
        }

        Ok(files)
    }
}

fn render_package_json(agent_name: &str) -> String {
    format!(
        r##"{{
  "name": "{}",
  "version": "0.0.0",
  "x-eve-rails": "generated",
  "type": "module",
  "imports": {{
    "#*": "./agent/*",
    "#evals/*": "./evals/*"
  }},
  "scripts": {{
    "build": "eve build",
    "dev": "eve dev",
    "start": "eve start",
    "typecheck": "tsc"
  }},
  "dependencies": {{
    "@vercel/connect": "0.2.2",
    "ai": "^7.0.26",
    "eve": "^0.24.4",
    "zod": "4.4.3"
  }},
  "devDependencies": {{
    "@types/node": "24.x",
    "typescript": "7.0.2"
  }},
  "overrides": {{
    "ai": "^7.0.26"
  }},
  "engines": {{
    "node": "24.x"
  }}
}}
"##,
        agent_name
    )
}

fn render_tsconfig_json() -> String {
    r#"{
  "x-eve-rails": "generated",
  "compilerOptions": {
    "target": "ES2022",
    "module": "esnext",
    "moduleResolution": "bundler",
    "types": ["node"],
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "noEmit": true
  },
  "include": ["agent/**/*.ts", "evals/**/*.ts"]
}
"#
    .to_string()
}

fn render_eve_channel() -> String {
    r#"// Generated by eve-rails. Do not edit generated regions.
// template: eve-channel

import { eveChannel } from "eve/channels/eve";
import { localDev, placeholderAuth, vercelOidc } from "eve/channels/auth";

export default eveChannel({
  auth: [
    vercelOidc(),
    localDev(),
    placeholderAuth(),
  ],
});
"#
    .to_string()
}

fn tool_description(component: &CatalogComponent) -> String {
    match &component.side_effects {
        Some(side_effects) => format!("Generated {side_effects} tool contract."),
        None => "Generated tool contract.".to_string(),
    }
}

fn render_subagent_placeholder(agent: &str, subagent: &str) -> String {
    with_generated_header(
        CommentStyle::Hash,
        agent,
        "subagent-placeholder",
        &format!(
            "# {subagent}\n\nThis subagent folder is generated as an Eve Rails convention placeholder.\n"
        ),
    )
}

fn render_subagent_agent_ts(agent: &str, subagent: &str, model: &str) -> String {
    with_generated_header(
        CommentStyle::Slash,
        agent,
        "subagent-agent",
        &format!(
            "import {{ defineAgent }} from \"eve\";\n\nexport default defineAgent({{\n  description: \"Generated {subagent} subagent skeleton.\",\n  model: \"{}\",\n}});\n",
            escape_ts_string(model)
        ),
    )
}

fn render_contract_json(kind: &str, name: &str, description: &str) -> String {
    format!(
        "{{\n  \"kind\": \"{}\",\n  \"name\": \"{}\",\n  \"description\": \"{}\"\n}}\n",
        escape_json(kind),
        escape_json(name),
        escape_json(description)
    )
}

fn escape_json(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn escape_ts_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
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
    let schedules = effective_schedules(agent, manifest);
    let evals = manifest
        .defaults
        .evals
        .iter()
        .chain(agent.evals.iter())
        .cloned()
        .collect::<Vec<_>>();
    append_string_list(&mut output, "channels", &channels);
    append_string_list(&mut output, "schedules", &schedules);
    append_string_list(&mut output, "evals", &evals);
    append_optional_string(&mut output, "risk", agent.risk.as_deref());
    append_optional_string(&mut output, "auth", agent.auth.as_deref());
    append_optional_string(&mut output, "visibility", agent.visibility.as_deref());
    if let Some(cost_budget) = agent.cost_budget {
        output.push_str(&format!("cost_budget: {cost_budget}\n"));
    }
    if let Some(token_budget) = agent.token_budget {
        output.push_str(&format!("token_budget: {token_budget}\n"));
    }
    append_optional_string(&mut output, "timeout", agent.timeout.as_deref());
    output.push_str("runtime: eve@0.24.4\n");
    output.push_str("compatibility:\n");
    output.push_str("  hot_load: true\n");
    output.push_str("  requires_restart: false\n");
    output.push_str("  min_runtime: eve@0.24.4\n");
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
    output.push_str(
        "  runtime/eve@0.24.4:\n    source: package.json\n    digest: runtime:eve:0.24.4\n",
    );
    append_lock_entries(&mut output, "tools", &agent.tools, &catalog.tools);
    append_lock_entries(&mut output, "skills", &agent.skills, &catalog.skills);
    append_lock_entries(&mut output, "memory", &agent.memory, &catalog.memory);
    for channel in manifest
        .defaults
        .channels
        .iter()
        .chain(agent.channels.iter())
    {
        let version = catalog_version(&catalog.channels, channel);
        let digest = component_digest("channels", channel, version);
        output.push_str(&format!(
            "  catalog/channels/{channel}@{version}:\n    source: manifests/catalog.yml\n    digest: {digest}\n"
        ));
    }
    for schedule in effective_schedules(agent, manifest) {
        let version = catalog_version(&catalog.schedules, &schedule);
        let digest = component_digest("schedules", &schedule, version);
        output.push_str(&format!(
            "  catalog/schedules/{schedule}@{version}:\n    source: manifests/catalog.yml\n    digest: {digest}\n"
        ));
    }
    for eval in manifest.defaults.evals.iter().chain(agent.evals.iter()) {
        let version = catalog_version(&catalog.evals, eval);
        let digest = component_digest("evals", eval, version);
        output.push_str(&format!(
            "  catalog/evals/{eval}@{version}:\n    source: manifests/catalog.yml\n    digest: {digest}\n"
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
enum UpdateKind {
    Current,
    Patch,
    Minor,
    Major,
    Missing,
    Invalid,
}

#[derive(Debug, Serialize)]
struct VersionReport {
    agent: String,
    component_kind: String,
    component: String,
    requested: String,
    resolved: Option<String>,
    update: UpdateKind,
    affected_evals: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Semver {
    major: u64,
    minor: u64,
    patch: u64,
}

fn collect_version_reports(
    manifest: &FleetManifest,
    catalog: &CatalogManifest,
    agent_filter: Option<&str>,
) -> Vec<VersionReport> {
    let mut reports = Vec::new();

    for agent in &manifest.agents {
        if agent_filter.is_some_and(|filter| filter != agent.name) {
            continue;
        }

        collect_component_reports(
            &mut reports,
            agent,
            "tool",
            &agent.tools,
            &catalog.tools,
            manifest,
        );
        collect_component_reports(
            &mut reports,
            agent,
            "skill",
            &agent.skills,
            &catalog.skills,
            manifest,
        );
        collect_component_reports(
            &mut reports,
            agent,
            "memory",
            &agent.memory,
            &catalog.memory,
            manifest,
        );
        for channel in manifest
            .defaults
            .channels
            .iter()
            .chain(agent.channels.iter())
        {
            collect_named_report(
                &mut reports,
                agent,
                "channel",
                channel,
                "catalog",
                &catalog.channels,
                manifest,
            );
        }
        for schedule in effective_schedules(agent, manifest) {
            collect_named_report(
                &mut reports,
                agent,
                "schedule",
                &schedule,
                "catalog",
                &catalog.schedules,
                manifest,
            );
        }
        for eval in manifest.defaults.evals.iter().chain(agent.evals.iter()) {
            collect_named_report(
                &mut reports,
                agent,
                "eval",
                eval,
                "catalog",
                &catalog.evals,
                manifest,
            );
        }
    }

    reports
}

fn collect_component_reports(
    reports: &mut Vec<VersionReport>,
    agent: &AgentManifest,
    kind: &str,
    components: &ComponentMap,
    catalog: &BTreeMap<String, CatalogComponent>,
    manifest: &FleetManifest,
) {
    for (name, requested) in components {
        collect_named_report(reports, agent, kind, name, requested, catalog, manifest);
    }
}

fn collect_named_report(
    reports: &mut Vec<VersionReport>,
    agent: &AgentManifest,
    kind: &str,
    name: &str,
    requested: &str,
    catalog: &BTreeMap<String, CatalogComponent>,
    manifest: &FleetManifest,
) {
    let resolved = catalog.get(name).map(|component| component.version.clone());
    let update = apply_version_policy(
        classify_update(requested, resolved.as_deref(), catalog.get(name)),
        kind,
        name,
        manifest,
    );
    reports.push(VersionReport {
        agent: agent.name.clone(),
        component_kind: kind.to_string(),
        component: name.to_string(),
        requested: requested.to_string(),
        resolved,
        update,
        affected_evals: effective_evals(agent, manifest),
    });
}

fn apply_version_policy(
    update: UpdateKind,
    kind: &str,
    name: &str,
    manifest: &FleetManifest,
) -> UpdateKind {
    let policy = match kind {
        "approval" => manifest.version_policy.approvals,
        "memory" => manifest.version_policy.memory,
        "tool" => manifest
            .version_policy
            .tools
            .get(name)
            .copied()
            .or(manifest.version_policy.default),
        _ => manifest.version_policy.default,
    };
    let Some(policy) = policy else {
        return update;
    };
    match policy {
        PolicyUpdateKind::Pin if update != UpdateKind::Current => UpdateKind::Major,
        PolicyUpdateKind::Patch | PolicyUpdateKind::PatchAuto
            if matches!(update, UpdateKind::Minor | UpdateKind::Major) =>
        {
            UpdateKind::Major
        }
        PolicyUpdateKind::Minor if update == UpdateKind::Major => UpdateKind::Major,
        _ => update,
    }
}

fn resolve_version(requested: &str, component: Option<&CatalogComponent>) -> Option<String> {
    let component = component?;
    if version_request_matches(requested, &component.version) {
        Some(component.version.clone())
    } else {
        None
    }
}

fn classify_update(
    requested: &str,
    resolved: Option<&str>,
    component: Option<&CatalogComponent>,
) -> UpdateKind {
    let Some(component) = component else {
        return UpdateKind::Missing;
    };
    let Some(resolved) = resolved else {
        return UpdateKind::Invalid;
    };
    if requested == "catalog" {
        return UpdateKind::Current;
    }
    if (requested.starts_with('^') || requested.starts_with('~'))
        && version_request_matches(requested, &component.version)
    {
        return UpdateKind::Current;
    }
    let Some(requested_version) = requested
        .trim_start_matches(['^', '~'])
        .parse::<Semver>()
        .ok()
    else {
        return UpdateKind::Invalid;
    };
    let Some(catalog_version) = component.version.parse::<Semver>().ok() else {
        return UpdateKind::Invalid;
    };

    if requested_version == catalog_version && resolved == component.version {
        UpdateKind::Current
    } else if requested_version.major != catalog_version.major {
        UpdateKind::Major
    } else if requested_version.minor != catalog_version.minor {
        UpdateKind::Minor
    } else if requested_version.patch != catalog_version.patch {
        UpdateKind::Patch
    } else {
        UpdateKind::Current
    }
}

fn version_request_matches(requested: &str, available: &str) -> bool {
    if requested == "catalog" {
        return true;
    }
    if requested == available {
        return true;
    }

    let Some(available) = available.parse::<Semver>().ok() else {
        return false;
    };

    if let Some(base) = requested
        .strip_prefix('^')
        .and_then(|value| value.parse::<Semver>().ok())
    {
        return available.major == base.major && available >= base;
    }
    if let Some(base) = requested
        .strip_prefix('~')
        .and_then(|value| value.parse::<Semver>().ok())
    {
        return available.major == base.major && available.minor == base.minor && available >= base;
    }

    false
}

fn effective_evals(agent: &AgentManifest, manifest: &FleetManifest) -> Vec<String> {
    manifest
        .defaults
        .evals
        .iter()
        .chain(agent.evals.iter())
        .cloned()
        .collect()
}

fn effective_components(
    shared: &[String],
    agent_components: &ComponentMap,
) -> Vec<(String, String)> {
    let mut components = Vec::new();
    for name in shared {
        components.push((name.clone(), "catalog".to_string()));
    }
    for (name, version) in agent_components {
        if let Some((_, existing_version)) =
            components.iter_mut().find(|(existing, _)| existing == name)
        {
            *existing_version = version.clone();
        } else {
            components.push((name.clone(), version.clone()));
        }
    }
    components
}

fn print_version_reports(reports: &[VersionReport]) {
    if reports.is_empty() {
        println!("No components found.");
        return;
    }

    for report in reports {
        if report.update == UpdateKind::Current {
            continue;
        }
        println!(
            "{} {}:{} requested {} resolved {} [{:?}] evals: {}",
            report.agent,
            report.component_kind,
            report.component,
            report.requested,
            report.resolved.as_deref().unwrap_or("<unresolved>"),
            report.update,
            report.affected_evals.join(",")
        );
    }
    if reports
        .iter()
        .all(|report| report.update == UpdateKind::Current)
    {
        println!("All components are current.");
    }
}

impl std::str::FromStr for Semver {
    type Err = ();

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        let mut parts = value.split('.');
        let major = parts.next().ok_or(())?.parse().map_err(|_| ())?;
        let minor = parts.next().ok_or(())?.parse().map_err(|_| ())?;
        let patch = parts.next().ok_or(())?.parse().map_err(|_| ())?;
        if parts.next().is_some() {
            return Err(());
        }
        Ok(Self {
            major,
            minor,
            patch,
        })
    }
}

impl PartialOrd for Semver {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Semver {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.major, self.minor, self.patch).cmp(&(other.major, other.minor, other.patch))
    }
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

fn append_optional_string(output: &mut String, label: &str, value: Option<&str>) {
    if let Some(value) = value.filter(|value| !value.trim().is_empty()) {
        output.push_str(&format!("{label}: {value:?}\n"));
    }
}

fn append_lock_entries(
    output: &mut String,
    kind: &str,
    components: &ComponentMap,
    catalog: &BTreeMap<String, CatalogComponent>,
) {
    for (name, requested) in components {
        let version =
            resolve_version(requested, catalog.get(name)).unwrap_or_else(|| requested.clone());
        let digest = component_digest(kind, name, &version);
        output.push_str(&format!(
            "  catalog/{kind}/{name}@{version}:\n    source: manifests/catalog.yml\n    digest: {digest}\n"
        ));
    }
}

fn component_digest(kind: &str, name: &str, version: &str) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in format!("{kind}:{name}:{version}").bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("fnv64:{hash:016x}")
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

#[derive(Debug, Serialize)]
struct DoctorReport {
    checks: Vec<DoctorCheck>,
}

#[derive(Debug, Serialize)]
struct DoctorCheck {
    name: String,
    status: DoctorStatus,
    message: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
enum DoctorStatus {
    Pass,
    Warn,
    Fail,
}

impl DoctorReport {
    fn has_failures(&self) -> bool {
        self.checks
            .iter()
            .any(|check| check.status == DoctorStatus::Fail)
    }
}

fn run_doctor(
    manifest: &FleetManifest,
    catalog: &CatalogManifest,
    command: &DoctorCommand,
) -> Result<DoctorReport> {
    let mut checks = Vec::new();
    let validation = validate_manifest(manifest, catalog);

    push_check(
        &mut checks,
        "manifest-schema",
        validation.errors.is_empty(),
        "manifest schema and required fields are valid",
        &format!("{} manifest validation error(s)", validation.errors.len()),
    );
    for error in &validation.errors {
        checks.push(DoctorCheck {
            name: "manifest-error".to_string(),
            status: DoctorStatus::Fail,
            message: error.clone(),
        });
    }
    for warning in &validation.warnings {
        checks.push(DoctorCheck {
            name: "manifest-warning".to_string(),
            status: DoctorStatus::Warn,
            message: warning.clone(),
        });
    }

    push_check(
        &mut checks,
        "catalog-references",
        validation
            .errors
            .iter()
            .all(|error| !error.contains("references missing")),
        "all manifest references resolve against catalog",
        "one or more manifest references are missing from catalog",
    );

    push_check(
        &mut checks,
        "approval-coverage",
        validation
            .errors
            .iter()
            .all(|error| !error.contains("risky tool")),
        "risky tools have approval coverage",
        "one or more risky tools lack approval coverage",
    );

    push_check(
        &mut checks,
        "memory-retention",
        validation
            .errors
            .iter()
            .all(|error| !error.contains("must declare retention")),
        "memory schemas declare retention",
        "one or more memory schemas are missing retention",
    );

    if command.templates {
        add_template_checks(&mut checks, manifest, catalog, &command.template_dir)?;
    }
    if command.updates {
        add_update_checks(&mut checks, manifest, catalog, &command.template_dir)?;
    }
    if command.budgets {
        add_budget_checks(&mut checks, manifest);
    }
    add_schedule_safety_checks(&mut checks, manifest, catalog);
    if command.env.is_some() || command.connections {
        add_environment_checks(&mut checks, command)?;
    }
    if command.fix {
        add_fix_plan_checks(&mut checks, manifest, catalog, &command.template_dir)?;
    }

    Ok(DoctorReport { checks })
}

fn add_budget_checks(checks: &mut Vec<DoctorCheck>, manifest: &FleetManifest) {
    let defaults_invalid = manifest
        .defaults
        .cost_budget
        .is_some_and(|value| value <= 0.0)
        || manifest
            .defaults
            .token_budget
            .is_some_and(|value| value == 0)
        || manifest
            .defaults
            .timeout
            .as_deref()
            .is_some_and(str::is_empty);
    let invalid = manifest
        .agents
        .iter()
        .filter(|agent| {
            agent.cost_budget.is_some_and(|value| value <= 0.0)
                || agent.token_budget.is_some_and(|value| value == 0)
                || agent.timeout.as_deref().is_some_and(str::is_empty)
        })
        .map(|agent| agent.name.clone())
        .collect::<Vec<_>>();
    let passed = invalid.is_empty() && !defaults_invalid;
    let message = if defaults_invalid {
        format!(
            "invalid default budget metadata{}",
            if invalid.is_empty() {
                String::new()
            } else {
                format!(" and invalid agent budgets: {}", invalid.join(","))
            }
        )
    } else {
        format!("invalid budget metadata for agents: {}", invalid.join(","))
    };
    push_check(
        checks,
        "budgets-valid",
        passed,
        "agent budgets are valid where configured",
        &message,
    );
}

fn add_schedule_safety_checks(
    checks: &mut Vec<DoctorCheck>,
    manifest: &FleetManifest,
    catalog: &CatalogManifest,
) {
    let mut missing = Vec::new();
    for agent in &manifest.agents {
        for schedule in effective_schedules(agent, manifest) {
            let metadata = catalog.schedules.get(&schedule);
            let has_owner = agent
                .owner
                .as_ref()
                .or(manifest.defaults.owner.as_ref())
                .or_else(|| metadata.and_then(|component| component.owner.as_ref()))
                .is_some();
            let has_auth = agent
                .auth
                .as_ref()
                .or(manifest.defaults.auth.as_ref())
                .or_else(|| metadata.and_then(|component| component.auth.as_ref()))
                .is_some();
            let has_visibility = agent
                .visibility
                .as_ref()
                .or(manifest.defaults.visibility.as_ref())
                .or_else(|| metadata.and_then(|component| component.visibility.as_ref()))
                .is_some();
            if !(has_owner && has_auth && has_visibility) {
                missing.push(format!("{}:{schedule}", agent.name));
            }
        }
    }
    push_check(
        checks,
        "schedule-safety-metadata",
        missing.is_empty(),
        "schedules have owner/auth/visibility metadata",
        &format!(
            "schedules missing owner/auth/visibility metadata: {}",
            missing.join(",")
        ),
    );
}

fn add_environment_checks(checks: &mut Vec<DoctorCheck>, command: &DoctorCommand) -> Result<()> {
    let env_name = command.env.as_deref().unwrap_or("development");
    let environments = load_environments(&command.environments)?;
    let Some(policy) = environments.environments.get(env_name) else {
        push_check(
            checks,
            "environment-defined",
            false,
            "environment exists",
            &format!("environment '{env_name}' is not defined"),
        );
        return Ok(());
    };
    push_check(
        checks,
        "environment-defined",
        true,
        "environment exists",
        "environment is missing",
    );
    let missing_env = policy
        .required_env
        .iter()
        .chain(policy.required_secrets.iter())
        .filter(|key| env::var_os(key).is_none())
        .cloned()
        .collect::<Vec<_>>();
    push_check(
        checks,
        "environment-variables",
        missing_env.is_empty(),
        "required env vars and secrets are present",
        &format!("missing env vars/secrets: {}", missing_env.join(",")),
    );
    if command.connections {
        push_check(
            checks,
            "connections-configured",
            policy.required_connections.is_empty(),
            "required connections are configured or none are required",
            &format!(
                "manual connection verification required: {}",
                policy.required_connections.join(",")
            ),
        );
    }
    if env_name == "production" {
        push_check(
            checks,
            "observability",
            policy.observability.unwrap_or(false),
            "production observability is enabled",
            "production environment must enable observability",
        );
    }
    Ok(())
}

fn add_fix_plan_checks(
    checks: &mut Vec<DoctorCheck>,
    manifest: &FleetManifest,
    catalog: &CatalogManifest,
    template_dir: &Path,
) -> Result<()> {
    let plan = batch_plan(manifest, catalog, template_dir)?;
    let repairs = plan
        .operations
        .iter()
        .filter(|operation| operation.action != BatchAction::Skip)
        .count();
    push_check(
        checks,
        "fix-plan",
        true,
        &format!("doctor --fix would apply {repairs} safe generated-file repair(s)"),
        "doctor --fix plan failed",
    );
    Ok(())
}

fn add_template_checks(
    checks: &mut Vec<DoctorCheck>,
    manifest: &FleetManifest,
    catalog: &CatalogManifest,
    template_dir: &Path,
) -> Result<()> {
    let plan = batch_plan(manifest, catalog, template_dir)?;
    let stale = plan
        .operations
        .iter()
        .filter(|operation| operation.action != BatchAction::Skip)
        .count();

    push_check(
        checks,
        "templates-render",
        true,
        "templates render without undefined variables",
        "templates failed to render",
    );
    push_check(
        checks,
        "generated-output-fresh",
        stale == 0,
        "generated files are fresh",
        &format!("{stale} generated file(s) are stale or missing"),
    );
    push_check(
        checks,
        "generated-metadata",
        generated_metadata_present(&plan),
        "generated files include metadata headers",
        "one or more generated files are missing metadata headers",
    );

    Ok(())
}

fn add_update_checks(
    checks: &mut Vec<DoctorCheck>,
    manifest: &FleetManifest,
    catalog: &CatalogManifest,
    template_dir: &Path,
) -> Result<()> {
    let plan = batch_plan(manifest, catalog, template_dir)?;
    let lockfiles = plan
        .operations
        .iter()
        .filter(|operation| operation.path.ends_with("versions.lock"))
        .collect::<Vec<_>>();
    let stale_lockfiles = lockfiles
        .iter()
        .filter(|operation| operation.action != BatchAction::Skip)
        .count();

    push_check(
        checks,
        "lockfiles-present",
        !lockfiles.is_empty()
            && lockfiles.iter().all(|operation| {
                operation.path.exists() || operation.action != BatchAction::Create
            }),
        "agent lockfiles are present",
        "one or more agent lockfiles are missing",
    );
    push_check(
        checks,
        "lockfiles-current",
        stale_lockfiles == 0,
        "agent lockfiles match rendered component versions",
        &format!("{stale_lockfiles} lockfile(s) are stale"),
    );

    Ok(())
}

fn generated_metadata_present(plan: &BatchPlan) -> bool {
    plan.operations.iter().all(|operation| {
        operation
            .content
            .starts_with("# Generated by eve-rails. Do not edit generated regions.")
            || operation
                .content
                .starts_with("// Generated by eve-rails. Do not edit generated regions.")
            || operation.content.contains(r#""x-eve-rails": "generated""#)
            || operation.content.contains(r#""kind": "#)
    })
}

fn push_check(
    checks: &mut Vec<DoctorCheck>,
    name: &str,
    passed: bool,
    pass_message: &str,
    fail_message: &str,
) {
    checks.push(DoctorCheck {
        name: name.to_string(),
        status: if passed {
            DoctorStatus::Pass
        } else {
            DoctorStatus::Fail
        },
        message: if passed {
            pass_message.to_string()
        } else {
            fail_message.to_string()
        },
    });
}

fn print_doctor_report(report: &DoctorReport) {
    println!();
    for check in &report.checks {
        println!(
            "[{}] {} - {}",
            doctor_status_label(check.status),
            check.name,
            check.message
        );
    }
}

fn doctor_status_label(status: DoctorStatus) -> &'static str {
    match status {
        DoctorStatus::Pass => "pass",
        DoctorStatus::Warn => "warn",
        DoctorStatus::Fail => "fail",
    }
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
    validate_named_list(
        &mut report,
        "shared",
        "schedule",
        &manifest.shared.schedules,
        &catalog.schedules,
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
            "schedule",
            &manifest.defaults.schedules,
            &catalog.schedules,
        );
        validate_named_list(
            &mut report,
            &format!("agent '{}'", agent.name),
            "schedule",
            &agent.schedules,
            &catalog.schedules,
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

    if !version_request_matches(requested_version, &component.version) {
        report.errors.push(format!(
            "{scope} {kind} '{name}' requests version {requested_version}, which does not resolve to catalog version {}",
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

fn effective_schedules(agent: &AgentManifest, manifest: &FleetManifest) -> Vec<String> {
    let mut schedules = Vec::new();
    for schedule in manifest
        .defaults
        .schedules
        .iter()
        .chain(manifest.shared.schedules.iter())
        .chain(agent.schedules.iter())
    {
        if !schedules.contains(schedule) {
            schedules.push(schedule.clone());
        }
    }
    schedules
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
schedules:
  weekday_triage:
    version: 1.0.0
    schedule: "0 9 * * 1-5"
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
  schedules: [weekday_triage]
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
                .contains("skill 'handle_refund' requests version 2.0.0, which does not resolve to catalog version 1.0.0")),
            "{:?}",
            report.errors
        );
    }

    #[test]
    fn version_ranges_resolve_deterministically() {
        assert!(version_request_matches("^1.0.0", "1.2.3"));
        assert!(version_request_matches("~1.2.0", "1.2.3"));
        assert!(!version_request_matches("^1.0.0", "2.0.0"));
        assert!(!version_request_matches("~1.2.0", "1.3.0"));
    }

    #[test]
    fn outdated_reports_minor_updates_and_affected_evals() {
        let mut manifest = valid_manifest();
        manifest.agents[0]
            .tools
            .insert("search_customers".to_string(), "1.0.0".to_string());
        let mut catalog = valid_catalog();
        catalog
            .tools
            .get_mut("search_customers")
            .expect("tool")
            .version = "1.1.0".to_string();

        let reports = collect_version_reports(&manifest, &catalog, Some("billing"));

        assert!(reports.iter().any(|report| {
            report.component == "search_customers"
                && report.update == UpdateKind::Minor
                && report.affected_evals == vec!["standard".to_string()]
        }));
    }

    #[test]
    fn lockfile_records_resolved_versions_and_digests() {
        let mut manifest = valid_manifest();
        manifest.agents[0]
            .tools
            .insert("search_customers".to_string(), "^1.0.0".to_string());
        let lockfile = render_versions_lock(&manifest.agents[0], &manifest, &valid_catalog());

        assert!(lockfile.contains("catalog/tools/search_customers@1.0.0"));
        assert!(lockfile.contains("digest: fnv64:"));
    }

    #[test]
    fn skill_patch_can_hotload() {
        let component = ComponentRef::parse("skill:handle_refund@1.0.1").expect("component");
        let classification = classify_hotload(&component, "1.0.0").expect("classification");

        assert_eq!(classification.action, HotloadAction::Hotload);
        assert_eq!(classification.update, UpdateKind::Patch);
    }

    #[test]
    fn tool_patch_requires_redeploy() {
        let component = ComponentRef::parse("tool:prepare_refund@1.0.1").expect("component");
        let classification = classify_hotload(&component, "1.0.0").expect("classification");

        assert_eq!(classification.action, HotloadAction::Redeploy);
    }

    #[test]
    fn memory_change_requires_migration() {
        let component = ComponentRef::parse("memory:customer_profile@1.0.1").expect("component");
        let classification = classify_hotload(&component, "1.0.0").expect("classification");

        assert_eq!(classification.action, HotloadAction::Migration);
    }

    #[test]
    fn approval_major_update_cannot_hotload() {
        let component = ComponentRef::parse("approval:money_movement@2.0.0").expect("component");
        let classification = classify_hotload(&component, "1.0.0").expect("classification");

        assert_eq!(classification.action, HotloadAction::Redeploy);
        assert_eq!(classification.update, UpdateKind::Major);
    }

    #[test]
    fn migration_generator_plans_timestamped_file() {
        let command = generate_command(
            "add_customer_tier",
            PathBuf::from("manifests/agents.yml"),
            PathBuf::from("manifests/catalog.yml"),
        );
        let changes = plan_generate(GeneratorKind::Migration, &command).expect("plan");

        assert_eq!(changes.len(), 1);
        assert!(
            changes[0]
                .path
                .to_string_lossy()
                .contains("add_customer_tier")
        );
        assert!(changes[0].content.contains("export async function up()"));
    }

    #[test]
    fn deploy_preflight_fails_when_doctor_gate_fails() {
        let mut manifest = valid_manifest();
        manifest.agents[0].approvals.clear();
        let report =
            deploy_preflight(&deploy_command(true, false), &manifest, &valid_catalog()).unwrap();

        assert!(
            report
                .gates
                .iter()
                .any(|gate| gate.name == "doctor" && !gate.passed)
        );
    }

    #[test]
    fn deploy_preflight_fails_when_eval_gate_fails() {
        let mut manifest = valid_manifest();
        manifest.defaults.evals.clear();
        manifest.agents[0].evals.clear();
        let report =
            deploy_preflight(&deploy_command(false, true), &manifest, &valid_catalog()).unwrap();

        assert!(
            report
                .gates
                .iter()
                .any(|gate| gate.name == "evals" && !gate.passed)
        );
    }

    #[test]
    fn rollback_restores_manifest_and_lockfile() {
        let manifest = valid_manifest();
        let report = rollback_plan(
            &rollback_command(Some("1.0.0"), None),
            &manifest,
            &valid_catalog(),
        )
        .expect("rollback");

        assert!(!report.blocked);
        assert!(
            report
                .operations
                .iter()
                .any(|operation| operation.path.ends_with("agent.manifest.yml"))
        );
        assert!(
            report
                .operations
                .iter()
                .any(|operation| operation.path.ends_with("versions.lock"))
        );
    }

    #[test]
    fn memory_component_rollback_requires_migration() {
        let manifest = valid_manifest();
        let report = rollback_plan(
            &rollback_command(None, Some("memory:customer_profile@1.0.0")),
            &manifest,
            &valid_catalog(),
        )
        .expect("rollback");

        assert!(report.blocked);
        assert!(
            report
                .reason
                .as_deref()
                .is_some_and(|reason| reason.contains("memory rollback"))
        );
    }

    #[test]
    fn inspect_summary_includes_effective_runtime_state() {
        let manifest = valid_manifest();
        let summary = summarize_agent(
            &manifest.agents[0],
            &manifest,
            &valid_catalog(),
            Path::new("templates/agent"),
        )
        .expect("summary");

        assert_eq!(summary.name, "billing");
        assert_eq!(summary.channels, vec!["web".to_string()]);
        assert_eq!(summary.evals, vec!["standard".to_string()]);
        assert_eq!(
            summary.tools.get("prepare_refund").map(String::as_str),
            Some("1.0.0")
        );
        assert!(summary.generated.lockfile_path.ends_with("versions.lock"));
    }

    #[test]
    fn format_components_is_dashboard_friendly() {
        let mut components = BTreeMap::new();
        components.insert("search_customers".to_string(), "1.0.0".to_string());

        assert_eq!(format_components(&components), "search_customers@1.0.0");
    }

    fn deploy_command(require_doctor: bool, require_evals: bool) -> DeployCommand {
        DeployCommand {
            agent: Some("billing".to_string()),
            fleet: None,
            env: "staging".to_string(),
            require_evals,
            require_doctor,
            require_approvals: false,
            canary: None,
            promote: false,
            rollback_to: None,
            dry_run: true,
            manifest: PathBuf::from("manifests/agents.yml"),
            catalog: PathBuf::from("manifests/catalog.yml"),
            template_dir: PathBuf::from("templates/agent"),
            json: false,
        }
    }

    fn rollback_command(to: Option<&str>, component: Option<&str>) -> RollbackCommand {
        RollbackCommand {
            agent: "billing".to_string(),
            to: to.map(str::to_string),
            component: component.map(str::to_string),
            manifest: PathBuf::from("manifests/agents.yml"),
            catalog: PathBuf::from("manifests/catalog.yml"),
            template_dir: PathBuf::from("templates/agent"),
            json: false,
        }
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

        assert_eq!(files.len(), 17);
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
        assert!(
            files
                .iter()
                .any(|file| file.path.ends_with("schedules/weekday_triage.ts")
                    && file.content.contains("0 9 * * 1-5"))
        );
        assert!(
            files
                .iter()
                .any(|file| file.path.ends_with("tools/prepare_refund.ts"))
        );
        assert!(
            files
                .iter()
                .any(|file| file.path.ends_with("memory/customer_profile.ts"))
        );
        assert!(
            files
                .iter()
                .any(|file| file.path.ends_with("evals/standard.eval.ts"))
        );
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
        fs::write(dir.join("tool.ts.j2"), "export default {};").expect("template");
        fs::write(dir.join("skill.md.j2"), "# skill").expect("template");
        fs::write(dir.join("schedule.ts.j2"), "export default {};").expect("template");
        fs::write(dir.join("approval.ts.j2"), "export default {};").expect("template");
        fs::write(dir.join("eval.ts.j2"), "export default {};").expect("template");
        fs::write(dir.join("memory.ts.j2"), "export default {};").expect("template");
        fs::write(dir.join("fixture.json.j2"), "{}").expect("template");
        fs::write(dir.join("agent.README.md.j2"), "# agent").expect("template");

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
    fn batch_action_classifies_create_update_and_skip() {
        let dir = temp_dir("batch-action");
        fs::create_dir_all(&dir).expect("temp dir");
        let missing = dir.join("missing.txt");
        let existing = dir.join("existing.txt");
        fs::write(&existing, "old").expect("write");

        assert_eq!(
            batch_action(&missing, "new").expect("create"),
            BatchAction::Create
        );
        assert_eq!(
            batch_action(&existing, "new").expect("update"),
            BatchAction::Update
        );
        assert_eq!(
            batch_action(&existing, "old").expect("skip"),
            BatchAction::Skip
        );
    }

    #[test]
    fn ten_agent_fixture_plans_expected_files() {
        let manifest = load_manifest(Path::new(
            "examples/basic-fleet/fixtures/batch-10-agents.yml",
        ))
        .expect("manifest");
        let catalog =
            load_catalog(Path::new("examples/basic-fleet/manifests/catalog.yml")).expect("catalog");
        let report = validate_manifest(&manifest, &catalog);
        assert!(report.errors.is_empty(), "{:?}", report.errors);

        let plan =
            batch_plan(&manifest, &catalog, Path::new("templates/agent")).expect("batch plan");

        assert_eq!(manifest.agents.len(), 10);
        assert_eq!(plan.operations.len(), 217);
    }

    #[test]
    fn doctor_reports_risky_tool_failure() {
        let mut manifest = valid_manifest();
        manifest.agents[0].approvals.clear();
        let report =
            run_doctor(&manifest, &valid_catalog(), &doctor_command(false, false)).expect("doctor");

        assert!(report.has_failures());
        assert!(report.checks.iter().any(|check| {
            check.name == "approval-coverage" && check.status == DoctorStatus::Fail
        }));
    }

    #[test]
    fn doctor_templates_fail_when_generated_files_are_missing() {
        let manifest = manifest(
            r#"
defaults:
  model: openai/gpt-5.5
  owner: agent-platform
  channels: [web]
  evals: [standard]
agents:
  - name: doctor_missing_agent
    version: 1.0.0
    responsibility: Exercise template doctor checks.
    tools:
      search_customers: 1.0.0
    skills:
      handle_refund: 1.0.0
    memory:
      customer_profile: 1.0.0
"#,
        );
        let report =
            run_doctor(&manifest, &valid_catalog(), &doctor_command(true, false)).expect("doctor");

        assert!(report.has_failures());
        assert!(report.checks.iter().any(|check| {
            check.name == "generated-output-fresh" && check.status == DoctorStatus::Fail
        }));
    }

    #[test]
    fn doctor_updates_fail_when_lockfiles_are_missing() {
        let manifest = manifest(
            r#"
defaults:
  model: openai/gpt-5.5
  owner: agent-platform
  channels: [web]
  evals: [standard]
agents:
  - name: doctor_missing_lockfile
    version: 1.0.0
    responsibility: Exercise update doctor checks.
    tools:
      search_customers: 1.0.0
    skills:
      handle_refund: 1.0.0
    memory:
      customer_profile: 1.0.0
"#,
        );
        let report =
            run_doctor(&manifest, &valid_catalog(), &doctor_command(false, true)).expect("doctor");

        assert!(report.has_failures());
        assert!(
            report.checks.iter().any(
                |check| check.name == "lockfiles-present" && check.status == DoctorStatus::Fail
            )
        );
    }

    fn doctor_command(templates: bool, updates: bool) -> DoctorCommand {
        DoctorCommand {
            all: true,
            updates,
            templates,
            fix: false,
            env: None,
            connections: false,
            budgets: true,
            json: false,
            manifest: PathBuf::from("manifests/agents.yml"),
            catalog: PathBuf::from("manifests/catalog.yml"),
            template_dir: PathBuf::from("templates/agent"),
            environments: PathBuf::from("manifests/environments.yml"),
        }
    }

    #[test]
    fn generate_agent_plans_manifest_update() {
        let dir = temp_dir("generate-agent");
        fs::create_dir_all(&dir).expect("temp dir");
        let manifest_path = dir.join("agents.yml");
        fs::write(&manifest_path, default_manifest_yaml()).expect("manifest");

        let mut command =
            generate_command("support", manifest_path.clone(), dir.join("catalog.yml"));
        command.with_tools = vec!["search_customers".to_string()];
        command.with_channels = vec!["web".to_string()];
        command.with_schedules = vec!["weekday_triage".to_string()];
        command.approval = Some("on-risk".to_string());
        command.risk = Some("medium".to_string());
        command.auth = Some("oauth".to_string());
        command.visibility = Some("team".to_string());
        command.cost_budget = Some(10.0);
        command.token_budget = Some(200_000);
        command.timeout = Some("10m".to_string());
        let changes = plan_generate(GeneratorKind::Agent, &command).expect("plan");

        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].path, manifest_path);
        assert!(changes[0].content.contains("- name: support"));
        assert!(changes[0].content.contains("search_customers: 1.0.0"));
        assert!(changes[0].content.contains("schedules: [weekday_triage]"));
        assert!(changes[0].content.contains("search_customers: on-risk"));
        assert!(changes[0].content.contains("risk: \"medium\""));
        assert!(changes[0].content.contains("token_budget: 200000"));
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
    fn generate_schedule_plans_catalog_and_stub() {
        let dir = temp_dir("generate-schedule");
        fs::create_dir_all(&dir).expect("temp dir");
        let catalog_path = dir.join("catalog.yml");
        fs::write(&catalog_path, default_catalog_yaml()).expect("catalog");

        let mut command = generate_command("weekday_triage", dir.join("agents.yml"), catalog_path);
        command.schedule = Some("0 9 * * 1-5".to_string());
        let changes = plan_generate(GeneratorKind::Schedule, &command).expect("plan");

        assert_eq!(changes.len(), 2);
        assert!(changes[0].content.contains("weekday_triage:"));
        assert!(changes[0].content.contains("schedule: \"0 9 * * 1-5\""));
        assert_eq!(
            changes[1].path,
            PathBuf::from("catalog/schedules/weekday_triage.ts")
        );
        assert!(changes[1].content.contains("0 9 * * 1-5"));
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
            with_tools: Vec::new(),
            with_skills: Vec::new(),
            with_subagents: Vec::new(),
            with_channels: Vec::new(),
            with_evals: Vec::new(),
            with_memory: Vec::new(),
            with_schedules: Vec::new(),
            approval: None,
            schedule: None,
            risk: None,
            auth: None,
            visibility: None,
            cost_budget: None,
            token_budget: None,
            timeout: None,
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
