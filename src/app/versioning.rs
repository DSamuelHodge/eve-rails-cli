use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(super) enum UpdateKind {
    Current,
    Patch,
    Minor,
    Major,
    Missing,
    Invalid,
}

#[derive(Debug, Serialize)]
pub(super) struct VersionReport {
    pub(super) agent: String,
    pub(super) component_kind: String,
    pub(super) component: String,
    pub(super) requested: String,
    pub(super) resolved: Option<String>,
    pub(super) update: UpdateKind,
    pub(super) affected_evals: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Semver {
    pub(super) major: u64,
    pub(super) minor: u64,
    pub(super) patch: u64,
}

pub(super) fn collect_version_reports(
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

pub(super) fn collect_component_reports(
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

pub(super) fn collect_named_report(
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

pub(super) fn apply_version_policy(
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

pub(super) fn resolve_version(
    requested: &str,
    component: Option<&CatalogComponent>,
) -> Option<String> {
    let component = component?;
    if version_request_matches(requested, &component.version) {
        Some(component.version.clone())
    } else {
        None
    }
}

pub(super) fn classify_update(
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

pub(super) fn version_request_matches(requested: &str, available: &str) -> bool {
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

pub(super) fn effective_evals(agent: &AgentManifest, manifest: &FleetManifest) -> Vec<String> {
    manifest
        .defaults
        .evals
        .iter()
        .chain(agent.evals.iter())
        .cloned()
        .collect()
}

pub(super) fn effective_components(
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

pub(super) fn print_version_reports(reports: &[VersionReport]) {
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
