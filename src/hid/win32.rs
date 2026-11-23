use std::ffi::c_void;
use std::mem::{size_of, zeroed};
use std::ptr::{null, null_mut};

use crate::hid::protocol::{
    ATLAS_PID_WIRED, ATLAS_PID_WIRELESS, ATLAS_VID, FEATURE_USAGE, FEATURE_USAGE_PAGE,
    MONITOR_USAGE, MONITOR_USAGE_PAGE,
};
use crate::model::ConnectionMode;

type HANDLE = *mut c_void;
type HDEVINFO = *mut c_void;
type BOOL = i32;
type DWORD = u32;
type ULONG = u32;
type USHORT = u16;
type BOOLEAN = u8;

const INVALID_HANDLE_VALUE: HANDLE = (-1isize) as HANDLE;
const GENERIC_READ: DWORD = 0x8000_0000;
const GENERIC_WRITE: DWORD = 0x4000_0000;
const FILE_SHARE_READ: DWORD = 0x0000_0001;
const FILE_SHARE_WRITE: DWORD = 0x0000_0002;
const OPEN_EXISTING: DWORD = 3;
const FILE_FLAG_OVERLAPPED: DWORD = 0x4000_0000;
const DIGCF_PRESENT: DWORD = 0x0000_0002;
const DIGCF_DEVICEINTERFACE: DWORD = 0x0000_0010;
const ERROR_IO_PENDING: DWORD = 997;
const WAIT_OBJECT_0: DWORD = 0;

#[repr(C)]
#[derive(Clone, Copy)]
struct GUID {
    data1: u32,
    data2: u16,
    data3: u16,
    data4: [u8; 8],
}

#[repr(C)]
struct SP_DEVICE_INTERFACE_DATA {
    cb_size: DWORD,
    interface_class_guid: GUID,
    flags: DWORD,
    reserved: usize,
}

#[repr(C)]
struct HIDD_ATTRIBUTES {
    size: ULONG,
    vendor_id: USHORT,
    product_id: USHORT,
    version_number: USHORT,
}

#[repr(C)]
struct HIDP_CAPS {
    usage: USHORT,
    usage_page: USHORT,
    input_report_byte_length: USHORT,
    output_report_byte_length: USHORT,
    feature_report_byte_length: USHORT,
    reserved: [USHORT; 17],
    number_link_collection_nodes: USHORT,
    number_input_button_caps: USHORT,
    number_input_value_caps: USHORT,
    number_input_data_indices: USHORT,
    number_output_button_caps: USHORT,
    number_output_value_caps: USHORT,
    number_output_data_indices: USHORT,
    number_feature_button_caps: USHORT,
    number_feature_value_caps: USHORT,
    number_feature_data_indices: USHORT,
}

#[repr(C)]
struct OVERLAPPED {
    internal: usize,
    internal_high: usize,
    offset: DWORD,
    offset_high: DWORD,
    h_event: HANDLE,
}

#[link(name = "hid")]
unsafe extern "system" {
    fn HidD_GetHidGuid(hid_guid: *mut GUID);
    fn HidD_GetAttributes(hid_device_object: HANDLE, attributes: *mut HIDD_ATTRIBUTES) -> BOOLEAN;
    fn HidD_GetPreparsedData(
        hid_device_object: HANDLE,
        preparsed_data: *mut *mut c_void,
    ) -> BOOLEAN;
    fn HidD_FreePreparsedData(preparsed_data: *mut c_void) -> BOOLEAN;
    fn HidP_GetCaps(preparsed_data: *mut c_void, capabilities: *mut HIDP_CAPS) -> i32;
    fn HidD_SetFeature(
        hid_device_object: HANDLE,
        report_buffer: *const c_void,
        report_buffer_length: ULONG,
    ) -> BOOLEAN;
    fn HidD_GetFeature(
        hid_device_object: HANDLE,
        report_buffer: *mut c_void,
        report_buffer_length: ULONG,
    ) -> BOOLEAN;
}

#[link(name = "setupapi")]
unsafe extern "system" {
    fn SetupDiGetClassDevsW(
        class_guid: *const GUID,
        enumerator: *const u16,
        hwnd_parent: HANDLE,
        flags: DWORD,
    ) -> HDEVINFO;
    fn SetupDiEnumDeviceInterfaces(
        device_info_set: HDEVINFO,
        device_info_data: *mut c_void,
        interface_class_guid: *const GUID,
        member_index: DWORD,
        device_interface_data: *mut SP_DEVICE_INTERFACE_DATA,
    ) -> BOOL;
    fn SetupDiGetDeviceInterfaceDetailW(
        device_info_set: HDEVINFO,
        device_interface_data: *mut SP_DEVICE_INTERFACE_DATA,
        device_interface_detail_data: *mut c_void,
        device_interface_detail_data_size: DWORD,
        required_size: *mut DWORD,
        device_info_data: *mut c_void,
    ) -> BOOL;
    fn SetupDiDestroyDeviceInfoList(device_info_set: HDEVINFO) -> BOOL;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn CreateFileW(
        file_name: *const u16,
        desired_access: DWORD,
        share_mode: DWORD,
        security_attributes: *mut c_void,
        creation_disposition: DWORD,
        flags_and_attributes: DWORD,
        template_file: HANDLE,
    ) -> HANDLE;
    fn CloseHandle(h_object: HANDLE) -> BOOL;
    fn ReadFile(
        h_file: HANDLE,
        lp_buffer: *mut c_void,
        n_number_of_bytes_to_read: DWORD,
        lp_number_of_bytes_read: *mut DWORD,
        lp_overlapped: *mut OVERLAPPED,
    ) -> BOOL;
    fn CreateEventW(
        lp_event_attributes: *mut c_void,
        b_manual_reset: BOOL,
        b_initial_state: BOOL,
        lp_name: *const u16,
    ) -> HANDLE;
    fn WaitForSingleObject(h_handle: HANDLE, dw_milliseconds: DWORD) -> DWORD;
    fn GetOverlappedResult(
        h_file: HANDLE,
        lp_overlapped: *mut OVERLAPPED,
        lp_number_of_bytes_transferred: *mut DWORD,
        b_wait: BOOL,
    ) -> BOOL;
    fn CancelIo(h_file: HANDLE) -> BOOL;
    fn GetLastError() -> DWORD;
}

#[derive(Debug, Clone)]
pub struct DiscoveredEndpoint {
    pub path_w: Vec<u16>,
    pub vid: u16,
    pub pid: u16,
    pub usage_page: u16,
    pub usage: u16,
    pub input_len: u16,
    pub feature_len: u16,
}

pub struct AtlasHidDevice {
    pub mode: ConnectionMode,
    pub vid: u16,
    pub pid: u16,
    feature_handle: HANDLE,
    feature_report_len: usize,
    monitor_handle: Option<HANDLE>,
    monitor_input_len: usize,
}

// Win32 file handles are safe to move to our dedicated HID worker thread.
unsafe impl Send for AtlasHidDevice {}

impl Drop for AtlasHidDevice {
    fn drop(&mut self) {
        unsafe {
            if !self.feature_handle.is_null() && self.feature_handle != INVALID_HANDLE_VALUE {
                CloseHandle(self.feature_handle);
            }
            if let Some(mh) = self.monitor_handle {
                if !mh.is_null() && mh != INVALID_HANDLE_VALUE {
                    CancelIo(mh);
                    CloseHandle(mh);
                }
            }
        }
    }
}

/// Enumerate all Press Play ATLAS HID interfaces (both Wired 1D57:2112 and Wireless 1D57:FA60).
pub fn enumerate_atlas_endpoints() -> Vec<DiscoveredEndpoint> {
    let mut found = Vec::new();
    unsafe {
        let mut hid_guid: GUID = zeroed();
        HidD_GetHidGuid(&mut hid_guid);

        let dev_info = SetupDiGetClassDevsW(
            &hid_guid,
            null(),
            null_mut(),
            DIGCF_PRESENT | DIGCF_DEVICEINTERFACE,
        );
        if dev_info.is_null() || dev_info == INVALID_HANDLE_VALUE {
            return found;
        }

        let mut index: DWORD = 0;
        loop {
            let mut if_data: SP_DEVICE_INTERFACE_DATA = zeroed();
            if_data.cb_size = size_of::<SP_DEVICE_INTERFACE_DATA>() as DWORD;

            if SetupDiEnumDeviceInterfaces(dev_info, null_mut(), &hid_guid, index, &mut if_data)
                == 0
            {
                break;
            }
            index += 1;

            let mut required_size: DWORD = 0;
            SetupDiGetDeviceInterfaceDetailW(
                dev_info,
                &mut if_data,
                null_mut(),
                0,
                &mut required_size,
                null_mut(),
            );
            if required_size < 8 || required_size > 8192 {
                continue;
            }

            let mut detail_buf = vec![0u8; required_size as usize];
            // On 64-bit Windows, sizeof(SP_DEVICE_INTERFACE_DETAIL_DATA_W) is 8 (DWORD cbSize + WCHAR[1] + 2 bytes padding).
            let cb_size: DWORD = if cfg!(target_pointer_width = "64") { 8 } else { 6 };
            detail_buf[0..4].copy_from_slice(&cb_size.to_le_bytes());

            if SetupDiGetDeviceInterfaceDetailW(
                dev_info,
                &mut if_data,
                detail_buf.as_mut_ptr() as *mut c_void,
                required_size,
                null_mut(),
                null_mut(),
            ) == 0
            {
                continue;
            }

            // DevicePath starts at byte offset 4 as UTF-16LE null-terminated string.
            let path_u16: Vec<u16> = detail_buf[4..]
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .take_while(|&w| w != 0)
                .chain(std::iter::once(0))
                .collect();

            // Probe metadata with zero access first so we never lock unrelated system mice/keyboards.
            let probe = CreateFileW(
                path_u16.as_ptr(),
                0,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                null_mut(),
                OPEN_EXISTING,
                0,
                null_mut(),
            );
            if probe.is_null() || probe == INVALID_HANDLE_VALUE {
                continue;
            }

            let mut attrs: HIDD_ATTRIBUTES = zeroed();
            attrs.size = size_of::<HIDD_ATTRIBUTES>() as ULONG;
            let ok_attr = HidD_GetAttributes(probe, &mut attrs) != 0;

            if ok_attr
                && attrs.vendor_id == ATLAS_VID
                && (attrs.product_id == ATLAS_PID_WIRED || attrs.product_id == ATLAS_PID_WIRELESS)
            {
                let mut preparsed: *mut c_void = null_mut();
                if HidD_GetPreparsedData(probe, &mut preparsed) != 0 && !preparsed.is_null() {
                    let mut caps: HIDP_CAPS = zeroed();
                    let status = HidP_GetCaps(preparsed, &mut caps);
                    HidD_FreePreparsedData(preparsed);
                    if status >= 0 {
                        found.push(DiscoveredEndpoint {
                            path_w: path_u16,
                            vid: attrs.vendor_id,
                            pid: attrs.product_id,
                            usage_page: caps.usage_page,
                            usage: caps.usage,
                            input_len: caps.input_report_byte_length,
                            feature_len: caps.feature_report_byte_length,
                        });
                    }
                }
            }

            CloseHandle(probe);
        }

        SetupDiDestroyDeviceInfoList(dev_info);
    }
    found
}

impl AtlasHidDevice {
    /// Tries to open Wired (`1D57:2112`) first, then Wireless (`1D57:FA60`).
    pub fn open_any() -> Option<Self> {
        let endpoints = enumerate_atlas_endpoints();
        if endpoints.is_empty() {
            return None;
        }

        // Prefer Wired if both dongle and USB cable are connected (matches Mouse.exe priority!)
        for (pid, mode) in [
            (ATLAS_PID_WIRED, ConnectionMode::Wired),
            (ATLAS_PID_WIRELESS, ConnectionMode::Wireless24G),
        ] {
            let feature_ep = endpoints.iter().find(|e| {
                e.pid == pid && e.usage_page == FEATURE_USAGE_PAGE && e.usage == FEATURE_USAGE
            });

            if let Some(fep) = feature_ep {
                let fh = unsafe {
                    CreateFileW(
                        fep.path_w.as_ptr(),
                        GENERIC_READ | GENERIC_WRITE,
                        FILE_SHARE_READ | FILE_SHARE_WRITE,
                        null_mut(),
                        OPEN_EXISTING,
                        0,
                        null_mut(),
                    )
                };
                if fh.is_null() || fh == INVALID_HANDLE_VALUE {
                    continue;
                }

                // Also try opening the Monitor Input endpoint (UsagePage 0x000A) for live battery/DPI events.
                let monitor_ep = endpoints.iter().find(|e| {
                    e.pid == pid && e.usage_page == MONITOR_USAGE_PAGE && e.usage == MONITOR_USAGE
                });

                let (monitor_handle, monitor_input_len) = if let Some(mep) = monitor_ep {
                    let mh = unsafe {
                        CreateFileW(
                            mep.path_w.as_ptr(),
                            GENERIC_READ | GENERIC_WRITE,
                            FILE_SHARE_READ | FILE_SHARE_WRITE,
                            null_mut(),
                            OPEN_EXISTING,
                            FILE_FLAG_OVERLAPPED,
                            null_mut(),
                        )
                    };
                    if !mh.is_null() && mh != INVALID_HANDLE_VALUE {
                        (Some(mh), mep.input_len.max(8) as usize)
                    } else {
                        (None, 0)
                    }
                } else {
                    (None, 0)
                };

                return Some(Self {
                    mode,
                    vid: fep.vid,
                    pid: fep.pid,
                    feature_handle: fh,
                    feature_report_len: fep.feature_len as usize,
                    monitor_handle,
                    monitor_input_len,
                });
            }
        }

        None
    }

    /// Sends a HID Feature Report using `HidD_SetFeature`.
    /// Pads the buffer to the endpoint's `feature_report_len` if the OS requires a larger exact length,
    /// while preserving exact report lengths when `feature_report_len == 0` or smaller.
    pub fn set_feature(&self, data: &[u8]) -> Result<(), String> {
        let mut buf = data.to_vec();
        if self.feature_report_len > buf.len() {
            // Try exact length first (matches Mouse.exe passing 0x0A, 0x0F, 0x09, 0x38, 0x3B, 0x40),
            // and fall back to padded feature_report_len if Windows HID class rejects short buffer.
            let ok_exact = unsafe {
                HidD_SetFeature(
                    self.feature_handle,
                    buf.as_ptr() as *const c_void,
                    buf.len() as ULONG,
                )
            };
            if ok_exact != 0 {
                return Ok(());
            }
            buf.resize(self.feature_report_len, 0);
        }

        let ok = unsafe {
            HidD_SetFeature(
                self.feature_handle,
                buf.as_ptr() as *const c_void,
                buf.len() as ULONG,
            )
        };
        if ok != 0 {
            Ok(())
        } else {
            let err = unsafe { GetLastError() };
            Err(format!("HidD_SetFeature failed (Win32 error {err})"))
        }
    }

    #[allow(dead_code)]
    pub fn get_feature(&self, report_id: u8, len: usize) -> Result<Vec<u8>, String> {
        let actual_len = len.max(self.feature_report_len);
        let mut buf = vec![0u8; actual_len];
        buf[0] = report_id;
        let ok = unsafe {
            HidD_GetFeature(
                self.feature_handle,
                buf.as_mut_ptr() as *mut c_void,
                buf.len() as ULONG,
            )
        };
        if ok != 0 {
            Ok(buf)
        } else {
            let err = unsafe { GetLastError() };
            Err(format!("HidD_GetFeature failed (Win32 error {err})"))
        }
    }

    /// Non-blocking / bounded-timeout overlapped read from the Monitor HID endpoint (`UsagePage 0x000A`).
    pub fn read_monitor(&self, timeout_ms: u32) -> Result<Option<Vec<u8>>, String> {
        let Some(mh) = self.monitor_handle else {
            return Ok(None);
        };
        let mut buf = vec![0u8; self.monitor_input_len.max(8)];
        unsafe {
            let ev = CreateEventW(null_mut(), 1, 0, null());
            if ev.is_null() {
                return Err("CreateEventW failed".into());
            }
            let mut ov: OVERLAPPED = zeroed();
            ov.h_event = ev;
            let mut bytes_read: DWORD = 0;

            let res = ReadFile(
                mh,
                buf.as_mut_ptr() as *mut c_void,
                buf.len() as DWORD,
                &mut bytes_read,
                &mut ov,
            );
            if res != 0 {
                CloseHandle(ev);
                buf.truncate(bytes_read as usize);
                return Ok(Some(buf));
            }

            let err = GetLastError();
            if err != ERROR_IO_PENDING {
                CloseHandle(ev);
                return Err(format!("Monitor ReadFile error {err}"));
            }

            let wait = WaitForSingleObject(ev, timeout_ms);
            if wait == WAIT_OBJECT_0 {
                let ok = GetOverlappedResult(mh, &mut ov, &mut bytes_read, 0);
                CloseHandle(ev);
                if ok != 0 && bytes_read > 0 {
                    buf.truncate(bytes_read as usize);
                    Ok(Some(buf))
                } else {
                    Ok(None)
                }
            } else {
                CancelIo(mh);
                let mut dummy: DWORD = 0;
                GetOverlappedResult(mh, &mut ov, &mut dummy, 1);
                CloseHandle(ev);
                Ok(None)
            }
        }
    }
}
