use windows::ApplicationModel::{StartupTask, StartupTaskState};
use windows::core::HSTRING;

const STARTUP_TASK_ID: &str = "WinSPStartup";

pub fn is_enabled() -> bool {
    startup_task_is_enabled().unwrap_or(false)
}

pub fn set_enabled(enabled: bool) {
    let _ = startup_task_set_enabled(enabled);
}

fn startup_task_is_enabled() -> windows::core::Result<bool> {
    let task = StartupTask::GetAsync(&HSTRING::from(STARTUP_TASK_ID))?.get()?;
    let state = task.State()?;
    Ok(matches!(
        state,
        StartupTaskState::Enabled | StartupTaskState::EnabledByPolicy
    ))
}

fn startup_task_set_enabled(enabled: bool) -> windows::core::Result<()> {
    let task = StartupTask::GetAsync(&HSTRING::from(STARTUP_TASK_ID))?.get()?;
    if enabled {
        task.RequestEnableAsync()?.get()?;
    } else {
        task.Disable()?;
    }
    Ok(())
}
