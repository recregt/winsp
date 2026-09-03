use windows_registry::Key;

pub(crate) fn read_dword(hive: &Key, subkey: &str, value: &str) -> Option<u32> {
    hive.open(subkey).ok()?.get_u32(value).ok()
}
