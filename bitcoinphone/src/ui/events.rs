use sv::script::Script;

pub enum UIEvent {
    Connect(Script),
    Disconnect(),

    Refund(Script)
}

pub enum AppEvent {
    ConnectionEstablished(Script),
}