use std::{cmp::Ordering, collections::BTreeMap};

use project_ledger_domain::{EvidenceRecord, PredicateAst, RuleResult, ScalarValue, StableId};
use serde_json::Value;

/// Evaluates the closed declarative predicate language against one evidence record.
///
/// Missing or type-incompatible critical fields never become truthy. The
/// function is pure and has no script, shell, clock, filesystem or network path.
pub fn evaluate_predicate(
    predicate: &PredicateAst,
    candidate: &EvidenceRecord,
    evidence: &BTreeMap<StableId, EvidenceRecord>,
) -> RuleResult {
    let Ok(value) = serde_json::to_value(candidate) else {
        return RuleResult::Error;
    };
    evaluate(predicate, candidate, evidence, &value)
}

#[allow(clippy::too_many_lines)] // Exhaustive closed-AST ownership is clearer in one side-effect-free match.
fn evaluate(
    predicate: &PredicateAst,
    candidate: &EvidenceRecord,
    evidence: &BTreeMap<StableId, EvidenceRecord>,
    value: &Value,
) -> RuleResult {
    match predicate {
        PredicateAst::All { predicates } => predicates
            .iter()
            .map(|item| evaluate(item, candidate, evidence, value))
            .fold(RuleResult::Pass, combine_all),
        PredicateAst::Any { predicates } => predicates
            .iter()
            .map(|item| evaluate(item, candidate, evidence, value))
            .fold(RuleResult::Fail, combine_any),
        PredicateAst::Not { predicate } => match evaluate(predicate, candidate, evidence, value) {
            RuleResult::Pass => RuleResult::Fail,
            RuleResult::Fail => RuleResult::Pass,
            other => other,
        },
        PredicateAst::Equals {
            path,
            value: expected,
        } => compare_scalar(path_value(value, path), expected, Ordering::Equal),
        PredicateAst::NotEquals {
            path,
            value: expected,
        } => negate_pass_fail(compare_scalar(
            path_value(value, path),
            expected,
            Ordering::Equal,
        )),
        PredicateAst::LessThan {
            path,
            value: expected,
        } => compare_scalar(path_value(value, path), expected, Ordering::Less),
        PredicateAst::LessOrEqual {
            path,
            value: expected,
        } => compare_order(path_value(value, path), expected, |ordering| {
            matches!(ordering, Ordering::Less | Ordering::Equal)
        }),
        PredicateAst::GreaterThan {
            path,
            value: expected,
        } => compare_scalar(path_value(value, path), expected, Ordering::Greater),
        PredicateAst::GreaterOrEqual {
            path,
            value: expected,
        } => compare_order(path_value(value, path), expected, |ordering| {
            matches!(ordering, Ordering::Greater | Ordering::Equal)
        }),
        PredicateAst::Exists { path } => {
            if path_value(value, path).is_some_and(|item| !item.is_null()) {
                RuleResult::Pass
            } else {
                RuleResult::Missing
            }
        }
        PredicateAst::Missing { path } => {
            if path_value(value, path).is_none_or(Value::is_null) {
                RuleResult::Pass
            } else {
                RuleResult::Fail
            }
        }
        PredicateAst::Contains {
            path,
            value: expected,
        } => {
            let Some(actual) = path_value(value, path) else {
                return RuleResult::Missing;
            };
            let expected_json = scalar_json(expected);
            match actual {
                Value::String(actual) => expected_json.as_str().map_or(RuleResult::Error, |text| {
                    if actual.contains(text) {
                        RuleResult::Pass
                    } else {
                        RuleResult::Fail
                    }
                }),
                Value::Array(items) => {
                    if items.contains(&expected_json) {
                        RuleResult::Pass
                    } else {
                        RuleResult::Fail
                    }
                }
                _ => RuleResult::Error,
            }
        }
        PredicateAst::DigestEquals {
            path,
            value: expected,
        } => {
            let Some(actual) = path_value(value, path) else {
                return RuleResult::Missing;
            };
            match actual.as_str() {
                Some(actual) if actual == expected.to_string() => RuleResult::Pass,
                Some(_) => RuleResult::Fail,
                None => RuleResult::Error,
            }
        }
        PredicateAst::StatusIn { path, values } | PredicateAst::FreshnessIn { path, values } => {
            let Some(actual) = path_value(value, path) else {
                return RuleResult::Missing;
            };
            match actual.as_str() {
                Some(actual) if values.iter().any(|allowed| allowed == actual) => RuleResult::Pass,
                Some(_) => RuleResult::Fail,
                None => RuleResult::Error,
            }
        }
        PredicateAst::CountEquals {
            path,
            value: expected,
        } => compare_count(path_value(value, path), *expected, Ordering::Equal),
        PredicateAst::CountAtLeast {
            path,
            value: expected,
        } => compare_count_at_least(path_value(value, path), *expected),
        PredicateAst::SameSnapshotAs { evidence_ref } => {
            let Some(reference) = evidence.get(evidence_ref) else {
                return RuleResult::Missing;
            };
            match (&candidate.repository_binding, &reference.repository_binding) {
                (Some(left), Some(right)) if left == right => RuleResult::Pass,
                (Some(_), Some(_)) => RuleResult::Fail,
                _ => RuleResult::Missing,
            }
        }
    }
}

fn path_value<'a>(root: &'a Value, path: &str) -> Option<&'a Value> {
    if path.is_empty() {
        return None;
    }
    path.split('.').try_fold(root, |value, component| {
        value.as_object().and_then(|object| object.get(component))
    })
}

fn compare_scalar(actual: Option<&Value>, expected: &ScalarValue, wanted: Ordering) -> RuleResult {
    compare_order(actual, expected, |ordering| ordering == wanted)
}

fn compare_order(
    actual: Option<&Value>,
    expected: &ScalarValue,
    accepted: impl FnOnce(Ordering) -> bool,
) -> RuleResult {
    let Some(actual) = actual else {
        return RuleResult::Missing;
    };
    let Some(ordering) = scalar_order(actual, expected) else {
        return RuleResult::Error;
    };
    if accepted(ordering) {
        RuleResult::Pass
    } else {
        RuleResult::Fail
    }
}

fn scalar_order(actual: &Value, expected: &ScalarValue) -> Option<Ordering> {
    match expected {
        ScalarValue::Bool(expected) => actual.as_bool()?.partial_cmp(expected),
        ScalarValue::Integer(expected) => {
            if let Some(actual) = actual.as_i64() {
                actual.partial_cmp(expected)
            } else {
                let actual = actual.as_u64()?;
                let expected = u64::try_from(*expected).ok()?;
                actual.partial_cmp(&expected)
            }
        }
        ScalarValue::Unsigned(expected) => {
            if let Some(actual) = actual.as_u64() {
                actual.partial_cmp(expected)
            } else {
                let actual = actual.as_i64()?;
                let expected = i64::try_from(*expected).ok()?;
                actual.partial_cmp(&expected)
            }
        }
        ScalarValue::String(expected) => actual.as_str()?.partial_cmp(expected.as_str()),
    }
}

fn scalar_json(value: &ScalarValue) -> Value {
    match value {
        ScalarValue::Bool(value) => Value::Bool(*value),
        ScalarValue::Integer(value) => Value::from(*value),
        ScalarValue::Unsigned(value) => Value::from(*value),
        ScalarValue::String(value) => Value::String(value.clone()),
    }
}

fn compare_count(actual: Option<&Value>, expected: u64, wanted: Ordering) -> RuleResult {
    let Some(actual) = actual else {
        return RuleResult::Missing;
    };
    let Some(count) = value_count(actual) else {
        return RuleResult::Error;
    };
    if count.cmp(&expected) == wanted {
        RuleResult::Pass
    } else {
        RuleResult::Fail
    }
}

fn compare_count_at_least(actual: Option<&Value>, expected: u64) -> RuleResult {
    let Some(actual) = actual else {
        return RuleResult::Missing;
    };
    let Some(count) = value_count(actual) else {
        return RuleResult::Error;
    };
    if count >= expected {
        RuleResult::Pass
    } else {
        RuleResult::Fail
    }
}

fn value_count(value: &Value) -> Option<u64> {
    let count = match value {
        Value::Array(items) => items.len(),
        Value::Object(items) => items.len(),
        Value::String(item) => item.chars().count(),
        _ => return None,
    };
    u64::try_from(count).ok()
}

const fn negate_pass_fail(result: RuleResult) -> RuleResult {
    match result {
        RuleResult::Pass => RuleResult::Fail,
        RuleResult::Fail => RuleResult::Pass,
        other => other,
    }
}

const fn combine_all(current: RuleResult, next: RuleResult) -> RuleResult {
    if matches!(current, RuleResult::Fail) || matches!(next, RuleResult::Fail) {
        return RuleResult::Fail;
    }
    max_uncertainty(current, next)
}

const fn combine_any(current: RuleResult, next: RuleResult) -> RuleResult {
    if matches!(current, RuleResult::Pass) || matches!(next, RuleResult::Pass) {
        return RuleResult::Pass;
    }
    max_uncertainty(current, next)
}

const fn max_uncertainty(left: RuleResult, right: RuleResult) -> RuleResult {
    if rule_priority(left) >= rule_priority(right) {
        left
    } else {
        right
    }
}

const fn rule_priority(result: RuleResult) -> u8 {
    match result {
        RuleResult::Error => 5,
        RuleResult::Conflicted => 4,
        RuleResult::Stale => 3,
        RuleResult::Missing => 2,
        RuleResult::Fail => 1,
        RuleResult::Pass => 0,
    }
}
