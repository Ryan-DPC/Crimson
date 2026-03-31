use tokio::sync::broadcast;

pub struct WsSender(pub broadcast::Sender<String>);
