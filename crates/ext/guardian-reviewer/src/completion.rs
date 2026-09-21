//! Converts reviewer outcomes into decisions and reports. The host publishes these
//! reports and applies them only to the action whose evidence was reviewed.

use codex_prompts::ResolvedModelMessages;
use codex_prompts::render_guardian_rejection;
use codex_protocol::openai_models::ModelInfo;
use codex_protocol::protocol::GuardianAssessmentEvent;
use codex_protocol::protocol::GuardianAssessmentOutcome;
use codex_protocol::protocol::GuardianAssessmentStatus;
use codex_protocol::protocol::GuardianRiskLevel;
use codex_protocol::protocol::GuardianUserAuthorization;
use codex_protocol::protocol::ReviewDecision;

use crate::GuardianReviewError;
use crate::GuardianReviewOutcome;

const REVIEW_FAILURE_INSTRUCTIONS: &str = concat!(
    "The action was not executed because automatic approval review could not be completed. ",
    "This is a review failure, not a determination that the action is unsafe. ",
    "Do not bypass the approval check; resolve the error or ask the user for guidance.",
);
const INPUT_BUDGET_MESSAGE: &str =
    "the complete action and minimum review context exceed the reviewer input budget";

pub struct ReviewCompletion {
    /// `None` requests user approval after optional review exhausts its input budget.
    pub decision: Option<ReviewDecision>,
    pub event: GuardianAssessmentEvent,
    pub warning: Option<String>,
    /// Only completed assessments may enter the evidence cache or count as policy denials.
    pub assessment_outcome: Option<GuardianAssessmentOutcome>,
}

pub fn complete_review(
    outcome: GuardianReviewOutcome,
    model: &ModelInfo,
    require_guardian: bool,
    mut event: GuardianAssessmentEvent,
) -> ReviewCompletion {
    let completed_assessment = match &outcome {
        GuardianReviewOutcome::Completed(assessment) => Some(assessment.outcome),
        GuardianReviewOutcome::Error(_) => None,
    };
    let assessment = match outcome {
        GuardianReviewOutcome::Completed(assessment) => {
            assessment
        }
        GuardianReviewOutcome::Error(error) => {
            if matches!(error, GuardianReviewError::InputBudgetExceeded) && !require_guardian {
                let rationale = format!("Automatic approval review failed: {INPUT_BUDGET_MESSAGE}");
                event.status = GuardianAssessmentStatus::Aborted;
                event.rationale = Some(rationale.clone());
                return ReviewCompletion {
                    decision: None,
                    event,
                    warning: Some(rationale),
                    assessment_outcome: None,
                };
            }
            match error {
                GuardianReviewError::Timeout => {
                    let rationale = "Automatic approval review timed out while evaluating the requested approval.".to_string();
                    event.status = GuardianAssessmentStatus::TimedOut;
                    event.rationale = Some(rationale.clone());
                    return ReviewCompletion {
                        decision: Some(ReviewDecision::TimedOut),
                        event,
                        warning: Some(rationale),
                        assessment_outcome: None,
                    };
                }
                GuardianReviewError::Cancelled => {
                    event.status = GuardianAssessmentStatus::Aborted;
                    return ReviewCompletion {
                        decision: Some(ReviewDecision::Abort),
                        event,
                        warning: None,
                        assessment_outcome: None,
                    };
                }
                GuardianReviewError::InputBudgetExceeded
                | GuardianReviewError::PromptBuild { .. }
                | GuardianReviewError::Session { .. }
                | GuardianReviewError::Parse { .. } => {
                    let message = match &error {
                        GuardianReviewError::InputBudgetExceeded => INPUT_BUDGET_MESSAGE,
                        GuardianReviewError::PromptBuild { message }
                        | GuardianReviewError::Session { message, .. }
                        | GuardianReviewError::Parse { message } => message,
                        GuardianReviewError::Timeout | GuardianReviewError::Cancelled => {
                            "guardian review failed"
                        }
                    };
                    let rationale = format!("Automatic approval review failed: {message}");
                    // Keep the existing blocked status for client compatibility.
                    event.status = GuardianAssessmentStatus::Denied;
                    event.rationale = Some(rationale.clone());
                    return ReviewCompletion {
                        decision: Some(ReviewDecision::denied(format!(
                            "{rationale}\n{REVIEW_FAILURE_INSTRUCTIONS}"
                        ))),
                        event,
                        warning: Some(rationale),
                        assessment_outcome: None,
                    };
                }
            }
        }
    };
    let approved = matches!(assessment.outcome, GuardianAssessmentOutcome::Allow);
    let verdict = if approved { "approved" } else { "denied" };
    let authorization = match assessment.user_authorization {
        GuardianUserAuthorization::Unknown => "unknown",
        GuardianUserAuthorization::Low => "low",
        GuardianUserAuthorization::Medium => "medium",
        GuardianUserAuthorization::High => "high",
    };
    let risk = match assessment.risk_level {
        GuardianRiskLevel::Low => "low",
        GuardianRiskLevel::Medium => "medium",
        GuardianRiskLevel::High => "high",
        GuardianRiskLevel::Critical => "critical",
    };
    let warning = format!(
        "Automatic approval review {verdict} (risk: {risk}, authorization: {authorization}): {}",
        assessment.rationale
    );
    event.status = if approved {
        GuardianAssessmentStatus::Approved
    } else {
        GuardianAssessmentStatus::Denied
    };
    event.risk_level = Some(assessment.risk_level);
    event.user_authorization = Some(assessment.user_authorization);
    event.rationale = Some(assessment.rationale.clone());
    let decision = if approved {
        ReviewDecision::Approved
    } else {
        let rejection_instructions = ResolvedModelMessages::from_model(model)
            .auto_review()
            .rejection_instructions;
        ReviewDecision::denied(render_guardian_rejection(
            &assessment.rationale,
            rejection_instructions,
        ))
    };
    ReviewCompletion {
        decision: Some(decision),
        event,
        warning: Some(warning),
        assessment_outcome: completed_assessment,
    }
}
