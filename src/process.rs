use windows::Win32::{
    Foundation::CloseHandle,
    System::{
        Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
            TH32CS_SNAPPROCESS,
        },
        Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE},
    },
};

pub unsafe fn get_process_pid(name: String) -> Result<u32, Box<dyn std::error::Error>> {
    let handle = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)?;

    let mut process_entry: PROCESSENTRY32W = std::mem::zeroed();
    process_entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;

    Process32FirstW(handle, &mut process_entry as *mut PROCESSENTRY32W)?;

    while Process32NextW(handle, &mut process_entry as *mut PROCESSENTRY32W).is_ok() {
        let process_name = String::from_utf16(&process_entry.szExeFile)?;
        if process_name.contains(&name) {
            CloseHandle(handle)?;
            return Ok(process_entry.th32ProcessID);
        }
    }

    return Err("failed to find process".into());
}

pub unsafe fn kill_process(pid: u32) -> Result<(), Box<dyn std::error::Error>> {
    let h_process = OpenProcess(PROCESS_TERMINATE, false, pid)?;
    TerminateProcess(h_process, 0)?;
    CloseHandle(h_process)?;

    Ok(())
}

pub fn kill_epic_games_launcher() -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        let pid = get_process_pid("EpicGamesLauncher.exe".to_string())?;

        kill_process(pid)?;
    }

    Ok(())
}