use thiserror::Error;

#[derive(Debug, Error)]
pub enum ResultError {
    #[error("A heartbeat hook is already set")]
    HookAlreadyExists,
    #[error("This function is only available for timerLegacy health check")]
    TimerLegacyHealCheckOnly,
    #[error("{0}")]
    UnclassifiedStatic(&'static str),
}
