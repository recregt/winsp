use windows::ApplicationModel::{StartupTask, StartupTaskState};
use windows::core::HSTRING;

use super::com::ComGuard;

pub enum SetAutostartError {
    DeniedByPolicy,
    Api(windows::core::Error),
}

impl std::fmt::Display for SetAutostartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SetAutostartError::DeniedByPolicy => write!(
                f,
                "Windows denied the request to start with Windows. Check Task Manager's Startup Apps settings or your system policy."
            ),
            SetAutostartError::Api(error) => {
                write!(f, "Failed to update Start with Windows: {error}")
            }
        }
    }
}

pub fn is_enabled(task_id: &str) -> bool {
    let _com = ComGuard::new();
    startup_task_is_enabled(task_id).unwrap_or(false)
}

pub fn set_enabled(task_id: &str, enabled: bool) -> Result<(), SetAutostartError> {
    let _com = ComGuard::new();
    match startup_task_set_enabled(task_id, enabled) {
        Ok(state) => {
            let is_enabled = matches!(
                state,
                StartupTaskState::Enabled | StartupTaskState::EnabledByPolicy
            );
            if enabled && !is_enabled {
                Err(SetAutostartError::DeniedByPolicy)
            } else {
                Ok(())
            }
        }
        Err(error) => Err(SetAutostartError::Api(error)),
    }
}

fn startup_task(task_id: &str) -> windows::core::Result<StartupTask> {
    StartupTask::GetAsync(&HSTRING::from(task_id))?.join()
}

fn startup_task_is_enabled(task_id: &str) -> windows::core::Result<bool> {
    let state = startup_task(task_id)?.State()?;
    Ok(matches!(
        state,
        StartupTaskState::Enabled | StartupTaskState::EnabledByPolicy
    ))
}

fn startup_task_set_enabled(
    task_id: &str,
    enabled: bool,
) -> windows::core::Result<StartupTaskState> {
    let task = startup_task(task_id)?;
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
        assert!(!is_enabled("WinSpTest_Autostart"));
    }
}
