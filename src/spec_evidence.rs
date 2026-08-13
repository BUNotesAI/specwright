use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::spec_core::{
    Evidence, Scenario, ScenarioVerification, SpecError, SpecResult, Verdict, VerificationReport,
};

/// Versioned external evidence import manifest.
#[derive(Debug, Deserialize)]
pub struct EvidenceManifest {
    pub schema_version: u32,
    pub spec: ManifestSpec,
    pub subject_commit: String,
    pub generated_at: String,
    pub evidence: Vec<ManifestEvidence>,
}

#[derive(Debug, Deserialize)]
pub struct ManifestSpec {
    pub identity: String,
    pub sha256: String,
}

#[derive(Debug, Deserialize)]
pub struct ManifestEvidence {
    pub scenario_name: String,
    pub evidence_id: String,
    pub artifact_url: String,
    pub digest: ManifestDigest,
    pub verdict: EvidenceVerdict,
    #[serde(default)]
    pub producer: Option<Producer>,
    #[serde(default)]
    pub attestation: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct ManifestDigest {
    pub algorithm: String,
    pub value: String,
}

#[derive(Debug, Deserialize)]
pub struct Producer {
    pub name: String,
    pub run_id: String,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceVerdict {
    Pass,
    Fail,
}

impl EvidenceVerdict {
    fn verdict(self) -> Verdict {
        match self {
            Self::Pass => Verdict::Pass,
            Self::Fail => Verdict::Fail,
        }
    }
}

/// Validate a complete manifest and atomically return the resolved report.
#[allow(clippy::too_many_lines)] // Validation stays atomic and linear for the versioned manifest contract.
pub fn resolve_evidence(
    manifest: EvidenceManifest,
    expected_spec_name: &str,
    expected_spec_hash: &str,
    expected_subject_commit: &str,
    scenarios: &[Scenario],
    report: VerificationReport,
) -> SpecResult<VerificationReport> {
    validate_header(
        &manifest,
        expected_spec_name,
        expected_spec_hash,
        expected_subject_commit,
    )?;

    let declared: HashMap<&str, (&str, &str)> = scenarios
        .iter()
        .filter(|scenario| scenario.verification == ScenarioVerification::External)
        .filter_map(|scenario| {
            scenario
                .evidence
                .as_deref()
                .map(|id| (id, (scenario.name.as_str(), id)))
        })
        .collect();
    let mut seen = HashSet::new();

    for item in &manifest.evidence {
        if !seen.insert(item.evidence_id.as_str()) {
            return Err(verification_error(format!(
                "duplicate Evidence ID `{}` in manifest",
                item.evidence_id
            )));
        }
        let Some((scenario_name, _)) = declared.get(item.evidence_id.as_str()) else {
            return Err(verification_error(format!(
                "unknown Evidence ID `{}` in manifest",
                item.evidence_id
            )));
        };
        if item.scenario_name != *scenario_name {
            return Err(verification_error(format!(
                "Evidence ID `{}` names scenario `{}`; expected `{scenario_name}`",
                item.evidence_id, item.scenario_name
            )));
        }
        validate_item(item)?;
        let current = report
            .results
            .iter()
            .find(|result| result.scenario_name == item.scenario_name)
            .ok_or_else(|| {
                verification_error(format!(
                    "scenario `{}` has no verification result",
                    item.scenario_name
                ))
            })?;
        if current.verdict != Verdict::ExternalPending {
            return Err(verification_error(format!(
                "Evidence ID `{}` cannot overwrite {:?}; only external_pending may be resolved",
                item.evidence_id, current.verdict
            )));
        }
    }

    for id in declared.keys() {
        if !seen.contains(id) {
            return Err(verification_error(format!(
                "missing Evidence ID `{id}` in manifest"
            )));
        }
    }

    let by_scenario: HashMap<&str, &ManifestEvidence> = manifest
        .evidence
        .iter()
        .map(|item| (item.scenario_name.as_str(), item))
        .collect();
    let results = report
        .results
        .into_iter()
        .map(|mut result| {
            if let Some(item) = by_scenario.get(result.scenario_name.as_str()) {
                let verdict = item.verdict.verdict();
                result.verdict = verdict;
                for step in &mut result.step_results {
                    step.verdict = verdict;
                    step.reason =
                        format!("resolved by external Evidence ID `{}`", item.evidence_id);
                }
                result.evidence = vec![Evidence::ExternalArtifact {
                    evidence_id: item.evidence_id.clone(),
                    artifact_url: item.artifact_url.clone(),
                    digest_algorithm: item.digest.algorithm.clone(),
                    digest_value: item.digest.value.clone(),
                    producer: item
                        .producer
                        .as_ref()
                        .map(|producer| format!("{}:{}", producer.name, producer.run_id)),
                }];
            }
            result
        })
        .collect();

    let mut resolved = VerificationReport::from_results(report.spec_name, results);
    resolved.schema_version = Some(1);
    Ok(resolved)
}

fn validate_header(
    manifest: &EvidenceManifest,
    expected_spec_name: &str,
    expected_spec_hash: &str,
    expected_subject_commit: &str,
) -> SpecResult<()> {
    if manifest.schema_version != 1 {
        return Err(verification_error(format!(
            "unsupported evidence schema_version {}; expected 1",
            manifest.schema_version
        )));
    }
    if manifest.spec.identity != expected_spec_name {
        return Err(verification_error(
            "evidence manifest spec identity mismatch",
        ));
    }
    if !manifest
        .spec
        .sha256
        .eq_ignore_ascii_case(expected_spec_hash)
    {
        return Err(verification_error("evidence manifest spec hash mismatch"));
    }
    if manifest.subject_commit != expected_subject_commit {
        return Err(verification_error(
            "evidence manifest subject commit mismatch",
        ));
    }
    if manifest.generated_at.trim().is_empty() {
        return Err(verification_error(
            "evidence manifest generated_at is empty",
        ));
    }
    Ok(())
}

fn validate_item(item: &ManifestEvidence) -> SpecResult<()> {
    if !(item.artifact_url.starts_with("https://") || item.artifact_url.starts_with("http://")) {
        return Err(verification_error(format!(
            "Evidence ID `{}` has an invalid artifact URL",
            item.evidence_id
        )));
    }
    if item.digest.algorithm != "sha256"
        || item.digest.value.len() != 64
        || !item
            .digest
            .value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(verification_error(format!(
            "Evidence ID `{}` requires a 64-character sha256 digest",
            item.evidence_id
        )));
    }
    let producer_valid = item
        .producer
        .as_ref()
        .is_some_and(|producer| !producer.name.is_empty() && !producer.run_id.is_empty());
    if !producer_valid && item.attestation.is_none() {
        return Err(verification_error(format!(
            "Evidence ID `{}` requires producer or attestation",
            item.evidence_id
        )));
    }
    Ok(())
}

fn verification_error(message: impl Into<String>) -> SpecError {
    SpecError::Verification(message.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec_core::{
        ReviewMode, ScenarioMode, ScenarioResult, Span, VerificationReport, VerificationSummary,
    };

    #[test]
    fn external_evidence_cannot_overwrite_a_non_pending_result() {
        let scenario = Scenario {
            name: "External build".into(),
            steps: Vec::new(),
            test_selector: None,
            tags: Vec::new(),
            verification: ScenarioVerification::External,
            evidence: Some("ci-build".into()),
            review: ReviewMode::Auto,
            mode: ScenarioMode::Standard,
            depends_on: Vec::new(),
            span: Span::default(),
        };
        let report = VerificationReport {
            schema_version: Some(1),
            spec_name: "External fixture".into(),
            results: vec![ScenarioResult {
                scenario_name: scenario.name.clone(),
                verdict: Verdict::Fail,
                step_results: Vec::new(),
                evidence: Vec::new(),
                duration_ms: 0,
            }],
            summary: VerificationSummary {
                total: 1,
                passed: 0,
                failed: 1,
                skipped: 0,
                uncertain: 0,
                pending_review: 0,
                external_pending: 0,
            },
        };
        let manifest = EvidenceManifest {
            schema_version: 1,
            spec: ManifestSpec {
                identity: "External fixture".into(),
                sha256: "a".repeat(64),
            },
            subject_commit: "abc123".into(),
            generated_at: "2026-08-14T00:00:00Z".into(),
            evidence: vec![ManifestEvidence {
                scenario_name: scenario.name.clone(),
                evidence_id: "ci-build".into(),
                artifact_url: "https://ci.example/build/1".into(),
                digest: ManifestDigest {
                    algorithm: "sha256".into(),
                    value: "b".repeat(64),
                },
                verdict: EvidenceVerdict::Pass,
                producer: Some(Producer {
                    name: "ci".into(),
                    run_id: "1".into(),
                }),
                attestation: None,
            }],
        };

        let error = match resolve_evidence(
            manifest,
            "External fixture",
            &"a".repeat(64),
            "abc123",
            &[scenario],
            report,
        ) {
            Ok(_) => panic!("mechanical failure must remain authoritative"),
            Err(error) => error,
        };

        assert!(error.to_string().contains(
            "Evidence ID `ci-build` cannot overwrite Fail; only external_pending may be resolved"
        ));
    }
}
