use super::*;

#[derive(Debug, Default)]
pub(super) struct ValidationReport {
    pub(super) errors: Vec<String>,
    pub(super) warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
pub(super) struct DoctorReport {
    pub(super) checks: Vec<DoctorCheck>,
}

#[derive(Debug, Serialize)]
pub(super) struct DoctorCheck {
    pub(super) name: String,
    pub(super) status: DoctorStatus,
    pub(super) message: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(super) enum DoctorStatus {
    Pass,
    Warn,
    Fail,
}

impl DoctorReport {
    pub(super) fn has_failures(&self) -> bool {
        self.checks
            .iter()
            .any(|check| check.status == DoctorStatus::Fail)
    }
}

pub(super) fn run_doctor(
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

pub(super) fn add_budget_checks(checks: &mut Vec<DoctorCheck>, manifest: &FleetManifest) {
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

pub(super) fn add_schedule_safety_checks(
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

pub(super) fn add_environment_checks(
    checks: &mut Vec<DoctorCheck>,
    command: &DoctorCommand,
) -> Result<()> {
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

pub(super) fn add_fix_plan_checks(
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

pub(super) fn add_template_checks(
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

pub(super) fn add_update_checks(
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

pub(super) fn generated_metadata_present(plan: &BatchPlan) -> bool {
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

pub(super) fn push_check(
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

pub(super) fn print_doctor_report(report: &DoctorReport) {
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

pub(super) fn doctor_status_label(status: DoctorStatus) -> &'static str {
    match status {
        DoctorStatus::Pass => "pass",
        DoctorStatus::Warn => "warn",
        DoctorStatus::Fail => "fail",
    }
}

pub(super) fn validate_manifest(
    manifest: &FleetManifest,
    catalog: &CatalogManifest,
) -> ValidationReport {
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

pub(super) fn validate_policy(
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

pub(super) fn validate_named_list(
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

pub(super) fn validate_component_version(
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

pub(super) fn print_validation_report(report: &ValidationReport) -> Result<()> {
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

pub(super) fn ensure_valid(report: &ValidationReport) -> Result<()> {
    if report.errors.is_empty() {
        Ok(())
    } else {
        bail!(
            "validation failed with {} error(s): {}",
            report.errors.len(),
            report.errors.join("; ")
        )
    }
}

pub(super) fn is_slug(value: &str) -> bool {
    value.chars().all(|char| {
        char.is_ascii_lowercase() || char.is_ascii_digit() || char == '-' || char == '_'
    })
}

pub(super) fn effective_schedules(agent: &AgentManifest, manifest: &FleetManifest) -> Vec<String> {
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

pub(super) fn is_risky_tool(tool: &str, catalog: &CatalogManifest) -> bool {
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
