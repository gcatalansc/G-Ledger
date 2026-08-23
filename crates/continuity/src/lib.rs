//! Deterministic continuity commands and read-only views.

mod audit;
mod checkpoint;
mod markdown;
mod resume;

pub use audit::{
    AuditCode, AuditFinding, AuditReport, AuditSeverity, VerificationReport, audit_projection,
    verify_stream_and_artifact,
};
pub use checkpoint::{ContinuityEngine, ContinuityError};
pub use markdown::{
    MarkdownRenderError, MarkdownRenderRequest, MarkdownRenderer, RenderedMarkdown,
    RendererPrivacyProfile,
};
pub use resume::{
    ConservativeTokenCounter, ResumeBudget, ResumeBuilder, ResumeError, render_agent_markdown,
    render_compact,
};
