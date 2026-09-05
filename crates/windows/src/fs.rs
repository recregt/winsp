use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use windows::Win32::Foundation::{
    CloseHandle, ERROR_ACCESS_DENIED, ERROR_FILE_EXISTS, ERROR_FILE_NOT_FOUND,
    ERROR_SHARING_VIOLATION, ERROR_UNABLE_TO_MOVE_REPLACEMENT, ERROR_UNABLE_TO_MOVE_REPLACEMENT_2,
    ERROR_UNABLE_TO_REMOVE_REPLACED, GENERIC_WRITE, HANDLE,
};
use windows::Win32::Storage::FileSystem::{
    CREATE_NEW, CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_NONE, FlushFileBuffers,
    MOVEFILE_REPLACE_EXISTING, MoveFileExW, REPLACEFILE_IGNORE_MERGE_ERRORS, ReplaceFileW,
    WriteFile,
};
use windows::core::HSTRING;

const MAX_NAME_ATTEMPTS: u32 = 8;
const MAX_REPLACE_RETRIES: u32 = 20;
const INITIAL_BACKOFF: Duration = Duration::from_millis(2);
const MAX_BACKOFF: Duration = Duration::from_millis(80);

pub fn atomic_write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    ensure_dir(parent)?;
    let (tmp_path, handle) = create_unique_file(parent)?;

    let written = write_all(handle, bytes);
    unsafe {
        let _ = CloseHandle(handle);
    }
    if let Err(err) = written {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(err);
    }

    replace_with_retries(path, &tmp_path)
}

fn ensure_dir(dir: &Path) -> io::Result<()> {
    match std::fs::create_dir_all(dir) {
        Ok(()) => Ok(()),
        Err(_) if dir.is_dir() => Ok(()),
        Err(err) => Err(err),
    }
}

fn create_unique_file(parent: &Path) -> io::Result<(PathBuf, HANDLE)> {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let pid = std::process::id();

    for _ in 0..MAX_NAME_ATTEMPTS {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|elapsed| elapsed.as_nanos())
            .unwrap_or_default();
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let candidate = parent.join(format!("settings.{pid:x}.{nanos:x}.{unique:x}.tmp"));
        let name = HSTRING::from(candidate.as_os_str());

        let result = unsafe {
            CreateFileW(
                &name,
                GENERIC_WRITE.0,
                FILE_SHARE_NONE,
                None,
                CREATE_NEW,
                FILE_ATTRIBUTE_NORMAL,
                None,
            )
        };

        match result {
            Ok(handle) => return Ok((candidate, handle)),
            Err(_)
                if io::Error::last_os_error().raw_os_error()
                    == Some(ERROR_FILE_EXISTS.0 as i32) =>
            {
                continue;
            }
            Err(_) => return Err(io::Error::last_os_error()),
        }
    }

    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not create a unique temp file",
    ))
}

fn write_all(handle: HANDLE, mut bytes: &[u8]) -> io::Result<()> {
    while !bytes.is_empty() {
        let mut written = 0u32;
        unsafe { WriteFile(handle, Some(bytes), Some(&mut written), None) }
            .map_err(|_| io::Error::last_os_error())?;
        if written == 0 {
            return Err(io::Error::new(
                io::ErrorKind::WriteZero,
                "WriteFile wrote zero bytes",
            ));
        }
        bytes = &bytes[written as usize..];
    }
    unsafe { FlushFileBuffers(handle) }.map_err(|_| io::Error::last_os_error())
}

fn replace_with_retries(dest: &Path, tmp: &Path) -> io::Result<()> {
    let dest_name = HSTRING::from(dest.as_os_str());
    let tmp_name = HSTRING::from(tmp.as_os_str());

    let mut retries_left = MAX_REPLACE_RETRIES;
    let mut backoff = INITIAL_BACKOFF;
    loop {
        let mut result = unsafe {
            ReplaceFileW(
                &dest_name,
                &tmp_name,
                None,
                REPLACEFILE_IGNORE_MERGE_ERRORS,
                None,
                None,
            )
        };

        if result.is_err()
            && io::Error::last_os_error().raw_os_error() == Some(ERROR_FILE_NOT_FOUND.0 as i32)
        {
            result = unsafe { MoveFileExW(&tmp_name, &dest_name, MOVEFILE_REPLACE_EXISTING) };
        }

        if result.is_ok() {
            return Ok(());
        }

        let code = io::Error::last_os_error().raw_os_error();
        let retriable = code == Some(ERROR_SHARING_VIOLATION.0 as i32)
            || code == Some(ERROR_ACCESS_DENIED.0 as i32)
            || code == Some(ERROR_FILE_NOT_FOUND.0 as i32)
            || code == Some(ERROR_UNABLE_TO_MOVE_REPLACEMENT.0 as i32)
            || code == Some(ERROR_UNABLE_TO_MOVE_REPLACEMENT_2.0 as i32)
            || code == Some(ERROR_UNABLE_TO_REMOVE_REPLACED.0 as i32);
        if retriable && retries_left > 0 {
            retries_left -= 1;
            std::thread::sleep(backoff);
            backoff = (backoff * 2).min(MAX_BACKOFF);
            continue;
        }

        let err = io::Error::last_os_error();
        let _ = std::fs::remove_file(tmp);
        return Err(err);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::Foundation::GENERIC_READ;
    use windows::Win32::Storage::FileSystem::{FILE_SHARE_READ, OPEN_EXISTING};

    #[test]
    fn atomic_write_creates_a_new_file_when_none_exists() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.msgpack");

        atomic_write(&path, b"hello").unwrap();

        assert_eq!(std::fs::read(&path).unwrap(), b"hello");
    }

    #[test]
    fn atomic_write_replaces_an_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.msgpack");
        atomic_write(&path, b"first").unwrap();

        atomic_write(&path, b"second").unwrap();

        assert_eq!(std::fs::read(&path).unwrap(), b"second");
    }

    #[test]
    fn atomic_write_leaves_no_temp_file_behind() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.msgpack");

        atomic_write(&path, b"hello").unwrap();

        let leftover: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.file_name())
            .filter(|name| *name != "settings.msgpack")
            .collect();
        assert!(
            leftover.is_empty(),
            "unexpected leftover files: {leftover:?}"
        );
    }

    #[test]
    fn concurrent_atomic_writes_to_the_same_path_never_fail_or_corrupt_it() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.msgpack");

        let (path_a, path_b) = (path.clone(), path.clone());
        let writer_a = std::thread::spawn(move || {
            for _ in 0..50 {
                atomic_write(&path_a, b"a").unwrap();
            }
        });
        let writer_b = std::thread::spawn(move || {
            for _ in 0..50 {
                atomic_write(&path_b, b"bb").unwrap();
            }
        });

        writer_a.join().unwrap();
        writer_b.join().unwrap();

        let loaded = std::fs::read(&path).unwrap();
        assert!(
            loaded == b"a" || loaded == b"bb",
            "concurrent writes must not corrupt the file into anything other than one of the written values"
        );
    }

    #[test]
    fn atomic_write_retries_past_a_transient_sharing_violation() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.msgpack");
        atomic_write(&path, b"first").unwrap();

        let blocking_name = HSTRING::from(path.as_os_str());
        let blocker = unsafe {
            CreateFileW(
                &blocking_name,
                GENERIC_READ.0,
                FILE_SHARE_READ,
                None,
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                None,
            )
        }
        .expect("setup: should be able to open the file for reading");

        let path_for_writer = path.clone();
        let writer = std::thread::spawn(move || atomic_write(&path_for_writer, b"second"));

        std::thread::sleep(Duration::from_millis(20));
        unsafe {
            let _ = CloseHandle(blocker);
        }

        writer
            .join()
            .unwrap()
            .expect("atomic_write should recover once the blocking handle closes");
        assert_eq!(std::fs::read(&path).unwrap(), b"second");
    }
}
