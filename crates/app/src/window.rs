#[cfg(windows)]
pub mod win32_window {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::sync::{Arc, Mutex, OnceLock};
    use windows_sys::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
    use windows_sys::Win32::Graphics::Dwm::{
        DWMSBT_TRANSIENTWINDOW, DWMWA_SYSTEMBACKDROP_TYPE, DWMWA_WINDOW_CORNER_PREFERENCE,
        DWMWCP_ROUND, DwmSetWindowAttribute,
    };
    use windows_sys::Win32::Graphics::Gdi::{
        BeginPaint, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreateFontW, CreatePen,
        CreateSolidBrush, DT_LEFT, DT_SINGLELINE, DT_VCENTER, DeleteDC, DeleteObject, DrawTextW,
        EndPaint, FW_NORMAL, FW_SEMIBOLD, FillRect, GetStockObject, HDC, InvalidateRect, LineTo,
        MoveToEx, PAINTSTRUCT, PS_SOLID, SRCCOPY, SelectObject, SetBkMode, SetTextColor,
        TRANSPARENT, WHITE_BRUSH,
    };
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::UI::HiDpi::{
        DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext,
    };
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
    use windows_sys::Win32::UI::Shell::{
        NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW, Shell_NotifyIconW,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::*;

    use crate::state::AppState;

    pub const WINDOW_WIDTH: i32 = 680;
    pub const SEARCH_BAR_HEIGHT: i32 = 64;
    pub const ITEM_ROW_HEIGHT: i32 = 54;
    pub const PADDING: i32 = 12;
    pub const WINDOW_CLASS_NAME: &str = "WinSP_Spotlight_Window";

    const WM_TRAYICON: u32 = WM_APP + 1;
    const TRAY_ICON_ID: u32 = 1;
    const ID_TRAY_TOGGLE: usize = 1001;
    const ID_TRAY_EXIT: usize = 1002;
    const ID_TRAY_AUTOSTART: usize = 1003;

    static APP_STATE: OnceLock<Arc<Mutex<AppState>>> = OnceLock::new();

    fn to_wide(s: &str) -> Vec<u16> {
        OsStr::new(s).encode_wide().chain(Some(0)).collect()
    }

    pub fn run_app(state: Arc<Mutex<AppState>>) -> Result<(), String> {
        let _ = APP_STATE.set(state);

        unsafe {
            SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);

            let instance = GetModuleHandleW(std::ptr::null());
            let class_name = to_wide(WINDOW_CLASS_NAME);
            let window_title = to_wide("WinSP");

            let wnd_class = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(wnd_proc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: instance,
                hIcon: LoadIconW(instance, 1u16 as _),
                hCursor: LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
                hbrBackground: GetStockObject(WHITE_BRUSH) as _,
                lpszMenuName: std::ptr::null(),
                lpszClassName: class_name.as_ptr(),
                hIconSm: LoadIconW(instance, 1u16 as _),
            };

            RegisterClassExW(&wnd_class);

            let hwnd = CreateWindowExW(
                WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
                class_name.as_ptr(),
                window_title.as_ptr(),
                WS_POPUP | WS_VISIBLE,
                100,
                100,
                WINDOW_WIDTH,
                SEARCH_BAR_HEIGHT,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                instance,
                std::ptr::null(),
            );

            if hwnd.is_null() {
                return Err("Failed to create WinSP window".into());
            }

            // Enable Windows 11 Acrylic backdrop & Rounded Corners
            let backdrop = DWMSBT_TRANSIENTWINDOW;
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_SYSTEMBACKDROP_TYPE as u32,
                &backdrop as *const _ as *const _,
                std::mem::size_of_val(&backdrop) as u32,
            );

            let corner_pref = DWMWCP_ROUND;
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_WINDOW_CORNER_PREFERENCE as u32,
                &corner_pref as *const _ as *const _,
                std::mem::size_of_val(&corner_pref) as u32,
            );

            // Center window on screen
            center_window(hwnd);

            // Register Alt + Space global hotkey (MOD_ALT = 0x0001, VK_SPACE = 0x20)
            let _ = RegisterHotKey(hwnd, 1, MOD_ALT, VK_SPACE as u32);

            add_tray_icon(hwnd);

            ShowWindow(hwnd, SW_SHOW);
            SetForegroundWindow(hwnd);

            // Win32 Message Loop
            let mut msg = std::mem::zeroed::<MSG>();
            while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            let _ = UnregisterHotKey(hwnd, 1);
            Ok(())
        }
    }

    fn tray_icon_data(hwnd: HWND) -> NOTIFYICONDATAW {
        let mut nid: NOTIFYICONDATAW = unsafe { std::mem::zeroed() };
        nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
        nid.hWnd = hwnd;
        nid.uID = TRAY_ICON_ID;
        nid
    }

    fn add_tray_icon(hwnd: HWND) {
        unsafe {
            let mut nid = tray_icon_data(hwnd);
            nid.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
            nid.uCallbackMessage = WM_TRAYICON;
            nid.hIcon = LoadIconW(GetModuleHandleW(std::ptr::null()), 1u16 as _);
            let tip = to_wide("WinSP");
            let len = tip.len().min(nid.szTip.len());
            nid.szTip[..len].copy_from_slice(&tip[..len]);
            Shell_NotifyIconW(NIM_ADD, &nid);
        }
    }

    fn remove_tray_icon(hwnd: HWND) {
        unsafe {
            let nid = tray_icon_data(hwnd);
            Shell_NotifyIconW(NIM_DELETE, &nid);
        }
    }

    fn toggle_visibility(hwnd: HWND) {
        unsafe {
            if IsWindowVisible(hwnd) != 0 {
                ShowWindow(hwnd, SW_HIDE);
            } else {
                center_window(hwnd);
                if let Some(state_arc) = APP_STATE.get() {
                    if let Ok(mut state) = state_arc.lock() {
                        state.clear_query();
                    }
                }
                ShowWindow(hwnd, SW_SHOW);
                SetForegroundWindow(hwnd);
                InvalidateRect(hwnd, std::ptr::null(), 1);
            }
        }
    }

    fn show_tray_menu(hwnd: HWND) {
        unsafe {
            let menu = CreatePopupMenu();
            let toggle_label = to_wide("Toggle Search");
            let autostart_label = to_wide("Start with Windows");
            let exit_label = to_wide("Exit");
            let autostart_flags = if crate::autostart::is_enabled() {
                MF_STRING | MF_CHECKED
            } else {
                MF_STRING | MF_UNCHECKED
            };
            AppendMenuW(menu, MF_STRING, ID_TRAY_TOGGLE, toggle_label.as_ptr());
            AppendMenuW(
                menu,
                autostart_flags,
                ID_TRAY_AUTOSTART,
                autostart_label.as_ptr(),
            );
            AppendMenuW(menu, MF_STRING, ID_TRAY_EXIT, exit_label.as_ptr());

            let mut cursor = std::mem::zeroed::<POINT>();
            GetCursorPos(&mut cursor);

            SetForegroundWindow(hwnd);
            TrackPopupMenu(
                menu,
                TPM_RIGHTBUTTON,
                cursor.x,
                cursor.y,
                0,
                hwnd,
                std::ptr::null(),
            );
            DestroyMenu(menu);
        }
    }

    fn center_window(hwnd: HWND) {
        unsafe {
            let screen_width = GetSystemMetrics(SM_CXSCREEN);
            let screen_height = GetSystemMetrics(SM_CYSCREEN);

            let x = (screen_width - WINDOW_WIDTH) / 2;
            let y = screen_height / 4; // Upper 1/3 like macOS Spotlight

            SetWindowPos(
                hwnd,
                HWND_TOPMOST,
                x,
                y,
                WINDOW_WIDTH,
                SEARCH_BAR_HEIGHT,
                SWP_NOACTIVATE,
            );
        }
    }

    unsafe extern "system" fn wnd_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match msg {
            WM_HOTKEY => {
                toggle_visibility(hwnd);
                0
            }
            WM_TRAYICON => {
                if lparam as u32 == WM_RBUTTONUP {
                    show_tray_menu(hwnd);
                }
                0
            }
            WM_COMMAND => {
                match wparam & 0xffff {
                    ID_TRAY_TOGGLE => toggle_visibility(hwnd),
                    ID_TRAY_AUTOSTART => {
                        crate::autostart::set_enabled(!crate::autostart::is_enabled());
                    }
                    ID_TRAY_EXIT => unsafe {
                        DestroyWindow(hwnd);
                    },
                    _ => {}
                }
                0
            }
            WM_KILLFOCUS => {
                // Auto-hide when focus is lost
                unsafe {
                    ShowWindow(hwnd, SW_HIDE);
                }
                0
            }
            WM_CHAR => {
                let c = char::from_u32(wparam as u32).unwrap_or('\0');
                if !c.is_control() {
                    if let Some(state_arc) = APP_STATE.get() {
                        if let Ok(mut state) = state_arc.lock() {
                            state.insert_char(c);
                            resize_window_for_results(hwnd, state.results.len());
                        }
                    }
                    unsafe {
                        InvalidateRect(hwnd, std::ptr::null(), 1);
                    }
                }
                0
            }
            WM_KEYDOWN => {
                let vk = wparam as u16;
                if let Some(state_arc) = APP_STATE.get() {
                    let mut should_resize = false;
                    let mut results_count = 0;
                    let mut should_hide = false;

                    if let Ok(mut state) = state_arc.lock() {
                        match vk {
                            VK_BACK => {
                                state.backspace();
                                should_resize = true;
                                results_count = state.results.len();
                            }
                            VK_DOWN | VK_TAB => {
                                state.select_next();
                            }
                            VK_UP => {
                                state.select_prev();
                            }
                            VK_RETURN => {
                                let _ = state.execute_selected();
                                should_hide = true;
                            }
                            VK_ESCAPE => {
                                should_hide = true;
                            }
                            _ => {}
                        }
                    }

                    unsafe {
                        if should_hide {
                            ShowWindow(hwnd, SW_HIDE);
                        } else {
                            if should_resize {
                                resize_window_for_results(hwnd, results_count);
                            }
                            InvalidateRect(hwnd, std::ptr::null(), 1);
                        }
                    }
                }
                0
            }
            WM_PAINT => {
                unsafe {
                    let mut ps = std::mem::zeroed::<PAINTSTRUCT>();
                    let hdc = BeginPaint(hwnd, &mut ps);
                    render_ui(hwnd, hdc);
                    EndPaint(hwnd, &ps);
                }
                0
            }
            WM_DESTROY => {
                remove_tray_icon(hwnd);
                unsafe {
                    PostQuitMessage(0);
                }
                0
            }
            _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
        }
    }

    fn resize_window_for_results(hwnd: HWND, results_count: usize) {
        unsafe {
            let height = if results_count == 0 {
                SEARCH_BAR_HEIGHT
            } else {
                SEARCH_BAR_HEIGHT + (results_count as i32 * ITEM_ROW_HEIGHT) + PADDING
            };

            let mut rect = std::mem::zeroed::<RECT>();
            let _ = GetWindowRect(hwnd, &mut rect);
            SetWindowPos(
                hwnd,
                HWND_TOPMOST,
                rect.left,
                rect.top,
                WINDOW_WIDTH,
                height,
                SWP_NOMOVE | SWP_NOACTIVATE,
            );
        }
    }

    unsafe fn render_ui(hwnd: HWND, hdc: HDC) {
        unsafe {
            let mut client_rect = std::mem::zeroed::<RECT>();
            let _ = GetClientRect(hwnd, &mut client_rect);

            // Double buffering to eliminate flicker
            let mem_dc = CreateCompatibleDC(hdc);
            let mem_bmp = CreateCompatibleBitmap(hdc, client_rect.right, client_rect.bottom);
            let old_bmp = SelectObject(mem_dc, mem_bmp);

            // Background brush (Dark / Translucent tone: #1E1E1E)
            let bg_brush = CreateSolidBrush(0x001E1E1E as COLORREF);
            FillRect(mem_dc, &client_rect, bg_brush);
            DeleteObject(bg_brush);

            SetBkMode(mem_dc, TRANSPARENT as i32);

            if let Some(state_arc) = APP_STATE.get() {
                if let Ok(state) = state_arc.lock() {
                    // 1. Draw Search Bar Text
                    let font_name = to_wide("Segoe UI Variable Display");
                    let font_title = CreateFontW(
                        26,
                        0,
                        0,
                        0,
                        FW_NORMAL as i32,
                        0,
                        0,
                        0,
                        0,
                        0,
                        0,
                        0,
                        0,
                        font_name.as_ptr(),
                    );
                    let old_font = SelectObject(mem_dc, font_title);

                    let display_text = if state.query.is_empty() {
                        SetTextColor(mem_dc, 0x00888888);
                        "Search apps, settings, math...".to_string()
                    } else {
                        SetTextColor(mem_dc, 0x00FFFFFF);
                        state.query.clone()
                    };

                    let mut text_wide = to_wide(&display_text);
                    let mut search_rect = RECT {
                        left: 24,
                        top: 14,
                        right: WINDOW_WIDTH - 24,
                        bottom: SEARCH_BAR_HEIGHT - 10,
                    };
                    DrawTextW(
                        mem_dc,
                        text_wide.as_mut_ptr(),
                        text_wide.len() as i32 - 1,
                        &mut search_rect,
                        DT_LEFT | DT_SINGLELINE | DT_VCENTER,
                    );

                    // 2. Draw Results
                    let font_item_name = to_wide("Segoe UI Variable Text");
                    let font_item_title = CreateFontW(
                        18,
                        0,
                        0,
                        0,
                        FW_SEMIBOLD as i32,
                        0,
                        0,
                        0,
                        0,
                        0,
                        0,
                        0,
                        0,
                        font_item_name.as_ptr(),
                    );
                    let font_item_sub = CreateFontW(
                        14,
                        0,
                        0,
                        0,
                        FW_NORMAL as i32,
                        0,
                        0,
                        0,
                        0,
                        0,
                        0,
                        0,
                        0,
                        font_item_name.as_ptr(),
                    );

                    let mut current_y = SEARCH_BAR_HEIGHT;

                    // Subtle separator line
                    if !state.results.is_empty() {
                        let sep_pen = CreatePen(PS_SOLID, 1, 0x00333333);
                        let old_pen = SelectObject(mem_dc, sep_pen);
                        MoveToEx(mem_dc, 16, current_y, std::ptr::null_mut());
                        LineTo(mem_dc, WINDOW_WIDTH - 16, current_y);
                        SelectObject(mem_dc, old_pen);
                        DeleteObject(sep_pen);
                        current_y += 8;
                    }

                    for (idx, result) in state.results.iter().enumerate() {
                        let is_selected = idx == state.selected_index;

                        let row_rect = RECT {
                            left: 12,
                            top: current_y,
                            right: WINDOW_WIDTH - 12,
                            bottom: current_y + ITEM_ROW_HEIGHT - 6,
                        };

                        // Draw selection highlight capsule
                        if is_selected {
                            let sel_brush = CreateSolidBrush(0x003A3A3A);
                            FillRect(mem_dc, &row_rect, sel_brush);
                            DeleteObject(sel_brush);
                        }

                        // Title
                        SelectObject(mem_dc, font_item_title);
                        SetTextColor(mem_dc, if is_selected { 0x00FFFFFF } else { 0x00E0E0E0 });
                        let mut title_wide = to_wide(&result.title);
                        let mut title_rect = RECT {
                            left: 32,
                            top: current_y + 4,
                            right: WINDOW_WIDTH - 32,
                            bottom: current_y + 26,
                        };
                        DrawTextW(
                            mem_dc,
                            title_wide.as_mut_ptr(),
                            title_wide.len() as i32 - 1,
                            &mut title_rect,
                            DT_LEFT | DT_SINGLELINE | DT_VCENTER,
                        );

                        // Subtitle
                        if let Some(sub) = &result.subtitle {
                            SelectObject(mem_dc, font_item_sub);
                            SetTextColor(mem_dc, 0x00999999);
                            let mut sub_wide = to_wide(sub);
                            let mut sub_rect = RECT {
                                left: 32,
                                top: current_y + 26,
                                right: WINDOW_WIDTH - 32,
                                bottom: current_y + ITEM_ROW_HEIGHT - 8,
                            };
                            DrawTextW(
                                mem_dc,
                                sub_wide.as_mut_ptr(),
                                sub_wide.len() as i32 - 1,
                                &mut sub_rect,
                                DT_LEFT | DT_SINGLELINE | DT_VCENTER,
                            );
                        }

                        current_y += ITEM_ROW_HEIGHT;
                    }

                    SelectObject(mem_dc, old_font);
                    DeleteObject(font_title);
                    DeleteObject(font_item_title);
                    DeleteObject(font_item_sub);
                }
            }

            // Copy back buffer to screen
            BitBlt(
                hdc,
                0,
                0,
                client_rect.right,
                client_rect.bottom,
                mem_dc,
                0,
                0,
                SRCCOPY,
            );

            SelectObject(mem_dc, old_bmp);
            DeleteObject(mem_bmp);
            DeleteDC(mem_dc);
        }
    }
}
