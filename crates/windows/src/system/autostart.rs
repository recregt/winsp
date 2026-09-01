use windows::ApplicationModel::{StartupTask, StartupTaskState};
use windows::core::HSTRING;

const STARTUP_TASK_ID: &str = "WinSPStartup";

pub fn is_enabled() -> bool {
    startup_task_is_enabled().unwrap_or(false)
}

pub fn set_enabled(enabled: bool) {
    if let Err(error) = startup_task_set_enabled(enabled) {
        super::toast::show(
            "WinSP",
            &format!("Failed to update Start with Windows: {error}"),
        );
    }
}

fn startup_task() -> windows::core::Result<StartupTask> {
    StartupTask::GetAsync(&HSTRING::from(STARTUP_TASK_ID))?.join()
}

fn startup_task_is_enabled() -> windows::core::Result<bool> {
    let state = startup_task()?.State()?;
    Ok(matches!(
        state,
        StartupTaskState::Enabled | StartupTaskState::EnabledByPolicy
    ))
}

fn startup_task_set_enabled(enabled: bool) -> windows::core::Result<()> {
    let task = startup_task()?;
    if enabled {
        task.RequestEnableAsync()?.join()?;
    } else {
        task.Disable()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_enabled_degrades_gracefully_without_package_identity() {
        assert!(!is_enabled());
    }
}
