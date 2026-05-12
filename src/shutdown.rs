use tokio_util::sync::CancellationToken;

#[derive(Debug)]
pub struct Shutdown {
    is_shutdown: bool,
    notify: CancellationToken,
}

impl Shutdown {
    pub fn new(notify: CancellationToken) -> Self {
        Self {
            is_shutdown: false,
            notify,
        }
    }

    pub fn is_shutdown(&self) -> bool {
        self.is_shutdown
    }

    pub async fn recv(&mut self) {
        if self.is_shutdown {
            return;
        }

        let _ = self.notify.cancelled().await;
        self.is_shutdown = true;
    }
}
