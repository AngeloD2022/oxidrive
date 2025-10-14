use sysinfo::System;

#[macro_export]
macro_rules! strmap {
    ($($key:expr => $val:expr),* $(,)?) => {{
        let mut map = HashMap::new();
        $(map.insert($key.to_string(), $val.to_string());)*
        map
    }};
}

pub fn get_os_version() -> String {
    let sys = System::new_all();
    let os_version = System::os_version().unwrap_or_else(|| "0".to_owned());
    os_version
}
