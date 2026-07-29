use super::*;

pub(super) struct Renderer<'source> {
    env: Environment<'source>,
}

pub(super) struct RenderedFile {
    pub(super) path: PathBuf,
    pub(super) content: String,
}

#[derive(Debug)]
pub(super) struct BatchPlan {
    pub(super) operations: Vec<BatchOperation>,
}

#[derive(Debug)]
pub(super) struct BatchOperation {
    pub(super) path: PathBuf,
    pub(super) action: BatchAction,
    pub(super) content: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(super) enum BatchAction {
    Create,
    Update,
    Skip,
}

#[derive(Debug, Serialize)]
pub(super) struct BatchOperationReport {
    pub(super) path: PathBuf,
    pub(super) action: BatchAction,
}

#[derive(Debug, Serialize)]
pub(super) struct BatchSummary {
    pub(super) create: usize,
    pub(super) update: usize,
    pub(super) skip: usize,
    pub(super) total: usize,
}

impl BatchPlan {
    pub(super) fn summary(&self) -> BatchSummary {
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

    pub(super) fn report(&self) -> Vec<BatchOperationReport> {
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

pub(super) fn batch_plan(
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

pub(super) fn batch_action(path: &Path, expected: &str) -> Result<BatchAction> {
    match fs::read_to_string(path) {
        Ok(actual) if actual == expected => Ok(BatchAction::Skip),
        Ok(_) => Ok(BatchAction::Update),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(BatchAction::Create),
        Err(error) => Err(error).with_context(|| format!("failed to read '{}'", path.display())),
    }
}

pub(super) fn print_batch_summary(batch: &BatchPlan) {
    let summary = batch.summary();
    println!(
        "Batch: {} create, {} update, {} skip, {} total files",
        summary.create, summary.update, summary.skip, summary.total
    );
}

impl<'source> Renderer<'source> {
    pub(super) fn load(template_dir: &Path) -> Result<Self> {
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

    pub(super) fn render_agent(
        &self,
        agent: &AgentManifest,
        manifest: &FleetManifest,
        catalog: &CatalogManifest,
    ) -> Result<Vec<RenderedFile>> {
        let app_root = PathBuf::from("agents").join(&agent.name);
        let output_root = app_root.join("agent");
        let rails_root = app_root.join(".eve-rails");
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
        let auth = agent
            .auth
            .as_deref()
            .or(manifest.defaults.auth.as_deref())
            .unwrap_or("platform-oauth");
        let version = agent.version.as_deref().unwrap_or("1.0.0");
        let agent_context = context! {
            name => agent.name.as_str(),
            version => version,
            owner => owner,
            responsibility => agent.responsibility.as_str(),
            model => model,
            topology => agent.x_topology.clone().unwrap_or_default(),
            runtime_policy => agent.x_runtime_policy.clone().unwrap_or_default(),
            subagents => subagent_names(agent),
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
                path: PathBuf::from("agents")
                    .join(&agent.name)
                    .join("evals")
                    .join("evals.config.ts"),
                content: render_evals_config(),
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
                content: render_eve_channel(auth),
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

        for channel in effective_string_list(&manifest.defaults.channels, &agent.channels) {
            let Some(component) = catalog.channels.get(&channel) else {
                continue;
            };
            let Some(kind) = component.kind.as_deref() else {
                continue;
            };
            if kind == "eve" {
                continue;
            }
            let filename = channel.as_str();
            files.push(RenderedFile {
                path: output_root.join("channels").join(format!("{filename}.ts")),
                content: render_platform_channel(&channel, component, auth),
            });
        }

        for (tool, version) in effective_components(&manifest.shared.tools, &agent.tools) {
            let component = catalog.tools.get(&tool);
            let description = component
                .map(|component| tool_description(&tool, component))
                .unwrap_or_else(|| format!("{tool} generated tool contract."));
            let side_effects = component
                .and_then(|component| component.side_effects.as_ref())
                .map(ToString::to_string)
                .unwrap_or_else(|| "read".to_string());
            let tool_context = context! {
                name_literal => ts_string_literal(&tool),
                version_literal => ts_string_literal(&version),
                description_literal => ts_string_literal(&format!("{description} Side effects: {side_effects}.")),
                side_effects_literal => ts_string_literal(&side_effects),
                required_approvals_literal => ts_string_array_literal(component.map(|component| component.required_approvals.as_slice()).unwrap_or(&[])),
                required_env_literal => ts_string_array_literal(component.map(|component| component.required_env.as_slice()).unwrap_or(&[])),
                required_connectors_literal => ts_string_array_literal(component.map(|component| component.required_connectors.as_slice()).unwrap_or(&[])),
                sandbox_compatibility_literal => ts_string_array_literal(component.map(|component| component.sandbox_compatibility.as_slice()).unwrap_or(&[])),
                failure_modes_literal => ts_string_array_literal(component.map(|component| component.failure_modes.as_slice()).unwrap_or(&[])),
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
            let role = subagent_role(agent, subagent);
            let subagent_name = subagent.name.as_str();
            let subagent_model = subagent.model.as_deref().unwrap_or(model);
            files.push(RenderedFile {
                path: output_root
                    .join("subagents")
                    .join(subagent_name)
                    .join("instructions.md"),
                content: render_subagent_placeholder(&agent.name, subagent, role.as_ref()),
            });
            files.push(RenderedFile {
                path: output_root
                    .join("subagents")
                    .join(subagent_name)
                    .join("agent.ts"),
                content: render_subagent_agent_ts(
                    &agent.name,
                    subagent,
                    subagent_model,
                    role.as_ref(),
                ),
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
                path: rails_root.join("approvals").join(format!("{approval}.ts")),
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
                path: rails_root
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
                path: rails_root.join("memory").join(format!("{memory}.ts")),
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
            path: rails_root
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

pub(super) fn render_package_json(agent_name: &str) -> String {
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
    "just-bash": "^3.1.0",
    "typescript": "7.0.2",
    "vercel": "^56.2.1"
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

pub(super) fn render_tsconfig_json() -> String {
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

pub(super) fn render_eve_channel(auth: &str) -> String {
    match auth {
        "http-basic-env" | "basic-env" => {
            r#"// Generated by eve-rails. Do not edit generated regions.
// template: eve-channel

import { eveChannel } from "eve/channels/eve";
import { httpBasic, localDev, vercelOidc } from "eve/channels/auth";

function envOrUnmatchable(name: string): string {
  const value = process.env[name]?.trim();
  return value && value.length > 0 ? value : `__EVE_RAILS_MISSING_${name}__`;
}

export default eveChannel({
  auth: [
    vercelOidc(),
    localDev(),
    httpBasic({
      username: envOrUnmatchable("EVE_RAILS_BASIC_AUTH_USERNAME"),
      password: envOrUnmatchable("EVE_RAILS_BASIC_AUTH_PASSWORD"),
    }),
  ],
});
"#
            .to_string()
        }
        _ => r#"// Generated by eve-rails. Do not edit generated regions.
// template: eve-channel

import { eveChannel } from "eve/channels/eve";
import { localDev, vercelOidc } from "eve/channels/auth";

export default eveChannel({
  auth: [
    vercelOidc(),
    localDev(),
  ],
});
"#
        .to_string(),
    }
}

pub(super) fn render_platform_channel(
    name: &str,
    component: &CatalogComponent,
    auth: &str,
) -> String {
    match component.kind.as_deref().unwrap_or(name) {
        "eve" => render_eve_channel(auth),
        "twilio" => render_twilio_channel(component),
        "slack" => render_slack_channel(component),
        "discord" => render_discord_channel(),
        "telegram" => render_telegram_channel(component),
        "linear" => render_linear_channel(component),
        "github" => render_github_channel(component),
        "teams" => render_teams_channel(),
        _ => render_custom_channel_placeholder(name),
    }
}

fn render_twilio_channel(component: &CatalogComponent) -> String {
    let allow_from = ts_config_value(
        component
            .allow_from
            .as_deref()
            .unwrap_or("env:TWILIO_ALLOWED_FROM"),
    );
    let messaging_from = ts_config_value(
        component
            .messaging_from
            .as_deref()
            .unwrap_or("env:TWILIO_FROM_NUMBER"),
    );
    format!(
        r#"// Generated by eve-rails. Do not edit generated regions.
// template: channel.twilio

import {{ twilioChannel }} from "eve/channels/twilio";

export default twilioChannel({{
  allowFrom: {allow_from},
  messaging: {{ from: {messaging_from} }},
}});
"#
    )
}

fn render_slack_channel(component: &CatalogComponent) -> String {
    let connect_uid =
        escape_ts_string(component.connect_uid.as_deref().unwrap_or("slack/my-agent"));
    format!(
        r#"// Generated by eve-rails. Do not edit generated regions.
// template: channel.slack

import {{ connectSlackCredentials }} from "@vercel/connect/eve";
import {{ slackChannel }} from "eve/channels/slack";

export default slackChannel({{
  credentials: connectSlackCredentials("{connect_uid}"),
}});
"#
    )
}

fn render_discord_channel() -> String {
    r#"// Generated by eve-rails. Do not edit generated regions.
// template: channel.discord

import { discordChannel } from "eve/channels/discord";

export default discordChannel();
"#
    .to_string()
}

fn render_telegram_channel(component: &CatalogComponent) -> String {
    let bot_username = escape_ts_string(component.bot_username.as_deref().unwrap_or("my_bot"));
    format!(
        r#"// Generated by eve-rails. Do not edit generated regions.
// template: channel.telegram

import {{ telegramChannel }} from "eve/channels/telegram";

export default telegramChannel({{
  botUsername: "{bot_username}",
}});
"#
    )
}

fn render_linear_channel(component: &CatalogComponent) -> String {
    let connect_uid = escape_ts_string(
        component
            .connect_uid
            .as_deref()
            .unwrap_or("linear/my-agent"),
    );
    format!(
        r#"// Generated by eve-rails. Do not edit generated regions.
// template: channel.linear

import {{ connectLinearCredentials }} from "@vercel/connect/eve";
import {{ linearChannel }} from "eve/channels/linear";

export default linearChannel({{
  credentials: connectLinearCredentials("{connect_uid}"),
}});
"#
    )
}

fn render_github_channel(component: &CatalogComponent) -> String {
    let connect_uid = escape_ts_string(
        component
            .connect_uid
            .as_deref()
            .unwrap_or("github/my-agent"),
    );
    let bot_name = escape_ts_string(component.bot_name.as_deref().unwrap_or("my-agent"));
    format!(
        r#"// Generated by eve-rails. Do not edit generated regions.
// template: channel.github

import {{ connectGitHubCredentials }} from "@vercel/connect/eve";
import {{ githubChannel }} from "eve/channels/github";

export default githubChannel({{
  botName: "{bot_name}",
  credentials: connectGitHubCredentials("{connect_uid}"),
}});
"#
    )
}

fn render_teams_channel() -> String {
    r#"// Generated by eve-rails. Do not edit generated regions.
// template: channel.teams

import { teamsChannel } from "eve/channels/teams";

export default teamsChannel();
"#
    .to_string()
}

fn render_custom_channel_placeholder(name: &str) -> String {
    format!(
        r#"// Generated by eve-rails. Do not edit generated regions.
// template: channel.custom

export default {{
  name: "{}",
}};
"#,
        escape_ts_string(name)
    )
}

fn ts_config_value(value: &str) -> String {
    if let Some(env_name) = value.strip_prefix("env:") {
        format!("process.env.{}!", env_name.trim())
    } else if value == "*" {
        "\"*\"".to_string()
    } else if value.contains(',') {
        format!(
            "[{}]",
            value
                .split(',')
                .map(|item| format!("\"{}\"", escape_ts_string(item.trim())))
                .collect::<Vec<_>>()
                .join(", ")
        )
    } else {
        format!("\"{}\"", escape_ts_string(value))
    }
}

fn render_evals_config() -> String {
    r#"// Generated by eve-rails. Do not edit generated regions.
// template: evals.config

import { defineEvalConfig } from "eve/evals";

export default defineEvalConfig({});
"#
    .to_string()
}

pub(super) fn tool_description(tool: &str, component: &CatalogComponent) -> String {
    component
        .description
        .clone()
        .unwrap_or_else(|| match &component.side_effects {
            Some(side_effects) => format!("Generated {tool} {side_effects} tool contract."),
            None => format!("Generated {tool} tool contract."),
        })
}

pub(super) fn subagent_names(agent: &AgentManifest) -> Vec<String> {
    agent
        .subagents
        .iter()
        .map(|subagent| subagent.name.clone())
        .collect()
}

pub(super) fn subagent_role(
    agent: &AgentManifest,
    subagent: &SubagentManifest,
) -> Option<RoleTopology> {
    if subagent.title.is_some()
        || subagent.role_id.is_some()
        || subagent.responsibility.is_some()
        || subagent.runtime_policy.is_some()
    {
        return Some(RoleTopology {
            name: subagent.name.clone(),
            title: subagent
                .title
                .clone()
                .unwrap_or_else(|| subagent.name.clone()),
            role_id: subagent
                .role_id
                .clone()
                .unwrap_or_else(|| format!("subagent:{}", subagent.name)),
            responsibility: subagent.responsibility.clone().unwrap_or_else(|| {
                "Execute delegated work within this subagent's bounded context.".to_string()
            }),
            runtime_policy: subagent.runtime_policy.clone(),
        });
    }
    let topology = agent.x_topology.as_ref()?;
    topology
        .principals
        .iter()
        .chain(topology.delegates.iter())
        .find(|role| role.name == subagent.name)
        .cloned()
}

pub(super) fn render_subagent_placeholder(
    agent: &str,
    subagent: &SubagentManifest,
    role: Option<&RoleTopology>,
) -> String {
    let subagent_name = subagent.name.as_str();
    let title = role
        .map(|role| role.title.as_str())
        .unwrap_or(subagent_name);
    let role_id = role
        .map(|role| role.role_id.as_str())
        .unwrap_or("<unknown>");
    let responsibility = role
        .map(|role| role.responsibility.as_str())
        .unwrap_or("Execute delegated work within this subagent's bounded context.");
    let policy = role.and_then(|role| role.runtime_policy.as_ref());
    let sandbox = policy
        .and_then(|policy| policy.sandbox.as_deref())
        .unwrap_or("read-only");
    let allowed_tools = policy
        .map(|policy| policy.allowed_tools.as_slice())
        .unwrap_or(&[]);
    let approvals = policy
        .map(|policy| policy.approvals.as_slice())
        .unwrap_or(&[]);
    let forbidden_actions = policy
        .map(|policy| policy.forbidden_actions.as_slice())
        .unwrap_or(&[]);

    with_generated_header(
        CommentStyle::Hash,
        agent,
        "subagent-placeholder",
        &format!(
            "# {title}\n\n## Role\n\n- Slug: `{subagent_name}`\n- Role id: `{role_id}`\n- Parent agent: `{agent}`\n\n## Responsibility\n\n{responsibility}\n\n## Runtime Boundary\n\n- Sandbox: `{sandbox}`\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n\n## Operating Rules\n\n- Accept delegated work only when it fits this role boundary.\n- Return concise findings, artifacts, decisions, and open risks to the parent agent.\n- Ask the parent agent to escalate when the task requires approval, credentials, production impact, legal judgment, security judgment, or customer-facing commitments.\n- Keep local assumptions explicit so the parent agent can review or re-delegate.\n",
            markdown_list("Allowed tools", allowed_tools),
            markdown_list("Approval gates", approvals),
            markdown_list("Forbidden actions", forbidden_actions),
            markdown_list("Declared tools", &subagent.tools),
            markdown_list("Declared skills", &subagent.skills),
            markdown_list("Declared memory", &subagent.memory),
            markdown_list("Declared channels", &subagent.channels)
        ),
    )
}

pub(super) fn render_subagent_agent_ts(
    agent: &str,
    subagent: &SubagentManifest,
    model: &str,
    role: Option<&RoleTopology>,
) -> String {
    let title = role
        .map(|role| role.title.as_str())
        .unwrap_or(subagent.name.as_str());
    let responsibility = role
        .map(|role| role.responsibility.as_str())
        .unwrap_or("Generated role-aware subagent skeleton.");
    with_generated_header(
        CommentStyle::Slash,
        agent,
        "subagent-agent",
        &format!(
            "import {{ defineAgent }} from \"eve\";\n\nexport default defineAgent({{\n  description: \"{}: {}\",\n  model: \"{}\",\n}});\n",
            escape_ts_string(title),
            escape_ts_string(responsibility),
            escape_ts_string(model)
        ),
    )
}

pub(super) fn render_contract_json(kind: &str, name: &str, description: &str) -> String {
    format!(
        "{{\n  \"kind\": \"{}\",\n  \"name\": \"{}\",\n  \"description\": \"{}\"\n}}\n",
        escape_json(kind),
        escape_json(name),
        escape_json(description)
    )
}

pub(super) fn escape_json(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

pub(super) fn escape_ts_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

pub(super) fn markdown_list(label: &str, values: &[String]) -> String {
    if values.is_empty() {
        return format!("- {label}: none");
    }
    format!("- {label}: {}", values.join(", "))
}

pub(super) fn ts_string_literal(value: &str) -> String {
    serde_json::to_string(value).expect("string literal serializes")
}

pub(super) fn ts_string_array_literal(values: &[String]) -> String {
    serde_json::to_string(values).expect("string array literal serializes")
}

fn append_subagent_manifest_entries(output: &mut String, subagents: &[SubagentManifest]) {
    output.push_str("subagents:");
    if subagents.is_empty() {
        output.push_str(" []\n");
        return;
    }
    output.push('\n');
    for subagent in subagents {
        output.push_str(&format!("  - name: {}\n", subagent.name));
        append_indented_optional_string(output, 4, "title", subagent.title.as_deref());
        append_indented_optional_string(output, 4, "role_id", subagent.role_id.as_deref());
        append_indented_optional_string(
            output,
            4,
            "responsibility",
            subagent.responsibility.as_deref(),
        );
        append_indented_optional_string(output, 4, "model", subagent.model.as_deref());
        append_indented_string_list(output, 4, "tools", &subagent.tools);
        append_indented_string_list(output, 4, "skills", &subagent.skills);
        append_indented_string_list(output, 4, "memory", &subagent.memory);
        append_indented_string_list(output, 4, "channels", &subagent.channels);
        if !subagent.approvals.is_empty() {
            output.push_str("    approvals:\n");
            for (tool, approval) in &subagent.approvals {
                output.push_str(&format!("      {tool}: {approval}\n"));
            }
        }
        if let Some(policy) = &subagent.runtime_policy {
            append_yaml_value_at_indent(output, 4, "runtime_policy", policy);
        }
    }
}

fn append_yaml_value<T: Serialize>(output: &mut String, label: &str, value: &T) {
    append_yaml_value_at_indent(output, 0, label, value);
}

fn append_yaml_value_at_indent<T: Serialize>(
    output: &mut String,
    indent: usize,
    label: &str,
    value: &T,
) {
    let yaml = serde_yaml::to_string(value).expect("generated metadata serializes");
    let padding = " ".repeat(indent);
    output.push_str(&format!("{padding}{label}:\n"));
    for line in yaml.lines().filter(|line| line.trim() != "---") {
        output.push_str(&padding);
        output.push_str("  ");
        output.push_str(line);
        output.push('\n');
    }
}

fn append_indented_optional_string(
    output: &mut String,
    indent: usize,
    label: &str,
    value: Option<&str>,
) {
    if let Some(value) = value.filter(|value| !value.trim().is_empty()) {
        output.push_str(&format!("{}{label}: {value:?}\n", " ".repeat(indent)));
    }
}

fn append_indented_string_list(output: &mut String, indent: usize, label: &str, values: &[String]) {
    if values.is_empty() {
        return;
    }
    output.push_str(&format!("{}{label}:\n", " ".repeat(indent)));
    for value in values {
        output.push_str(&format!("{}- {value}\n", " ".repeat(indent + 2)));
    }
}

pub(super) enum CommentStyle {
    Hash,
    Slash,
}

pub(super) fn with_generated_header(
    style: CommentStyle,
    agent: &str,
    template: &str,
    body: &str,
) -> String {
    let prefix = match style {
        CommentStyle::Hash => "#",
        CommentStyle::Slash => "//",
    };
    format!(
        "{prefix} Generated by eve-rails. Do not edit generated regions.\n{prefix} agent: {agent}\n{prefix} template: {template}\n\n{}\n",
        body.trim_end()
    )
}

pub(super) fn render_agent_manifest(agent: &AgentManifest, manifest: &FleetManifest) -> String {
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
    append_subagent_manifest_entries(&mut output, &agent.subagents);
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
    if let Some(policy) = &agent.x_runtime_policy {
        append_yaml_value(&mut output, "runtime_policy", policy);
    }
    if let Some(topology) = &agent.x_topology {
        append_yaml_value(&mut output, "topology", topology);
    }
    output.push_str("runtime: eve@0.24.4\n");
    output.push_str("compatibility:\n");
    output.push_str("  hot_load: true\n");
    output.push_str("  requires_restart: false\n");
    output.push_str("  min_runtime: eve@0.24.4\n");
    output
}

pub(super) fn render_versions_lock(
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
    for subagent in &agent.subagents {
        let digest = component_digest("subagents", &subagent.name, "manifest");
        output.push_str(&format!(
            "  manifest/subagents/{}@manifest:\n    source: manifests/agents.yml\n    digest: {digest}\n",
            subagent.name
        ));
    }
    output
}

pub(super) fn catalog_version<'a>(
    components: &'a BTreeMap<String, CatalogComponent>,
    name: &str,
) -> &'a str {
    components
        .get(name)
        .map(|component| component.version.as_str())
        .unwrap_or("unknown")
}
