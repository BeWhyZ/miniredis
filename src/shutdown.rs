use tokio::sync::broadcast;


#[derive(Debug)]
pub(crate) struct Shutdown  {
    is_shutdown: bool,
    notify: broadcast::Receiver<()>,
}


/// Listens for the server shutdown signal.
///
/// Shutdown is signalled using a `broadcast::Receiver`. Only a single value is
/// ever sent. Once a value has been sent via the broadcast channel, the server
/// should shutdown.
///
/// The `Shutdown` struct listens for the signal and tracks that the signal has
/// been received. Callers may query for whether the shutdown signal has been
/// received or not.
impl Shutdown {
    pub(crate) fn new(notify: broadcast::Receiver<()>) -> Self {
        Self {
            notify,
            is_shutdown: false,
        }
    }

    pub(crate) fn is_shutdown(&self) -> bool {
        self.is_shutdown
    }

    pub(crate) async fn recv(&mut self) {
        // If the shutdown signal has already been received, then return
        // immediately.
        if self.is_shutdown {
            return
        }

        // wait all sender close
        let _ = self.notify.recv().await;

        self.is_shutdown = true;
    }


}
