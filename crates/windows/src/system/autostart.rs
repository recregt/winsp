use windows::ApplicationModel::{StartupTask, StartupTaskState};
use windows::core::HSTRING;

use super::com::ComGuard;

const STARTUP_TASK_ID: &str = "WinSPStartup";

pub fn is_enabled() -> bool {
    let _com = ComGuard::new();
    startup_task_is_enabled().unwrap_or(false)
}

pub fn set_enabled(enabled: bool) {
    let _com = ComGuard::new();
    match startup_task_set_enabled(enabled) {
        Ok(state) => {
            let is_enabled = matches!(
                state,
                StartupTaskState::Enabled | StartupTaskState::EnabledByPolicy
            );
            if enabled && !is_enabled {
                super::toast::show(
                    "WinSP",
                    "Windows denied the request to start with Windows. Check Task Manager's Startup Apps settings or your system policy.",
                );
            }
        }
        Err(error) => {
            super::toast::show(
                "WinSP",
                &format!("Failed to update Start with Windows: {error}"),
            );
        }
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

fn startup_task_set_enabled(enabled: bool) -> windows::core::Result<StartupTaskState> {
    let task = startup_task()?;
    if enabled {
        task.RequestEnableAsync()?.join()
    } else {
        task.Disable()?;
        Ok(StartupTaskState::Disabled)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_enabled_degrades_gracefully_without_package_identity() {
        assert!(!is_enabled());
    }
}
