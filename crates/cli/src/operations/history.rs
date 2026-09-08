use super::Selection;
use crate::errors::CliError;
use clap::{Args, Subcommand};
use project_ledger_projection_engine::HistoryView;
use project_ledger_storage_sqlite::SqliteEventStore;

#[derive(Debug, Args)]
pub struct CutArgs {
    #[command(flatten)]
    pub selection: Selection,
    #[arg(long)]
    pub version: Option<u64>,
}

#[derive(Debug, Args)]
pub struct TraceArgs {
    pub id: String,
    #[command(flatten)]
    pub cut: CutArgs,
    #[arg(long)]
    pub cursor: Option<String>,
}

#[derive(Debug, Subcommand)]
pub enum HistoryCommand {
    At(CutArgs),
    Diff {
        #[command(flatten)]
        selection: Selection,
        #[arg(long)]
        from: u64,
        #[arg(long)]
        to: u64,
    },
    Get {
        reference: String,
        #[command(flatten)]
        cut: CutArgs,
    },
}

fn read(selection: &Selection) -> Result<Vec<project_ledger_domain::EventEnvelope>, CliError> {
    let (database, project) = selection.resolve()?;
    let store = SqliteEventStore::open_read_only(&database)?;
    Ok(store.load_verified_snapshot(&project)?)
}

impl HistoryCommand {
    pub fn run(self) -> Result<String, CliError> {
        let value = match self {
            Self::At(cut) => HistoryView::at(&read(&cut.selection)?, cut.version)
                .map_err(CliError::domain)?
                .state(),
            Self::Get { reference, cut } => HistoryView::at(&read(&cut.selection)?, cut.version)
                .and_then(|view| view.get(&reference))
                .map_err(CliError::domain)?,
            Self::Diff {
                selection,
                from,
                to,
            } => {
                let events = read(&selection)?;
                let before = HistoryView::at(&events, Some(from)).map_err(CliError::domain)?;
                let after = HistoryView::at(&events, Some(to)).map_err(CliError::domain)?;
                before.diff(&after).map_err(CliError::domain)?
            }
        };
        serde_json::to_string(&value).map_err(CliError::input)
    }
}

pub fn trace(args: &TraceArgs) -> Result<String, CliError> {
    let events = read(&args.cut.selection)?;
    let value = HistoryView::trace(&events, &args.id, args.cut.version, args.cursor.as_deref())
        .map_err(CliError::domain)?;
    serde_json::to_string(&value).map_err(CliError::input)
}
