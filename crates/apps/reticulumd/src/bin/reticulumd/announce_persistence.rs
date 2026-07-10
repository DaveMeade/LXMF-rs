use rns_transport::transport::Transport;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

const RETICULUM_PATH_TABLE_SAVE_DEBOUNCE: Duration = Duration::from_secs(2);

pub(super) type PathTableSaveSender = tokio::sync::mpsc::Sender<()>;

#[derive(Clone)]
pub(super) struct PathTablePersistenceContext {
    transport: Arc<Transport>,
    path: PathBuf,
}

impl PathTablePersistenceContext {
    pub(super) fn new(transport: Arc<Transport>, path: PathBuf) -> Self {
        Self { transport, path }
    }
}

pub(super) async fn flush_reticulum_path_table(
    context: &PathTablePersistenceContext,
) -> io::Result<usize> {
    context.transport.save_reticulum_path_table(&context.path).await
}

pub(super) async fn flush_reticulum_path_table_if_configured(
    context: Option<PathTablePersistenceContext>,
) {
    if let Some(context) = context {
        if let Err(err) = flush_reticulum_path_table(&context).await {
            log::error!("[daemon] failed to persist Reticulum path table: {err}");
        }
    }
}

pub(super) fn spawn_path_table_persistence_worker(
    context: PathTablePersistenceContext,
) -> PathTableSaveSender {
    let (tx, mut rx) = tokio::sync::mpsc::channel::<()>(1);
    tokio::spawn(async move {
        while rx.recv().await.is_some() {
            sleep(RETICULUM_PATH_TABLE_SAVE_DEBOUNCE).await;
            while rx.try_recv().is_ok() {}
            if let Err(err) = flush_reticulum_path_table(&context).await {
                log::error!("[daemon] failed to persist Reticulum path table: {err}");
            }
        }
    });
    tx
}
