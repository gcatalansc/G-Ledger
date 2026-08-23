use std::collections::BTreeSet;

use project_ledger_domain::{CommandType, EventType};
use project_ledger_schemas::command_event_matrix;

#[test]
fn every_command_has_exactly_one_matrix_row() {
    let matrix = command_event_matrix();
    let commands: BTreeSet<_> = matrix.rows.iter().map(|row| row.command_type).collect();

    assert_eq!(matrix.rows.len(), 53);
    assert_eq!(commands.len(), 53);
    assert_eq!(commands, CommandType::ALL.iter().copied().collect());
}

#[test]
fn every_event_has_a_command_origin_or_a_derived_only_owner() {
    let matrix = command_event_matrix();
    let mut covered = BTreeSet::new();
    for row in &matrix.rows {
        for outcome in &row.accepted_outcomes {
            covered.extend(outcome.required_events.iter().copied());
        }
        covered.extend(row.possible_events.iter().copied());
    }
    covered.extend(
        matrix
            .derived_only_events
            .iter()
            .map(|entry| entry.event_type),
    );

    assert_eq!(covered, EventType::ALL.iter().copied().collect());
}

#[test]
fn authority_refresh_and_phase_completion_use_explicit_alternative_outcomes() {
    let matrix = command_event_matrix();
    let authority = matrix
        .row(CommandType::RefreshAuthorityDocumentDigest)
        .unwrap();
    let authority_outcomes: BTreeSet<_> = authority
        .accepted_outcomes
        .iter()
        .map(|outcome| outcome.required_events.as_slice())
        .collect();
    assert!(authority_outcomes.contains(&[EventType::AuthorityDocumentDigestRefreshed][..]));
    assert!(authority_outcomes.contains(&[EventType::AuthorityDocumentMarkedUnavailable][..]));
    assert!(authority_outcomes.contains(&[EventType::AuthorityDocumentDigestMismatch][..]));

    let completion = matrix.row(CommandType::RequestPhaseCompletion).unwrap();
    let completion_outcomes: BTreeSet<_> = completion
        .accepted_outcomes
        .iter()
        .map(|outcome| outcome.required_events.as_slice())
        .collect();
    assert!(completion_outcomes.contains(&[EventType::PhaseMarkedReadyToClose][..]));
    assert!(completion_outcomes.contains(&[EventType::PhaseCompleted][..]));
}

#[test]
fn lifecycle_events_have_no_external_command_owner() {
    let matrix = command_event_matrix();
    let derived: BTreeSet<_> = matrix
        .derived_only_events
        .iter()
        .map(|entry| entry.event_type)
        .collect();

    assert!(derived.contains(&EventType::ProjectStatusDerived));
    assert!(derived.contains(&EventType::GateVerified));
    assert!(derived.contains(&EventType::PhaseCompleted));
}
