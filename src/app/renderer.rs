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
                    .map(|component| tool_description(&tool, component))
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

pub(super) fn render_eve_channel() -> String {
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

fn render_evals_config() -> String {
    r#"// Generated by eve-rails. Do not edit generated regions.
// template: evals.config

import { defineEvalConfig } from "eve/evals";

export default defineEvalConfig({});
"#
    .to_string()
}

pub(super) fn tool_description(tool: &str, component: &CatalogComponent) -> String {
    match &component.side_effects {
        Some(side_effects) => format!("Generated {tool} {side_effects} tool contract."),
        None => format!("Generated {tool} tool contract."),
    }
}

pub(super) fn render_subagent_placeholder(agent: &str, subagent: &str) -> String {
    with_generated_header(
        CommentStyle::Hash,
        agent,
        "subagent-placeholder",
        &format!(
            "# {subagent}\n\nThis subagent folder is generated as an Eve Rails convention placeholder.\n"
        ),
    )
}

pub(super) fn render_subagent_agent_ts(agent: &str, subagent: &str, model: &str) -> String {
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
